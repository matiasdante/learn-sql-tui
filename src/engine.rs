//! Motor de ejercicios: prepara el estado inicial, toma el snapshot y verifica.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::checks::Ctx;
use crate::console::Console;
use crate::content::{Action, Exercise, SolutionStep, seed_script};
use crate::i18n::Text;

pub enum Verdict {
    Pass,
    /// Explicación del primer check que falló.
    Fail(Text),
}

/// Deja el sandbox listo para el ejercicio y guarda el snapshot de ese estado.
pub fn start(console: &mut Console, exercise: &Exercise) -> Result<()> {
    console.reset_exercise()?;
    let files = console.files_dir();
    let db = console.sandbox().practice_db();
    for action in &exercise.setup {
        apply(&db, &files, action)
            .with_context(|| format!("setup of exercise '{}' failed", exercise.id))?;
    }
    console.snapshot()
}

/// Corre los checks en orden sobre la sesión viva y devuelve la explicación del primero que
/// falla. No modifica nada.
pub fn first_failure(console: &Console, exercise: &Exercise) -> Result<Option<Text>> {
    let files = console.files_dir();
    console.inspect(|conn, last, history| {
        let ctx = Ctx {
            conn,
            last,
            history,
            files,
        };
        exercise
            .checks
            .iter()
            .find(|check| !check.passes(&ctx))
            .map(|check| check.fail.clone())
    })
}

/// Verifica el ejercicio. Si algún check falla, devuelve su explicación y restaura el
/// snapshot del inicio del ejercicio.
pub fn verify(console: &mut Console, exercise: &Exercise) -> Result<Verdict> {
    match first_failure(console, exercise)? {
        None => Ok(Verdict::Pass),
        Some(explanation) => {
            console.restore_snapshot()?;
            Ok(Verdict::Fail(explanation))
        }
    }
}

fn apply(db: &Path, files: &Path, action: &Action) -> Result<()> {
    match action {
        Action::Sql { script } => run_script(db, script)?,
        Action::Seed { name } => run_script(
            db,
            seed_script(name).with_context(|| format!("unknown seed '{name}'"))?,
        )?,
        Action::Write { path, content } => {
            let target = safe_join(files, path)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, content)?;
        }
        Action::Mkdir { path } => std::fs::create_dir_all(safe_join(files, path)?)?,
        Action::Rm { path } => {
            let target = safe_join(files, path)?;
            if target.is_dir() {
                crate::sandbox::remove_dir_all_force(&target)?;
            } else {
                std::fs::remove_file(&target)?;
            }
        }
    }
    Ok(())
}

/// El script de preparación corre con una conexión común: es contenido nuestro, no del usuario.
fn run_script(db: &Path, script: &str) -> Result<()> {
    let conn = rusqlite::Connection::open(db)?;
    // Igual que la consola: las claves foráneas arrancan apagadas (el SQLite embebido las
    // trae activadas), así un ejercicio puede sembrar filas huérfanas a propósito.
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    conn.execute_batch(script)?;
    Ok(())
}

/// Une una ruta relativa del contenido con la carpeta de archivos, sin dejar que se escape.
fn safe_join(base: &Path, rel: &str) -> Result<PathBuf> {
    let rel = Path::new(rel);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        bail!("invalid path in lesson content: {}", rel.display());
    }
    Ok(base.join(rel))
}

/// Ejecuta un paso de la solución de referencia, como si el usuario lo tipeara.
pub fn run_step(console: &mut Console, step: &SolutionStep) -> Result<(Option<i32>, String)> {
    match step {
        SolutionStep::Line(line) => crate::console::run_blocking(console, line),
        SolutionStep::Write { write, content } => {
            let target = safe_join(&console.files_dir(), write)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(target, content)?;
            Ok((Some(0), String::new()))
        }
    }
}
