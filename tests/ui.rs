use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

use learn_sql_tui::app::{App, ExStatus, ExamState, ExerciseState, Screen};
use learn_sql_tui::content::Content;
use learn_sql_tui::i18n::Lang;
use learn_sql_tui::progress::Progress;
use learn_sql_tui::sandbox::{Sandbox, remove_dir_all_force};
use learn_sql_tui::ui;

#[test]
fn the_accent_is_ff8400_with_truecolor_and_index_208_without() {
    assert_eq!(
        ui::accent_for(Some("truecolor"), false),
        Color::Rgb(0xff, 0x84, 0x00)
    );
    assert_eq!(
        ui::accent_for(Some("24bit"), false),
        Color::Rgb(0xff, 0x84, 0x00)
    );
    assert_eq!(
        ui::accent_for(None, true),
        Color::Rgb(0xff, 0x84, 0x00),
        "Windows Terminal"
    );
    assert_eq!(ui::accent_for(None, false), Color::Indexed(208));
    assert_eq!(ui::accent_for(Some("256color"), false), Color::Indexed(208));
}

fn uses_accent(app: &App) -> bool {
    let mut terminal = Terminal::new(TestBackend::new(110, 40)).unwrap();
    terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    let accent = ui::accent();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .any(|cell| cell.fg == accent)
}

#[test]
fn every_screen_is_drawn_in_the_accent_color() {
    let root = std::env::temp_dir().join(format!("learn-sql-tui-ui-{}", std::process::id()));
    remove_dir_all_force(&root).unwrap();
    let progress = Progress {
        lang: Some(Lang::En),
        exam_done: true,
        ..Progress::default()
    };
    let mut app = App::new(
        Sandbox::new(root.clone()).unwrap(),
        Content::load().unwrap(),
        progress,
        None,
    );
    app.progress.course_done = true;

    let exercise = Screen::Exercise(ExerciseState {
        lesson: 0,
        index: 0,
        hints_shown: 1,
        failures: 0,
        status: ExStatus::Passed { points: 100 },
    });
    let screens = [
        ("lang", Screen::LangSelect),
        ("exam offer", Screen::ExamOffer { selected: 0 }),
        (
            "exam",
            Screen::Exam(ExamState {
                question: 0,
                selected: 0,
                first_wrong: None,
                result: None,
            }),
        ),
        ("map", Screen::Map),
        ("level", Screen::Level(1)),
        (
            "theory",
            Screen::Theory {
                lesson: 0,
                scroll: 0,
            },
        ),
        ("exercise", exercise),
        ("free", Screen::Free),
        ("achievements", Screen::Achievements),
        ("whats next", Screen::WhatsNext),
    ];
    for (name, screen) in screens {
        app.screen = screen;
        assert!(uses_accent(&app), "the {name} screen has no accent color");
    }
    let _ = remove_dir_all_force(&root);
}
