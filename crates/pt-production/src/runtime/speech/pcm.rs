//! Exact PCM16 frame copying. No resampling, mixing, trimming, silence or timestamp inference.
use super::*;
use std::io::Cursor;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Limits {
    pub transcription_max_wav_bytes: u64,
    pub transcription_max_seconds: u64,
    pub transcription_max_segments: u64,
    #[serde(default = "default_batches")]
    pub transcription_max_batches: u64,
    pub max_unit_text_bytes: u64,
    pub max_units: u64,
}
fn default_batches() -> u64 {
    128
}
impl Limits {
    fn caps(&self) -> Result<(u64, u64, usize, usize)> {
        let b = self.transcription_max_wav_bytes.min(16 * META);
        let s = self.transcription_max_seconds.min(90);
        let n = self.transcription_max_segments.min(64) as usize;
        let k = self.transcription_max_batches.min(256) as usize;
        if b < 45 || s == 0 || n == 0 || k == 0 {
            return fail("speech_pcm_bounds", "empty copy budget");
        }
        Ok((b, s, n, k))
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Segment {
    pub unit_id: String,
    pub audio: c::ArtifactRef,
    pub native_span: [u64; 2],
    pub batch_span: [u64; 2],
    pub cross_cut_uncertainty: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Duration {
    numerator: u64,
    denominator: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Batch {
    pub id: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub frames: u64,
    pub wav_bytes_if_materialized: u64,
    pub duration: Duration,
    pub segments: Vec<Segment>,
    sample_origin: String,
    materialized_audio: Option<Value>,
    proposal: Option<Value>,
    model_payload: Value,
}
fn bad<T>(message: &str) -> Result<T> {
    fail("speech_pcm_plan", message)
}
pub(super) fn recipe() -> String {
    digest(
        &[
            b"native-pcm16-frame-copy-v1".as_slice(),
            include_bytes!("pcm.rs"),
            include_bytes!("../media.rs"),
            include_bytes!("../../../../pt-contracts/src/wav.rs"),
        ]
        .concat(),
    )
}
fn inspect(a: &c::ArtifactRef, b: &[u8]) -> Result<c::Pcm16WavInfo> {
    if a.file.bytes > 8 * META || a.file.media_type != "audio/wav" {
        return bad("native recording cap/MIME");
    }
    a.file.verify(b).map_err(contract)?;
    c::inspect_pcm16_wav(&mut Cursor::new(b), b.len() as u64).map_err(contract)
}
fn mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b)
        .ok_or_else(|| Error::new("speech_pcm_plan", "frame arithmetic overflow"))
}
fn extent(s: [u64; 2]) -> Result<u64> {
    s[1].checked_sub(s[0])
        .filter(|n| *n > 0)
        .ok_or_else(|| Error::new("speech_pcm_plan", "empty/reversed frame span"))
}
fn format(b: &Batch, limits: &Limits) -> Result<()> {
    let (max_bytes, max_seconds, max_segments, _) = limits.caps()?;
    if b.bits_per_sample != 16
        || !matches!(b.channels, 1 | 2)
        || !(8000..=192000).contains(&b.sample_rate)
        || b.frames == 0
        || b.frames > mul(b.sample_rate as u64, max_seconds)?
        || b.wav_bytes_if_materialized
            != mul(b.frames, b.channels as u64 * 2)?
                .checked_add(44)
                .ok_or_else(|| Error::new("speech_pcm_plan", "WAV size overflow"))?
        || b.wav_bytes_if_materialized > max_bytes
        || b.duration.numerator != b.frames
        || b.duration.denominator != b.sample_rate as u64
        || b.segments.is_empty()
        || b.segments.len() > max_segments
        || b.sample_origin != "proposed-batch-local"
        || b.materialized_audio.is_some()
        || b.proposal.is_some()
        || b.model_payload
            != json!({"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":null})
    {
        return bad("format, frame, byte, duration, segment or payload mismatch");
    }
    let mut cursor = 0;
    for s in &b.segments {
        if s.batch_span[0] != cursor
            || s.batch_span[1] > b.frames
            || extent(s.batch_span)? != extent(s.native_span)?
        {
            return bad("invalid batch frame layout");
        }
        cursor = s.batch_span[1];
    }
    if cursor != b.frames {
        return bad("incomplete batch frame layout");
    }
    Ok(())
}
/// Independently enforce all planned/unplanned coverage against the committed take manifest.
pub(super) fn validate(
    plan: &Value,
    manifest: &Value,
    limits: &Limits,
    mut read: impl FnMut(&c::ArtifactRef) -> Result<Vec<u8>>,
    mut active: impl FnMut() -> Result<()>,
) -> Result<Vec<Batch>> {
    let (_, _, _, max_batches) = limits.caps()?;
    if plan["schema_version"] != 1
        || plan["copy_recipe"] != "native-pcm16-concatenation-plan-v1"
        || plan["copy_recipe_code_sha256"]
            != digest(include_bytes!("../../generation/speech_transcription.rs"))
        || plan["expected_prose_supplied"] != false
        || plan["dispatch_permitted"] != false
        || plan["production_approved"] != false
        || plan["batch_materialization_required"] != true
        || !plan["measured_word_intervals"].is_null()
        || !plan["fact_bounds"].is_null()
        || !plan["narration_assembly"].is_null()
        || manifest["production_approved"] != false
        || !manifest["narration_assembly"].is_null()
        || !manifest["delivery_master"].is_null()
    {
        return bad("unsupported plan/authority/sample domain");
    }
    let rows = plan["batches"]
        .as_array()
        .ok_or_else(|| Error::new("speech_pcm_plan", "batch list missing"))?;
    if rows.len() > max_batches {
        return bad("batch fanout cap");
    }
    let batches: Vec<Batch> = serde_json::from_value(plan["batches"].clone())?;
    let takes = manifest["takes"]
        .as_array()
        .ok_or_else(|| Error::new("speech_pcm_plan", "take manifest missing"))?;
    if takes.len() > 32 {
        return bad("take fanout");
    }
    let mut originals = BTreeMap::new();
    let mut coverage: BTreeMap<String, Vec<[u64; 2]>> = BTreeMap::new();
    for (n, t) in takes.iter().enumerate() {
        active()?;
        let unit = t["unit_id"]
            .as_str()
            .ok_or_else(|| Error::new("speech_pcm_plan", "unit missing"))?
            .to_owned();
        let a: c::ArtifactRef = serde_json::from_value(t["audio"].clone())?;
        let b = read(&a)?;
        let f = inspect(&a, &b)?;
        let source: [u64; 2] = serde_json::from_value(t["source_span"].clone())?;
        if extent(source).is_err()
            || t["source_sha256"] != manifest["source_sha256"]
            || t["native_span"] != json!([0, f.frame_count])
            || t["native"]
                != json!({"sample_rate":f.sample_rate,"channels":f.channels,"bits_per_sample":16,"frames":f.frame_count})
            || t["production_approved"] != false
            || !t["global_narration_span"].is_null()
            || originals.insert(unit.clone(), (n, a, f, source)).is_some()
        {
            return bad("take source/native/identity mismatch");
        }
        coverage.insert(unit, Vec::new());
    }
    let mut previous = None;
    for (n, b) in batches.iter().enumerate() {
        active()?;
        format(b, limits)?;
        if b.id != format!("b{n:04}") {
            return bad("batch order/identity changed");
        }
        let mut cursor = 0u64;
        let mut within: Option<(&str, [u64; 2], u64, [u64; 2])> = None;
        for s in &b.segments {
            let (index, a, f, source) = originals
                .get(&s.unit_id)
                .ok_or_else(|| Error::new("speech_pcm_plan", "unknown source unit"))?;
            let len = extent(s.native_span)?;
            if !equal(a, &s.audio)
                || f.sample_rate != b.sample_rate
                || f.channels != b.channels
                || s.native_span[1] > f.frame_count
                || s.batch_span[0] != cursor
                || extent(s.batch_span)? != len
                || s.cross_cut_uncertainty
                    != (s.native_span[0] > 0 || s.native_span[1] < f.frame_count)
            {
                return bad("segment identity/format/extent/cut mismatch");
            }
            if previous
                .is_some_and(|(i, end)| *index < i || (*index == i && s.native_span[0] != end))
            {
                return bad("reordered/repeated/skipped source frames");
            }
            previous = Some((*index, s.native_span[1]));
            if let Some((unit, last, native_frames, last_source)) = within {
                if unit == s.unit_id {
                    if last[1] != s.native_span[0] {
                        return bad("native gap in batch");
                    }
                } else if last[1] != native_frames
                    || s.native_span[0] != 0
                    || last_source[1] != source[0]
                {
                    return bad("batch crosses missing source coverage");
                }
            }
            within = Some((&s.unit_id, s.native_span, f.frame_count, *source));
            cursor = s.batch_span[1];
            coverage.get_mut(&s.unit_id).unwrap().push(s.native_span);
        }
        if cursor != b.frames {
            return bad("batch physical frame count differs");
        }
    }
    let unplanned = plan["unplanned_audio"]
        .as_array()
        .ok_or_else(|| Error::new("speech_pcm_plan", "unplanned list missing"))?;
    if unplanned.len() > 32 {
        return bad("unplanned fanout");
    }
    for u in unplanned {
        let unit = u["unit_id"]
            .as_str()
            .ok_or_else(|| Error::new("speech_pcm_plan", "unplanned unit"))?;
        let (_, a, f, _) = originals
            .get(unit)
            .ok_or_else(|| Error::new("speech_pcm_plan", "unknown unplanned source"))?;
        let s: [u64; 2] = serde_json::from_value(u["native_span"].clone())?;
        if u["audio"] != json!(a) || extent(s).is_err() || s[1] > f.frame_count {
            return bad("unplanned identity/extent");
        }
        coverage.get_mut(unit).unwrap().push(s);
    }
    for (unit, spans) in &mut coverage {
        spans.sort_unstable();
        let mut end = 0;
        for s in spans {
            if s[0] != end {
                return bad("missing or duplicate native coverage");
            }
            end = s[1];
        }
        if end != originals[unit].2.frame_count {
            return bad("native remainder omitted");
        }
    }
    active()?;
    Ok(batches)
}
fn header(b: &Batch) -> Result<Vec<u8>> {
    let bytes = u32::try_from(mul(b.frames, b.channels as u64 * 2)?)
        .map_err(|_| Error::new("speech_pcm_plan", "RIFF length overflow"))?;
    let mut out = Vec::with_capacity(b.wav_bytes_if_materialized as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(
        &(bytes
            .checked_add(36)
            .ok_or_else(|| Error::new("speech_pcm_plan", "RIFF overflow"))?)
        .to_le_bytes(),
    );
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&b.channels.to_le_bytes());
    out.extend_from_slice(&b.sample_rate.to_le_bytes());
    out.extend_from_slice(&(b.sample_rate * b.channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(b.channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&bytes.to_le_bytes());
    Ok(out)
}
pub(super) fn copy(
    batch: &Batch,
    limits: &Limits,
    mut read: impl FnMut(&c::ArtifactRef) -> Result<Vec<u8>>,
    mut active: impl FnMut() -> Result<()>,
) -> Result<Vec<u8>> {
    format(batch, limits)?;
    active()?;
    let mut out = header(batch)?;
    for s in &batch.segments {
        active()?;
        let b = read(&s.audio)?;
        let f = inspect(&s.audio, &b)?;
        if f.channels != batch.channels
            || f.sample_rate != batch.sample_rate
            || s.native_span[1] > f.frame_count
        {
            return bad("native format/span changed during copy");
        }
        let start = f
            .data_offset
            .checked_add(mul(s.native_span[0], f.block_align as u64)?)
            .ok_or_else(|| Error::new("speech_pcm_plan", "source offset overflow"))?
            as usize;
        let end = f
            .data_offset
            .checked_add(mul(s.native_span[1], f.block_align as u64)?)
            .ok_or_else(|| Error::new("speech_pcm_plan", "source offset overflow"))?
            as usize;
        if end <= start || end > b.len() {
            return bad("invalid native copy extent");
        }
        for chunk in b[start..end].chunks(8192) {
            active()?;
            out.extend_from_slice(chunk);
        }
    }
    let physical =
        c::inspect_pcm16_wav(&mut Cursor::new(&out), out.len() as u64).map_err(contract)?;
    if physical.frame_count != batch.frames
        || physical.sample_rate != batch.sample_rate
        || physical.channels != batch.channels
        || physical.file_bytes != batch.wav_bytes_if_materialized
    {
        return bad("materialized WAV differs from plan");
    }
    // Separate pass binds actual emitted PCM to independently reopened, verified recordings.
    for s in &batch.segments {
        active()?;
        let original = read(&s.audio)?;
        let f = inspect(&s.audio, &original)?;
        let block = f.block_align as u64;
        let a = f.data_offset + mul(s.native_span[0], block)?;
        let z = f.data_offset + mul(s.native_span[1], block)?;
        let dest = physical.data_offset + mul(s.batch_span[0], block)?;
        let end = physical.data_offset + mul(s.batch_span[1], block)?;
        if z > original.len() as u64
            || end > out.len() as u64
            || original[a as usize..z as usize] != out[dest as usize..end as usize]
        {
            return bad("emitted PCM identity mismatch");
        }
    }
    active()?;
    Ok(out)
}
#[cfg(test)]
#[path = "pcm_tests.rs"]
mod tests;
