//! Progreso del usuario en `progress.json`: idioma, examen, ejercicios hechos, puntos y logros.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

pub const BASE_POINTS: u32 = 100;
pub const HINT_PENALTY: u32 = 25;
pub const FAILURE_PENALTY: u32 = 10;
pub const MIN_POINTS: u32 = 25;

/// 100 por ejercicio, −25 por pista usada, −10 por verificación fallida, con mínimo de 25.
pub fn score(hints: u32, failures: u32) -> u32 {
    BASE_POINTS
        .saturating_sub(HINT_PENALTY * hints + FAILURE_PENALTY * failures)
        .max(MIN_POINTS)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub points: u32,
    pub hints: u32,
    pub failures: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    pub lang: Option<Lang>,
    /// Ya se ofreció el examen de ubicación (se haya rendido o no).
    #[serde(default)]
    pub exam_done: bool,
    /// Nivel donde arranca el usuario según el examen.
    pub placement: Option<u8>,
    #[serde(default)]
    pub exercises: BTreeMap<String, Record>,
    #[serde(default)]
    pub achievements: BTreeSet<String>,
    /// Ya se aprobó el último ejercicio práctico: la pantalla "¿Y ahora?" queda en el mapa.
    #[serde(default)]
    pub course_done: bool,
}

impl Progress {
    pub fn default_path() -> Result<PathBuf> {
        let base = dirs::data_dir().context("could not find the user data directory")?;
        Ok(base.join("learn-sql-tui").join("progress.json"))
    }

    /// Si el archivo no existe empieza de cero. Si está roto lo aparta como `.bak` para no perderlo.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_else(|_| {
            let _ = fs::rename(path, path.with_extension("json.bak"));
            Self::default()
        })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Se escribe a un temporal y se renombra para no dejar el archivo a medias.
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(&tmp, path).context("could not save progress")
    }

    pub fn is_done(&self, exercise_id: &str) -> bool {
        self.exercises.contains_key(exercise_id)
    }

    pub fn total_points(&self) -> u32 {
        self.exercises.values().map(|r| r.points).sum()
    }

    /// Registra un ejercicio resuelto y devuelve los puntos ganados.
    /// Si ya estaba hecho se conserva el mejor resultado.
    pub fn record_pass(&mut self, exercise_id: &str, hints: u32, failures: u32) -> u32 {
        let points = score(hints, failures);
        let better = self
            .exercises
            .get(exercise_id)
            .is_none_or(|old| points > old.points);
        if better {
            self.exercises.insert(
                exercise_id.to_string(),
                Record {
                    points,
                    hints,
                    failures,
                },
            );
        }
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_follows_the_plan() {
        assert_eq!(score(0, 0), 100);
        assert_eq!(score(1, 0), 75);
        assert_eq!(score(0, 2), 80);
        assert_eq!(score(2, 3), 20u32.max(MIN_POINTS));
        assert_eq!(score(10, 10), 25, "never below the minimum");
    }

    #[test]
    fn keeps_the_best_result_and_roundtrips() {
        let dir =
            std::env::temp_dir().join(format!("learn-sql-tui-progress-{}", std::process::id()));
        let path = dir.join("progress.json");
        let mut progress = Progress::default();
        assert_eq!(progress.record_pass("x", 2, 0), 50);
        assert_eq!(progress.record_pass("x", 0, 1), 90);
        assert_eq!(progress.record_pass("x", 3, 0), 25);
        assert_eq!(progress.total_points(), 90);
        progress.lang = Some(Lang::Es);
        progress.save(&path).unwrap();
        let loaded = Progress::load(&path);
        assert_eq!(loaded.lang, Some(Lang::Es));
        assert_eq!(loaded.total_points(), 90);

        fs::write(&path, "{ not json").unwrap();
        assert_eq!(Progress::load(&path).total_points(), 0);
        assert!(path.with_extension("json.bak").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
