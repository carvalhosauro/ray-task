use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

pub struct Paths {
    pub db: PathBuf,
    pub log_dir: PathBuf,
    pub lock: PathBuf,
}

impl Paths {
    /// Caminhos XDG: dados em ~/.local/share/ray-task, log em ~/.local/state/ray-task,
    /// lock em $XDG_RUNTIME_DIR (ou na pasta de dados).
    pub fn from_env() -> Self {
        let data = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("ray-task");
        let log_dir = dirs::state_dir().map(|d| d.join("ray-task")).unwrap_or_else(|| data.clone());
        let lock = dirs::runtime_dir().map(|d| d.join("ray-task.lock")).unwrap_or_else(|| data.join("ray-task.lock"));
        Self { db: data.join("ray-task.db"), log_dir, lock }
    }
}

#[derive(Debug)]
pub enum LockError {
    AlreadyRunning,
    Io(io::Error),
}

/// Mantém o lock enquanto existir.
pub struct InstanceLock {
    _file: File,
}

pub fn acquire_lock(path: &Path) -> Result<InstanceLock, LockError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(LockError::Io)?;
    }
    let file = OpenOptions::new().create(true).truncate(false).write(true).open(path).map_err(LockError::Io)?;
    match file.try_lock() {
        Ok(()) => Ok(InstanceLock { _file: file }),
        Err(TryLockError::WouldBlock) => Err(LockError::AlreadyRunning),
        Err(TryLockError::Error(e)) => Err(LockError::Io(e)),
    }
}
