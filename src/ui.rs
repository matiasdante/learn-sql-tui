//! Dibujo de las pantallas: menú, teoría, ejercicio y modo libre.

use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::app::{App, ExStatus, ExamState, ExerciseState, MapItem, Screen};
use crate::banner;
use crate::i18n::Lang;
use crate::i18n::tr;
use crate::sandbox::Workspace;

pub fn draw(frame: &mut Frame, app: &App) {
    if let Some(editor) = &app.editor {
        frame.render_widget(&editor.textarea, frame.area());
        return;
    }
    match &app.screen {
        Screen::LangSelect => draw_lang(frame, app),
        Screen::ExamOffer { selected } => draw_exam_offer(frame, app, *selected),
        Screen::Exam(exam) => draw_exam(frame, app, exam),
        Screen::Map => draw_map(frame, app),
        Screen::Level(level) => draw_level(frame, app, *level),
        Screen::Theory { lesson, scroll } => draw_theory(frame, app, *lesson, *scroll),
        Screen::Exercise(state) => draw_exercise(frame, app, state),
        Screen::Free => draw_free(frame, app),
        Screen::Achievements => draw_achievements(frame, app),
        Screen::WhatsNext => draw_whats_next(frame, app),
    }
}

/// Naranja #ff8400: el único color de acento de todas las pantallas.
pub const ACCENT_RGB: (u8, u8, u8) = (0xff, 0x84, 0x00);

/// Con truecolor se usa el naranja exacto; sin él, el índice 208 de la paleta de 256 colores
/// (#ff8700, casi idéntico). Windows Terminal no define COLORTERM pero sí `WT_SESSION`.
pub fn accent_for(colorterm: Option<&str>, windows_terminal: bool) -> Color {
    let truecolor = windows_terminal
        || colorterm.is_some_and(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("truecolor") || v.contains("24bit")
        });
    if truecolor {
        Color::Rgb(ACCENT_RGB.0, ACCENT_RGB.1, ACCENT_RGB.2)
    } else {
        Color::Indexed(208)
    }
}

pub fn accent() -> Color {
    static ACCENT: OnceLock<Color> = OnceLock::new();
    *ACCENT.get_or_init(|| {
        accent_for(
            std::env::var("COLORTERM").ok().as_deref(),
            std::env::var_os("WT_SESSION").is_some(),
        )
    })
}

fn accent_bold() -> Style {
    Style::new().fg(accent()).add_modifier(Modifier::BOLD)
}

/// Un marco con el título en el color de acento.
fn framed(title: impl Into<String>) -> Block<'static> {
    Block::bordered().title(Span::styled(title.into(), accent_bold()))
}

fn dim() -> Style {
    Style::new().add_modifier(Modifier::DIM)
}

fn draw_help(frame: &mut Frame, area: Rect, text: &str) {
    frame.render_widget(Paragraph::new(text.to_string()).style(dim()), area);
}

fn centered(frame: &Frame, width: u16, height: u16) -> Rect {
    let area = frame.area();
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

fn option_line(label: String, selected: bool) -> Line<'static> {
    if selected {
        Line::from(Span::styled(format!("  ▶ {label}"), accent_bold()))
    } else {
        Line::raw(format!("    {label}"))
    }
}

/// Banner, eslogan y nota, como cabecera de las pantallas de inicio.
fn splash_header(lang: Lang, width: u16, with_banner: bool) -> Vec<Line<'static>> {
    let mut lines = if with_banner {
        banner::lines(width)
    } else {
        vec![Line::styled("SQL", accent_bold())]
    };
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        tr(
            lang,
            "Learn SQL by practicing real SQL, inside your terminal",
        ),
        Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::styled(
        tr(
            lang,
            "-A sandbox: embedded SQLite, nothing outside it is ever touched-",
        ),
        Style::new().fg(Color::DarkGray),
    ));
    lines.push(Line::raw(""));
    lines
}

/// Dibuja la cabecera y `body` centrados en la pantalla. Si no entra, saca el banner grande.
fn draw_splash(frame: &mut Frame, lang: Lang, body: Vec<Line<'static>>) {
    let area = frame.area();
    let full = splash_header(lang, area.width, true);
    let needed = (full.len() + body.len()) as u16;
    let lines: Vec<Line<'static>> = if area.height >= needed + 2 {
        full.into_iter().chain(body).collect()
    } else {
        splash_header(lang, area.width, false)
            .into_iter()
            .chain(body)
            .collect()
    };
    let height = (lines.len() as u16).min(area.height);
    let rect = Rect::new(
        area.x,
        area.y + (area.height - height) / 2,
        area.width,
        height,
    );
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

/// Las opciones se rellenan al mismo ancho para que se vean alineadas al centrarlas.
fn aligned_options(labels: &[String], selected: usize) -> Vec<Line<'static>> {
    let width = labels
        .iter()
        .map(|l| Line::raw(l.clone()).width())
        .max()
        .unwrap_or(0);
    labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let pad = " ".repeat(width - Line::raw(label.clone()).width());
            option_line(format!("{label}{pad}"), i == selected)
        })
        .collect()
}

fn draw_lang(frame: &mut Frame, app: &App) {
    let labels: Vec<String> = Lang::ALL
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{}  {}", i + 1, l.native_name()))
        .collect();
    let selected = Lang::ALL.iter().position(|l| *l == app.lang).unwrap_or(0);
    let mut body = vec![
        Line::raw("Choose your language · Elegí tu idioma · Escolha o idioma · 选择语言"),
        Line::raw(""),
    ];
    body.extend(aligned_options(&labels, selected));
    body.push(Line::raw(""));
    body.push(Line::styled("↑/↓ · 1-4 · Enter", dim()));
    draw_splash(frame, app.lang, body);
}

fn draw_exam_offer(frame: &mut Frame, app: &App, selected: usize) {
    let labels = vec![
        tr(app.lang, "Take the placement exam").to_string(),
        tr(app.lang, "Skip it and start at level 1").to_string(),
    ];
    let mut body = vec![
        Line::raw(tr(
            app.lang,
            "Do you already know some SQL? A 10-question placement exam picks your starting level.",
        )),
        Line::raw(""),
    ];
    body.extend(aligned_options(&labels, selected));
    body.push(Line::raw(""));
    body.push(Line::styled(
        tr(
            app.lang,
            "Earlier levels always stay available.  ↑/↓ · Enter · L language",
        ),
        dim(),
    ));
    draw_splash(frame, app.lang, body);
}

fn draw_exam(frame: &mut Frame, app: &App, exam: &ExamState) {
    let area = centered(frame, 90, 16);
    let title = tr(app.lang, " Placement exam ");
    if let Some(level) = exam.result {
        let name = app
            .content
            .level(level)
            .map_or("", |l| l.title.get(app.lang));
        let lines = vec![
            Line::raw(tr(app.lang, "Exam finished.")),
            Line::raw(""),
            Line::styled(
                format!("{} {level} — {name}", tr(app.lang, "You start at level")),
                accent_bold(),
            ),
            Line::raw(""),
            Line::styled(tr(app.lang, "Enter: continue"), dim()),
        ];
        frame.render_widget(Paragraph::new(lines).block(framed(title)), area);
        return;
    }
    let question = &app.content.exam[exam.question];
    let mut lines = vec![
        Line::styled(
            format!(
                "{} {}/{}",
                tr(app.lang, "Question"),
                exam.question + 1,
                app.content.exam.len()
            ),
            dim(),
        ),
        Line::raw(question.text.get(app.lang).to_string()),
        Line::raw(""),
    ];
    for (i, option) in question.options.iter().enumerate() {
        lines.push(option_line(
            format!("{}  {}", i + 1, option.get(app.lang)),
            i == exam.selected,
        ));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        tr(app.lang, "↑/↓ or 1-4 to choose · Enter to answer"),
        dim(),
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(framed(title))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_map(frame: &mut Frame, app: &App) {
    let area = frame.area();
    // Con pantalla alta el mapa lleva el banner arriba.
    let header = (area.height >= 30 && area.width >= banner::WIDTH)
        .then(|| splash_header(app.lang, area.width, true));
    let header_height = header.as_ref().map_or(0, |h| h.len() as u16);
    let [top, body, help] = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(area);
    if let Some(lines) = header {
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), top);
    }
    let title = format!(
        " learn-sql-tui — {} {} ",
        app.progress.total_points(),
        tr(app.lang, "points")
    );
    let mut lines: Vec<Line> = Vec::new();
    for (i, item) in app.map_items().into_iter().enumerate() {
        let label = match item {
            MapItem::Level(id) => {
                let name = app.content.level(id).map_or("", |l| l.title.get(app.lang));
                let (done, total) = app.content.level_progress(id, &app.progress);
                let mark = if total > 0 && done == total {
                    "✓"
                } else {
                    " "
                };
                let start = if app.progress.placement == Some(id) {
                    tr(app.lang, "  ← your start")
                } else {
                    ""
                };
                format!(
                    "{mark} {} {id:>2} — {name}  [{done}/{total}]{start}",
                    tr(app.lang, "Level")
                )
            }
            MapItem::WhatsNext => format!("★ {}", tr(app.lang, "What now?")),
            MapItem::Free => format!("  {}", tr(app.lang, "Free mode")),
            MapItem::Achievements => format!(
                "  {} ({}/{})",
                tr(app.lang, "Achievements"),
                app.progress.achievements.len(),
                app.content.achievements.len()
            ),
        };
        if i == app.content.levels.len() {
            lines.push(Line::raw(""));
        }
        lines.push(option_line(label, i == app.map_selected));
    }
    frame.render_widget(Paragraph::new(lines).block(framed(title)), body);
    draw_help(
        frame,
        help,
        tr(
            app.lang,
            "↑/↓ move · Enter open · L: English/Español · Q quit",
        ),
    );
}

fn draw_level(frame: &mut Frame, app: &App, level: u8) {
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let name = app
        .content
        .level(level)
        .map_or("", |l| l.title.get(app.lang));
    let mut lines: Vec<Line> = Vec::new();
    for (pos, (_, lesson)) in app.content.lessons_of(level).enumerate() {
        let done = !lesson.exercises.is_empty()
            && lesson.exercises.iter().all(|e| app.progress.is_done(&e.id));
        let mark = if done { "✓" } else { " " };
        lines.push(option_line(
            format!("{mark} {}", lesson.title.get(app.lang)),
            pos == app.level_selected,
        ));
    }
    if lines.is_empty() {
        lines.push(Line::raw(tr(app.lang, "Coming soon.")));
    }
    let block = framed(format!(" {} {level} — {name} ", tr(app.lang, "Level")));
    frame.render_widget(Paragraph::new(lines).block(block), body);
    draw_help(
        frame,
        help,
        tr(
            app.lang,
            "↑/↓ move · Enter open lesson · L language · Esc back",
        ),
    );
}

fn draw_achievements(frame: &mut Frame, app: &App) {
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let mut lines: Vec<Line> = Vec::new();
    for a in &app.content.achievements {
        let unlocked = app.progress.achievements.contains(&a.id);
        let (mark, style) = if unlocked {
            ("★", accent_bold())
        } else {
            ("☆", dim())
        };
        lines.push(Line::styled(
            format!("{mark} {}", a.title.get(app.lang)),
            style,
        ));
        lines.push(Line::styled(
            format!("    {}", a.description.get(app.lang)),
            dim(),
        ));
    }
    let title = format!(
        " {} — {} {} ",
        tr(app.lang, "Achievements"),
        app.progress.total_points(),
        tr(app.lang, "points")
    );
    frame.render_widget(Paragraph::new(lines).block(framed(title)), body);
    draw_help(frame, help, tr(app.lang, "Esc: back"));
}

/// Pantalla final: recomienda lazysql y muestra dónde está la base del modo libre.
fn draw_whats_next(frame: &mut Frame, app: &App) {
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let free_db = app.console.sandbox().db_path(Workspace::Free);
    let heading = |text: &'static str| Line::styled(tr(app.lang, text), accent_bold());
    let command = |text: &str| Line::styled(format!("    {text}"), Style::new().fg(accent()));
    let lines = vec![
        heading("You finished every practice exercise. Well done!"),
        Line::raw(""),
        Line::raw(tr(
            app.lang,
            "To keep practicing on real databases, try lazysql: a terminal UI for databases, the same idea as lazygit is for git.",
        )),
        Line::raw(""),
        heading("Install it (pick one):"),
        command("brew install lazysql          (macOS / Linux, Homebrew)"),
        command("go install github.com/jorgerojas26/lazysql@latest"),
        command("paru -S lazysql               (Arch Linux, AUR)"),
        Line::raw(tr(
            app.lang,
            "    Or download a binary for Windows, macOS or Linux from github.com/jorgerojas26/lazysql/releases",
        )),
        Line::raw(""),
        heading("Open your free-mode database with it:"),
        command(&format!("lazysql \"{}\"", free_db.display())),
        Line::raw(tr(
            app.lang,
            "    Everything you create in Free mode is saved in that file between sessions.",
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(framed(format!(" {} ", tr(app.lang, "What now?"))))
            .wrap(Wrap { trim: false }),
        body,
    );
    draw_help(frame, help, tr(app.lang, "Esc: back"));
}

fn draw_theory(frame: &mut Frame, app: &App, lesson_index: usize, scroll: u16) {
    let lesson = &app.content.lessons[lesson_index];
    let [body, help] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let block = framed(format!(" {} ", lesson.title.get(app.lang)));
    let mut lines: Vec<Line> = lesson
        .theory
        .get(app.lang)
        .lines()
        .map(|l| Line::raw(l.to_string()))
        .collect();
    if lesson.exercises.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            tr(app.lang, "This topic is explained without an exercise."),
            dim(),
        ));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        body,
    );
    let can_practice = !lesson.exercises.is_empty();
    draw_help(
        frame,
        help,
        if can_practice {
            tr(
                app.lang,
                "↑/↓ scroll · Enter start exercises · L language · Esc back",
            )
        } else {
            tr(app.lang, "↑/↓ scroll · L language · Esc back")
        },
    );
}

fn draw_exercise(frame: &mut Frame, app: &App, state: &ExerciseState) {
    let lesson = &app.content.lessons[state.lesson];
    let exercise = &lesson.exercises[state.index];
    let prompt_height = (frame.area().height / 3).clamp(6, 13);
    let [prompt_area, output_area, input_area, help_area] = Layout::vertical([
        Constraint::Length(prompt_height),
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let mut lines = vec![Line::styled(
        exercise.prompt.get(app.lang).to_string(),
        Style::new().add_modifier(Modifier::BOLD),
    )];
    for (i, hint) in exercise.hints.iter().take(state.hints_shown).enumerate() {
        lines.push(Line::raw(format!(
            "{} {}: {}",
            tr(app.lang, "Hint"),
            i + 1,
            hint.get(app.lang)
        )));
    }
    match &state.status {
        ExStatus::Doing => {}
        ExStatus::Failed(why) => {
            lines.push(Line::raw(""));
            lines.push(Line::raw(format!("✗ {}", why.get(app.lang))));
        }
        ExStatus::Passed { points } => {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                format!(
                    "{} +{points} {}",
                    tr(app.lang, "✓ Correct!"),
                    tr(app.lang, "points")
                ),
                accent_bold(),
            ));
        }
    }
    let title = format!(
        " {} — {} {}/{} ",
        lesson.title.get(app.lang),
        tr(app.lang, "exercise"),
        state.index + 1,
        lesson.exercises.len()
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(framed(title))
            .wrap(Wrap { trim: false }),
        prompt_area,
    );

    draw_console(
        frame,
        app,
        output_area,
        input_area,
        tr(app.lang, " Console "),
    );
    let help = if app.running.is_some() {
        tr(app.lang, "Running… Ctrl+C: stop the query")
    } else if matches!(state.status, ExStatus::Passed { .. }) {
        tr(app.lang, "F6: continue · Esc: back to the menu")
    } else {
        tr(
            app.lang,
            "F5: check · F1: hint · Ctrl+R: restart · Esc: menu · Ctrl+Q: quit",
        )
    };
    draw_help(frame, help_area, help);
}

fn draw_free(frame: &mut Frame, app: &App) {
    let [output_area, input_area, help_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    draw_console(
        frame,
        app,
        output_area,
        input_area,
        tr(app.lang, " Free mode "),
    );
    let help = if app.confirm_reset {
        tr(
            app.lang,
            "Erase the free-mode database? Everything you created will be lost.  y: yes · any other key: no",
        )
    } else if app.running.is_some() {
        tr(app.lang, "Running… Ctrl+C: stop the query")
    } else {
        tr(
            app.lang,
            "Enter: run · Ctrl+R: erase the database · Esc: menu · PgUp/PgDn: scroll · Ctrl+Q: quit",
        )
    };
    draw_help(frame, help_area, help);
}

fn draw_console(frame: &mut Frame, app: &App, output_area: Rect, input_area: Rect, title: &str) {
    draw_output(frame, app, output_area, title);
    draw_input(frame, app, input_area);
}

fn draw_output(frame: &mut Frame, app: &App, area: Rect, title: &str) {
    let block = framed(title.to_string());
    let inner = block.inner(area);
    let height = inner.height as usize;
    let rows = tail_rows(
        &app.output.lines,
        inner.width.max(1) as usize,
        height + app.scroll,
    );
    let end = rows.len().saturating_sub(app.scroll);
    let start = end.saturating_sub(height);
    let text = rows[start..end].join("\n");
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered();
    let inner = block.inner(area);
    let prompt = app.prompt();
    let prompt_len = prompt.chars().count();
    let width = inner.width.max(1) as usize;
    // Si la línea es más larga que el panel, se muestra la parte que contiene el cursor.
    let offset = (prompt_len + app.cursor).saturating_sub(width - 1);
    let visible: Vec<char> = format!("{prompt}{}", app.input)
        .chars()
        .skip(offset)
        .take(width)
        .collect();
    // El prompt va en el color de acento y lo que se tipeó en el normal.
    let prompt_shown = prompt_len.saturating_sub(offset).min(visible.len());
    let line = Line::from(vec![
        Span::styled(
            visible[..prompt_shown].iter().collect::<String>(),
            accent_bold(),
        ),
        Span::raw(visible[prompt_shown..].iter().collect::<String>()),
    ]);
    frame.render_widget(Paragraph::new(line).block(block), area);
    frame.set_cursor_position(Position::new(
        inner.x + (prompt_len + app.cursor - offset) as u16,
        inner.y,
    ));
}

/// Las últimas `need` filas de pantalla, con las líneas largas cortadas al ancho del panel.
fn tail_rows(lines: &[String], width: usize, need: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for line in lines.iter().rev() {
        let chars: Vec<char> = line.chars().collect();
        let chunks: Vec<String> = if chars.is_empty() {
            vec![String::new()]
        } else {
            chars.chunks(width).map(|c| c.iter().collect()).collect()
        };
        rows.extend(chunks.into_iter().rev());
        if rows.len() >= need {
            break;
        }
    }
    rows.reverse();
    rows
}
