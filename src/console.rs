//! Consola SQL: sentencias de varias líneas que terminan en `;`, dot-commands al estilo
//! sqlite3 y ejecución en un hilo aparte (Ctrl+C corta la consulta con `interrupt_handle`).
//!
//! Aislamiento: la consola nunca toca nada fuera del sandbox. Las rutas de `.read`, `.import`,
//! `edit`, `ATTACH` y `VACUUM INTO` se resuelven dentro de la carpeta de archivos del
//! sandbox; además un authorizer de SQLite rechaza cualquier `ATTACH` que apunte afuera,
//! `load_extension` y los pragmas que cambian dónde SQLite escribe.

use std::ffi::CString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::config::DbConfig;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::{Connection, ErrorCode, InterruptHandle, OpenFlags};

use crate::result::{Mode, ResultSet, format_result, read_rows};
use crate::sandbox::{Sandbox, Workspace};

/// Lo que el hilo de fondo le informa a la interfaz.
pub enum Event {
    Output(String),
    /// `Some(0)` terminó bien, `Some(1)` terminó con un error ya informado, `None` se interrumpió.
    Finished(Option<i32>),
}

pub enum Outcome {
    /// Terminó en el acto.
    Done,
    /// La sentencia sigue en la línea siguiente (falta el `;`).
    More,
    Clear,
    Edit(PathBuf),
    /// Hay una sentencia corriendo en segundo plano.
    Running(Running),
}

/// Mango de una ejecución en curso.
pub struct Running {
    cancel: Arc<AtomicBool>,
    interrupt: Arc<InterruptHandle>,
}

impl Running {
    /// Corta la consulta (por ejemplo un CTE recursivo infinito) y las sentencias que faltan.
    pub fn kill(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.interrupt.interrupt();
    }
}

/// Páginas máximas de la base de práctica (128 MB con páginas de 4 KB).
const MAX_PAGES: u32 = 32_768;
const MAX_SCRIPT_BYTES: u64 = 1 << 20;
const MAX_CSV_BYTES: u64 = 5 << 20;

const HELP: &str = "\
SQL statements end with ';' and can span several lines.
.tables [pattern]        list tables and views
.schema [name]           show the CREATE statements
.indexes [table]         list indexes
.mode box|csv|line       how results are shown
.headers on|off          column names in csv mode
.read FILE               run the SQL statements of a file
.import [--skip N] FILE TABLE
                         load a CSV file into a table (created from the header row if missing)
edit FILE                open a file in the built-in editor
.clear                   clear the screen
.help                    this text
Ctrl+C stops a running query.";

// ---------- sesión ----------

struct Session {
    conn: Option<Connection>,
    interrupt: Option<Arc<InterruptHandle>>,
    db_path: PathBuf,
    /// Carpeta de archivos del sandbox: de ahí no se sale.
    files: PathBuf,
    mode: Mode,
    headers: bool,
    /// Resultado de la última consulta que devolvió filas.
    last: Option<ResultSet>,
}

impl Session {
    fn close(&mut self) {
        self.interrupt = None;
        self.conn = None;
    }

    fn reset_state(&mut self) {
        self.close();
        self.mode = Mode::Box;
        self.headers = true;
        self.last = None;
    }

    fn ensure_open(&mut self) -> Result<()> {
        if self.conn.is_none() {
            if let Some(parent) = self.db_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::create_dir_all(&self.files)?;
            let conn = open_connection(&self.db_path, &self.files)?;
            self.interrupt = Some(Arc::new(conn.get_interrupt_handle()));
            self.conn = Some(conn);
        }
        Ok(())
    }

    fn conn(&self) -> Result<&Connection> {
        self.conn.as_ref().context("the database is not open")
    }
}

/// Abre la base con las protecciones del sandbox.
fn open_connection(db_path: &Path, files: &Path) -> Result<Connection> {
    // Sin SQLITE_OPEN_URI: un nombre `file:...` no se interpreta como URI.
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let mut conn = Connection::open_with_flags(db_path, flags)
        .with_context(|| format!("could not open {}", db_path.display()))?;
    // Evita corromper el esquema por accidente (writable_schema, tablas sombra).
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    // El SQLite embebido arranca con las claves foráneas activadas; el de `sqlite3` no.
    // Las lecciones enseñan `PRAGMA foreign_keys = ON`, así que se parte como el original.
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, false)?;
    conn.query_row(&format!("PRAGMA max_page_count = {MAX_PAGES}"), [], |_| {
        Ok(())
    })?;
    let root = files.to_path_buf();
    conn.authorizer(Some(move |ctx: AuthContext<'_>| match ctx.action {
        AuthAction::Attach { filename } if !attach_allowed(filename, &root) => Authorization::Deny,
        // SQLite no informa el nombre cuando el ATTACH no es un literal (`ATTACH (SELECT ...)`):
        // no se puede revisar, así que se rechaza.
        AuthAction::Unknown { code, .. } if code == rusqlite::ffi::SQLITE_ATTACH => {
            Authorization::Deny
        }
        AuthAction::Function { function_name }
            if function_name.eq_ignore_ascii_case("load_extension") =>
        {
            Authorization::Deny
        }
        AuthAction::Pragma { pragma_name, .. }
            if ["temp_store_directory", "data_store_directory"]
                .iter()
                .any(|p| pragma_name.eq_ignore_ascii_case(p)) =>
        {
            Authorization::Deny
        }
        _ => Authorization::Allow,
    }))?;
    Ok(conn)
}

/// `ATTACH` solo vale para memoria, bases temporales o archivos dentro de la carpeta del sandbox.
fn attach_allowed(name: &str, root: &Path) -> bool {
    if name.is_empty() || name == ":memory:" {
        return true;
    }
    let path = Path::new(name);
    path.is_absolute()
        && path.starts_with(root)
        && !path.components().any(|c| matches!(c, Component::ParentDir))
}

// ---------- rutas ----------

const OUTSIDE: &str = "path is outside the sandbox";

/// Resuelve una ruta del usuario dentro de `root`. Una `/` inicial apunta a la raíz del sandbox.
pub fn resolve_in(root: &Path, raw: &str) -> Result<PathBuf> {
    let rest = raw.strip_prefix('/').unwrap_or(raw);
    if rest.trim().is_empty() {
        bail!("a file name is needed");
    }
    let mut out = root.to_path_buf();
    for comp in Path::new(rest).components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if out == root {
                    bail!(OUTSIDE);
                }
                out.pop();
            }
            Component::Normal(name) => out.push(name),
            // Prefijos de unidad (C:\) o raíces absolutas del sistema.
            _ => bail!(OUTSIDE),
        }
    }
    // Un symlink que existe no puede sacarnos del sandbox.
    let mut probe = out.as_path();
    while !probe.exists() {
        match probe.parent() {
            Some(parent) => probe = parent,
            None => break,
        }
    }
    if let (Ok(real), Ok(real_root)) = (probe.canonicalize(), root.canonicalize())
        && !real.starts_with(real_root)
    {
        bail!(OUTSIDE);
    }
    if out == root {
        bail!("a file name is needed");
    }
    Ok(out)
}

// ---------- análisis de sentencias ----------

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokKind {
    Word,
    Str,
    Other,
}

struct Tok {
    kind: TokKind,
    start: usize,
    end: usize,
}

/// Tokenizador mínimo: palabras, literales entre comillas simples y el resto. Salta comentarios.
fn tokens(sql: &str) -> Vec<Tok> {
    let bytes = sql.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if sql[i..].starts_with("--") {
            i = sql[i..].find('\n').map_or(bytes.len(), |n| i + n);
        } else if sql[i..].starts_with("/*") {
            i = sql[i + 2..]
                .find("*/")
                .map_or(bytes.len(), |n| i + 2 + n + 2);
        } else if c == b'\'' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    if bytes.get(i + 1) == Some(&b'\'') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(Tok {
                kind: TokKind::Str,
                start,
                end: i,
            });
        } else if c == b'"' || c == b'`' || c == b'[' {
            let close = if c == b'[' { b']' } else { c };
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != close {
                i += 1;
            }
            i = (i + 1).min(bytes.len());
            out.push(Tok {
                kind: TokKind::Other,
                start,
                end: i,
            });
        } else if c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80 {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] >= 0x80)
            {
                i += 1;
            }
            out.push(Tok {
                kind: TokKind::Word,
                start,
                end: i,
            });
        } else {
            out.push(Tok {
                kind: TokKind::Other,
                start: i,
                end: i + 1,
            });
            i += 1;
        }
    }
    out
}

/// Primera palabra de la sentencia en mayúsculas (`SELECT`, `INSERT`...), sin comentarios.
fn first_word(sql: &str) -> String {
    tokens(sql)
        .first()
        .filter(|t| t.kind == TokKind::Word)
        .map(|t| sql[t.start..t.end].to_ascii_uppercase())
        .unwrap_or_default()
}

/// No hay SQL: solo espacios, comentarios y `;`.
fn is_blank_sql(sql: &str) -> bool {
    tokens(sql)
        .iter()
        .all(|t| t.kind == TokKind::Other && &sql[t.start..t.end] == ";")
}

/// Lo que sqlite3 considera una sentencia completa (termina en `;`, respeta triggers).
fn complete(sql: &str) -> bool {
    CString::new(sql)
        .map(|c| unsafe { rusqlite::ffi::sqlite3_complete(c.as_ptr()) } != 0)
        .unwrap_or(false)
}

/// Parte un texto en sentencias. Lo que queda sin `;` al final también cuenta como sentencia.
pub fn split_statements(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut push = |chunk: &str| {
        let chunk = chunk.trim();
        if !is_blank_sql(chunk) {
            out.push(chunk.to_string());
        }
    };
    for (i, ch) in text.char_indices() {
        if ch == ';' && complete(&text[start..=i]) {
            push(&text[start..=i]);
            start = i + 1;
        }
    }
    push(&text[start..]);
    out
}

fn unquote(literal: &str) -> String {
    literal.trim_matches('\'').replace("''", "'")
}

/// Reemplaza el nombre de archivo literal de `ATTACH` y `VACUUM INTO` por su ruta dentro del
/// sandbox. Un nombre que no es un literal pasa tal cual y lo rechaza el authorizer.
fn rewrite_paths(sql: &str, files: &Path) -> Result<String> {
    let toks = tokens(sql);
    let word = |i: usize| {
        toks.get(i)
            .filter(|t| t.kind == TokKind::Word)
            .map(|t| sql[t.start..t.end].to_ascii_uppercase())
            .unwrap_or_default()
    };
    let literal_at = |i: usize| toks.get(i).filter(|t| t.kind == TokKind::Str);
    let replace = |tok: &Tok| -> Result<String> {
        let name = unquote(&sql[tok.start..tok.end]);
        if name.is_empty() || name == ":memory:" {
            return Ok(sql.to_string());
        }
        let path = resolve_in(files, &name)?;
        let quoted = path.to_string_lossy().replace('\'', "''");
        Ok(format!(
            "{}'{quoted}'{}",
            &sql[..tok.start],
            &sql[tok.end..]
        ))
    };
    match word(0).as_str() {
        "ATTACH" => {
            let at = if word(1) == "DATABASE" { 2 } else { 1 };
            match literal_at(at) {
                Some(tok) => replace(tok),
                None => Ok(sql.to_string()),
            }
        }
        "VACUUM" => match (1..toks.len()).find(|&i| word(i) == "INTO") {
            Some(into) => match literal_at(into + 1) {
                Some(tok) => replace(tok),
                None => bail!("VACUUM INTO only accepts a quoted file name"),
            },
            None => Ok(sql.to_string()),
        },
        _ => Ok(sql.to_string()),
    }
}

/// Parte los argumentos de un dot-command; las comillas dobles o simples agrupan palabras.
fn split_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;
    for ch in text.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '\'' | '"') => {
                quote = Some(ch);
                has_token = true;
            }
            (None, c) if c.is_whitespace() => {
                if has_token {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            (None, c) => {
                current.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        args.push(current);
    }
    args
}

// ---------- consola ----------

enum Job {
    Sql(String),
    Read(PathBuf),
    Import {
        path: PathBuf,
        table: String,
        skip: usize,
    },
}

pub struct Console {
    sandbox: Sandbox,
    workspace: Workspace,
    session: Arc<Mutex<Session>>,
    /// Sentencia a medias (las líneas escritas hasta ahora).
    pending: String,
    /// Sentencias y dot-commands ejecutados desde el último reset (para checks `ran`).
    history: Vec<String>,
}

impl Console {
    pub fn new(sandbox: Sandbox) -> Self {
        let workspace = Workspace::Exercise;
        let session = Session {
            conn: None,
            interrupt: None,
            db_path: sandbox.db_path(workspace),
            files: sandbox.files_dir(workspace),
            mode: Mode::Box,
            headers: true,
            last: None,
        };
        Self {
            sandbox,
            workspace,
            session: Arc::new(Mutex::new(session)),
            pending: String::new(),
            history: Vec::new(),
        }
    }

    fn session(&self) -> MutexGuard<'_, Session> {
        self.session.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    pub fn workspace(&self) -> Workspace {
        self.workspace
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Descarta la sentencia a medias (Ctrl+C).
    pub fn cancel_pending(&mut self) {
        self.pending.clear();
    }

    /// `sql> `, `...> ` en una sentencia a medias, `sql*> ` con una transacción abierta.
    pub fn prompt(&self) -> String {
        if !self.pending.is_empty() {
            return "...> ".to_string();
        }
        let in_transaction = self
            .session
            .try_lock()
            .ok()
            .and_then(|s| s.conn.as_ref().map(|c| !c.is_autocommit()))
            .unwrap_or(false);
        if in_transaction { "sql*> " } else { "sql> " }.to_string()
    }

    /// Pasa a trabajar con la base del ejercicio o con la del modo libre.
    pub fn use_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
        let mut s = self.session();
        s.db_path = self.sandbox.db_path(workspace);
        s.files = self.sandbox.files_dir(workspace);
        s.reset_state();
        drop(s);
        self.pending.clear();
        self.history.clear();
    }

    /// Deja los ejercicios en blanco (base y archivos vacíos).
    pub fn reset_exercise(&mut self) -> Result<()> {
        self.use_workspace(Workspace::Exercise);
        self.sandbox.reset_exercise()
    }

    /// Borra la base y los archivos del modo libre y vuelve a empezar ahí.
    pub fn reset_free(&mut self) -> Result<()> {
        self.use_workspace(Workspace::Free);
        self.sandbox.reset_free()
    }

    /// Cierra la conexión (descarta una transacción abierta). Hay que hacerlo antes de copiar el archivo.
    pub fn close(&mut self) {
        self.session().close();
    }

    /// Guarda el estado del ejercicio. La conexión se cierra antes de copiar el archivo.
    pub fn snapshot(&mut self) -> Result<()> {
        self.close();
        self.sandbox.snapshot()
    }

    /// Vuelve la base y los archivos al snapshot del inicio del ejercicio.
    pub fn restore_snapshot(&mut self) -> Result<()> {
        {
            let mut s = self.session();
            s.close();
            s.last = None;
        }
        self.pending.clear();
        self.sandbox.restore()
    }

    /// Carpeta de archivos del espacio de trabajo actual.
    pub fn files_dir(&self) -> PathBuf {
        self.sandbox.files_dir(self.workspace)
    }

    /// Da acceso a la conexión viva (con sus pragmas y su transacción), al último resultado y
    /// al historial: así miran los checks.
    pub fn inspect<R>(
        &self,
        f: impl FnOnce(&Connection, Option<&ResultSet>, &[String]) -> R,
    ) -> Result<R> {
        let mut s = self.session();
        s.ensure_open()?;
        let conn = s.conn()?;
        Ok(f(conn, s.last.as_ref(), &self.history))
    }

    /// Ejecuta una línea escrita por el usuario.
    pub fn execute(&mut self, line: &str, tx: &Sender<Event>) -> Result<Outcome> {
        let trimmed = line.trim();
        if self.pending.is_empty() {
            if trimmed.is_empty() {
                return Ok(Outcome::Done);
            }
            if trimmed.starts_with('.') || ["edit", "clear"].contains(&first_word_lower(trimmed)) {
                self.history.push(trimmed.to_string());
                return self.dot_command(trimmed, tx);
            }
        }
        if !self.pending.is_empty() {
            self.pending.push('\n');
        }
        self.pending.push_str(line);
        if is_blank_sql(&self.pending) {
            // Solo comentarios: no hay nada que ejecutar.
            self.pending.clear();
            return Ok(Outcome::Done);
        }
        if !complete(&self.pending) {
            return Ok(Outcome::More);
        }
        let sql = std::mem::take(&mut self.pending);
        self.history.push(sql.trim().to_string());
        self.spawn(Job::Sql(sql), tx)
    }

    fn say(tx: &Sender<Event>, text: impl Into<String>) {
        let mut text = text.into();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let _ = tx.send(Event::Output(text));
    }

    fn dot_command(&mut self, line: &str, tx: &Sender<Event>) -> Result<Outcome> {
        let args = split_args(line);
        let Some((cmd, args)) = args.split_first() else {
            return Ok(Outcome::Done);
        };
        let cmd = cmd.to_ascii_lowercase();
        match cmd.as_str() {
            ".help" => Self::say(tx, HELP),
            ".clear" | "clear" => return Ok(Outcome::Clear),
            "edit" => {
                let name = args.first().context("usage: edit FILE")?;
                let path = resolve_in(&self.files_dir(), name)?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                return Ok(Outcome::Edit(path));
            }
            ".tables" => {
                let names = self.query_strings(
                    "SELECT name FROM sqlite_schema WHERE type IN ('table','view') \
                     AND name NOT LIKE 'sqlite_%' AND name LIKE ?1 ORDER BY name",
                    args.first().map_or("%", String::as_str),
                )?;
                if !names.is_empty() {
                    Self::say(tx, names.join("  "));
                }
            }
            ".schema" => {
                let filter = args.first().map_or("%", String::as_str);
                let sqls = self.query_strings(
                    "SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL \
                     AND name NOT LIKE 'sqlite_%' AND (name LIKE ?1 OR tbl_name LIKE ?1) \
                     ORDER BY rowid",
                    filter,
                )?;
                for sql in sqls {
                    Self::say(tx, format!("{sql};"));
                }
            }
            ".indexes" | ".indices" => {
                let names = self.query_strings(
                    "SELECT name FROM sqlite_schema WHERE type = 'index' \
                     AND tbl_name LIKE ?1 ORDER BY name",
                    args.first().map_or("%", String::as_str),
                )?;
                if !names.is_empty() {
                    Self::say(tx, names.join("  "));
                }
            }
            ".mode" => match args.first() {
                None => {
                    let mode = self.session().mode;
                    Self::say(tx, format!("mode: {mode:?}").to_lowercase());
                }
                Some(name) => {
                    let mode = Mode::parse(name).ok_or_else(|| anyhow!("modes: box, csv, line"))?;
                    self.session().mode = mode;
                }
            },
            ".headers" | ".header" => match args.first().map(|a| a.to_ascii_lowercase()) {
                Some(v) if v == "on" => self.session().headers = true,
                Some(v) if v == "off" => self.session().headers = false,
                _ => bail!("usage: .headers on|off"),
            },
            ".read" => {
                let name = args.first().context("usage: .read FILE")?;
                let path = resolve_in(&self.files_dir(), name)?;
                return self.spawn(Job::Read(path), tx);
            }
            ".import" => {
                let mut skip = 0;
                let mut rest: Vec<&String> = Vec::new();
                let mut it = args.iter();
                while let Some(arg) = it.next() {
                    match arg.as_str() {
                        "--csv" => {}
                        "--skip" => {
                            skip = it
                                .next()
                                .and_then(|n| n.parse().ok())
                                .context("--skip needs a number")?;
                        }
                        _ => rest.push(arg),
                    }
                }
                let [file, table] = rest.as_slice() else {
                    bail!("usage: .import [--skip N] FILE TABLE");
                };
                let path = resolve_in(&self.files_dir(), file)?;
                return self.spawn(
                    Job::Import {
                        path,
                        table: (*table).clone(),
                        skip,
                    },
                    tx,
                );
            }
            other => bail!("unknown command '{other}'. Type .help"),
        }
        Ok(Outcome::Done)
    }

    /// Una consulta rápida de una columna de texto, con un parámetro (para `.tables` y compañía).
    fn query_strings(&self, sql: &str, param: &str) -> Result<Vec<String>> {
        let mut s = self.session();
        s.ensure_open()?;
        let conn = s.conn()?;
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([param], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    fn spawn(&mut self, job: Job, tx: &Sender<Event>) -> Result<Outcome> {
        let interrupt = {
            let mut s = self.session();
            s.ensure_open()?;
            s.interrupt.clone().context("the database is not open")?
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (session, tx, flag) = (Arc::clone(&self.session), tx.clone(), Arc::clone(&cancel));
        thread::spawn(move || {
            let mut s = session.lock().unwrap_or_else(|p| p.into_inner());
            let code = run_job(&mut s, job, &flag, &tx);
            let _ = tx.send(Event::Finished(code));
        });
        Ok(Outcome::Running(Running { cancel, interrupt }))
    }
}

fn first_word_lower(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

// ---------- ejecución (hilo de fondo) ----------

fn run_job(s: &mut Session, job: Job, cancel: &AtomicBool, tx: &Sender<Event>) -> Option<i32> {
    let result = match job {
        Job::Sql(sql) => run_script(s, &sql, cancel, tx),
        Job::Read(path) => match read_limited(&path, MAX_SCRIPT_BYTES) {
            Ok(text) => run_script(s, &text, cancel, tx),
            Err(e) => Err(e),
        },
        Job::Import { path, table, skip } => import_csv(s, &path, &table, skip, tx),
    };
    match result {
        Ok(()) => Some(0),
        Err(e) if is_interrupt(&e) || cancel.load(Ordering::SeqCst) => None,
        Err(e) => {
            Console::say(tx, friendly_error(&e));
            Some(1)
        }
    }
}

fn is_interrupt(e: &anyhow::Error) -> bool {
    e.downcast_ref::<rusqlite::Error>().is_some_and(|e| {
        matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::OperationInterrupted)
    })
}

fn friendly_error(e: &anyhow::Error) -> String {
    let text = e.to_string();
    if text.contains("not authorized") {
        return format!(
            "Error: {text} (the sandbox only allows files inside its own folder, and no extensions)"
        );
    }
    if e.downcast_ref::<rusqlite::Error>().is_some() {
        format!("Error: {text}")
    } else {
        format!("error: {e:#}")
    }
}

fn read_limited(path: &Path, max: u64) -> Result<String> {
    let meta = fs::metadata(path).with_context(|| format!("cannot read {}", file_name(path)))?;
    if meta.len() > max {
        bail!("{} is too big", file_name(path));
    }
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", file_name(path)))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn run_script(s: &mut Session, text: &str, cancel: &AtomicBool, tx: &Sender<Event>) -> Result<()> {
    for sql in split_statements(text) {
        if cancel.load(Ordering::SeqCst) {
            bail!(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERRUPT),
                None
            ));
        }
        run_statement(s, &sql, tx)?;
    }
    Ok(())
}

enum Done {
    Rows(ResultSet),
    Changed(Option<u64>),
}

fn run_statement(s: &mut Session, sql: &str, tx: &Sender<Event>) -> Result<()> {
    let sql = rewrite_paths(sql, &s.files)?;
    let done = {
        let conn = s.conn()?;
        let mut stmt = conn.prepare(&sql)?;
        if stmt.column_count() == 0 {
            stmt.execute([])?;
            let dml =
                ["INSERT", "UPDATE", "DELETE", "REPLACE"].contains(&first_word(&sql).as_str());
            Done::Changed(dml.then(|| conn.changes()))
        } else {
            let result = read_rows(&mut stmt)?;
            Done::Rows(result)
        }
    };
    match done {
        Done::Rows(result) => {
            let text = format_result(&result, s.mode, s.headers);
            s.last = Some(result);
            if s.last.as_ref().is_some_and(|r| r.rows.is_empty()) {
                Console::say(tx, "(no rows)");
            } else {
                Console::say(tx, text);
            }
        }
        Done::Changed(Some(n)) => Console::say(tx, format!("OK, {n} row(s) changed")),
        Done::Changed(None) => Console::say(tx, "OK"),
    }
    Ok(())
}

// ---------- .import ----------

/// CSV según RFC 4180: comillas dobles, `""` dentro de un campo, saltos de línea entre comillas.
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                '"' => in_quotes = false,
                c => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => in_quotes = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            c => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn import_csv(
    s: &mut Session,
    path: &Path,
    table: &str,
    skip: usize,
    tx: &Sender<Event>,
) -> Result<()> {
    let mut rows = parse_csv(&read_limited(path, MAX_CSV_BYTES)?);
    if rows.is_empty() {
        bail!("{} is empty", file_name(path));
    }
    let conn = s.conn()?;
    let exists: bool = conn.query_row(
        "SELECT count(*) > 0 FROM sqlite_schema WHERE type = 'table' AND name = ?1 COLLATE NOCASE",
        [table],
        |r| r.get(0),
    )?;
    let mut skip = skip;
    // Sin tabla, la primera fila da los nombres de las columnas (todas TEXT), como sqlite3.
    if !exists {
        let header = rows.remove(0);
        let columns: Vec<String> = header
            .iter()
            .map(|c| format!("{} TEXT", quote_ident(c)))
            .collect();
        conn.execute_batch(&format!(
            "CREATE TABLE {} ({});",
            quote_ident(table),
            columns.join(", ")
        ))?;
        skip = skip.saturating_sub(1);
    }
    let rows = &rows[skip.min(rows.len())..];
    let width = conn.query_row(
        &format!(
            "SELECT count(*) FROM pragma_table_info({})",
            quote_literal(table)
        ),
        [],
        |r| r.get::<_, i64>(0).map(|n| n as usize),
    )?;
    let placeholders = vec!["?"; width].join(", ");
    conn.execute_batch("SAVEPOINT import_csv")?;
    let result = (|| -> Result<()> {
        let mut insert = conn.prepare(&format!(
            "INSERT INTO {} VALUES ({placeholders})",
            quote_ident(table)
        ))?;
        for (n, row) in rows.iter().enumerate() {
            if row.len() != width {
                bail!(
                    "row {} has {} values but the table has {width} columns",
                    n + 1 + skip,
                    row.len()
                );
            }
            insert.execute(rusqlite::params_from_iter(row.iter()))?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => conn.execute_batch("RELEASE import_csv")?,
        Err(e) => {
            conn.execute_batch("ROLLBACK TO import_csv; RELEASE import_csv")?;
            return Err(e);
        }
    }
    Console::say(tx, format!("Imported {} row(s) into {table}", rows.len()));
    Ok(())
}

fn quote_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

// ---------- uso sin interfaz (tests y motor) ----------

/// Escribe `input` en la consola línea por línea, como si el usuario lo tipeara, y espera.
/// Devuelve el código final y toda la salida. Corta en la primera sentencia con error.
pub fn run_blocking(console: &mut Console, input: &str) -> Result<(Option<i32>, String)> {
    let (tx, rx) = mpsc::channel();
    let mut output = String::new();
    let mut code = Some(0);
    for line in input.lines() {
        match console.execute(line, &tx)? {
            Outcome::Running(_) => loop {
                match rx.recv_timeout(Duration::from_secs(60)) {
                    Ok(Event::Output(text)) => output.push_str(&text),
                    Ok(Event::Finished(c)) => {
                        code = c;
                        break;
                    }
                    Err(_) => bail!("timeout running `{line}`"),
                }
            },
            Outcome::Edit(_) => bail!("`{line}`: the editor cannot be opened here"),
            Outcome::Done | Outcome::More | Outcome::Clear => {}
        }
        if code != Some(0) {
            break;
        }
    }
    for event in rx.try_iter() {
        if let Event::Output(text) = event {
            output.push_str(&text);
        }
    }
    if code == Some(0) && console.has_pending() {
        console.cancel_pending();
        bail!("incomplete statement (missing ';'?): {input}");
    }
    Ok((code, output))
}
