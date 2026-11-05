use learn_sql_tui::console::Console;
use learn_sql_tui::content::Content;
use learn_sql_tui::engine::{self, Verdict};
use learn_sql_tui::sandbox::{Sandbox, remove_dir_all_force};

#[test]
fn every_text_exists_in_the_four_languages() {
    let mut content = Content::load().unwrap();
    let missing: Vec<String> = content
        .texts_mut()
        .into_iter()
        .filter(|(_, text)| !text.is_complete())
        .map(|(key, text)| {
            format!(
                "{key} {:?}",
                text.missing().iter().map(|l| l.code()).collect::<Vec<_>>()
            )
        })
        .collect();
    assert!(
        missing.is_empty(),
        "{} texts without all languages, for example: {:?}",
        missing.len(),
        &missing[..missing.len().min(8)]
    );
    for lesson in &content.lessons {
        assert!(
            content.level(lesson.level).is_some(),
            "lesson {} has an unknown level",
            lesson.id
        );
    }
}

/// Para cada ejercicio: el estado inicial sin tocar falla los checks y la solución de
/// referencia los pasa. Así no puede haber ejercicios rotos.
#[test]
fn every_exercise_fails_untouched_and_passes_with_its_solution() {
    let content = Content::load().unwrap();
    let root = std::env::temp_dir().join(format!("learn-sql-tui-content-{}", std::process::id()));
    remove_dir_all_force(&root).unwrap();
    let mut console = Console::new(Sandbox::new(root.clone()).unwrap());

    let mut count = 0;
    for lesson in &content.lessons {
        for ex in &lesson.exercises {
            let ctx = format!("exercise '{}'", ex.id);

            engine::start(&mut console, ex).unwrap_or_else(|e| panic!("{ctx}: {e:#}"));
            assert!(
                matches!(engine::verify(&mut console, ex).unwrap(), Verdict::Fail(_)),
                "{ctx}: the untouched initial state must fail the checks"
            );

            engine::start(&mut console, ex).unwrap();
            for step in &ex.solution {
                let (code, output) =
                    engine::run_step(&mut console, step).unwrap_or_else(|e| panic!("{ctx}: {e:#}"));
                assert_eq!(code, Some(0), "{ctx}: {step:?} failed: {output}");
            }
            if let Verdict::Fail(why) = engine::verify(&mut console, ex).unwrap() {
                panic!("{ctx}: the reference solution does not pass: {}", why.en);
            }
            count += 1;
        }
    }
    assert!(count > 0);
    let _ = remove_dir_all_force(&root);
}

#[test]
fn every_exercise_has_the_texts_it_declares() {
    let content = Content::load().unwrap();
    for lesson in &content.lessons {
        for ex in &lesson.exercises {
            assert!(ex.hint_count >= 1, "{}: at least one hint", ex.id);
            assert!(!ex.checks.is_empty(), "{}: no checks", ex.id);
            assert!(!ex.solution.is_empty(), "{}: no solution", ex.id);
        }
    }
}

#[test]
fn exam_has_two_questions_for_each_of_levels_1_to_5() {
    let content = Content::load().unwrap();
    for level in 1..=5u8 {
        let n = content.exam.iter().filter(|q| q.level == level).count();
        assert_eq!(n, 2, "exam level {level}");
    }
    assert_eq!(content.exam.len(), 10);
}

#[test]
fn the_course_covers_the_eleven_levels_of_the_plan() {
    let content = Content::load().unwrap();
    assert_eq!(content.levels.len(), 11);
    for level in 1..=11u8 {
        assert!(
            content.lessons.iter().any(|l| l.level == level),
            "level {level} has no lessons"
        );
    }
    // El nivel 11 es solo explicación; todos los demás tienen ejercicios.
    for level in 1..=10u8 {
        assert!(
            content
                .lessons
                .iter()
                .filter(|l| l.level == level)
                .any(|l| !l.exercises.is_empty()),
            "level {level} has no exercises"
        );
    }
    assert!(
        content
            .lessons
            .iter()
            .filter(|l| l.level == 11)
            .all(|l| l.exercises.is_empty()),
        "level 11 is explanation only"
    );
}

/// El examen nombra una respuesta correcta por pregunta: no puede haber una repetida entre opciones.
#[test]
fn exam_options_are_distinct() {
    let content = Content::load().unwrap();
    for q in &content.exam {
        let mut texts: Vec<&str> = q.options.iter().map(|o| o.en.as_str()).collect();
        texts.sort();
        texts.dedup();
        assert_eq!(texts.len(), q.options.len(), "{}", q.id);
    }
}

#[test]
fn every_achievement_can_be_unlocked() {
    // `validate_references` ya falla al cargar si un logro apunta a algo inexistente.
    let content = Content::load().unwrap();
    assert!(content.achievements.len() >= 20);
}
