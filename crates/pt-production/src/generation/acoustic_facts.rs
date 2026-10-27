//! Pure source-fact matching in the original narration sample domain.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
#[derive(Debug)]
struct Failure {
    outcome: ReviewOutcome,
    reason: &'static str,
}
type Checked<T> = Result<T, Failure>;
fn fail(reason: &'static str) -> Failure {
    Failure {
        outcome: ReviewOutcome::Fail,
        reason,
    }
}
fn missing_fact(reason: &'static str) -> Failure {
    Failure {
        outcome: ReviewOutcome::Missing,
        reason,
    }
}
fn uncertain(reason: &'static str) -> Failure {
    Failure {
        outcome: ReviewOutcome::Uncertain,
        reason,
    }
}
fn check(ok: bool, reason: &'static str) -> Checked<()> {
    if ok { Ok(()) } else { Err(fail(reason)) }
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Checked<T> {
    serde_json::from_slice(bytes).map_err(|_| fail("malformed_evidence"))
}
fn file<'a>(files: &'a Files, path: &str) -> Checked<&'a (FileRef, Vec<u8>)> {
    let entry = files
        .get(path)
        .ok_or_else(|| missing_fact("evidence_file_missing"))?;
    check(entry.0.sha256 == digest(&entry.1), "artifact_hash_mismatch")?;
    Ok(entry)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema_version: u32,
    coordinate_system: String,
    source_sha256: Digest,
    narration_sha256: Digest,
    acoustic_manifest_sha256: Digest,
    sample_rate: u32,
    primary_recipe_sha256: Digest,
    independent_receipts: Vec<ReceiptPin>,
    required_fact_ids: Vec<Id>,
    intervals: Vec<IntervalSpec>,
    facts: Vec<FactSpec>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptPin {
    path: String,
    sha256: Digest,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IntervalSpec {
    id: Id,
    source_span: SourceSpan,
    primary_unit_ids: Vec<Id>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FactSpec {
    id: Id,
    source_spans: Vec<SourceSpan>,
    interval_ids: Vec<Id>,
}
#[derive(Deserialize)]
struct Manifest {
    source_sha256: Digest,
    audio_sha256: Digest,
    sample_rate: u32,
    source_span_characters: SourceSpan,
    units: Vec<Unit>,
}
#[derive(Deserialize)]
struct Unit {
    turn: u32,
    text: String,
    source_span_characters: SourceSpan,
    audio: String,
    audio_sha256: Digest,
    start_sample: Sample,
    end_sample: Sample,
}
#[derive(Deserialize)]
struct Cached {
    model: String,
    units: Vec<CachedUnit>,
}
#[derive(Deserialize)]
struct CachedUnit {
    case: String,
    request_sha256: Digest,
    mappings: Vec<Mapping>,
}
#[derive(Deserialize)]
struct Mapping {
    source_range: [usize; 2],
    heard_range: [usize; 2],
}
#[derive(Deserialize)]
struct Receipt {
    model: String,
    request_sha256: Digest,
    response_sha256: Digest,
    audio_sha256: Digest,
    #[serde(default)]
    harness_sha256: Digest,
    #[serde(default)]
    case: String,
}
#[derive(Deserialize)]
struct PrimaryResponse {
    candidates: Vec<Candidate>,
}
#[derive(Deserialize)]
struct Candidate {
    #[serde(rename = "finishReason")]
    finish_reason: String,
    content: Content,
}
#[derive(Deserialize)]
struct Content {
    parts: Vec<Part>,
}
#[derive(Deserialize)]
struct Part {
    #[serde(rename = "audioTranscription")]
    audio_transcription: Option<Transcription>,
}
#[derive(Deserialize)]
struct Transcription {
    words: Vec<RawWord>,
}
#[derive(Deserialize)]
struct RawWord {
    word: String,
    #[serde(rename = "startOffset")]
    start: String,
    #[serde(rename = "endOffset")]
    end: String,
}
#[derive(Deserialize)]
struct Meta {
    transcript: String,
    turns: Vec<MetaTurn>,
}
#[derive(Deserialize)]
struct MetaTurn {
    #[serde(rename = "startMs")]
    start: serde_json::Number,
    #[serde(rename = "endMs")]
    end: serde_json::Number,
    transcript: String,
}

#[derive(Clone)]
struct Token {
    text: String,
    span: SourceSpan,
}
fn lex(text: &str) -> Vec<Token> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_alphanumeric() {
            i += 1;
            continue;
        }
        let start = i;
        let mut word = String::new();
        while i < chars.len() {
            if chars[i].is_alphanumeric() {
                word.extend(chars[i].to_lowercase());
                i += 1;
            } else if matches!(chars[i], '\'' | '’')
                && i + 1 < chars.len()
                && chars[i + 1].is_alphanumeric()
            {
                i += 1;
            } else {
                break;
            }
        }
        result.push(Token {
            text: word,
            span: [start as u64, i as u64],
        });
    }
    result
}
fn tokens(text: &str) -> Vec<String> {
    lex(text).into_iter().map(|w| w.text).collect()
}
fn occurrences(haystack: &[String], needle: &[String]) -> usize {
    if needle.is_empty() {
        0
    } else {
        haystack
            .windows(needle.len())
            .filter(|w| *w == needle)
            .count()
    }
}
fn unique<T: Ord>(values: impl Iterator<Item = T>) -> bool {
    let v = values.collect::<Vec<_>>();
    v.iter().collect::<BTreeSet<_>>().len() == v.len()
}
fn text<'a>(source: &'a [char], span: SourceSpan) -> Checked<String> {
    let [a, b] = span;
    check(a < b && b <= source.len() as u64, "source_span_invalid")?;
    Ok(source[a as usize..b as usize].iter().collect())
}
// Exact decimal-to-sample ceiling. No binary-float timestamp subtraction.
fn samples(decimal: &str, rate: u32, units_per_second: u128) -> Checked<Sample> {
    let parts = decimal.split(['e', 'E']).collect::<Vec<_>>();
    check(parts.len() <= 2, "timestamp_invalid")?;
    let exponent = if parts.len() == 2 {
        parts[1]
            .parse::<i32>()
            .map_err(|_| fail("timestamp_invalid"))?
    } else {
        0
    };
    check((-18..=18).contains(&exponent), "timestamp_invalid")?;
    let number = parts[0].split('.').collect::<Vec<_>>();
    check(number.len() <= 2, "timestamp_invalid")?;
    let digits = number.concat();
    check(
        !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit()) && digits.len() <= 30,
        "timestamp_invalid",
    )?;
    let fraction = if number.len() == 2 {
        number[1].len()
    } else {
        0
    };
    check(fraction <= 18, "timestamp_invalid")?;
    let mut numerator = digits
        .parse::<u128>()
        .map_err(|_| fail("timestamp_invalid"))?
        .checked_mul(rate as u128)
        .ok_or_else(|| fail("timestamp_invalid"))?;
    let mut denominator = 10u128
        .pow(fraction as u32)
        .checked_mul(units_per_second)
        .ok_or_else(|| fail("timestamp_invalid"))?;
    if exponent > 0 {
        numerator = numerator
            .checked_mul(10u128.pow(exponent as u32))
            .ok_or_else(|| fail("timestamp_invalid"))?;
    }
    if exponent < 0 {
        denominator = denominator
            .checked_mul(10u128.pow((-exponent) as u32))
            .ok_or_else(|| fail("timestamp_invalid"))?;
    }
    let result = numerator / denominator + u128::from(numerator % denominator != 0);
    check(result <= MAX_SAFE_INTEGER as u128, "timestamp_invalid")?;
    Ok(result as u64)
}
fn word_sample(value: &str, rate: u32) -> Checked<Sample> {
    samples(
        value
            .strip_suffix('s')
            .ok_or_else(|| fail("timestamp_invalid"))?,
        rate,
        1,
    )
}
fn wav<'a>(bytes: &'a [u8], rate: u32) -> Checked<&'a [u8]> {
    check(
        bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "audio_format_invalid",
    )?;
    let mut pos = 12;
    let mut format = None;
    let mut data = None;
    while pos + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let end = (pos + 8)
            .checked_add(size)
            .ok_or_else(|| fail("audio_format_invalid"))?;
        check(end <= bytes.len(), "audio_format_invalid")?;
        match &bytes[pos..pos + 4] {
            b"fmt " => {
                check(size >= 16, "audio_format_invalid")?;
                let p = &bytes[pos + 8..end];
                format = Some((
                    u16::from_le_bytes(p[..2].try_into().unwrap()),
                    u16::from_le_bytes(p[2..4].try_into().unwrap()),
                    u32::from_le_bytes(p[4..8].try_into().unwrap()),
                    u16::from_le_bytes(p[12..14].try_into().unwrap()),
                    u16::from_le_bytes(p[14..16].try_into().unwrap()),
                ));
            }
            b"data" => {
                check(data.is_none(), "audio_format_invalid")?;
                data = Some(&bytes[pos + 8..end]);
            }
            _ => {}
        }
        pos = end + size % 2;
    }
    check(format == Some((1, 1, rate, 2, 16)), "audio_format_invalid")?;
    let data = data.ok_or_else(|| fail("audio_format_invalid"))?;
    check(
        !data.is_empty() && data.len() % 2 == 0,
        "audio_format_invalid",
    )?;
    Ok(data)
}

struct Primary {
    ordinal: usize,
    span: SourceSpan,
    tokens: Vec<String>,
    start: Sample,
    end: Sample,
    positive_terminal: bool,
    evidence: Value,
}
struct Independent {
    full: Vec<String>,
    all_turns: Vec<String>,
    turns: Vec<Turn>,
    model: String,
    evidence: Value,
}
struct Turn {
    tokens: Vec<String>,
    start: Sample,
    end: Sample,
    index: usize,
}
struct Context {
    source: Vec<char>,
    source_tokens: Vec<String>,
    primary_tokens: Vec<String>,
    primary: BTreeMap<Id, Primary>,
    independent: Vec<Independent>,
    rate: u32,
}

fn context(inv: &Inventory, files: &Files, normalized: &Value) -> Checked<Context> {
    let (source_ref, source_bytes) = file(files, "source.txt")?;
    let source = std::str::from_utf8(source_bytes)
        .map_err(|_| fail("source_utf8_invalid"))?
        .chars()
        .collect::<Vec<_>>();
    let (manifest_ref, manifest_bytes) = file(files, "manifest.json")?;
    let manifest: Manifest = decode(manifest_bytes)?;
    let (audio_ref, audio_bytes) = file(files, "excerpt.wav")?;
    let full_pcm = wav(audio_bytes, manifest.sample_rate)?;
    let frames = (full_pcm.len() / 2) as u64;
    check(
        inv.schema_version == 1
            && inv.coordinate_system == "excerpt_unicode_scalars"
            && inv.source_sha256 == source_ref.sha256
            && inv.source_sha256 == manifest.source_sha256
            && inv.narration_sha256 == audio_ref.sha256
            && inv.narration_sha256 == manifest.audio_sha256
            && inv.acoustic_manifest_sha256 == manifest_ref.sha256
            && inv.sample_rate == manifest.sample_rate
            && inv.sample_rate > 0
            && valid_digest(&inv.primary_recipe_sha256),
        "fact_inventory_binding_mismatch",
    )?;
    check(
        !inv.required_fact_ids.is_empty()
            && unique(inv.required_fact_ids.iter())
            && unique(inv.intervals.iter().map(|i| &i.id))
            && unique(inv.facts.iter().map(|f| &f.id))
            && inv
                .required_fact_ids
                .iter()
                .all(|id| valid_id(id) && id.len() <= 180)
            && inv
                .intervals
                .iter()
                .all(|i| valid_id(&i.id) && i.id.len() <= 180)
            && inv
                .facts
                .iter()
                .all(|f| inv.required_fact_ids.contains(&f.id)),
        "fact_inventory_ids_invalid",
    )?;
    let cached: Cached = decode(&file(files, "segmented-acoustic-report.json")?.1)?;
    check(
        cached.units.len() == manifest.units.len(),
        "primary_coverage_missing",
    )?;
    let normalized_words = normalized["word_track"]
        .as_array()
        .ok_or_else(|| fail("normalized_primary_missing"))?;
    let normalized_inputs = normalized["primary_inputs"]
        .as_array()
        .ok_or_else(|| fail("normalized_primary_missing"))?;
    let base = manifest.source_span_characters[0];
    let mut primary = BTreeMap::new();
    let mut primary_tokens = Vec::new();
    let mut previous_source = 0;
    let mut previous_audio = 0;
    for (ordinal, (unit, saved)) in manifest.units.iter().zip(&cached.units).enumerate() {
        let id = format!("turn-{:03}", unit.turn);
        check(
            id == saved.case && valid_digest(&saved.request_sha256),
            "primary_request_mismatch",
        )?;
        let a = unit.source_span_characters[0]
            .checked_sub(base)
            .ok_or_else(|| fail("primary_source_span_invalid"))?;
        let b = unit.source_span_characters[1]
            .checked_sub(base)
            .ok_or_else(|| fail("primary_source_span_invalid"))?;
        check(
            a >= previous_source
                && unit.start_sample >= previous_audio
                && unit.start_sample < unit.end_sample
                && unit.end_sample <= frames,
            "primary_interval_invalid",
        )?;
        check(
            text(&source, [a, b])? == unit.text,
            "primary_source_text_mismatch",
        )?;
        previous_source = b;
        previous_audio = unit.end_sample;
        let (unit_ref, unit_bytes) = file(files, &unit.audio)?;
        let pcm = wav(unit_bytes, manifest.sample_rate)?;
        check(
            unit_ref.sha256 == unit.audio_sha256
                && pcm.len() / 2 == (unit.end_sample - unit.start_sample) as usize
                && pcm == &full_pcm[unit.start_sample as usize * 2..unit.end_sample as usize * 2],
            "primary_audio_domain_mismatch",
        )?;
        let receipt_path = format!("transcribe/{}.receipt.json", saved.request_sha256);
        let (receipt_ref, receipt_bytes) = file(files, &receipt_path)?;
        let receipt: Receipt = decode(receipt_bytes)?;
        let raw_path = format!("transcribe/{}.json", saved.request_sha256);
        let (raw_ref, raw_bytes) = file(files, &raw_path)?;
        check(
            receipt.case == id
                && receipt.model == cached.model
                && receipt.request_sha256 == saved.request_sha256
                && receipt.response_sha256 == raw_ref.sha256
                && receipt.audio_sha256 == unit_ref.sha256
                && receipt.harness_sha256 == inv.primary_recipe_sha256,
            "primary_receipt_binding_mismatch",
        )?;
        let records = normalized_inputs
            .iter()
            .filter(|r| r["case"] == id)
            .collect::<Vec<_>>();
        check(
            records.len() == 1
                && records[0]["request_sha256"] == receipt.request_sha256
                && records[0]["response_sha256"] == receipt.response_sha256
                && records[0]["audio_sha256"] == receipt.audio_sha256
                && records[0]["model"] == receipt.model,
            "normalized_primary_binding_mismatch",
        )?;
        let raw: PrimaryResponse = decode(raw_bytes)?;
        check(
            raw.candidates.len() == 1 && raw.candidates[0].finish_reason == "STOP",
            "primary_response_incomplete",
        )?;
        let words = raw
            .candidates
            .into_iter()
            .next()
            .unwrap()
            .content
            .parts
            .into_iter()
            .filter_map(|p| p.audio_transcription)
            .flat_map(|t| t.words)
            .collect::<Vec<_>>();
        check(!words.is_empty(), "primary_words_missing")?;
        let mut measured = Vec::new();
        let mut last = [0, 0];
        let mut heard = Vec::new();
        for word in &words {
            let start = word_sample(&word.start, manifest.sample_rate)?;
            let end = word_sample(&word.end, manifest.sample_rate)?;
            check(
                start <= end
                    && end <= unit.end_sample - unit.start_sample
                    && start >= last[0]
                    && end >= last[1],
                "primary_word_interval_invalid",
            )?;
            last = [start, end];
            measured.push([unit.start_sample + start, unit.start_sample + end]);
            heard.extend(tokens(&word.word));
        }
        let source_words = lex(&unit.text);
        check(
            saved.mappings.len() == source_words.len(),
            "primary_coverage_missing",
        )?;
        let mut heard_cursor = 0;
        for (j, mapping) in saved.mappings.iter().enumerate() {
            let [c, d] = mapping.heard_range;
            check(
                mapping.source_range == [j, j + 1]
                    && c == heard_cursor
                    && c < d
                    && d <= words.len(),
                "primary_coverage_missing",
            )?;
            heard_cursor = d;
            let span = [
                unit.source_span_characters[0] + source_words[j].span[0],
                unit.source_span_characters[0] + source_words[j].span[1],
            ];
            let matching = normalized_words
                .iter()
                .filter(|w| w["source_span_characters"] == json!(span))
                .collect::<Vec<_>>();
            let recognized = words[c..d]
                .iter()
                .map(|w| w.word.clone())
                .collect::<Vec<_>>();
            check(
                matching.len() == 1
                    && matching[0]["recognized_words"] == json!(recognized)
                    && matching[0]["start_sample"] == measured[c][0]
                    && matching[0]["end_sample"] == measured[d - 1][1],
                "normalized_primary_measurement_mismatch",
            )?;
        }
        check(heard_cursor == words.len(), "primary_coverage_missing")?;
        primary_tokens.extend(heard.iter().cloned());
        primary.insert(id.clone(),Primary{ordinal,span:[a,b],tokens:heard,start:measured[0][0],end:measured.last().unwrap()[1],positive_terminal:last[0]<last[1],
            evidence:json!({"unit_id":saved.case,"model":receipt.model,"request_sha256":receipt.request_sha256,"provider_recipe_sha256":receipt.harness_sha256,
                "response":raw_ref,"receipt":receipt_ref,"unit_audio":unit_ref,"source_span":[a,b],"audio_placement":[unit.start_sample,unit.end_sample]})});
    }
    if inv.independent_receipts.is_empty() {
        return Err(missing_fact("independent_receipts_missing"));
    }
    check(
        unique(inv.independent_receipts.iter().map(|p| &p.path)),
        "independent_receipt_ambiguous",
    )?;
    let mut independent = Vec::new();
    for pin in &inv.independent_receipts {
        check(
            pin.path.starts_with("meta/")
                && pin.path.ends_with(".receipt.json")
                && valid_digest(&pin.sha256),
            "independent_receipt_path_invalid",
        )?;
        let (receipt_ref, receipt_bytes) = file(files, &pin.path)?;
        check(
            receipt_ref.sha256 == pin.sha256,
            "independent_receipt_hash_mismatch",
        )?;
        let receipt: Receipt = decode(receipt_bytes)?;
        check(
            valid_digest(&receipt.request_sha256)
                && pin.path == format!("meta/{}.receipt.json", receipt.request_sha256)
                && receipt.audio_sha256 == audio_ref.sha256
                && !receipt.model.is_empty(),
            "independent_receipt_binding_mismatch",
        )?;
        check(
            receipt.model.to_lowercase() != cached.model.to_lowercase(),
            "models_not_independent",
        )?;
        let raw_path = format!("meta/{}.json", receipt.request_sha256);
        let (raw_ref, raw_bytes) = file(files, &raw_path)?;
        check(
            receipt.response_sha256 == raw_ref.sha256,
            "independent_response_hash_mismatch",
        )?;
        let raw: Meta = decode(raw_bytes)?;
        check(!raw.turns.is_empty(), "independent_turns_missing")?;
        let records = normalized["independent"]
            .as_array()
            .ok_or_else(|| fail("normalized_independent_missing"))?
            .iter()
            .filter(|r| r["request_sha256"] == receipt.request_sha256)
            .collect::<Vec<_>>();
        check(
            records.len() == 1
                && records[0]["response_sha256"] == receipt.response_sha256
                && records[0]["model"] == receipt.model,
            "normalized_independent_binding_mismatch",
        )?;
        let norm_turns = records[0]["turns"]
            .as_array()
            .ok_or_else(|| fail("normalized_independent_missing"))?;
        check(
            norm_turns.len() == raw.turns.len(),
            "independent_coverage_missing",
        )?;
        let mut turns = Vec::new();
        let mut last_end = 0;
        for (index, (turn, norm)) in raw.turns.iter().zip(norm_turns).enumerate() {
            let start = samples(&turn.start.to_string(), manifest.sample_rate, 1000)?;
            let end = samples(&turn.end.to_string(), manifest.sample_rate, 1000)?;
            check(
                start < end && end <= frames && start >= last_end,
                "independent_interval_invalid",
            )?;
            last_end = end;
            let nstart = norm["start"]
                .as_number()
                .ok_or_else(|| fail("normalized_independent_missing"))?;
            let nend = norm["end"]
                .as_number()
                .ok_or_else(|| fail("normalized_independent_missing"))?;
            check(
                norm["text"] == turn.transcript
                    && samples(&nstart.to_string(), manifest.sample_rate, 1)? == start
                    && samples(&nend.to_string(), manifest.sample_rate, 1)? == end,
                "normalized_independent_measurement_mismatch",
            )?;
            turns.push(Turn {
                tokens: tokens(&turn.transcript),
                start,
                end,
                index,
            });
        }
        let all_turns = turns
            .iter()
            .flat_map(|t| t.tokens.iter().cloned())
            .collect();
        independent.push(Independent{full:tokens(&raw.transcript),all_turns,turns,model:receipt.model.clone(),
            evidence:json!({"model":receipt.model,"request_sha256":receipt.request_sha256,"response":raw_ref,"receipt":receipt_ref,"narration":audio_ref})});
    }
    Ok(Context {
        source_tokens: tokens(&source.iter().collect::<String>()),
        source,
        primary_tokens,
        primary,
        independent,
        rate: manifest.sample_rate,
    })
}

#[derive(Serialize)]
struct IntervalResult {
    id: Id,
    source_span: SourceSpan,
    primary_unit_ids: Vec<Id>,
    outcome: ReviewOutcome,
    reason: String,
    primary_start_sample: Option<Sample>,
    primary_end_sample: Option<Sample>,
    independent_start_sample: Option<Sample>,
    independent_end_sample: Option<Sample>,
    not_before_sample: Option<Sample>,
    primary_evidence: Vec<Value>,
    independent_evidence: Option<Value>,
}
fn interval(spec: &IntervalSpec, ctx: &Context) -> Checked<IntervalResult> {
    check(
        !spec.primary_unit_ids.is_empty() && unique(spec.primary_unit_ids.iter()),
        "primary_interval_ambiguous",
    )?;
    let selected = spec
        .primary_unit_ids
        .iter()
        .map(|id| {
            ctx.primary
                .get(id)
                .ok_or_else(|| missing_fact("primary_unit_missing"))
        })
        .collect::<Checked<Vec<_>>>()?;
    check(
        selected
            .windows(2)
            .all(|p| p[1].ordinal == p[0].ordinal + 1)
            && spec.source_span == [selected[0].span[0], selected.last().unwrap().span[1]],
        "primary_interval_coverage_incomplete",
    )?;
    let expected = tokens(&text(&ctx.source, spec.source_span)?);
    let heard = selected
        .iter()
        .flat_map(|p| p.tokens.iter().cloned())
        .collect::<Vec<_>>();
    if expected != heard {
        return Err(uncertain("primary_complete_text_mismatch"));
    }
    check(
        selected.last().unwrap().positive_terminal,
        "primary_terminal_is_point",
    )?;
    let terminal = &expected[expected.len().saturating_sub(3)..];
    if terminal.len() < 2
        || occurrences(&ctx.source_tokens, &expected) != 1
        || occurrences(&ctx.primary_tokens, &expected) != 1
        || occurrences(&ctx.source_tokens, terminal) != 1
        || occurrences(&ctx.primary_tokens, terminal) != 1
    {
        return Err(uncertain("source_or_primary_occurrence_ambiguous"));
    }
    let matches = ctx
        .independent
        .iter()
        .flat_map(|e| {
            e.turns
                .iter()
                .filter(|t| t.tokens == expected)
                .map(move |t| (e, t))
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(uncertain(
            "independent_complete_interval_missing_or_ambiguous",
        ));
    }
    let (e, turn) = matches[0];
    if occurrences(&e.full, &expected) != 1
        || occurrences(&e.all_turns, &expected) != 1
        || occurrences(&e.full, terminal) != 1
        || occurrences(&e.all_turns, terminal) != 1
    {
        return Err(uncertain("independent_occurrence_ambiguous"));
    }
    let end = selected.last().unwrap().end;
    if end.abs_diff(turn.end) * 4 > ctx.rate as u64 * 3 {
        return Err(uncertain("endpoint_disagreement_exceeds_0_75_seconds"));
    }
    Ok(IntervalResult {
        id: spec.id.clone(),
        source_span: spec.source_span,
        primary_unit_ids: spec.primary_unit_ids.clone(),
        outcome: ReviewOutcome::Pass,
        reason: "complete_unique_intervals_match".into(),
        primary_start_sample: Some(selected[0].start),
        primary_end_sample: Some(end),
        independent_start_sample: Some(turn.start),
        independent_end_sample: Some(turn.end),
        not_before_sample: Some(end.max(turn.end)),
        primary_evidence: selected.iter().map(|p| p.evidence.clone()).collect(),
        independent_evidence: Some(
            json!({"binding":e.evidence,"turn_index":turn.index,"model":e.model}),
        ),
    })
}

pub(super) fn recipe_digest() -> Digest {
    digest(include_bytes!("acoustic_facts.rs"))
}

pub(super) struct Matching {
    pub report: Value,
    pub findings: Vec<Finding>,
}
pub(super) fn evaluate(
    input: &StageInput,
    files: &Files,
    normalized_bytes: &[u8],
    normalized_ref: &FileRef,
) -> Result<Option<Matching>, StageError> {
    let Some((facts_ref, facts_bytes)) = files.get("source-facts.json") else {
        return Ok(None);
    };
    let inv: Inventory = parse(facts_bytes)?;
    let normalized: Value = parse(normalized_bytes)?;
    let ctx = context(&inv, files, &normalized);
    let mut intervals = BTreeMap::new();
    for spec in &inv.intervals {
        let result = ctx
            .as_ref()
            .map_err(|e| Failure {
                outcome: e.outcome,
                reason: e.reason,
            })
            .and_then(|ctx| interval(spec, ctx));
        let result = result.unwrap_or_else(|e| IntervalResult {
            id: spec.id.clone(),
            source_span: spec.source_span,
            primary_unit_ids: spec.primary_unit_ids.clone(),
            outcome: e.outcome,
            reason: e.reason.into(),
            primary_start_sample: None,
            primary_end_sample: None,
            independent_start_sample: None,
            independent_end_sample: None,
            not_before_sample: None,
            primary_evidence: vec![],
            independent_evidence: None,
        });
        intervals.insert(spec.id.clone(), result);
    }
    let mut rows = Vec::new();
    let mut findings = Vec::new();
    for id in &inv.required_fact_ids {
        let definition = inv.facts.iter().find(|f| &f.id == id);
        let result = (|| -> Checked<Sample> {
            let ctx = ctx.as_ref().map_err(|e| Failure {
                outcome: e.outcome,
                reason: e.reason,
            })?;
            let fact = definition.ok_or_else(|| missing_fact("required_fact_missing"))?;
            check(
                !fact.source_spans.is_empty()
                    && !fact.interval_ids.is_empty()
                    && unique(fact.interval_ids.iter()),
                "fact_coverage_missing",
            )?;
            let supports = fact
                .interval_ids
                .iter()
                .map(|key| {
                    intervals
                        .get(key)
                        .ok_or_else(|| missing_fact("fact_interval_missing"))
                })
                .collect::<Checked<Vec<_>>>()?;
            for span in &fact.source_spans {
                text(&ctx.source, *span)?;
                let mut spoken = false;
                for scalar in span[0]..span[1] {
                    if ctx.source[scalar as usize].is_alphanumeric() {
                        spoken = true;
                        check(
                            supports
                                .iter()
                                .any(|i| i.source_span[0] <= scalar && scalar < i.source_span[1]),
                            "fact_not_completely_covered",
                        )?;
                    }
                }
                check(spoken, "fact_scope_has_no_spoken_words")?;
            }
            for support in &supports {
                if support.outcome != ReviewOutcome::Pass {
                    return Err(Failure {
                        outcome: support.outcome,
                        reason: "supporting_interval_unverified",
                    });
                }
            }
            Ok(supports
                .iter()
                .filter_map(|i| i.not_before_sample)
                .max()
                .unwrap())
        })();
        let (outcome, bound, reason) = match result {
            Ok(bound) => (ReviewOutcome::Pass, Some(bound), "all_fact_spans_covered"),
            Err(e) => (e.outcome, None, e.reason),
        };
        let spans = definition
            .map(|f| f.source_spans.clone())
            .unwrap_or_default();
        let ids = definition
            .map(|f| f.interval_ids.clone())
            .unwrap_or_default();
        rows.push(json!({"id":id,"source_spans":spans,"interval_ids":ids,"outcome":outcome,"reason":reason,"not_before_sample":bound}));
        findings.push(Finding{id:format!("acoustic-fact:{id}"),field:"acoustic.source_facts".into(),outcome,
            message:format!("{reason}; bound applies only to the digest-bound explicit fact inventory in the original narration domain."),evidence:input.inputs.clone()});
    }
    if let Err(e) = &ctx {
        findings.push(Finding {
            id: "source-fact-inventory".into(),
            field: "acoustic.source_facts".into(),
            outcome: e.outcome,
            message: e.reason.into(),
            evidence: input.inputs.clone(),
        });
    }
    let all_matched = !rows.is_empty() && rows.iter().all(|r| r["outcome"] == "pass");
    let report = json!({"schema_version":1,"provenance":"diagnostic","production_approved":false,"provider_calls":0,
        "coordinate_system":inv.coordinate_system,"sample_rate":inv.sample_rate,"source_sha256":inv.source_sha256,"narration_sha256":inv.narration_sha256,
        "source_facts_sha256":facts_ref.sha256,"normalized_report_sha256":normalized_ref.sha256,"matching_recipe_sha256":recipe_digest(),
        "source_facts":facts_ref,"normalized_report":normalized_ref,"acoustic_manifest_sha256":inv.acoustic_manifest_sha256,
        "normalization_policy":"lowercase-and-apostrophe-v1; no spelling substitutions or repetition removal",
        "tolerance_samples_numerator":inv.sample_rate as u64*3,"tolerance_samples_denominator":4,
        "all_facts_matched":all_matched,"intervals":intervals.into_values().collect::<Vec<_>>(),"facts":rows,
        "reveal_permissions":[],"scope":"Source-fact acoustic observations only; final presentation mapping, image applicability and production acceptance remain separate."});
    Ok(Some(Matching { report, findings }))
}
