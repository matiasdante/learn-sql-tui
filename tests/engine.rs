mod common;

use common::Harness;
use learn_sql_tui::content::Exercise;
use learn_sql_tui::engine::{self, Verdict};

fn exercise(body: &str) -> Exercise {
    toml::from_str(&format!("id = \"x\"\nsolution = []\n{body}")).unwrap()
}

/// Evalúa los checks sin restaurar nada, para probar varios sobre el mismo estado.
fn passes(h: &mut Harness, ex: &Exercise) -> bool {
    engine::first_failure(&h.console, ex).unwrap().is_none()
}

fn check(h: &mut Harness, check: &str) -> bool {
    let ex = exercise(&format!("checks = [{check}]"));
    passes(h, &ex)
}

#[test]
fn schema_checks_look_at_the_database_state() {
    let mut h = Harness::new("schema-checks");
    h.run(
        "CREATE TABLE dept (id INTEGER PRIMARY KEY, name TEXT);\n\
         CREATE TABLE emp (id INTEGER PRIMARY KEY, name TEXT NOT NULL, dept_id INTEGER REFERENCES dept(id) ON DELETE CASCADE, email TEXT);\n\
         CREATE UNIQUE INDEX idx_email ON emp(email);\n\
         CREATE INDEX idx_name ON emp(name);\n\
         CREATE VIEW v AS SELECT name FROM emp;\n\
         CREATE TRIGGER trg AFTER INSERT ON emp BEGIN SELECT 1; END;\n\
         INSERT INTO dept VALUES (1, 'IT'), (2, 'Ops');",
    );
    let yes = [
        r#"{ type = "table_exists", name = "EMP" }"#,
        r#"{ type = "table_missing", name = "nope" }"#,
        r#"{ type = "column", table = "emp", name = "name", data_type = "text", notnull = true, pk = false }"#,
        r#"{ type = "column", table = "emp", name = "id", pk = true }"#,
        r#"{ type = "foreign_key", table = "emp", from = "dept_id", ref_table = "dept", to = "id", on_delete = "cascade" }"#,
        r#"{ type = "index_exists", name = "idx_email", table = "emp", unique = true }"#,
        r#"{ type = "index_exists", name = "idx_name", unique = false }"#,
        r#"{ type = "view_exists", name = "v" }"#,
        r#"{ type = "trigger_exists", name = "trg", table = "emp" }"#,
        r#"{ type = "row_count", table = "dept", count = 2 }"#,
        r#"{ type = "row_count", table = "dept", count = 1, filter = "name = 'IT'" }"#,
        r#"{ type = "query_equals", sql = "SELECT id, name FROM dept ORDER BY id", rows = [["1", "IT"], ["2", "Ops"]] }"#,
        r#"{ type = "pragma_equals", pragma = "foreign_keys", value = "0" }"#,
        r#"{ type = "autocommit" }"#,
    ];
    for c in yes {
        assert!(check(&mut h, c), "should pass: {c}");
    }
    let no = [
        r#"{ type = "table_exists", name = "nope" }"#,
        r#"{ type = "table_missing", name = "emp" }"#,
        r#"{ type = "column", table = "emp", name = "name", notnull = false }"#,
        r#"{ type = "column", table = "emp", name = "name", data_type = "integer" }"#,
        r#"{ type = "foreign_key", table = "emp", from = "dept_id", ref_table = "dept", on_delete = "set null" }"#,
        r#"{ type = "index_exists", name = "idx_name", unique = true }"#,
        r#"{ type = "view_exists", name = "emp" }"#,
        r#"{ type = "trigger_exists", name = "trg", table = "dept" }"#,
        r#"{ type = "row_count", table = "dept", count = 3 }"#,
        r#"{ type = "query_equals", sql = "SELECT id FROM dept", rows = [["1"]] }"#,
        r#"{ type = "pragma_equals", pragma = "foreign_keys", value = "1" }"#,
        r#"{ type = "ran", any = [".tables"] }"#,
    ];
    for c in no {
        assert!(!check(&mut h, c), "should fail: {c}");
    }
}

#[test]
fn session_state_is_checked_on_the_live_connection() {
    let mut h = Harness::new("session-checks");
    assert!(!check(
        &mut h,
        r#"{ type = "pragma_equals", pragma = "foreign_keys", value = "1" }"#
    ));
    h.run("PRAGMA foreign_keys = ON;");
    assert!(check(
        &mut h,
        r#"{ type = "pragma_equals", pragma = "foreign_keys", value = "1" }"#
    ));
    h.run("BEGIN;");
    assert!(
        !check(&mut h, r#"{ type = "autocommit" }"#),
        "open transaction"
    );
    h.run("ROLLBACK;");
    h.run(".tables");
    assert!(check(&mut h, r#"{ type = "ran", any = [".tables"] }"#));
}

#[test]
fn last_result_same_as_accepts_any_correct_query() {
    let mut h = Harness::new("last-result");
    h.run("CREATE TABLE t (n INTEGER, s TEXT); INSERT INTO t VALUES (3,'c'),(1,'a'),(2,'b');");
    let unordered = r#"{ type = "last_result_same_as", sql = "SELECT s FROM t WHERE n > 1" }"#;
    let ordered =
        r#"{ type = "last_result_same_as", sql = "SELECT s FROM t ORDER BY n", ordered = true }"#;
    assert!(!check(&mut h, unordered), "no result yet");
    // Otra forma de escribir lo mismo, con otro alias y otro orden: sirve.
    h.run("SELECT s AS letter FROM t WHERE n >= 2 ORDER BY s DESC;");
    assert!(check(&mut h, unordered));
    // Cuando el orden importa, solo vale el orden pedido.
    h.run("SELECT s FROM t ORDER BY n DESC;");
    assert!(!check(&mut h, ordered));
    h.run("SELECT s FROM t ORDER BY n;");
    assert!(check(&mut h, ordered));
    // Un entero y un real iguales cuentan como el mismo valor.
    h.run("SELECT n * 1.0 FROM t ORDER BY n;");
    assert!(check(
        &mut h,
        r#"{ type = "last_result_same_as", sql = "SELECT n FROM t ORDER BY n", ordered = true }"#
    ));
}

#[test]
fn a_failed_verification_restores_the_start_of_the_exercise() {
    let mut h = Harness::new("restore");
    let ex = exercise(
        r#"setup = [
  { do = "sql", script = "CREATE TABLE t (x); INSERT INTO t VALUES (1);" },
  { do = "write", path = "data.csv", content = "a,b\n1,2\n" },
]
checks = [{ type = "row_count", table = "t", count = 5 }]"#,
    );
    engine::start(&mut h.console, &ex).unwrap();
    h.run("INSERT INTO t VALUES (2);");
    h.run("CREATE TABLE junk (y);");
    std::fs::write(h.console.files_dir().join("extra.txt"), "x").unwrap();
    assert!(matches!(
        engine::verify(&mut h.console, &ex).unwrap(),
        Verdict::Fail(_)
    ));
    assert!(
        check(&mut h, r#"{ type = "row_count", table = "t", count = 1 }"#),
        "row from the user is gone"
    );
    assert!(check(
        &mut h,
        r#"{ type = "table_missing", name = "junk" }"#
    ));
    assert!(h.console.files_dir().join("data.csv").exists());
    assert!(!h.console.files_dir().join("extra.txt").exists());
}

#[test]
fn starting_an_exercise_cleans_the_previous_one_but_keeps_free_mode() {
    use learn_sql_tui::sandbox::Workspace;
    let mut h = Harness::new("free-persists");
    h.console.use_workspace(Workspace::Free);
    h.run("CREATE TABLE mine (x); INSERT INTO mine VALUES (42);");
    h.console.close();

    let ex = exercise(
        r#"setup = [{ do = "sql", script = "CREATE TABLE only_here (x);" }]
checks = [{ type = "table_exists", name = "only_here" }]"#,
    );
    engine::start(&mut h.console, &ex).unwrap();
    assert!(check(
        &mut h,
        r#"{ type = "table_missing", name = "mine" }"#
    ));

    h.console.use_workspace(Workspace::Free);
    assert!(check(
        &mut h,
        r#"{ type = "row_count", table = "mine", count = 1 }"#
    ));
    assert!(check(
        &mut h,
        r#"{ type = "table_missing", name = "only_here" }"#
    ));
    // Borrar el modo libre sí lo vacía.
    h.console.reset_free().unwrap();
    assert!(check(
        &mut h,
        r#"{ type = "table_missing", name = "mine" }"#
    ));
}
