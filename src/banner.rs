//! Banner de inicio: "SQL" en letras grandes, en el color de acento.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::ui::accent;

const S: [&str; 6] = [
    "███████╗",
    "██╔════╝",
    "███████╗",
    "╚════██║",
    "███████║",
    "╚══════╝",
];
const Q: [&str; 6] = [
    " ██████╗ ",
    "██╔═══██╗",
    "██║   ██║",
    "██║▄▄ ██║",
    "╚██████╔╝",
    " ╚══▀▀═╝ ",
];
const L: [&str; 6] = [
    "██╗     ",
    "██║     ",
    "██║     ",
    "██║     ",
    "███████╗",
    "╚══════╝",
];

/// Ancho en columnas del banner completo.
pub const WIDTH: u16 = 25;
pub const HEIGHT: u16 = 6;

fn rows() -> [String; 6] {
    std::array::from_fn(|row| format!("{}{}{}", S[row], Q[row], L[row]))
}

/// El bloque (`█`) va en el acento y la sombra (`╗╔║═╝╚`) en gris.
fn styled(row: &str) -> Line<'static> {
    let block = Style::new().fg(accent()).add_modifier(Modifier::BOLD);
    let shadow = Style::new().fg(Color::DarkGray);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_is_block = true;
    for ch in row.chars() {
        let is_block = ch == '█' || ch == ' ';
        if is_block != current_is_block && !current.is_empty() {
            let style = if current_is_block { block } else { shadow };
            spans.push(Span::styled(std::mem::take(&mut current), style));
        }
        current_is_block = is_block;
        current.push(ch);
    }
    if !current.is_empty() {
        spans.push(Span::styled(
            current,
            if current_is_block { block } else { shadow },
        ));
    }
    Line::from(spans)
}

/// El banner completo, o una línea simple si no entra.
pub fn lines(width: u16) -> Vec<Line<'static>> {
    if width >= WIDTH {
        rows().iter().map(|r| styled(r)).collect()
    } else {
        vec![Line::styled(
            "SQL",
            Style::new().fg(accent()).add_modifier(Modifier::BOLD),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_have_the_declared_width() {
        for row in rows() {
            assert_eq!(row.chars().count() as u16, WIDTH, "{row}");
        }
    }
}
