//! Resultados de consultas: valores, comparación y formato de salida (box, csv, line).

use rusqlite::types::{Value, ValueRef};

/// Cuántas filas se muestran en pantalla; el resto se resume en una línea.
pub const DISPLAY_ROWS: usize = 200;
/// Cuántas filas se guardan como "último resultado" (una consulta infinita se corta acá).
pub const STORED_ROWS: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Box,
    Csv,
    Line,
}

impl Mode {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "box" => Some(Mode::Box),
            "csv" => Some(Mode::Csv),
            "line" => Some(Mode::Line),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    /// Hubo más filas que `STORED_ROWS` y se dejó de leer.
    pub truncated: bool,
}

impl ResultSet {
    /// Las filas como texto canónico (ver `canon`).
    pub fn canonical_rows(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|row| row.iter().map(canon).collect())
            .collect()
    }

    /// ¿Mismas filas que `other`? Ignora los nombres de columna. Sin `ordered`, el orden no cuenta.
    pub fn same_rows_as(&self, other: &ResultSet, ordered: bool) -> bool {
        if self.truncated || other.truncated {
            return false;
        }
        let (mut a, mut b) = (self.canonical_rows(), other.canonical_rows());
        if !ordered {
            a.sort();
            b.sort();
        }
        a == b
    }
}

/// Lee las filas de una sentencia ya preparada, hasta `STORED_ROWS`.
pub fn read_rows(stmt: &mut rusqlite::Statement<'_>) -> rusqlite::Result<ResultSet> {
    let mut result = ResultSet {
        columns: stmt.column_names().map(|c| c.to_string()).collect(),
        ..ResultSet::default()
    };
    let width = result.columns.len();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if result.rows.len() >= STORED_ROWS {
            result.truncated = true;
            break;
        }
        let values = (0..width)
            .map(|i| owned(row.get_ref(i)?))
            .collect::<rusqlite::Result<Vec<Value>>>()?;
        result.rows.push(values);
    }
    Ok(result)
}

fn owned(value: ValueRef<'_>) -> rusqlite::Result<Value> {
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Integer(i),
        ValueRef::Real(f) => Value::Real(f),
        ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::Blob(b.to_vec()),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Un valor como texto para comparar: 3 y 3.0 son lo mismo, NULL es `NULL`.
pub fn canon(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Integer(i) => i.to_string(),
        // Nueve decimales: 290.47 y 290.47000000000003 (el promedio de otra forma) son iguales.
        Value::Real(f) if f.is_finite() => {
            let text = format!("{f:.9}");
            let text = text.trim_end_matches('0').trim_end_matches('.');
            if text == "-0" {
                "0".to_string()
            } else {
                text.to_string()
            }
        }
        Value::Real(f) => format!("{f}"),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => format!("x'{}'", hex(b)),
    }
}

/// Un valor como se ve en la tabla: igual que `canon`, pero un real entero lleva `.0`.
fn show(value: &Value) -> String {
    match value {
        Value::Real(f) if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 => {
            format!("{f:.1}")
        }
        other => canon(other),
    }
}

fn is_number(value: &Value) -> bool {
    matches!(value, Value::Integer(_) | Value::Real(_))
}

fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ").replace('\t', "    ")
}

/// Ancho en columnas de pantalla (los ideogramas ocupan dos).
fn width(text: &str) -> usize {
    text.chars()
        .map(|c| {
            if c as u32 >= 0x1100 && is_wide(c) {
                2
            } else {
                1
            }
        })
        .sum()
}

fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x20000..=0x3FFFD)
}

fn pad(text: &str, to: usize, right: bool) -> String {
    let fill = " ".repeat(to.saturating_sub(width(text)));
    if right {
        format!("{fill}{text}")
    } else {
        format!("{text}{fill}")
    }
}

/// Texto listo para mostrar. `headers` solo cambia el modo CSV; la tabla siempre lleva títulos.
pub fn format_result(result: &ResultSet, mode: Mode, headers: bool) -> String {
    let shown = &result.rows[..result.rows.len().min(DISPLAY_ROWS)];
    let mut out = match mode {
        Mode::Box => format_box(&result.columns, shown),
        Mode::Csv => format_csv(&result.columns, shown, headers),
        Mode::Line => format_line(&result.columns, shown),
    };
    if result.truncated {
        out.push_str(&format!(
            "(stopped after {STORED_ROWS} rows; more rows were not read)\n"
        ));
    } else if result.rows.len() > shown.len() {
        out.push_str(&format!(
            "(showing the first {} of {} rows)\n",
            shown.len(),
            result.rows.len()
        ));
    }
    out
}

fn format_box(columns: &[String], rows: &[Vec<Value>]) -> String {
    let cells: Vec<Vec<String>> = rows
        .iter()
        .map(|r| r.iter().map(|v| one_line(&show(v))).collect())
        .collect();
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(i, name)| {
            cells
                .iter()
                .map(|r| width(&r[i]))
                .chain([width(name)])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let rule = |l: &str, m: &str, r: &str| {
        let parts: Vec<String> = widths.iter().map(|w| "─".repeat(w + 2)).collect();
        format!("{l}{}{r}\n", parts.join(m))
    };
    let mut out = rule("┌", "┬", "┐");
    let head: Vec<String> = columns
        .iter()
        .zip(&widths)
        .map(|(c, w)| format!(" {} ", pad(c, *w, false)))
        .collect();
    out.push_str(&format!("│{}│\n", head.join("│")));
    out.push_str(&rule("├", "┼", "┤"));
    for (row, texts) in rows.iter().zip(&cells) {
        let line: Vec<String> = texts
            .iter()
            .zip(row)
            .zip(&widths)
            .map(|((t, v), w)| format!(" {} ", pad(t, *w, is_number(v))))
            .collect();
        out.push_str(&format!("│{}│\n", line.join("│")));
    }
    out.push_str(&rule("└", "┴", "┘"));
    out
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn format_csv(columns: &[String], rows: &[Vec<Value>], headers: bool) -> String {
    let mut out = String::new();
    if headers {
        let line: Vec<String> = columns.iter().map(|c| csv_field(c)).collect();
        out.push_str(&format!("{}\n", line.join(",")));
    }
    for row in rows {
        let line: Vec<String> = row
            .iter()
            .map(|v| match v {
                Value::Null => String::new(),
                other => csv_field(&show(other)),
            })
            .collect();
        out.push_str(&format!("{}\n", line.join(",")));
    }
    out
}

fn format_line(columns: &[String], rows: &[Vec<Value>]) -> String {
    let name_width = columns.iter().map(|c| width(c)).max().unwrap_or(0);
    let mut out = String::new();
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for (name, value) in columns.iter().zip(row) {
            out.push_str(&format!(
                "{} = {}\n",
                pad(name, name_width, true),
                one_line(&show(value))
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ResultSet {
        ResultSet {
            columns: vec!["name".into(), "age".into()],
            rows: vec![
                vec![Value::Text("Ada".into()), Value::Integer(36)],
                vec![Value::Text("Bo, Jr".into()), Value::Null],
            ],
            truncated: false,
        }
    }

    #[test]
    fn canonical_values_ignore_int_real_differences() {
        assert_eq!(canon(&Value::Real(3.0)), canon(&Value::Integer(3)));
        assert_eq!(canon(&Value::Real(2.5)), "2.5");
        assert_eq!(canon(&Value::Real(0.1 + 0.2)), canon(&Value::Real(0.3)));
        assert_eq!(canon(&Value::Real(-0.0)), "0");
        assert_eq!(canon(&Value::Null), "NULL");
    }

    #[test]
    fn unordered_comparison_sorts_rows() {
        let a = sample();
        let mut b = sample();
        b.rows.reverse();
        assert!(a.same_rows_as(&b, false));
        assert!(!a.same_rows_as(&b, true));
    }

    #[test]
    fn box_mode_aligns_columns() {
        let text = format_result(&sample(), Mode::Box, true);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "┌────────┬──────┐");
        assert_eq!(lines[1], "│ name   │ age  │");
        assert_eq!(lines[3], "│ Ada    │   36 │");
        assert_eq!(lines[4], "│ Bo, Jr │ NULL │");
    }

    #[test]
    fn csv_quotes_fields_and_honours_headers() {
        let text = format_result(&sample(), Mode::Csv, false);
        assert_eq!(text, "Ada,36\n\"Bo, Jr\",\n");
        assert!(format_result(&sample(), Mode::Csv, true).starts_with("name,age\n"));
    }

    #[test]
    fn line_mode_prints_one_field_per_line() {
        let text = format_result(&sample(), Mode::Line, true);
        assert!(text.starts_with("name = Ada\n age = 36\n\nname = Bo, Jr\n age = NULL\n"));
    }
}
