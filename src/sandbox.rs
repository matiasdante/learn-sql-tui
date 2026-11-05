//! Sandbox: una carpeta propia con la base de práctica (`practice.db`), su snapshot y los
//! archivos que usan `.read`, `.import` y `edit`. El modo libre tiene su propia base, que
//! se conserva entre sesiones.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Dónde trabaja la consola: la base del ejercicio (se reinicia) o la del modo libre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workspace {
    Exercise,
    Free,
}

#[derive(Clone, Debug)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// `<datos del usuario>/learn-sql-tui/sandbox`
    pub fn default_root() -> Result<PathBuf> {
        let base = dirs::data_dir().context("could not find the user data directory")?;
        Ok(base.join("learn-sql-tui").join("sandbox"))
    }

    pub fn new(root: PathBuf) -> Result<Self> {
        let sandbox = Self { root };
        sandbox.prepare()?;
        Ok(sandbox)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Base de datos de los ejercicios.
    pub fn practice_db(&self) -> PathBuf {
        self.root.join("practice.db")
    }

    /// Archivos de los ejercicios (para `.read`, `.import`, `edit`, `ATTACH`).
    pub fn work_dir(&self) -> PathBuf {
        self.root.join("work")
    }

    fn snapshot_db(&self) -> PathBuf {
        self.root.join("snapshot.db")
    }

    fn snapshot_work(&self) -> PathBuf {
        self.root.join("snapshot-work")
    }

    /// Base de datos del modo libre: se conserva entre sesiones.
    pub fn free_db(&self) -> PathBuf {
        self.root.join("free").join("free.db")
    }

    pub fn free_work_dir(&self) -> PathBuf {
        self.root.join("free").join("work")
    }

    pub fn db_path(&self, ws: Workspace) -> PathBuf {
        match ws {
            Workspace::Exercise => self.practice_db(),
            Workspace::Free => self.free_db(),
        }
    }

    pub fn files_dir(&self, ws: Workspace) -> PathBuf {
        match ws {
            Workspace::Exercise => self.work_dir(),
            Workspace::Free => self.free_work_dir(),
        }
    }

    /// Guarda una copia de la base y de los archivos del ejercicio. La conexión tiene que
    /// estar cerrada: se copia el archivo tal cual.
    pub fn snapshot(&self) -> Result<()> {
        remove_db_files(&self.snapshot_db())?;
        remove_dir_all_force(&self.snapshot_work())?;
        copy_db(&self.practice_db(), &self.snapshot_db())?;
        copy_dir_all(&self.work_dir(), &self.snapshot_work())
    }

    /// Vuelve la base y los archivos del ejercicio al último snapshot (con la conexión cerrada).
    pub fn restore(&self) -> Result<()> {
        remove_db_files(&self.practice_db())?;
        remove_dir_all_force(&self.work_dir())?;
        copy_db(&self.snapshot_db(), &self.practice_db())?;
        copy_dir_all(&self.snapshot_work(), &self.work_dir())
    }

    /// Deja los ejercicios en blanco: sin base y sin archivos. No toca el modo libre.
    pub fn reset_exercise(&self) -> Result<()> {
        remove_db_files(&self.practice_db())?;
        remove_dir_all_force(&self.work_dir())?;
        self.prepare()
    }

    /// Borra la base y los archivos del modo libre.
    pub fn reset_free(&self) -> Result<()> {
        remove_dir_all_force(&self.root.join("free"))?;
        self.prepare()
    }

    fn prepare(&self) -> Result<()> {
        for dir in [self.work_dir(), self.free_work_dir()] {
            fs::create_dir_all(&dir)
                .with_context(|| format!("could not create {}", dir.display()))?;
        }
        // Un archivo vacío es una base SQLite válida: así lazysql puede abrirlo desde el principio.
        let free = self.free_db();
        if !free.exists() {
            fs::write(&free, b"").context("could not create the free-mode database")?;
        }
        Ok(())
    }
}

/// Archivos que SQLite puede dejar junto a una base.
const DB_SUFFIXES: [&str; 4] = ["", "-journal", "-wal", "-shm"];

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

pub fn remove_db_files(path: &Path) -> Result<()> {
    for suffix in DB_SUFFIXES {
        let file = with_suffix(path, suffix);
        if file.exists() {
            fs::remove_file(&file)
                .with_context(|| format!("could not remove {}", file.display()))?;
        }
    }
    Ok(())
}

/// Copia una base cerrada; si el origen no existe, el destino queda sin base.
fn copy_db(from: &Path, to: &Path) -> Result<()> {
    if from.exists() {
        fs::copy(from, to).with_context(|| format!("could not copy {}", from.display()))?;
    }
    Ok(())
}

/// Borra un directorio aunque tenga archivos de solo lectura.
pub fn remove_dir_all_force(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if fs::remove_dir_all(path).is_ok() {
        return Ok(());
    }
    make_writable(path);
    fs::remove_dir_all(path).with_context(|| format!("could not remove {}", path.display()))
}

#[allow(clippy::permissions_set_readonly_false)]
fn make_writable(path: &Path) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    let mut perms = meta.permissions();
    perms.set_readonly(false);
    let _ = fs::set_permissions(path, perms);
    if meta.is_dir()
        && let Ok(entries) = fs::read_dir(path)
    {
        for entry in entries.flatten() {
            make_writable(&entry.path());
        }
    }
}

/// Copia recursiva; ignora enlaces simbólicos (el sandbox no los usa).
fn copy_dir_all(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    if !from.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(from)?.flatten() {
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target)
                .with_context(|| format!("could not copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}
