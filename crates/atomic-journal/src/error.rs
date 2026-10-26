use thiserror::Error;

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Checksum mismatch for {path}: expected {expected}, calculated {actual}")]
    ChecksumMismatch {
        expected: String,
        actual: String,
        path: String,
    },

    #[error("Corrupted journal entry at offset {offset}: {reason}")]
    CorruptedEntry { offset: u64, reason: String },

    #[error("Stale build stamp: {reason}")]
    StaleBuildStamp { reason: String },

    #[error("Atomic rename from {source_path} to {target_path} failed: {reason}")]
    AtomicRenameFailed {
        source_path: String,
        target_path: String,
        reason: String,
    },

    #[error("Invalid digest '{digest}': {reason}")]
    InvalidDigest { digest: String, reason: String },

    #[error("Invalid extension '{extension}': {reason}")]
    InvalidExtension { extension: String, reason: String },

    #[error("Duplicate recipe key: {key}")]
    DuplicateRecipeKey { key: String },
}
