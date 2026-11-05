//! Idiomas. Los textos del contenido viven en `content/i18n/<idioma>/*.toml` (uno por
//! idioma, con las mismas claves); los archivos de lecciones solo tienen la estructura.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Es,
    Pt,
    Zh,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::En, Lang::Es, Lang::Pt, Lang::Zh];

    pub fn next(self) -> Self {
        match self {
            Lang::En => Lang::Es,
            Lang::Es => Lang::Pt,
            Lang::Pt => Lang::Zh,
            Lang::Zh => Lang::En,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Lang::En => Lang::Zh,
            Lang::Es => Lang::En,
            Lang::Pt => Lang::Es,
            Lang::Zh => Lang::Pt,
        }
    }

    /// Carpeta de las traducciones en `content/i18n/`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::Pt => "pt",
            Lang::Zh => "zh",
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Pt => "Português",
            Lang::Zh => "中文",
        }
    }
}

/// Texto de la interfaz que no vive en el contenido: se busca por su texto en inglés en
/// `ui_strings` (si falta una traducción se muestra en inglés; un test lo evita).
pub fn tr(lang: Lang, en: &'static str) -> &'static str {
    crate::ui_strings::lookup(lang, en).unwrap_or(en)
}

/// Un texto en los cuatro idiomas. Se completa al cargar el contenido.
#[derive(Clone, Debug, Default)]
pub struct Text {
    pub en: String,
    pub es: String,
    pub pt: String,
    pub zh: String,
}

impl Text {
    /// Si falta la traducción se muestra en inglés (los tests exigen que no falte ninguna).
    pub fn get(&self, lang: Lang) -> &str {
        let text = match lang {
            Lang::En => &self.en,
            Lang::Es => &self.es,
            Lang::Pt => &self.pt,
            Lang::Zh => &self.zh,
        };
        if text.trim().is_empty() {
            &self.en
        } else {
            text
        }
    }

    pub fn set(&mut self, lang: Lang, value: String) {
        match lang {
            Lang::En => self.en = value,
            Lang::Es => self.es = value,
            Lang::Pt => self.pt = value,
            Lang::Zh => self.zh = value,
        }
    }

    /// Los cuatro idiomas tienen texto.
    pub fn is_complete(&self) -> bool {
        [&self.en, &self.es, &self.pt, &self.zh]
            .iter()
            .all(|t| !t.trim().is_empty())
    }

    pub fn missing(&self) -> Vec<Lang> {
        Lang::ALL
            .into_iter()
            .filter(|l| match l {
                Lang::En => self.en.trim().is_empty(),
                Lang::Es => self.es.trim().is_empty(),
                Lang::Pt => self.pt.trim().is_empty(),
                Lang::Zh => self.zh.trim().is_empty(),
            })
            .collect()
    }
}
