//! Resource envelopes: only validated delivered WAVs receive the larger file limit.
use super::store::{Error, Result, fail};
use std::io::{Cursor, Read, Seek, SeekFrom};
pub const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_MASTER_BYTES: usize = 256 * 1024 * 1024;
pub fn artifact_limit(media_type: &str) -> usize {
    if media_type == "audio/wav" {
        MAX_MASTER_BYTES
    } else {
        MAX_ARTIFACT_BYTES
    }
}
pub fn validate_artifact_size(size: u64, media_type: &str) -> Result<()> {
    if size > artifact_limit(media_type) as u64 {
        return fail("artifact_too_large", "artifact exceeds its media envelope");
    }
    Ok(())
}
pub fn validate_artifact_bytes(bytes: &[u8], media_type: &str) -> Result<()> {
    validate_artifact_size(bytes.len() as u64, media_type)?;
    if media_type == "audio/wav" {
        validate_wav(&mut Cursor::new(bytes), bytes.len() as u64)?;
    }
    Ok(())
}
fn invalid() -> Error {
    Error::new(
        "invalid_wav",
        "expected a complete PCM16 WAV with consistent RIFF/chunk/frame sizes",
    )
}
pub(crate) fn validate_wav(reader: &mut (impl Read + Seek), size: u64) -> Result<()> {
    pt_contracts::inspect_pcm16_wav(reader, size)
        .map(|_| ())
        .map_err(|_| invalid())
}

/// Verified open handle; HTTP reads this same immutable object after validation.
pub struct VerifiedArtifact {
    pub file: std::fs::File,
    pub bytes: u64,
    pub media_type: String,
    pub sha256: String,
}
impl VerifiedArtifact {
    pub(crate) fn checked(
        mut file: std::fs::File,
        size: u64,
        media_type: String,
        expected: String,
    ) -> Result<Self> {
        use sha2::{Digest, Sha256};
        validate_artifact_size(size, &media_type)?;
        if !super::store::valid_hash(&expected)
            || !file.metadata()?.is_file()
            || file.metadata()?.len() != size
        {
            return fail(
                "corrupt_artifact",
                "artifact size or digest metadata changed",
            );
        }
        if media_type == "audio/wav" {
            validate_wav(&mut file, size)
                .map_err(|_| Error::new("corrupt_artifact", "WAV structure changed"))?;
        }
        file.seek(SeekFrom::Start(0))?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut count = 0u64;
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > size {
                return fail("corrupt_artifact", "artifact grew during verification");
            }
            hash.update(&buffer[..n]);
        }
        if count != size || format!("{:x}", hash.finalize()) != expected {
            return fail("corrupt_artifact", "artifact digest changed");
        }
        file.seek(SeekFrom::Start(0))?;
        Ok(Self {
            file,
            bytes: size,
            media_type,
            sha256: expected,
        })
    }
    pub fn open(root: &std::path::Path, reference: &pt_contracts::FileRef) -> Result<Self> {
        reference
            .validate()
            .map_err(|e| Error::new("invalid_artifact", e.to_string()))?;
        validate_artifact_size(reference.bytes, &reference.media_type)?;
        let root = root.canonicalize()?;
        let mut path = root.clone();
        for part in reference.path.split('/') {
            path.push(part);
            if path.is_symlink() {
                return fail("invalid_path", "symlink in artifact path");
            }
        }
        if !path.canonicalize()?.starts_with(root) || !path.is_file() {
            return fail("invalid_path", "artifact escaped allowed root");
        }
        Self::checked(
            std::fs::File::open(path)?,
            reference.bytes,
            reference.media_type.clone(),
            reference.sha256.clone(),
        )
    }
}
