//! Original measured decimal offsets only. No expected words or interpolated durations.
use super::*;
#[derive(Clone)]
pub(super) struct Word {
    pub word: String,
    pub normalized: String,
    pub start: String,
    pub end: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub samples: Interval,
}
pub(super) struct Parsed {
    pub status: String,
    pub words: Vec<Word>,
    pub invalid_word: Option<usize>,
}
impl Parsed {
    pub fn empty(reason: &str) -> Self {
        Self {
            status: reason.into(),
            words: vec![],
            invalid_word: None,
        }
    }
}
pub(super) fn nanoseconds(s: &str) -> Option<u64> {
    let s = s.strip_suffix('s')?;
    let (whole, frac) = s.split_once('.').map_or((s, ""), |(a, b)| (a, b));
    if whole.is_empty()
        || whole.len() > 12
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || frac.len() > 9
        || !frac.bytes().all(|c| c.is_ascii_digit())
        || s.contains('.') && frac.is_empty()
    {
        return None;
    }
    let whole = whole.parse::<u64>().ok()?;
    let fraction = if frac.is_empty() {
        0
    } else {
        frac.parse::<u64>()
            .ok()?
            .checked_mul(10u64.pow((9 - frac.len()) as u32))?
    };
    whole.checked_mul(1_000_000_000)?.checked_add(fraction)
}
pub(super) fn parse_response(
    raw: &[u8],
    model: &str,
    info: &Pcm16WavInfo,
    limits: &Limits,
) -> Result<Parsed, StageError> {
    let response: Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return Ok(Parsed::empty("malformed-json")),
    };
    if let Some(version) = response["modelVersion"].as_str() {
        need(
            version == model,
            "raw primary modelVersion differs from pinned model",
        )?;
    }
    if !response["promptFeedback"]["blockReason"].is_null() {
        return Ok(Parsed::empty("refused"));
    }
    let Some(candidates) = response["candidates"].as_array() else {
        return Ok(Parsed::empty("missing-candidate"));
    };
    if candidates.len() != 1 {
        return Ok(Parsed::empty("ambiguous-candidates"));
    }
    if candidates[0]["finishReason"] != "STOP" {
        return Ok(Parsed::empty(
            if candidates[0]["finishReason"] == "SAFETY" {
                "refused"
            } else {
                "incomplete"
            },
        ));
    }
    let Some(parts) = candidates[0]["content"]["parts"].as_array() else {
        return Ok(Parsed::empty("missing-parts"));
    };
    let mut result = Parsed::empty("completed");
    let mut found = false;
    let mut last = (0, 0);
    for part in parts {
        if part.get("audioTranscription").is_none() {
            continue;
        }
        found = true;
        let Some(words) = part["audioTranscription"]["words"].as_array() else {
            return Ok(Parsed::empty("missing-word-timestamps"));
        };
        need(
            result.words.len() + words.len() <= limits.max_words as usize,
            "primary measured word limit",
        )?;
        for w in words {
            let checked = (|| {
                let word = w["word"].as_str()?;
                let a = w["startOffset"].as_str()?;
                let b = w["endOffset"].as_str()?;
                let an = nanoseconds(a)?;
                let bn = nanoseconds(b)?;
                let tokens = monotonic_align::tokenize_source(word);
                if tokens.len() != 1
                    || tokens[0].normalized.is_empty()
                    || an > bn
                    || an < last.0
                    || bn < last.1
                    || (bn as u128 * info.sample_rate as u128)
                        > info.frame_count as u128 * 1_000_000_000
                {
                    return None;
                }
                let sample = |ns: u64| {
                    ((ns as u128 * info.sample_rate as u128).div_ceil(1_000_000_000)) as u64
                };
                Some(Word {
                    word: word.into(),
                    normalized: tokens[0].normalized.clone(),
                    start: a.into(),
                    end: b.into(),
                    start_ns: an,
                    end_ns: bn,
                    samples: [sample(an), sample(bn)],
                })
            })();
            let Some(word) = checked else {
                let mut failed = Parsed::empty("invalid-word-or-measured-offset");
                failed.invalid_word = Some(result.words.len());
                return Ok(failed);
            };
            last = (word.start_ns, word.end_ns);
            result.words.push(word);
        }
    }
    if !found {
        result.status = "missing-word-timestamps".into();
    } else if result.words.is_empty() {
        result.status = "empty-recognition".into();
    }
    Ok(result)
}
