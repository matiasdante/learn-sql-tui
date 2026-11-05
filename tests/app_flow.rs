use std::path::PathBuf;
use std::time::{Duration, Instant};

use learn_sql_tui::app::{App, ExStatus, Screen};
use learn_sql_tui::content::{Content, SolutionStep};
use learn_sql_tui::i18n::Lang;
use learn_sql_tui::progress::Progress;
use learn_sql_tui::sandbox::{Sandbox, remove_dir_all_force};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn english() -> Progress {
    Progress {
        lang: Some(Lang::En),
        exam_done: true,
        ..Progress::default()
    }
}

struct Driver {
    app: App,
    root: PathBuf,
}

impl Driver {
    fn new(name: &str, progress: Progress) -> Self {
        let root =
            std::env::temp_dir().join(format!("learn-sql-tui-flow-{}-{name}", std::process::id()));
        remove_dir_all_force(&root).unwrap();
        let sandbox = Sandbox::new(root.clone()).unwrap();
        Self {
            app: App::new(sandbox, Content::load().unwrap(), progress, None),
            root,
        }
    }

    fn press(&mut self, code: KeyCode) {
        self.app.on_key(key(code));
    }

    /// Escribe una línea y espera a que termine de ejecutarse.
    fn type_line(&mut self, text: &str) {
        for c in text.chars() {
            self.press(KeyCode::Char(c));
        }
        self.press(KeyCode::Enter);
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            self.app.tick();
            if self.app.running.is_none() {
                return;
            }
            assert!(Instant::now() < deadline, "statement did not finish");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Abre la lección `id` desde el mapa y entra a su primer ejercicio.
    fn open_exercise(&mut self, lesson_id: &str) {
        let (level, position) = {
            let lesson = self
                .app
                .content
                .lessons
                .iter()
                .find(|l| l.id == lesson_id)
                .unwrap();
            let position = self
                .app
                .content
                .lessons_of(lesson.level)
                .position(|(_, l)| l.id == lesson_id)
                .unwrap();
            (lesson.level, position)
        };
        self.app.screen = Screen::Map;
        self.app.map_selected = level as usize - 1;
        self.press(KeyCode::Enter);
        for _ in 0..position {
            self.press(KeyCode::Down);
        }
        self.press(KeyCode::Enter); // teoría
        self.press(KeyCode::Enter); // primer ejercicio
        assert!(matches!(self.app.screen, Screen::Exercise(_)));
    }

    /// Resuelve el ejercicio actual tipeando su solución de referencia.
    fn solve_current(&mut self) {
        let Screen::Exercise(state) = &self.app.screen else {
            panic!("not in an exercise");
        };
        let steps: Vec<String> = self.app.content.lessons[state.lesson].exercises[state.index]
            .solution
            .iter()
            .map(|s| match s {
                SolutionStep::Line(l) => l.clone(),
                SolutionStep::Write { write, content } => {
                    let path = self.app_files().join(write);
                    std::fs::write(path, content).unwrap();
                    String::new()
                }
            })
            .collect();
        for step in steps {
            for line in step.lines() {
                self.type_line(line);
            }
        }
        self.press(KeyCode::F(5));
    }

    fn app_files(&self) -> PathBuf {
        self.root.join("work")
    }

    fn status(&self) -> &ExStatus {
        match &self.app.screen {
            Screen::Exercise(s) => &s.status,
            _ => panic!("expected the exercise screen"),
        }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        let _ = remove_dir_all_force(&self.root);
    }
}

#[test]
fn first_run_goes_language_then_exam_offer_then_map() {
    let mut d = Driver::new("first-run", Progress::default());
    assert!(matches!(d.app.screen, Screen::LangSelect));
    d.press(KeyCode::Char('2'));
    assert_eq!(d.app.lang, Lang::Es);
    assert_eq!(d.app.progress.lang, Some(Lang::Es));
    assert!(matches!(d.app.screen, Screen::ExamOffer { .. }));
    d.press(KeyCode::Down); // omitir
    d.press(KeyCode::Enter);
    assert!(matches!(d.app.screen, Screen::Map));
    assert!(d.app.progress.exam_done);
    assert_eq!(d.app.progress.placement, None);
}

#[test]
fn hints_and_failures_reduce_points() {
    let mut d = Driver::new("points", english());
    assert!(matches!(d.app.screen, Screen::Map));

    d.open_exercise("select");
    d.press(KeyCode::F(1)); // una pista: -25
    d.press(KeyCode::F(5)); // sin hacer nada: falla, -10
    match &d.app.screen {
        Screen::Exercise(s) => assert!(matches!(s.status, ExStatus::Failed(_)) && s.failures == 1),
        _ => panic!("expected the exercise screen"),
    }
    d.solve_current();
    assert!(
        matches!(d.status(), ExStatus::Passed { points: 65 }),
        "100 - 25 - 10"
    );
    assert_eq!(d.app.progress.total_points(), 65);
    assert!(d.app.progress.achievements.contains("first-query"));
}

#[test]
fn perfect_exam_places_after_the_tested_levels_and_a_mistake_places_lower() {
    let content = Content::load().unwrap();
    for mistake_at in [None, Some(0usize), Some(5)] {
        let mut d = Driver::new("exam", Progress::default());
        d.press(KeyCode::Char('1'));
        d.press(KeyCode::Enter); // rendir el examen
        for (i, q) in content.exam.iter().enumerate() {
            let pick = if Some(i) == mistake_at {
                (q.answer + 1) % q.options.len()
            } else {
                q.answer
            };
            d.press(KeyCode::Char(
                char::from_digit(pick as u32 + 1, 10).unwrap(),
            ));
            d.press(KeyCode::Enter);
        }
        d.press(KeyCode::Enter); // resultado
        assert!(matches!(d.app.screen, Screen::Map));
        let expected = match mistake_at {
            None => 6, // todo bien arranca en el nivel siguiente al último evaluado
            Some(i) => content.exam[i].level,
        };
        assert_eq!(d.app.progress.placement, Some(expected));
    }
}

#[test]
fn a_failed_check_restores_the_exercise_snapshot() {
    let mut d = Driver::new("restore", english());
    d.open_exercise("select");
    d.type_line("CREATE TABLE extra (x);");
    d.type_line("SELECT name FROM customers;"); // casi: falta la ciudad
    d.press(KeyCode::F(5));
    assert!(matches!(d.status(), ExStatus::Failed(_)));
    d.type_line("SELECT count(*) AS n FROM sqlite_schema WHERE name = 'extra';");
    let last = d
        .app
        .output
        .lines
        .iter()
        .rev()
        .find(|l| l.contains('│') && l.contains('0'));
    assert!(last.is_some(), "the extra table is gone after the restore");
}

#[test]
fn ctrl_c_clears_a_half_typed_statement() {
    let mut d = Driver::new("ctrl-c", english());
    d.open_exercise("select");
    d.type_line("SELECT name");
    assert_eq!(d.app.prompt(), "...> ");
    d.app
        .on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(d.app.prompt(), "sql> ");
}

#[test]
fn free_mode_keeps_its_database_between_visits() {
    let mut d = Driver::new("free", english());
    for _ in 0..2 {
        d.app.screen = Screen::Map;
        d.app.map_selected = d.app.map_items().len() - 2;
        d.press(KeyCode::Enter);
        assert!(matches!(d.app.screen, Screen::Free));
        if d.app.output.lines.iter().all(|l| !l.contains("kept")) {
            d.type_line("CREATE TABLE IF NOT EXISTS kept (x);");
            d.type_line("INSERT INTO kept VALUES (1);");
        }
        d.press(KeyCode::Esc);
    }
    d.app.screen = Screen::Map;
    d.app.map_selected = d.app.map_items().len() - 2;
    d.press(KeyCode::Enter);
    d.type_line("SELECT count(*) AS n FROM kept;");
    assert!(
        d.app.output.lines.iter().any(|l| l.contains("│ 2 ")),
        "{:?}",
        d.app.output.lines
    );
}

/// Aprobar el último ejercicio práctico muestra "¿Y ahora?" y deja la entrada fija en el mapa.
#[test]
fn finishing_the_last_exercise_shows_the_whats_next_screen() {
    let mut progress = english();
    let content = Content::load().unwrap();
    let last = content
        .lessons
        .iter()
        .rev()
        .find(|l| !l.exercises.is_empty())
        .unwrap();
    for lesson in &content.lessons {
        for ex in &lesson.exercises {
            if lesson.id != last.id {
                progress.record_pass(&ex.id, 0, 0);
            }
        }
    }
    let mut d = Driver::new("whats-next", progress);
    assert!(
        !d.app
            .map_items()
            .contains(&learn_sql_tui::app::MapItem::WhatsNext)
    );

    d.open_exercise(&last.id);
    for i in 0..last.exercises.len() {
        d.solve_current();
        assert!(
            matches!(d.status(), ExStatus::Passed { .. }),
            "exercise {i}"
        );
        d.press(KeyCode::F(6));
    }
    assert!(matches!(d.app.screen, Screen::WhatsNext));
    assert!(d.app.progress.course_done);
    assert!(
        d.app
            .map_items()
            .contains(&learn_sql_tui::app::MapItem::WhatsNext)
    );

    // Se puede volver a abrir desde el mapa.
    d.press(KeyCode::Esc);
    assert!(matches!(d.app.screen, Screen::Map));
    let pos = d
        .app
        .map_items()
        .iter()
        .position(|i| *i == learn_sql_tui::app::MapItem::WhatsNext)
        .unwrap();
    d.app.map_selected = pos;
    d.press(KeyCode::Enter);
    assert!(matches!(d.app.screen, Screen::WhatsNext));
}
