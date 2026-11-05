//! Editor integrado (`edit <archivo>`): corre dentro del mismo proceso, sin lanzar nada externo.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::Block;
use tui_textarea::TextArea;

pub enum EditorAction {
    Continue,
    Saved,
    Cancelled,
}

pub struct EditorState {
    pub path: PathBuf,
    pub textarea: TextArea<'static>,
}

impl EditorState {
    pub fn open(path: PathBuf) -> Result<Self> {
        let content = match fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).with_context(|| format!("could not read {}", path.display())),
        };
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        let mut textarea = TextArea::new(if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        });
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        textarea.set_block(
            Block::bordered().title(format!(" {name}  —  Ctrl+S save & close · Esc cancel ")),
        );
        Ok(Self { path, textarea })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<EditorAction> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('s'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.save()?;
                Ok(EditorAction::Saved)
            }
            (KeyCode::Esc, _) => Ok(EditorAction::Cancelled),
            _ => {
                self.textarea.input(key);
                Ok(EditorAction::Continue)
            }
        }
    }

    fn save(&self) -> Result<()> {
        let mut text = self.textarea.lines().join("\n");
        text.push('\n');
        fs::write(&self.path, text)
            .with_context(|| format!("could not save {}", self.path.display()))
    }
}
