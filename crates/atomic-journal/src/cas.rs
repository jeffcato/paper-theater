use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::JournalError;
use crate::lock::SidecarLock;
use crate::writer::atomic_write_bytes;

/// Computes the SHA-256 hexadecimal digest for raw bytes.
pub fn sha256_digest(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Validates that `hash` is a canonical 64-character lowercase hexadecimal SHA-256 digest.
pub fn validate_digest(hash: &str) -> Result<(), JournalError> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(JournalError::InvalidDigest {
            digest: hash.to_string(),
            reason: "digest must be exactly 64 lowercase hexadecimal characters".into(),
        });
    }
    Ok(())
}

/// Validates that `extension` is a safe single or compound filename extension.
pub fn validate_extension(extension: &str) -> Result<(), JournalError> {
    let clean = extension.strip_prefix('.').unwrap_or(extension);
    if clean.is_empty()
        || clean.contains('/')
        || clean.contains('\\')
        || clean.contains("..")
        || clean.ends_with('.')
        || clean
            .chars()
            .any(|c| c.is_ascii_control() || c.is_whitespace())
    {
        return Err(JournalError::InvalidExtension {
            extension: extension.to_string(),
            reason: "extension contains invalid characters, path separators, or traversal"
                .to_string(),
        });
    }
    for part in clean.split('.') {
        if part.is_empty()
            || !part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(JournalError::InvalidExtension {
                extension: extension.to_string(),
                reason: "extension component must be alphanumeric".to_string(),
            });
        }
    }
    Ok(())
}

/// Content-Addressable Storage (CAS) engine.
///
/// Stores media, boundary slices, and artifacts keyed by their cryptographic SHA-256 digest,
/// providing instant deduplication, on-read checksum verification, and coordinated GC.
#[derive(Debug, Clone)]
pub struct CasStore {
    root: PathBuf,
    sharded: bool,
}

impl CasStore {
    pub fn new(root: impl Into<PathBuf>, sharded: bool) -> Self {
        Self {
            root: root.into(),
            sharded,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn gate_lock(&self) -> SidecarLock {
        SidecarLock::new(self.root.join(".cas_gate.lock"))
    }

    /// Fallible path builder requiring a valid digest and valid extension.
    pub fn try_path_for_hash(
        &self,
        hash: &str,
        extension: Option<&str>,
    ) -> Result<PathBuf, JournalError> {
        validate_digest(hash)?;
        if let Some(ext) = extension {
            validate_extension(ext)?;
        }
        let ext_str = match extension {
            Some(ext) => {
                let clean = ext.strip_prefix('.').unwrap_or(ext);
                format!(".{}", clean)
            }
            None => String::new(),
        };

        if self.sharded {
            Ok(self
                .root
                .join(&hash[..2])
                .join(format!("{}{}", &hash[2..], ext_str)))
        } else {
            Ok(self.root.join(format!("{}{}", hash, ext_str)))
        }
    }

    /// Formats the target filesystem path for a given hash and optional file extension without panicking.
    pub fn path_for_hash(&self, hash: &str, extension: Option<&str>) -> PathBuf {
        let ext_str = match extension {
            Some(ext) => {
                let clean = ext.strip_prefix('.').unwrap_or(ext);
                format!(".{}", clean)
            }
            None => String::new(),
        };

        if self.sharded && hash.len() >= 2 && hash.is_char_boundary(2) {
            self.root
                .join(&hash[..2])
                .join(format!("{}{}", &hash[2..], ext_str))
        } else {
            self.root.join(format!("{}{}", hash, ext_str))
        }
    }

    /// Internal storage helper executed under a shared CAS gate lock.
    fn store_internal(&self, data: &[u8], extension: Option<&str>) -> Result<String, JournalError> {
        if let Some(ext) = extension {
            validate_extension(ext)?;
        }

        let hash = sha256_digest(data);
        let dest = self.path_for_hash(&hash, extension);

        if dest.exists() {
            if let Ok(existing) = fs::read(&dest) {
                if sha256_digest(&existing) == hash {
                    return Ok(hash);
                }
            }
        }

        atomic_write_bytes(&dest, data)?;
        Ok(hash)
    }

    /// Begins a publication transaction holding the shared CAS gate lock.
    /// The lock is held until `CasPublication` is dropped, guaranteeing that GC cannot
    /// snapshot live references or prune files while blobs are being stored and references published.
    pub fn begin_publication(&self) -> Result<CasPublication<'_>, JournalError> {
        let gate = self.gate_lock();
        let guard = gate.lock_shared()?;
        Ok(CasPublication {
            store: self,
            _guard: guard,
        })
    }

    /// Reads bytes for `hash` and verifies that the computed checksum matches `hash`.
    ///
    /// Returns `Err(JournalError::ChecksumMismatch)` if content was corrupted.
    pub fn read(&self, hash: &str, extension: Option<&str>) -> Result<Vec<u8>, JournalError> {
        validate_digest(hash)?;
        if let Some(ext) = extension {
            validate_extension(ext)?;
        }

        let path = self.path_for_hash(hash, extension);
        let bytes = fs::read(&path)?;
        let actual = sha256_digest(&bytes);

        if actual != hash {
            return Err(JournalError::ChecksumMismatch {
                expected: hash.to_string(),
                actual,
                path: path.display().to_string(),
            });
        }

        Ok(bytes)
    }

    /// Checks if a file for `hash` exists in the store.
    pub fn contains(&self, hash: &str, extension: Option<&str>) -> bool {
        if validate_digest(hash).is_err() {
            return false;
        }
        if let Some(ext) = extension {
            if validate_extension(ext).is_err() {
                return false;
            }
        }
        self.path_for_hash(hash, extension).is_file()
    }

    /// Prunes unreferenced CAS files by executing a snapshot callback under the exclusive gate lock.
    ///
    /// The snapshot callback inspects authoritative manifests while holding the exclusive CAS gate,
    /// guaranteeing that no in-flight publication can add new references after the snapshot is taken.
    ///
    /// # Mandatory Lock Ordering
    /// To avoid lock inversion, callers must acquire locks in this sequence:
    /// 1. CAS Gate Lock (acquired exclusively inside `prune_with_snapshot`)
    /// 2. Manifest / Book Lock (acquired within `snapshot_fn` if inspecting locked manifests)
    pub fn prune_with_snapshot<F>(&self, snapshot_fn: F) -> Result<usize, JournalError>
    where
        F: FnOnce() -> Result<HashSet<String>, JournalError>,
    {
        if !self.root.exists() {
            return Ok(0);
        }

        let gate = self.gate_lock();
        let _guard = gate.lock_exclusive()?;

        let active_hashes = snapshot_fn()?;
        self.prune_internal(&active_hashes)
    }

    fn prune_internal(&self, active_hashes: &HashSet<String>) -> Result<usize, JournalError> {
        let mut removed = 0;

        if self.sharded {
            for entry in fs::read_dir(&self.root)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let shard = entry.file_name().to_string_lossy().to_string();
                    if shard.len() != 2
                        || !shard
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    {
                        continue;
                    }
                    let mut shard_empty = true;
                    for file_entry in fs::read_dir(&path)? {
                        let file_entry = file_entry?;
                        let file_path = file_entry.path();
                        let metadata = file_entry.metadata()?;
                        if !metadata.is_file() || metadata.file_type().is_symlink() {
                            continue;
                        }
                        let file_name = file_entry.file_name().to_string_lossy().to_string();
                        if file_name.starts_with(".pending-") || file_name == ".cas_gate.lock" {
                            shard_empty = false;
                            continue;
                        }
                        let full_hash = match parse_sharded_cas_filename(&shard, &file_name) {
                            Some(h) => h,
                            None => continue,
                        };
                        if !active_hashes.contains(&full_hash) {
                            fs::remove_file(&file_path)?;
                            removed += 1;
                        } else {
                            shard_empty = false;
                        }
                    }
                    if shard_empty {
                        let _ = fs::remove_dir(&path);
                    }
                }
            }
        } else {
            for entry in fs::read_dir(&self.root)? {
                let entry = entry?;
                let path = entry.path();
                let metadata = entry.metadata()?;
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    continue;
                }
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with(".pending-") || file_name == ".cas_gate.lock" {
                    continue;
                }
                let hash_part = match parse_flat_cas_filename(&file_name) {
                    Some(h) => h,
                    None => continue,
                };
                if !active_hashes.contains(hash_part) {
                    fs::remove_file(&path)?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}

/// An RAII guard for a publication transaction holding the shared CAS gate lock.
///
/// # Mandatory Lock Ordering
/// To prevent deadlock and lock inversion, locks must ALWAYS be acquired in this sequence:
/// 1. CAS Gate Lock (acquired as shared upon `CasStore::begin_publication()`, or exclusive in `CasStore::prune_with_snapshot()`)
/// 2. Manifest / Book Lock (acquired when committing the updated manifest or index)
pub struct CasPublication<'a> {
    store: &'a CasStore,
    _guard: crate::lock::LockGuard,
}

impl<'a> CasPublication<'a> {
    /// Stores raw bytes under the held shared publication lock. Keep this guard
    /// alive until every new reference has been durably published. Dropping it
    /// without publishing aborts the operation and leaves the blobs collectable.
    pub fn store(&self, data: &[u8], extension: Option<&str>) -> Result<String, JournalError> {
        self.store.store_internal(data, extension)
    }

    pub fn store_ref(&self) -> &CasStore {
        self.store
    }

    /// Atomically writes manifest bytes to `dest` while holding the shared publication lock.
    pub fn publish_manifest_bytes(&self, dest: &Path, data: &[u8]) -> Result<(), JournalError> {
        crate::writer::atomic_write_bytes(dest, data)
    }

    /// Atomically writes serialized JSON manifest to `dest` while holding the shared publication lock.
    pub fn publish_manifest_json<T: serde::Serialize>(
        &self,
        dest: &Path,
        value: &T,
    ) -> Result<(), JournalError> {
        crate::writer::atomic_write_json(dest, value)
    }
}

fn parse_flat_cas_filename(file_name: &str) -> Option<&str> {
    if file_name.len() < 64 || !file_name.is_char_boundary(64) {
        return None;
    }
    let hash_part = &file_name[..64];
    if !hash_part
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    if file_name.len() > 64 {
        if !file_name[64..].starts_with('.') {
            return None;
        }
        let ext = &file_name[65..];
        if validate_extension(ext).is_err() {
            return None;
        }
    }
    Some(hash_part)
}

fn parse_sharded_cas_filename(shard: &str, file_name: &str) -> Option<String> {
    if shard.len() != 2
        || !shard
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    if file_name.len() < 62 || !file_name.is_char_boundary(62) {
        return None;
    }
    let rest = &file_name[..62];
    if !rest
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    if file_name.len() > 62 {
        if !file_name[62..].starts_with('.') {
            return None;
        }
        let ext = &file_name[63..];
        if validate_extension(ext).is_err() {
            return None;
        }
    }
    Some(format!("{}{}", shard, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cas_store_and_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cas = CasStore::new(dir.path(), true);

        let data = b"audio-chunk-sample-data-48khz";
        let hash = cas
            .begin_publication()
            .unwrap()
            .store(data, Some("wav"))
            .unwrap();

        assert_eq!(hash.len(), 64);
        assert!(cas.contains(&hash, Some("wav")));

        let retrieved = cas.read(&hash, Some("wav")).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_cas_deduplication_skips_rewriting_identical_hash() {
        let dir = tempfile::tempdir().unwrap();
        let cas = CasStore::new(dir.path(), false);

        let data = b"canvas-plate-slice-1024x192";
        let h1 = cas
            .begin_publication()
            .unwrap()
            .store(data, Some(".png"))
            .unwrap();
        let path = cas.path_for_hash(&h1, Some(".png"));

        let mod1 = fs::metadata(&path).unwrap().modified().unwrap();

        // Storing identical data again
        let h2 = cas
            .begin_publication()
            .unwrap()
            .store(data, Some(".png"))
            .unwrap();
        assert_eq!(h1, h2);

        let mod2 = fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mod1, mod2, "Timestamp should not change on dedup hit");
    }

    #[test]
    fn test_cas_read_detects_corrupted_data_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let cas = CasStore::new(dir.path(), true);

        let data = b"sam-mask-polygon-layer-bytes";
        let hash = cas
            .begin_publication()
            .unwrap()
            .store(data, Some("bin"))
            .unwrap();
        let path = cas.path_for_hash(&hash, Some("bin"));

        // Simulate bitrot/corruption on disk
        fs::write(&path, b"corrupted-tampered-data").unwrap();

        let err = cas.read(&hash, Some("bin")).unwrap_err();
        assert!(matches!(err, JournalError::ChecksumMismatch { .. }));
    }

    #[test]
    fn test_cas_sharded_and_flat_topologies() {
        let dir = tempfile::tempdir().unwrap();

        // Sharded
        let cas_sharded = CasStore::new(dir.path().join("sharded"), true);
        let data = b"topology test";
        let hash = cas_sharded
            .begin_publication()
            .unwrap()
            .store(data, Some("txt"))
            .unwrap();
        let path_sharded = cas_sharded.path_for_hash(&hash, Some("txt"));
        assert!(path_sharded.to_string_lossy().contains(&hash[..2]));

        // Flat
        let cas_flat = CasStore::new(dir.path().join("flat"), false);
        let hash_flat = cas_flat
            .begin_publication()
            .unwrap()
            .store(data, Some("txt"))
            .unwrap();
        let path_flat = cas_flat.path_for_hash(&hash_flat, Some("txt"));
        assert_eq!(path_flat.parent().unwrap(), cas_flat.root());
    }

    #[test]
    fn test_cas_prune_unreferenced_removes_orphans() {
        let dir = tempfile::tempdir().unwrap();
        let cas = CasStore::new(dir.path(), true);

        let h1 = cas
            .begin_publication()
            .unwrap()
            .store(b"keep me 1", Some("txt"))
            .unwrap();
        let h2 = cas
            .begin_publication()
            .unwrap()
            .store(b"keep me 2", Some("txt"))
            .unwrap();
        let h3 = cas
            .begin_publication()
            .unwrap()
            .store(b"delete me orphan", Some("txt"))
            .unwrap();

        let mut active = HashSet::new();
        active.insert(h1.clone());
        active.insert(h2.clone());

        let pruned = cas.prune_with_snapshot(|| Ok(active)).unwrap();
        assert_eq!(pruned, 1);

        assert!(cas.contains(&h1, Some("txt")));
        assert!(cas.contains(&h2, Some("txt")));
        assert!(!cas.contains(&h3, Some("txt")));
    }
}
