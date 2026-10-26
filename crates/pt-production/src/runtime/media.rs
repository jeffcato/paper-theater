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
fn read<const N: usize>(reader: &mut impl Read) -> Result<[u8; N]> {
    let mut b = [0; N];
    reader.read_exact(&mut b).map_err(|_| invalid())?;
    Ok(b)
}
pub(crate) fn validate_wav(reader: &mut (impl Read + Seek), size: u64) -> Result<()> {
    if size < 44 || size > MAX_MASTER_BYTES as u64 {
        return Err(invalid());
    }
    reader.seek(SeekFrom::Start(0))?;
    let header = read::<12>(reader)?;
    if &header[..4] != b"RIFF"
        || &header[8..] != b"WAVE"
        || u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64 + 8 != size
    {
        return Err(invalid());
    }
    let mut pos = 12;
    let mut format = None;
    let mut data = false;
    let mut chunks = 0;
    while pos < size {
        chunks += 1;
        if chunks > 1024 || size - pos < 8 {
            return Err(invalid());
        }
        reader.seek(SeekFrom::Start(pos))?;
        let h = read::<8>(reader)?;
        let length = u32::from_le_bytes(h[4..].try_into().unwrap()) as u64;
        let start = pos + 8;
        let end = start.checked_add(length).ok_or_else(invalid)?;
        let next = end.checked_add(length % 2).ok_or_else(invalid)?;
        if next > size {
            return Err(invalid());
        }
        match &h[..4] {
            b"fmt " => {
                if format.is_some() || data || !(length == 16 || length == 18) {
                    return Err(invalid());
                }
                let f = read::<16>(reader)?;
                let word = |i| u16::from_le_bytes([f[i], f[i + 1]]);
                let rate = u32::from_le_bytes(f[4..8].try_into().unwrap());
                let byte_rate = u32::from_le_bytes(f[8..12].try_into().unwrap());
                let channels = word(2);
                let block = word(12);
                if word(0) != 1
                    || !matches!(channels, 1 | 2)
                    || !(8000..=192000).contains(&rate)
                    || word(14) != 16
                    || block != channels * 2
                    || byte_rate != rate * block as u32
                {
                    return Err(invalid());
                }
                if length == 18 && read::<2>(reader)? != [0, 0] {
                    return Err(invalid());
                }
                if size > MAX_ARTIFACT_BYTES as u64 && (channels != 2 || rate != 48000) {
                    return fail("invalid_wav", "chapter masters must be 48 kHz stereo PCM16");
                }
                format = Some(block as u64);
            }
            b"data" => {
                let block = format.ok_or_else(invalid)?;
                if data || length == 0 || length % block != 0 {
                    return Err(invalid());
                }
                data = true;
            }
            _ => {}
        }
        pos = next;
    }
    if !data || format.is_none() {
        return Err(invalid());
    }
    reader.seek(SeekFrom::Start(0))?;
    Ok(())
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
