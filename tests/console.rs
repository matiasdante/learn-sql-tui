mod common;

use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::{Harness, cells};
use learn_sql_tui::console::{Event, Outcome};

#[test]
fn statements_can_span_lines_and_results_render_as_a_table() {
    let mut h = Harness::new("multiline");
    h.run("CREATE TABLE t (id INTEGER, name TEXT);");
    let out = h.run("INSERT INTO t VALUES (1, 'Ada'),\n  (2, 'Bo');");
    assert!(out.contains("2 row(s) changed"), "{out}");
    let out = h.run("SELECT *\nFROM t\nORDER BY id;");
    assert!(out.contains("┌────┬──────┐"), "{out}");
    assert!(out.contains("│  1 │ Ada  │"), "{out}");
}

#[test]
fn a_statement_without_semicolon_waits_for_more_lines() {
    let mut h = Harness::new("pending");
    let (tx, _rx) = mpsc::channel();
    assert!(matches!(
        h.console.execute("SELECT 1", &tx).unwrap(),
        Outcome::More
    ));
    assert!(h.console.has_pending());
    assert_eq!(h.console.prompt(), "...> ");
    h.console.cancel_pending();
    assert!(!h.console.has_pending());
}

#[test]
fn triggers_are_one_statement_until_end() {
    let mut h = Harness::new("trigger");
    h.run(
        "CREATE TABLE a (x); CREATE TABLE log (x);\n\
         CREATE TRIGGER t AFTER INSERT ON a BEGIN\n  INSERT INTO log VALUES (new.x);\nEND;\n\
         INSERT INTO a VALUES (7);\nSELECT x FROM log;",
    );
}

#[test]
fn errors_are_reported_and_the_session_keeps_working() {
    let mut h = Harness::new("errors");
    let out = h.fails("SELECT * FROM nope;");
    assert!(out.contains("no such table: nope"), "{out}");
    assert!(h.run("SELECT 1 AS one;").contains("one"));
}

#[test]
fn foreign_keys_start_off_like_the_sqlite3_shell() {
    let mut h = Harness::new("fk");
    assert_eq!(
        cells(&h.run("PRAGMA foreign_keys;")),
        [["0"]],
        "default off"
    );
    h.run("PRAGMA foreign_keys = ON;");
    assert_eq!(cells(&h.run("PRAGMA foreign_keys;")), [["1"]]);
}

#[test]
fn transactions_span_lines_and_the_prompt_shows_them() {
    let mut h = Harness::new("tx");
    h.run("CREATE TABLE t (x);");
    assert_eq!(h.console.prompt(), "sql> ");
    h.run("BEGIN;");
    assert_eq!(h.console.prompt(), "sql*> ");
    h.run("INSERT INTO t VALUES (1);");
    h.run("ROLLBACK;");
    assert_eq!(h.console.prompt(), "sql> ");
    assert_eq!(cells(&h.run("SELECT count(*) AS n FROM t;")), [["0"]]);
}

#[test]
fn dot_commands_describe_the_database() {
    let mut h = Harness::new("dots");
    h.run("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT UNIQUE);");
    h.run("CREATE INDEX idx_name ON users(name);");
    h.run("CREATE VIEW v AS SELECT name FROM users;");
    assert_eq!(h.run(".tables").trim(), "users  v");
    assert_eq!(h.run(".tables us%").trim(), "users");
    assert!(h.run(".schema users").contains("CREATE TABLE users"));
    let idx = h.run(".indexes users");
    assert!(
        idx.contains("idx_name") && idx.contains("sqlite_autoindex_users_1"),
        "{idx}"
    );
    assert!(h.run(".help").contains(".import"));
}

#[test]
fn output_modes_and_headers() {
    let mut h = Harness::new("modes");
    h.run("CREATE TABLE t (a, b); INSERT INTO t VALUES (1, 'x,y');");
    h.run(".mode csv");
    assert_eq!(h.run("SELECT * FROM t;"), "a,b\n1,\"x,y\"\n");
    h.run(".headers off");
    assert_eq!(h.run("SELECT * FROM t;"), "1,\"x,y\"\n");
    h.run(".mode line");
    assert!(h.run("SELECT * FROM t;").contains("a = 1"));
    h.fails_dot(".mode xml");
}

impl Harness {
    fn fails_dot(&mut self, line: &str) {
        let (tx, _rx) = mpsc::channel();
        assert!(self.console.execute(line, &tx).is_err(), "{line}");
    }
}

#[test]
fn read_and_import_use_files_of_the_sandbox() {
    let mut h = Harness::new("files");
    let files = h.console.files_dir();
    std::fs::write(
        files.join("seed.sql"),
        "CREATE TABLE p (n); INSERT INTO p VALUES (5);",
    )
    .unwrap();
    std::fs::write(
        files.join("people.csv"),
        "name,age\nAda,36\n\"Bo, Jr\",40\n",
    )
    .unwrap();
    h.run(".read seed.sql");
    assert_eq!(cells(&h.run("SELECT n FROM p;")), [["5"]]);
    assert!(
        h.run(".import people.csv people")
            .contains("Imported 2 row(s)")
    );
    assert!(
        h.run("SELECT name FROM people WHERE age = '40';")
            .contains("Bo, Jr")
    );
    // Con la tabla ya creada se puede saltar el encabezado.
    h.run("CREATE TABLE ages (name TEXT, age INTEGER);");
    h.run(".import --skip 1 people.csv ages");
    assert_eq!(cells(&h.run("SELECT sum(age) AS s FROM ages;")), [["76"]]);
    assert!(h.raw(".read missing.sql").1.contains("cannot read"));
}

#[test]
fn nothing_can_be_written_outside_the_sandbox() {
    let mut h = Harness::new("isolation");
    let outside =
        std::env::temp_dir().join(format!("learn-sql-tui-outside-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).unwrap();
    let target = outside.join("evil.db");
    let t = target.to_string_lossy().replace('\\', "/");

    // Una ruta absoluta se interpreta dentro del sandbox: no existe esa carpeta, falla.
    h.fails(&format!("ATTACH DATABASE '{t}' AS o;"));
    h.fails(&format!("VACUUM INTO '{t}';"));
    // Subir con `..` se rechaza al resolver la ruta.
    assert!(
        h.fails("ATTACH '../../evil.db' AS o;")
            .contains("outside the sandbox")
    );
    assert!(
        h.fails("VACUUM INTO '../evil.db';")
            .contains("outside the sandbox")
    );
    // Un nombre calculado no se puede revisar de antemano: lo frena el authorizer.
    let out = h.fails(&format!("ATTACH (SELECT '{t}') AS o;"));
    assert!(out.contains("not authorized"), "{out}");
    let out = h.fails("VACUUM INTO (SELECT 'x');");
    assert!(out.contains("VACUUM INTO only accepts"), "{out}");
    // Las extensiones no se cargan.
    assert!(
        h.fails("SELECT load_extension('nothing');")
            .contains("not authorized")
    );
    // Los pragmas que mueven los archivos temporales tampoco.
    h.fails(&format!("PRAGMA temp_store_directory = '{t}';"));
    // Los dot-commands tampoco salen.
    for line in [".read ../x.sql", ".import ../x.csv t", "edit ../x.txt"] {
        let (tx, _rx) = mpsc::channel();
        let err = h.console.execute(line, &tx).err().expect(line);
        assert!(
            err.to_string().contains("outside the sandbox"),
            "{line}: {err}"
        );
    }
    assert!(
        !target.exists(),
        "nothing may be created outside the sandbox"
    );
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn attach_and_vacuum_into_work_inside_the_sandbox() {
    let mut h = Harness::new("attach");
    h.run("CREATE TABLE t (x); INSERT INTO t VALUES (1);");
    h.run("ATTACH DATABASE 'archive.db' AS archive;");
    h.run("CREATE TABLE archive.t (x); INSERT INTO archive.t SELECT * FROM t;");
    assert_eq!(
        cells(&h.run("SELECT count(*) AS n FROM archive.t;")),
        [["1"]]
    );
    assert!(h.console.files_dir().join("archive.db").exists());
    h.run("DETACH archive;");
    h.run("VACUUM INTO 'copy.db';");
    assert!(h.console.files_dir().join("copy.db").exists());
    h.run("ATTACH ':memory:' AS scratch;");
}

#[test]
fn defensive_mode_blocks_schema_tampering() {
    let mut h = Harness::new("defensive");
    h.run("CREATE TABLE t (x);");
    h.fails("PRAGMA writable_schema = ON; UPDATE sqlite_schema SET name = 'x';");
}

#[test]
fn ctrl_c_stops_an_infinite_recursive_query() {
    let mut h = Harness::new("interrupt");
    let (tx, rx) = mpsc::channel();
    let sql =
        "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT count(*) FROM c;";
    let Outcome::Running(running) = h.console.execute(sql, &tx).unwrap() else {
        panic!("the query should run in the background");
    };
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        rx.try_recv().is_err(),
        "the infinite query must still be running"
    );
    let started = Instant::now();
    running.kill();
    loop {
        match rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the query was not interrupted")
        {
            Event::Finished(code) => {
                assert_eq!(code, None, "interrupted");
                break;
            }
            Event::Output(_) => {}
        }
    }
    assert!(started.elapsed() < Duration::from_secs(5));
    // La sesión sigue sirviendo.
    assert!(h.run("SELECT 1 AS ok;").contains("ok"));
}

#[test]
fn an_infinite_row_stream_stops_at_the_row_cap() {
    let mut h = Harness::new("cap");
    let out =
        h.run("WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT x FROM c;");
    assert!(
        out.contains("showing") || out.contains("stopped after"),
        "{}",
        &out[out.len().saturating_sub(120)..]
    );
}

#[test]
fn bundled_sqlite_has_what_the_course_needs() {
    let mut h = Harness::new("features");
    h.run("CREATE TABLE a (x); CREATE TABLE b (x); INSERT INTO a VALUES (1),(2); INSERT INTO b VALUES (2),(3);");
    assert_eq!(
        cells(&h.run("SELECT a.x, b.x FROM a RIGHT JOIN b ON a.x = b.x ORDER BY b.x;")),
        [["2", "2"], ["NULL", "3"]]
    );
    assert_eq!(
        cells(
            &h.run("SELECT a.x, b.x FROM a FULL JOIN b ON a.x = b.x ORDER BY coalesce(a.x, b.x);")
        )
        .len(),
        3
    );
    h.run(
        "CREATE VIRTUAL TABLE docs USING fts5(body); INSERT INTO docs VALUES ('hello sql world');",
    );
    assert!(
        h.run("SELECT body FROM docs WHERE docs MATCH 'sql';")
            .contains("hello sql world")
    );
    assert_eq!(
        cells(&h.run("SELECT json_extract('{\"a\":[1,2]}', '$.a[1]') AS v;")),
        [["2"]]
    );
    assert_eq!(
        cells(&h.run("SELECT sum(x) OVER (ORDER BY x) AS s FROM a;")),
        [["1"], ["3"]]
    );
    assert_eq!(
        cells(&h.run("INSERT INTO a VALUES (9) RETURNING x;")),
        [["9"]]
    );
}
