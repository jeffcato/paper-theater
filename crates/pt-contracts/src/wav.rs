//! Stateless bounded PCM16 inspection, preserving R11 native admission semantics.
use crate::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
pub const MAX_PCM16_WAV_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_NATIVE_PCM16_WAV_BYTES: u64 = 32 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pcm16WavInfo {
    pub encoding: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub block_align: u16,
    pub byte_rate: u32,
    pub file_bytes: u64,
    pub data_offset: u64,
    pub data_bytes: u64,
    pub frame_count: u64,
}
fn invalid_wav() -> ContractError {
    ContractError {
        code: "invalid_wav".into(),
        field: "audio".into(),
        message: "expected complete bounded PCM16 RIFF/WAVE with consistent chunks and frames"
            .into(),
    }
}
fn read<const N: usize>(r: &mut impl Read) -> ContractResult<[u8; N]> {
    let mut b = [0; N];
    r.read_exact(&mut b).map_err(|_| invalid_wav())?;
    Ok(b)
}
/// Native mono/stereo PCM16: 8..192 kHz below 32 MiB; larger files require delivery domain.
/// Uses fixed buffers/seeks only, checks physical extent, and rewinds on success.
pub fn inspect_pcm16_wav(r: &mut (impl Read + Seek), size: u64) -> ContractResult<Pcm16WavInfo> {
    if size < 44 || size > MAX_PCM16_WAV_BYTES {
        return Err(invalid_wav());
    }
    if r.seek(SeekFrom::End(0)).map_err(|_| invalid_wav())? != size {
        return Err(invalid_wav());
    }
    r.seek(SeekFrom::Start(0)).map_err(|_| invalid_wav())?;
    let h = read::<12>(r)?;
    if &h[..4] != b"RIFF"
        || &h[8..] != b"WAVE"
        || u32::from_le_bytes(h[4..8].try_into().unwrap()) as u64 + 8 != size
    {
        return Err(invalid_wav());
    }
    let mut pos = 12;
    let mut format = None;
    let mut data = None;
    let mut chunks = 0;
    while pos < size {
        chunks += 1;
        if chunks > 1024 || size - pos < 8 {
            return Err(invalid_wav());
        }
        r.seek(SeekFrom::Start(pos)).map_err(|_| invalid_wav())?;
        let h = read::<8>(r)?;
        let length = u32::from_le_bytes(h[4..].try_into().unwrap()) as u64;
        let start = pos + 8;
        let end = start.checked_add(length).ok_or_else(invalid_wav)?;
        let next = end.checked_add(length % 2).ok_or_else(invalid_wav)?;
        if next > size {
            return Err(invalid_wav());
        }
        match &h[..4] {
            b"fmt " => {
                if format.is_some() || data.is_some() || !matches!(length, 16 | 18) {
                    return Err(invalid_wav());
                }
                let f = read::<16>(r)?;
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
                    return Err(invalid_wav());
                }
                if length == 18 && read::<2>(r)? != [0, 0] {
                    return Err(invalid_wav());
                }
                if size > MAX_NATIVE_PCM16_WAV_BYTES && (channels != 2 || rate != 48000) {
                    return Err(invalid_wav());
                }
                format = Some((channels, rate, block, byte_rate));
            }
            b"data" => {
                let (_, _, block, _) = format.ok_or_else(invalid_wav)?;
                if data.is_some() || length == 0 || length % block as u64 != 0 {
                    return Err(invalid_wav());
                }
                data = Some((start, length));
            }
            _ => {}
        }
        pos = next;
    }
    let (channels, sample_rate, block_align, byte_rate) = format.ok_or_else(invalid_wav)?;
    let (data_offset, data_bytes) = data.ok_or_else(invalid_wav)?;
    r.seek(SeekFrom::Start(0)).map_err(|_| invalid_wav())?;
    Ok(Pcm16WavInfo {
        encoding: 1,
        channels,
        sample_rate,
        bits_per_sample: 16,
        block_align,
        byte_rate,
        file_bytes: size,
        data_offset,
        data_bytes,
        frame_count: data_bytes / block_align as u64,
    })
}
pub fn inspect_delivery_wav(r: &mut (impl Read + Seek), size: u64) -> ContractResult<Pcm16WavInfo> {
    let info = inspect_pcm16_wav(r, size)?;
    info.require_delivery_domain()?;
    Ok(info)
}
impl Pcm16WavInfo {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.encoding == 1
                && matches!(self.channels, 1 | 2)
                && (8000..=192000).contains(&self.sample_rate)
                && self.bits_per_sample == 16
                && self.block_align == self.channels * 2
                && self.byte_rate == self.sample_rate * self.block_align as u32
                && self.file_bytes >= 44
                && self.file_bytes <= MAX_PCM16_WAV_BYTES
                && self.data_offset >= 44
                && self.data_offset % 2 == 0
                && self.data_bytes > 0
                && self
                    .data_offset
                    .checked_add(self.data_bytes)
                    .is_some_and(|n| n <= self.file_bytes)
                && self.frame_count.checked_mul(self.block_align as u64) == Some(self.data_bytes)
                && (self.file_bytes <= MAX_NATIVE_PCM16_WAV_BYTES
                    || (self.channels == 2 && self.sample_rate == 48000)),
            "audio_format",
            "format",
            "invalid native PCM16 metadata",
        )
    }
    pub fn require_delivery_domain(&self) -> ContractResult<()> {
        self.validate()?;
        require(
            self.channels == 2 && self.sample_rate == 48000,
            "audio_domain",
            "format",
            "delivery requires stereo 48 kHz PCM16 at every file size",
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryAudioObservationV1 {
    pub schema_version: u32,
    pub master_sha256: Digest,
    pub timeline_sha256: Digest,
    pub wav_label: bool,
    pub format: Pcm16WavInfo,
    pub manifest_sample_rate: u32,
    pub timeline_sample_rate: u32,
    pub manifest_frame_count: u64,
    pub timeline_frame_count: u64,
}
impl DeliveryAudioObservationV1 {
    pub fn validate(&self) -> ContractResult<()> {
        self.format.validate()?;
        require(
            self.schema_version == 1
                && valid_digest(&self.master_sha256)
                && valid_digest(&self.timeline_sha256)
                && self.manifest_sample_rate > 0
                && self.manifest_sample_rate <= 192000
                && self.timeline_sample_rate > 0
                && self.timeline_sample_rate <= 192000
                && self.manifest_frame_count > 0
                && self.manifest_frame_count <= MAX_SAFE_INTEGER
                && self.timeline_frame_count > 0
                && self.timeline_frame_count <= MAX_SAFE_INTEGER,
            "audio_format",
            "delivery_audio",
            "invalid observed delivery binding",
        )
    }
    pub fn matches_delivery(&self) -> bool {
        self.wav_label
            && self.format.require_delivery_domain().is_ok()
            && self.manifest_sample_rate == 48000
            && self.timeline_sample_rate == 48000
            && self.manifest_frame_count == self.format.frame_count
            && self.timeline_frame_count == self.format.frame_count
    }
}
