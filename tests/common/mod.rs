#![allow(dead_code)]

use std::path::PathBuf;

use learn_sql_tui::console::{Console, run_blocking};
use learn_sql_tui::sandbox::{Sandbox, remove_dir_all_force};

pub struct Harness {
    pub console: Console,
    pub root: PathBuf,
}

impl Harness {
    pub fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("learn-sql-tui-test-{}-{name}", std::process::id()));
        remove_dir_all_force(&root).unwrap();
        let sandbox = Sandbox::new(root.clone()).unwrap();
        Self {
            console: Console::new(sandbox),
            root,
        }
    }

    /// Tipea `input` en la consola y devuelve el código final y la salida.
    pub fn raw(&mut self, input: &str) -> (Option<i32>, String) {
        run_blocking(&mut self.console, input).unwrap()
    }

    /// Como `raw`, pero exige que no haya errores.
    pub fn run(&mut self, input: &str) -> String {
        let (code, output) = self.raw(input);
        assert_eq!(code, Some(0), "`{input}` failed: {output}");
        output
    }

    /// Una sentencia que debe fallar: devuelve el mensaje de error.
    pub fn fails(&mut self, input: &str) -> String {
        let (code, output) = self.raw(input);
        assert_eq!(code, Some(1), "`{input}` should fail, got: {output}");
        output
    }
}

/// Las filas de una tabla en modo box como texto: sin encabezado, celdas sin espacios.
pub fn cells(output: &str) -> Vec<Vec<String>> {
    output
        .lines()
        .filter(|l| l.starts_with('│'))
        .skip(1)
        .map(|l| {
            l.trim_matches('│')
                .split('│')
                .map(|c| c.trim().to_string())
                .collect()
        })
        .collect()
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = remove_dir_all_force(&self.root);
    }
}
