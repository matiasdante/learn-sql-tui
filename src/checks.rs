//! Checks declarativos. Miran el estado de la base (el esquema, las filas, los pragmas de la
//! sesión) y no lo que el usuario tipeó; la excepción es `ran`, para los dot-commands.

use std::path::PathBuf;

use rusqlite::Connection;
use serde::Deserialize;

use crate::i18n::Text;
use crate::result::{ResultSet, canon, read_rows};

#[derive(Debug, Clone, Deserialize)]
pub struct Check {
    #[serde(flatten)]
    pub kind: CheckKind,
    /// Qué explicarle al usuario si este check falla (se completa al cargar el contenido).
    #[serde(skip)]
    pub fail: Text,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckKind {
    TableExists {
        name: String,
    },
    TableMissing {
        name: String,
    },
    /// Una columna: tipo declarado, `NOT NULL` y clave primaria (cada uno solo si se indica).
    Column {
        table: String,
        name: String,
        #[serde(default)]
        data_type: Option<String>,
        #[serde(default)]
        notnull: Option<bool>,
        #[serde(default)]
        pk: Option<bool>,
    },
    /// Clave foránea `table.from -> ref_table(to)`, con `ON DELETE` opcional.
    ForeignKey {
        table: String,
        from: String,
        ref_table: String,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        on_delete: Option<String>,
    },
    IndexExists {
        name: String,
        #[serde(default)]
        table: Option<String>,
        #[serde(default)]
        unique: Option<bool>,
    },
    ViewExists {
        name: String,
    },
    TriggerExists {
        name: String,
        #[serde(default)]
        table: Option<String>,
    },
    /// Cantidad de filas de una tabla, opcionalmente filtradas con `filter` (el WHERE).
    RowCount {
        table: String,
        count: i64,
        #[serde(default)]
        filter: Option<String>,
    },
    /// Corre un SQL y compara sus filas, en orden, con `rows` (valores como texto, `NULL` = nulo).
    QueryEquals {
        sql: String,
        rows: Vec<Vec<String>>,
    },
    /// El primer valor de `PRAGMA <pragma>` en la sesión actual.
    PragmaEquals {
        pragma: String,
        value: String,
    },
    /// Para ejercicios de solo SELECT: el último resultado del usuario tiene las mismas filas
    /// que la consulta de referencia. El orden solo cuenta con `ordered = true`.
    LastResultSameAs {
        sql: String,
        #[serde(default)]
        ordered: bool,
    },
    /// No quedó ninguna transacción abierta (se hizo COMMIT o ROLLBACK).
    Autocommit,
    FileExists {
        path: String,
    },
    FileContains {
        path: String,
        contains: String,
    },
    /// Para temas sin estado que verificar (dot-commands): se mira lo que se tipeó.
    Ran {
        any: Vec<String>,
    },
}

pub struct Ctx<'a> {
    pub conn: &'a Connection,
    pub last: Option<&'a ResultSet>,
    pub history: &'a [String],
    /// Carpeta de archivos del sandbox.
    pub files: PathBuf,
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

impl Ctx<'_> {
    fn count(&self, sql: &str, params: &[&str]) -> i64 {
        self.conn
            .query_row(sql, rusqlite::params_from_iter(params), |r| r.get(0))
            .unwrap_or(0)
    }

    fn rows(&self, sql: &str) -> Option<ResultSet> {
        let mut stmt = self.conn.prepare(sql).ok()?;
        read_rows(&mut stmt).ok()
    }

    fn object_exists(&self, kind: &str, name: &str) -> bool {
        self.count(
            "SELECT count(*) FROM sqlite_schema WHERE type = ?1 AND name = ?2 COLLATE NOCASE",
            &[kind, name],
        ) > 0
    }
}

impl Check {
    pub fn passes(&self, ctx: &Ctx) -> bool {
        match &self.kind {
            CheckKind::TableExists { name } => ctx.object_exists("table", name),
            CheckKind::TableMissing { name } => !ctx.object_exists("table", name),
            CheckKind::ViewExists { name } => ctx.object_exists("view", name),
            CheckKind::Column {
                table,
                name,
                data_type,
                notnull,
                pk,
            } => column_ok(ctx, table, name, data_type.as_deref(), *notnull, *pk),
            CheckKind::ForeignKey {
                table,
                from,
                ref_table,
                to,
                on_delete,
            } => foreign_key_ok(
                ctx,
                table,
                from,
                ref_table,
                to.as_deref(),
                on_delete.as_deref(),
            ),
            CheckKind::IndexExists {
                name,
                table,
                unique,
            } => index_ok(ctx, name, table.as_deref(), *unique),
            CheckKind::TriggerExists { name, table } => {
                ctx.count(
                    "SELECT count(*) FROM sqlite_schema WHERE type = 'trigger' \
                 AND name = ?1 COLLATE NOCASE AND (?2 = '' OR tbl_name = ?2 COLLATE NOCASE)",
                    &[name, table.as_deref().unwrap_or("")],
                ) > 0
            }
            CheckKind::RowCount {
                table,
                count,
                filter,
            } => {
                let mut sql = format!("SELECT count(*) FROM {}", quote_ident(table));
                if let Some(filter) = filter {
                    sql.push_str(&format!(" WHERE {filter}"));
                }
                ctx.conn
                    .query_row(&sql, [], |r| r.get::<_, i64>(0))
                    .is_ok_and(|n| n == *count)
            }
            CheckKind::QueryEquals { sql, rows } => ctx
                .rows(sql)
                .is_some_and(|result| result.canonical_rows() == *rows),
            CheckKind::PragmaEquals { pragma, value } => ctx
                .rows(&format!("PRAGMA {pragma}"))
                .and_then(|r| r.rows.first().and_then(|row| row.first()).map(canon))
                .is_some_and(|v| v.eq_ignore_ascii_case(value)),
            CheckKind::LastResultSameAs { sql, ordered } => match (ctx.last, ctx.rows(sql)) {
                (Some(last), Some(reference)) => last.same_rows_as(&reference, *ordered),
                _ => false,
            },
            CheckKind::Autocommit => ctx.conn.is_autocommit(),
            CheckKind::FileExists { path } => ctx.files.join(path).is_file(),
            CheckKind::FileContains { path, contains } => {
                std::fs::read_to_string(ctx.files.join(path))
                    .is_ok_and(|c| c.contains(contains.as_str()))
            }
            CheckKind::Ran { any } => ctx.history.iter().any(|line| {
                let line = line.to_lowercase();
                any.iter()
                    .any(|needle| line.contains(&needle.to_lowercase()))
            }),
        }
    }
}

fn column_ok(
    ctx: &Ctx,
    table: &str,
    name: &str,
    data_type: Option<&str>,
    notnull: Option<bool>,
    pk: Option<bool>,
) -> bool {
    let found = ctx.conn.query_row(
        "SELECT type, \"notnull\", pk FROM pragma_table_info(?1) WHERE name = ?2 COLLATE NOCASE",
        [table, name],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        },
    );
    let Ok((ty, nn, pk_pos)) = found else {
        return false;
    };
    data_type.is_none_or(|t| ty.trim().eq_ignore_ascii_case(t.trim()))
        && notnull.is_none_or(|n| (nn != 0) == n)
        && pk.is_none_or(|p| (pk_pos > 0) == p)
}

fn foreign_key_ok(
    ctx: &Ctx,
    table: &str,
    from: &str,
    ref_table: &str,
    to: Option<&str>,
    on_delete: Option<&str>,
) -> bool {
    let found = ctx.conn.query_row(
        "SELECT \"table\", \"to\", on_delete FROM pragma_foreign_key_list(?1) \
         WHERE \"from\" = ?2 COLLATE NOCASE",
        [table, from],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
            ))
        },
    );
    let Ok((target, target_col, action)) = found else {
        return false;
    };
    target.eq_ignore_ascii_case(ref_table)
        && to.is_none_or(|t| {
            target_col
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case(t))
        })
        && on_delete.is_none_or(|a| action.eq_ignore_ascii_case(a))
}

fn index_ok(ctx: &Ctx, name: &str, table: Option<&str>, unique: Option<bool>) -> bool {
    let found = ctx.conn.query_row(
        "SELECT tbl_name FROM sqlite_schema WHERE type = 'index' AND name = ?1 COLLATE NOCASE",
        [name],
        |r| r.get::<_, String>(0),
    );
    let Ok(owner) = found else {
        return false;
    };
    if table.is_some_and(|t| !owner.eq_ignore_ascii_case(t)) {
        return false;
    }
    let Some(want_unique) = unique else {
        return true;
    };
    ctx.conn
        .query_row(
            "SELECT \"unique\" FROM pragma_index_list(?1) WHERE name = ?2 COLLATE NOCASE",
            [owner.as_str(), name],
            |r| r.get::<_, i64>(0),
        )
        .is_ok_and(|u| (u != 0) == want_unique)
}
