use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use crate::error::JournalError;

/// An RAII lock guard that releases the standard-library file lock when dropped.
pub struct LockGuard {
    file: File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// A reusable sidecar file lock using standard library shared/exclusive locks.
///
/// Open a fresh lock handle per acquisition; lock files remain stable and are never deleted.
#[derive(Debug, Clone)]
pub struct SidecarLock {
    lock_path: PathBuf,
}

impl SidecarLock {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            lock_path: path.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.lock_path
    }

    /// Acquires an exclusive file lock, blocking until acquired.
    pub fn lock_exclusive(&self) -> Result<LockGuard, JournalError> {
        if let Some(parent) = self.lock_path.parent() {
            if !parent.as_os_str().is_empty() {
                crate::writer::ensure_dir_all_synced(parent)?;
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)?;
        file.lock()?;
        Ok(LockGuard { file })
    }

    /// Acquires a shared file lock, blocking until acquired.
    pub fn lock_shared(&self) -> Result<LockGuard, JournalError> {
        if let Some(parent) = self.lock_path.parent() {
            if !parent.as_os_str().is_empty() {
                crate::writer::ensure_dir_all_synced(parent)?;
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)?;
        file.lock_shared()?;
        Ok(LockGuard { file })
    }
}
