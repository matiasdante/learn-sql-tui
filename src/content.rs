//! Lecciones en TOML embebidas en el binario.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use serde::Deserialize;

use crate::checks::Check;
use crate::i18n::{Lang, Text};
use crate::progress::Progress;

static CONTENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/content");

#[derive(Debug, Deserialize)]
pub struct Level {
    pub id: u8,
    /// Se completa al cargar, desde `content/i18n/<idioma>/`.
    #[serde(skip)]
    pub title: Text,
}

#[derive(Debug, Deserialize)]
struct LevelsFile {
    level: Vec<Level>,
}

#[derive(Debug, Deserialize)]
pub struct Achievement {
    pub id: String,
    #[serde(skip)]
    pub title: Text,
    #[serde(skip)]
    pub description: Text,
    pub condition: Condition,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    ExerciseDone {
        exercise: String,
    },
    LevelDone {
        level: u8,
    },
    /// Todos los ejercicios del nivel hechos sin usar ninguna pista.
    LevelNoHints {
        level: u8,
    },
}

#[derive(Debug, Deserialize)]
struct AchievementsFile {
    achievement: Vec<Achievement>,
}

#[derive(Debug, Deserialize)]
pub struct Question {
    pub id: String,
    pub level: u8,
    #[serde(skip)]
    pub text: Text,
    /// En el archivo es la cantidad de opciones; los textos se completan al cargar.
    #[serde(rename = "options")]
    pub option_count: usize,
    #[serde(skip)]
    pub options: Vec<Text>,
    /// Índice (desde 0) de la opción correcta.
    pub answer: usize,
}

#[derive(Debug, Deserialize)]
struct ExamFile {
    question: Vec<Question>,
}

#[derive(Debug, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub level: u8,
    pub order: u32,
    #[serde(skip)]
    pub title: Text,
    #[serde(skip)]
    pub theory: Text,
    #[serde(default, rename = "exercise")]
    pub exercises: Vec<Exercise>,
}

#[derive(Debug, Deserialize)]
pub struct Exercise {
    pub id: String,
    #[serde(skip)]
    pub prompt: Text,
    #[serde(default)]
    pub setup: Vec<Action>,
    /// En el archivo es la cantidad de pistas; los textos se completan al cargar.
    #[serde(default, rename = "hints")]
    pub hint_count: usize,
    #[serde(skip)]
    pub hints: Vec<Text>,
    pub checks: Vec<Check>,
    /// Pasos que resuelven el ejercicio (los usa el test de contenido).
    pub solution: Vec<SolutionStep>,
}

/// Un paso de la solución de referencia: lo que el usuario tipearía en la consola (puede
/// tener varias líneas), o un archivo que "escribe" con `edit`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SolutionStep {
    Line(String),
    Write { write: String, content: String },
}

/// Preparación del estado inicial: acciones del motor, nunca una shell.
#[derive(Debug, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Action {
    /// Un script SQL que corre sobre `practice.db`.
    Sql {
        script: String,
    },
    /// Un script SQL compartido de `content/seeds/<name>.sql`.
    Seed {
        name: String,
    },
    /// Un archivo en la carpeta de archivos del ejercicio (para `.read` y `.import`).
    Write {
        path: String,
        content: String,
    },
    Mkdir {
        path: String,
    },
    Rm {
        path: String,
    },
}

/// Script de datos compartido por varios ejercicios (`content/seeds/<name>.sql`).
pub fn seed_script(name: &str) -> Option<&'static str> {
    CONTENT_DIR
        .get_file(format!("seeds/{name}.sql"))
        .and_then(|f| f.contents_utf8())
}

pub struct Content {
    pub levels: Vec<Level>,
    pub lessons: Vec<Lesson>,
    pub achievements: Vec<Achievement>,
    pub exam: Vec<Question>,
}

impl Content {
    pub fn load() -> Result<Self> {
        let levels_file = CONTENT_DIR
            .get_file("levels.toml")
            .context("content/levels.toml is missing")?;
        let levels: LevelsFile = toml::from_str(
            levels_file
                .contents_utf8()
                .context("levels.toml is not UTF-8")?,
        )
        .context("invalid levels.toml")?;

        let mut lessons = Vec::new();
        let mut files = Vec::new();
        collect_toml(&CONTENT_DIR, &mut files);
        for file in files {
            let path = file.path();
            if ["levels.toml", "achievements.toml", "exam.toml"]
                .iter()
                .any(|name| path.ends_with(name))
            {
                continue;
            }
            let text = file
                .contents_utf8()
                .with_context(|| format!("{} is not UTF-8", path.display()))?;
            let lesson: Lesson = toml::from_str(text)
                .with_context(|| format!("invalid lesson {}", path.display()))?;
            lessons.push(lesson);
        }
        lessons.sort_by_key(|l| (l.level, l.order));

        let mut seen = HashSet::new();
        for lesson in &lessons {
            if !seen.insert(lesson.id.clone()) {
                bail!("duplicate lesson id '{}'", lesson.id);
            }
            for ex in &lesson.exercises {
                if !seen.insert(ex.id.clone()) {
                    bail!("duplicate exercise id '{}'", ex.id);
                }
            }
        }
        let achievements: AchievementsFile = read_toml("achievements.toml")?;
        let exam: ExamFile = read_toml("exam.toml")?;
        let mut content = Self {
            levels: levels.level,
            lessons,
            achievements: achievements.achievement,
            exam: exam.question,
        };
        content.validate_references()?;
        content.allocate_text_slots();
        content.apply_translations()?;
        Ok(content)
    }

    /// Todos los textos traducibles con su clave:
    /// `level.N.title`, `ach.ID.title|description`, `exam.ID.text|optN`,
    /// `LECCION.title|theory`, `EJERCICIO.prompt|hintN|checkN.fail`.
    pub fn texts_mut(&mut self) -> Vec<(String, &mut Text)> {
        let mut out: Vec<(String, &mut Text)> = Vec::new();
        for level in &mut self.levels {
            out.push((format!("level.{}.title", level.id), &mut level.title));
        }
        for a in &mut self.achievements {
            out.push((format!("ach.{}.title", a.id), &mut a.title));
            out.push((format!("ach.{}.description", a.id), &mut a.description));
        }
        for q in &mut self.exam {
            out.push((format!("exam.{}.text", q.id), &mut q.text));
            for (i, option) in q.options.iter_mut().enumerate() {
                out.push((format!("exam.{}.opt{}", q.id, i + 1), option));
            }
        }
        for lesson in &mut self.lessons {
            out.push((format!("{}.title", lesson.id), &mut lesson.title));
            out.push((format!("{}.theory", lesson.id), &mut lesson.theory));
            for ex in &mut lesson.exercises {
                out.push((format!("{}.prompt", ex.id), &mut ex.prompt));
                for (i, hint) in ex.hints.iter_mut().enumerate() {
                    out.push((format!("{}.hint{}", ex.id, i + 1), hint));
                }
                for (i, check) in ex.checks.iter_mut().enumerate() {
                    out.push((format!("{}.check{}.fail", ex.id, i + 1), &mut check.fail));
                }
            }
        }
        out
    }

    /// Los archivos de contenido solo dicen cuántas pistas y opciones hay: acá se crean los
    /// lugares para sus textos.
    fn allocate_text_slots(&mut self) {
        for lesson in &mut self.lessons {
            for ex in &mut lesson.exercises {
                ex.hints = vec![Text::default(); ex.hint_count];
            }
        }
        for q in &mut self.exam {
            q.options = vec![Text::default(); q.option_count];
        }
    }

    /// Carga los textos de `content/i18n/<idioma>/*.toml` (tablas `"clave" = "texto"`), uno
    /// por idioma. Una clave que no existe es un error: casi seguro es un typo.
    fn apply_translations(&mut self) -> Result<()> {
        for lang in Lang::ALL {
            let Some(dir) = CONTENT_DIR.get_dir(format!("i18n/{}", lang.code())) else {
                continue;
            };
            let mut translations: HashMap<String, String> = HashMap::new();
            for file in dir.files() {
                let path = file.path();
                let text = file
                    .contents_utf8()
                    .with_context(|| format!("{} is not UTF-8", path.display()))?;
                let table: HashMap<String, String> = toml::from_str(text)
                    .with_context(|| format!("invalid translation file {}", path.display()))?;
                for (key, value) in table {
                    if translations.insert(key.clone(), value).is_some() {
                        bail!("translation key '{key}' is duplicated ({})", lang.code());
                    }
                }
            }
            for (key, text) in self.texts_mut() {
                if let Some(value) = translations.remove(&key) {
                    text.set(lang, value);
                }
            }
            if let Some(unknown) = translations.keys().min() {
                bail!(
                    "translation key '{unknown}' does not exist ({})",
                    lang.code()
                );
            }
        }
        Ok(())
    }

    /// Los logros y el examen no pueden apuntar a niveles o ejercicios que no existen.
    fn validate_references(&self) -> Result<()> {
        for a in &self.achievements {
            match &a.condition {
                Condition::ExerciseDone { exercise } => {
                    if self.exercise(exercise).is_none() {
                        bail!(
                            "achievement '{}' refers to unknown exercise '{exercise}'",
                            a.id
                        );
                    }
                }
                Condition::LevelDone { level } | Condition::LevelNoHints { level } => {
                    if self.level(*level).is_none() {
                        bail!("achievement '{}' refers to unknown level {level}", a.id);
                    }
                }
            }
        }
        for lesson in &self.lessons {
            for ex in &lesson.exercises {
                for action in &ex.setup {
                    if let Action::Seed { name } = action
                        && seed_script(name).is_none()
                    {
                        bail!("exercise '{}' uses the unknown seed '{name}'", ex.id);
                    }
                }
            }
        }
        for q in &self.exam {
            if q.option_count < 2 || q.answer >= q.option_count {
                bail!("question '{}' has invalid options or answer", q.id);
            }
        }
        Ok(())
    }

    pub fn exercise(&self, id: &str) -> Option<&Exercise> {
        self.lessons
            .iter()
            .flat_map(|l| &l.exercises)
            .find(|e| e.id == id)
    }

    pub fn lessons_of(&self, level: u8) -> impl Iterator<Item = (usize, &Lesson)> {
        self.lessons
            .iter()
            .enumerate()
            .filter(move |(_, l)| l.level == level)
    }

    pub fn max_level(&self) -> u8 {
        self.levels.iter().map(|l| l.id).max().unwrap_or(1)
    }

    /// Logros cuya condición se cumple con este progreso y que todavía no se otorgaron.
    pub fn newly_unlocked(&self, progress: &Progress) -> Vec<&Achievement> {
        self.achievements
            .iter()
            .filter(|a| !progress.achievements.contains(&a.id))
            .filter(|a| match &a.condition {
                Condition::ExerciseDone { exercise } => progress.is_done(exercise),
                Condition::LevelDone { level } => self
                    .level_exercises(*level)
                    .all(|e| progress.is_done(&e.id)),
                Condition::LevelNoHints { level } => self
                    .level_exercises(*level)
                    .all(|e| progress.exercises.get(&e.id).is_some_and(|r| r.hints == 0)),
            })
            .collect()
    }

    fn level_exercises(&self, level: u8) -> impl Iterator<Item = &Exercise> {
        self.lessons
            .iter()
            .filter(move |l| l.level == level)
            .flat_map(|l| &l.exercises)
    }

    /// Todos los ejercicios prácticos del curso están hechos.
    pub fn all_exercises_done(&self, progress: &Progress) -> bool {
        let mut all = self.lessons.iter().flat_map(|l| &l.exercises).peekable();
        all.peek().is_some() && all.all(|e| progress.is_done(&e.id))
    }

    /// (hechos, total) de ejercicios de un nivel.
    pub fn level_progress(&self, level: u8, progress: &Progress) -> (usize, usize) {
        let total = self.level_exercises(level).count();
        let done = self
            .level_exercises(level)
            .filter(|e| progress.is_done(&e.id))
            .count();
        (done, total)
    }

    pub fn level(&self, id: u8) -> Option<&Level> {
        self.levels.iter().find(|l| l.id == id)
    }
}

fn collect_toml<'a>(dir: &'a Dir<'static>, out: &mut Vec<&'a include_dir::File<'static>>) {
    for file in dir.files() {
        if file.path().extension().is_some_and(|e| e == "toml") {
            out.push(file);
        }
    }
    for sub in dir.dirs() {
        // Las traducciones (content/i18n) no son lecciones.
        if sub.path().file_name().is_some_and(|n| n == "i18n") {
            continue;
        }
        collect_toml(sub, out);
    }
}

fn read_toml<T: serde::de::DeserializeOwned>(name: &str) -> Result<T> {
    let file = CONTENT_DIR
        .get_file(name)
        .with_context(|| format!("content/{name} is missing"))?;
    toml::from_str(file.contents_utf8().context("not UTF-8")?)
        .with_context(|| format!("invalid {name}"))
}
