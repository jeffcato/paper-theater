//! Exact diagnostic reconstruction and integer boundary mapping. No allocation of resampled PCM.
use super::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub(super) struct Failure {
    pub outcome: ReviewOutcome,
    pub reason: &'static str,
}
pub(super) type Checked<T> = Result<T, Failure>;
pub(super) fn failure(reason: &'static str) -> Failure {
    Failure {
        outcome: ReviewOutcome::Fail,
        reason,
    }
}
pub(super) fn absent(reason: &'static str) -> Failure {
    Failure {
        outcome: ReviewOutcome::Missing,
        reason,
    }
}
pub(super) fn check(ok: bool, reason: &'static str) -> Checked<()> {
    if ok { Ok(()) } else { Err(failure(reason)) }
}
pub(super) fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Checked<T> {
    serde_json::from_slice(bytes).map_err(|_| failure("malformed_mapping_evidence"))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Map {
    pub schema_version: u32,
    pub source_audio_sha256: Digest,
    pub source_sample_rate: u32,
    pub delivery_sample_rate: u32,
    pub segments: Vec<Placement>,
    pub gaps: Vec<Interval>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Placement {
    pub source: Interval,
    pub destination: Interval,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recipe {
    pub schema_version: u32,
    pub provenance: Provenance,
    pub kind: String,
    pub algorithm: String,
    pub channel_map: String,
    pub narration_sha256: Digest,
    pub delivery_sha256: Digest,
    pub map_sha256: Digest,
    pub source_frames: Sample,
    pub delivery_frames: Sample,
    pub source_omissions: Vec<Interval>,
    pub segments: Vec<Edit>,
    #[serde(default)]
    pub historical: Option<Historical>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Historical {
    pub description: FileRef,
    pub edit_record: FileRef,
    pub edit_implementation: FileRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Edit {
    pub id: Id,
    pub processing_delay_frames: Sample,
    pub compensation_frames: Sample,
    pub trim_start_frames: Sample,
    pub trim_end_frames: Sample,
}
struct Pcm<'a> {
    data: &'a [u8],
    rate: u32,
    channels: u16,
    frames: u64,
}
fn pcm(bytes: &[u8]) -> Checked<Pcm<'_>> {
    crate::runtime::validate_artifact_bytes(bytes, "audio/wav")
        .map_err(|_| failure("invalid_pcm_wav"))?;
    let mut pos = 12;
    let mut rate = 0;
    let mut channels = 0;
    let mut data = &bytes[0..0];
    while pos < bytes.len() {
        let n = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let p = &bytes[pos + 8..pos + 8 + n];
        match &bytes[pos..pos + 4] {
            b"fmt " => {
                channels = u16::from_le_bytes(p[2..4].try_into().unwrap());
                rate = u32::from_le_bytes(p[4..8].try_into().unwrap());
            }
            b"data" => data = p,
            _ => {}
        }
        pos += 8 + n + n % 2;
    }
    Ok(Pcm {
        data,
        rate,
        channels,
        frames: data.len() as u64 / (channels as u64 * 2),
    })
}
fn ceil(n: u64, rate: u32, source_rate: u32) -> u64 {
    ((n as u128 * rate as u128).div_ceil(source_rate as u128)) as u64
}
fn partition(mut parts: Vec<Interval>, frames: u64, reason: &'static str) -> Checked<()> {
    parts.sort();
    let mut cursor = 0;
    for [a, b] in parts {
        check(a == cursor && a < b && b <= frames, reason)?;
        cursor = b;
    }
    check(cursor == frames, reason)
}
struct Segment {
    id: Id,
    source: Interval,
    destination: Interval,
    prefix: u64,
    trim: u64,
}
pub(super) struct Verified {
    segments: Vec<Segment>,
    source_frames: u64,
    source_rate: u32,
    delivery_rate: u32,
}
impl Verified {
    fn coordinate(&self, segment: &Segment, n: u64) -> Checked<u64> {
        let raw =
            segment.prefix + ceil(n - segment.source[0], self.delivery_rate, self.source_rate);
        check(
            raw >= segment.trim
                && raw - segment.trim <= segment.destination[1] - segment.destination[0],
            "interval_trimmed",
        )?;
        Ok(segment.destination[0] + raw - segment.trim)
    }
    /// A start/point at a cut belongs to the following segment; a positive interval end belongs to the preceding segment.
    pub fn point(&self, n: u64, end: bool) -> Checked<u64> {
        let segment = self
            .segments
            .iter()
            .find(|s| {
                if end {
                    s.source[0] < n && n <= s.source[1]
                } else {
                    s.source[0] <= n && n < s.source[1]
                }
            })
            .ok_or_else(|| absent("coordinate_unmapped"))?;
        self.coordinate(segment, n)
    }
    pub fn interval(&self, [a, b]: Interval) -> Checked<(u64, Vec<Value>)> {
        check(a < b && b <= self.source_frames, "interval_domain_invalid")?;
        let mut cursor = a;
        let mut pieces = Vec::new();
        let mut bound = 0;
        for s in &self.segments {
            if s.source[1] <= cursor {
                continue;
            }
            if s.source[0] >= b {
                break;
            }
            if s.source[0] > cursor {
                return Err(absent("interval_unmapped"));
            }
            let end = b.min(s.source[1]);
            let mapped_start = self.coordinate(s, cursor)?;
            let mapped_end = self.coordinate(s, end)?;
            check(mapped_start < mapped_end, "interval_not_represented")?;
            bound = bound.max(mapped_end);
            pieces.push(json!({"segment_id":s.id,"source":[cursor,end],"destination":[mapped_start,mapped_end]}));
            cursor = end;
            if cursor == b {
                return Ok((bound, pieces));
            }
        }
        Err(absent("interval_unmapped"))
    }
}
/// Verifies all physical output frames, including declared silence, before exposing any mapping operation.
pub(super) fn verify(
    map_bytes: &[u8],
    recipe_bytes: &[u8],
    source_bytes: &[u8],
    delivery_bytes: &[u8],
) -> Checked<Verified> {
    let m: Map = decode(map_bytes)?;
    let r: Recipe = decode(recipe_bytes)?;
    let source = pcm(source_bytes)?;
    let delivery = pcm(delivery_bytes)?;
    check(
        m.schema_version == 1
            && r.schema_version == 1
            && r.provenance == Provenance::Diagnostic
            && r.kind == "pcm-reconstruction-recipe; not an original execution receipt"
            && r.algorithm == "rational-zero-order-hold-pcm16-v1"
            && r.channel_map == "mono-duplicate-stereo",
        "unsupported_reconstruction_recipe",
    )?;
    check(
        m.source_audio_sha256 == digest(source_bytes)
            && r.narration_sha256 == m.source_audio_sha256
            && r.delivery_sha256 == digest(delivery_bytes)
            && r.map_sha256 == digest(map_bytes)
            && m.source_sample_rate == source.rate
            && m.delivery_sample_rate == delivery.rate
            && source.channels == 1
            && delivery.channels == 2
            && delivery.rate == 48000
            && r.source_frames == source.frames
            && r.delivery_frames == delivery.frames,
        "mapping_binding_mismatch",
    )?;
    check(
        !m.segments.is_empty()
            && m.segments.len() <= 1024
            && m.gaps.len() <= 1024
            && r.source_omissions.len() <= 1024
            && m.segments.len() == r.segments.len(),
        "mapping_domain_invalid",
    )?;
    let mut previous_source = 0;
    let mut previous_delivery = 0;
    let mut ids = std::collections::BTreeSet::new();
    let mut segments = Vec::new();
    for (p, e) in m.segments.iter().zip(&r.segments) {
        let [a, b] = p.source;
        let [c, d] = p.destination;
        check(
            a < b
                && b <= source.frames
                && a >= previous_source
                && c < d
                && d <= delivery.frames
                && c >= previous_delivery
                && valid_id(&e.id)
                && ids.insert(&e.id),
            "mapping_segment_invalid",
        )?;
        check(
            [
                e.processing_delay_frames,
                e.compensation_frames,
                e.trim_start_frames,
                e.trim_end_frames,
            ]
            .iter()
            .all(|v| *v <= MAX_SAFE_INTEGER)
                && e.compensation_frames <= e.processing_delay_frames,
            "mapping_edit_invalid",
        )?;
        let n = ceil(b - a, delivery.rate, source.rate);
        let prefix = e.processing_delay_frames - e.compensation_frames;
        let retained = n
            .checked_add(prefix)
            .and_then(|n| n.checked_sub(e.trim_start_frames))
            .and_then(|n| n.checked_sub(e.trim_end_frames));
        check(retained == Some(d - c), "mapping_edit_length_mismatch")?;
        for k in 0..d - c {
            let raw = k + e.trim_start_frames;
            let expected = if raw < prefix {
                [0, 0, 0, 0]
            } else {
                let index = a
                    + ((raw - prefix) as u128 * source.rate as u128 / delivery.rate as u128) as u64;
                check(index < b, "mapping_edit_invalid")?;
                let q = &source.data[index as usize * 2..index as usize * 2 + 2];
                [q[0], q[1], q[0], q[1]]
            };
            let actual = &delivery.data[(c + k) as usize * 4..(c + k) as usize * 4 + 4];
            check(actual == expected, "delivered_pcm_mismatch")?;
        }
        previous_source = b;
        previous_delivery = d;
        segments.push(Segment {
            id: e.id.clone(),
            source: p.source,
            destination: p.destination,
            prefix,
            trim: e.trim_start_frames,
        });
    }
    partition(
        m.segments
            .iter()
            .map(|s| s.source)
            .chain(r.source_omissions.iter().copied())
            .collect(),
        source.frames,
        "source_partition_invalid",
    )?;
    partition(
        m.segments
            .iter()
            .map(|s| s.destination)
            .chain(m.gaps.iter().copied())
            .collect(),
        delivery.frames,
        "delivery_partition_invalid",
    )?;
    for [a, b] in m.gaps {
        check(
            delivery.data[a as usize * 4..b as usize * 4]
                .iter()
                .all(|b| *b == 0),
            "declared_gap_not_silent",
        )?;
    }
    Ok(Verified {
        segments,
        source_frames: source.frames,
        source_rate: source.rate,
        delivery_rate: delivery.rate,
    })
}
