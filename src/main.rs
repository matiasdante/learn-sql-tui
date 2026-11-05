use std::env;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use learn_sql_tui::app::App;
use learn_sql_tui::content::Content;
use learn_sql_tui::progress::Progress;
use learn_sql_tui::sandbox::Sandbox;
use learn_sql_tui::ui;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

fn main() -> ExitCode {
    if env::args_os()
        .nth(1)
        .is_some_and(|a| a == "--version" || a == "-V")
    {
        println!(
            "learn-sql-tui {} (SQLite {})",
            env!("CARGO_PKG_VERSION"),
            rusqlite::version()
        );
        return ExitCode::SUCCESS;
    }
    match run_tui() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run_tui() -> Result<()> {
    let sandbox = Sandbox::new(Sandbox::default_root()?)?;
    let progress_path = Progress::default_path()?;
    let progress = Progress::load(&progress_path);
    let mut app = App::new(sandbox, Content::load()?, progress, Some(progress_path));
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    app.shutdown();
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        if event::poll(Duration::from_millis(50))? {
            // En Windows crossterm también emite Release/Repeat; solo nos importa Press.
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.on_key(key);
            }
        }
        app.tick();
    }
    Ok(())
}
