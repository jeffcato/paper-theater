//! Selected atomic-file and CAS helpers. SQLite owns all durable job/event/budget state.
pub mod cas;
pub mod writer;
pub mod lock;
pub mod error;
pub use cas::{sha256_digest, validate_digest, validate_extension, CasPublication, CasStore};
pub use writer::{atomic_write_bytes, atomic_write_json, ensure_dir_all_synced, read_json};
pub use lock::{LockGuard, SidecarLock};
pub use error::JournalError;
