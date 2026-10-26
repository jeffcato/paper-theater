use std::fs;
use std::io::Write;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::JournalError;

/// Recursively creates a directory hierarchy, syncing each parent directory entry as created.
/// Tolerates concurrent creation by verifying existing paths are directories and preserving durability syncs.
pub fn ensure_dir_all_synced(path: &Path) -> Result<(), JournalError> {
    // Include cwd's ancestry for relative paths; a concurrent creator may not yet
    // have persisted the entry naming cwd itself.
    let absolute_path = std::path::absolute(path)?;
    let path = absolute_path.as_path();
    let mut stack = Vec::new();
    let mut current = path.to_path_buf();
    while !current.as_os_str().is_empty() {
        match fs::metadata(&current) {
            Ok(m) => {
                if !m.is_dir() {
                    return Err(JournalError::Io(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!("Path {} exists and is not a directory", current.display()),
                    )));
                }
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                stack.push(current.clone());
                match current.parent() {
                    Some(p) => current = p.to_path_buf(),
                    None => break,
                }
            }
            Err(e) => return Err(JournalError::Io(e)),
        }
    }
    while let Some(dir) = stack.pop() {
        match fs::create_dir(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let m = fs::metadata(&dir)?;
                if !m.is_dir() {
                    return Err(JournalError::Io(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!("Path {} exists and is not a directory", dir.display()),
                    )));
                }
            }
            Err(e) => return Err(JournalError::Io(e)),
        }
        if let Some(parent) = dir.parent() {
            let parent_to_sync = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            let f = fs::File::open(parent_to_sync)?;
            f.sync_all()?;
        }
    }

    // Complete ancestor durability: walk all ancestor directories up to root/anchor,
    // fsyncing each containing parent directory and propagating any errors.
    let mut curr_ancestor = Some(path);
    while let Some(p) = curr_ancestor {
        if p.as_os_str().is_empty() {
            break;
        }
        if let Some(parent) = p.parent() {
            let parent_to_sync = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            let m = fs::metadata(parent_to_sync)?;
            if m.is_dir() {
                let f = fs::File::open(parent_to_sync)?;
                f.sync_all()?;
            }
        }
        curr_ancestor = p.parent();
    }

    Ok(())
}

/// Atomically writes bytes to `dest` using a temporary file in the destination's parent directory,
/// calling `fsync` (`sync_all()`) before renaming over `dest` and syncing the containing parent directory afterward.
///
/// Ensures missing parent directories are created automatically and that no partially written
/// or corrupted files are ever visible to readers.
pub fn atomic_write_bytes(dest: &Path, data: &[u8]) -> Result<(), JournalError> {
    // Resolve once so directory synchronization and publication use the same path.
    let absolute_dest = std::path::absolute(dest)?;
    let dest = absolute_dest.as_path();
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    ensure_dir_all_synced(parent)?;

    let mut temp = tempfile::Builder::new()
        .prefix(".pending-")
        .tempfile_in(parent)?;

    temp.write_all(data)?;
    temp.flush()?;
    temp.as_file().sync_all()?;

    temp.persist(dest)
        .map_err(|e| JournalError::AtomicRenameFailed {
            source_path: e.file.path().display().to_string(),
            target_path: dest.display().to_string(),
            reason: e.error.to_string(),
        })?;

    let parent_to_sync = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let parent_file = fs::File::open(parent_to_sync)?;
    parent_file.sync_all()?;

    Ok(())
}

/// Atomically writes a serializable value to `dest` as formatted JSON with a trailing newline.
pub fn atomic_write_json<T: Serialize>(dest: &Path, value: &T) -> Result<(), JournalError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    atomic_write_bytes(dest, &bytes)
}

/// Reads and deserializes a JSON file from `src`.
pub fn read_json<T: DeserializeOwned>(src: &Path) -> Result<T, JournalError> {
    let data = fs::read(src)?;
    let val = serde_json::from_slice(&data)?;
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct SampleData {
        id: String,
        count: u64,
        tags: Vec<String>,
    }

    #[test]
    fn test_atomic_write_bytes_creates_file_and_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested").join("sub").join("data.bin");

        assert!(!target.exists());
        atomic_write_bytes(&target, b"hello atomic world").unwrap();

        assert!(target.exists());
        assert_eq!(fs::read(&target).unwrap(), b"hello atomic world");
    }

    #[test]
    fn test_atomic_write_overwrites_existing_file_safely() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("versioned.txt");

        atomic_write_bytes(&target, b"version 1").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"version 1");

        atomic_write_bytes(&target, b"version 2 updated").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"version 2 updated");
    }

    #[test]
    fn test_atomic_write_json_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sample.json");

        let sample = SampleData {
            id: "book-123".into(),
            count: 42,
            tags: vec!["audio".into(), "prose".into()],
        };

        atomic_write_json(&target, &sample).unwrap();
        assert!(target.exists());

        let loaded: SampleData = read_json(&target).unwrap();
        assert_eq!(loaded, sample);

        // Verify trailing newline
        let raw = fs::read_to_string(&target).unwrap();
        assert!(raw.ends_with('\n'));
    }

    #[test]
    fn test_atomic_write_no_leftover_pending_files() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("clean.txt");

        atomic_write_bytes(&target, b"clean").unwrap();

        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(entries, vec!["clean.txt"]);
    }
}
