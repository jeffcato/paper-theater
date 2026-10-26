use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub type Sample = u64;
pub type Id = String;
pub type Digest = String;
pub type Interval = [Sample; 2];
pub type SourceSpan = [u64; 2];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, thiserror::Error)]
#[error("{code} at {field}: {message}")]
#[serde(deny_unknown_fields)]
pub struct ContractError {
    pub code: String,
    pub field: String,
    pub message: String,
}
pub type ContractResult<T> = Result<T, ContractError>;
pub fn invalid<T>(
    code: &str,
    field: impl Into<String>,
    message: impl Into<String>,
) -> ContractResult<T> {
    Err(ContractError {
        code: code.into(),
        field: field.into(),
        message: message.into(),
    })
}
pub fn require(
    ok: bool,
    code: &str,
    field: impl Into<String>,
    message: &str,
) -> ContractResult<()> {
    if ok {
        Ok(())
    } else {
        invalid(code, field, message)
    }
}
pub fn digest(bytes: &[u8]) -> Digest {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn valid_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn valid_id(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
}
pub fn safe_path(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(['\\', ':', '?', '#', '%'])
        && !s.chars().any(char::is_control)
        && s.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    pub path: String,
    pub sha256: Digest,
    pub bytes: u64,
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<[u32; 2]>,
}
impl FileRef {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            safe_path(&self.path),
            "path",
            &self.path,
            "expected release-local path",
        )?;
        require(
            valid_digest(&self.sha256),
            "digest",
            &self.path,
            "expected lowercase SHA-256",
        )?;
        require(
            self.bytes <= MAX_SAFE_INTEGER && !self.media_type.is_empty(),
            "file",
            &self.path,
            "invalid size or media type",
        )?;
        if self.media_type == "image/png" || self.dimensions.is_some() {
            require(
                self.dimensions
                    .is_some_and(|d| d.iter().all(|n| *n > 0 && *n <= 4096)),
                "dimensions",
                &self.path,
                "image dimensions must be 1..4096",
            )?;
        }
        Ok(())
    }
    pub fn verify(&self, bytes: &[u8]) -> ContractResult<()> {
        self.validate()?;
        require(
            bytes.len() as u64 == self.bytes && digest(bytes) == self.sha256,
            "file_hash",
            &self.path,
            "exact bytes differ",
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseRef {
    pub manifest_path: String,
    pub manifest_sha256: Digest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Autonomous,
    Imported,
    Synthetic,
    Diagnostic,
}
