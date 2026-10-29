//! Preserve measured points and native domains. Joins/cuts never manufacture a word interval.
use super::*;
use std::collections::BTreeSet;
pub(super) fn normalize(
    verified: &verify::Verified,
    parsed: &words::Parsed,
    limits: &Limits,
) -> Result<Value, StageError> {
    let matched = matching::align(
        verified.speech["source_text"].as_str().unwrap(),
        &parsed.words,
        limits,
    )?;
    let segments = verified.batch["segments"].as_array().unwrap();
    let takes = verified.manifest["takes"].as_array().unwrap();
    let mut result = vec![];
    let mut covered = BTreeSet::new();
    for (i, w) in parsed.words.iter().enumerate() {
        let point = w.start_ns == w.end_ns;
        let mut reasons = vec![];
        let candidates = segments
            .iter()
            .filter(|s| {
                let a = s["batch_span"][0].as_u64().unwrap();
                let b = s["batch_span"][1].as_u64().unwrap();
                if point {
                    a <= w.samples[0] && w.samples[0] <= b
                } else {
                    a <= w.samples[0] && w.samples[0] < b && w.samples[1] <= b
                }
            })
            .collect::<Vec<_>>();
        let mut audio = Value::Null;
        let mut native = Value::Null;
        let mut span = Value::Null;
        if candidates.len() != 1 {
            reasons.push("artificial-join-or-boundary");
        } else if !point && w.samples[0] == w.samples[1] {
            reasons.push("positive-interval-collapsed-by-ceiling");
        } else {
            let s = candidates[0];
            let b = s["batch_span"][0].as_u64().unwrap();
            let a = s["native_span"][0].as_u64().unwrap();
            audio = s["audio"].clone();
            native = json!([a + w.samples[0] - b, a + w.samples[1] - b]);
            if s["cross_cut_uncertainty"] == true {
                reasons.push("native-interior-cut-uncertainty");
            }
            if let Some(index) = matched.forced[i] {
                let token = &matched.source[index];
                let take = takes
                    .iter()
                    .find(|t| t["unit_id"] == s["unit_id"] && t["audio"] == audio)
                    .unwrap();
                if token.start_char as u64 >= take["source_span"][0].as_u64().unwrap()
                    && token.end_char as u64 <= take["source_span"][1].as_u64().unwrap()
                {
                    if reasons.is_empty() {
                        span = json!([token.start_char, token.end_char]);
                        covered.insert(index);
                    }
                } else {
                    reasons.push("global-source-match-outside-native-take");
                }
            } else {
                reasons.push("global-source-ambiguity-or-mismatch");
            }
        }
        let source_candidates = matched.candidates[i]
            .iter()
            .map(|k| {
                let t = &matched.source[*k];
                json!([t.start_char, t.end_char])
            })
            .collect::<Vec<_>>();
        result.push(json!({"index":i,"word":w.word,"original_offsets":{"start":w.start,"end":w.end},"nanoseconds":[w.start_ns,w.end_ns],"batch_samples":w.samples,"timing_kind":if point{"point"}else{"interval"},"native_audio":audio,"native_samples":native,"global_source_candidates":source_candidates,"source_span":span,"uncertainties":reasons}));
    }
    let mut groups = vec![];
    let mut pending = vec![];
    let mut unresolved = vec![];
    for (i, w) in parsed.words.iter().enumerate() {
        if w.start_ns == w.end_ns {
            pending.push(i);
            continue;
        }
        let r = &result[i];
        let eligible = !r["source_span"].is_null()
            && !r["native_samples"].is_null()
            && w.samples[0] < w.samples[1];
        let mut members = vec![];
        for p in pending.drain(..) {
            if eligible
                && parsed.words[p].start_ns == w.start_ns
                && result[p]["source_span"].is_array()
                && result[p]["native_audio"] == r["native_audio"]
                && result[p]["native_samples"][0] == r["native_samples"][0]
            {
                members.push(p);
            } else {
                unresolved.push(p);
            }
        }
        if eligible {
            members.push(i);
            groups.push(json!({"word_indices":members,"interval_word_index":i,"native_audio":r["native_audio"],"native_samples":r["native_samples"]}));
        }
    }
    unresolved.extend(pending);
    let unmatched=matched.source.iter().enumerate().filter(|(i,_)|!covered.contains(i)).map(|(i,t)|json!({"word":t.text,"source_span":[t.start_char,t.end_char],"outcome":if matched.source_candidates[i]{"uncertain"}else{"missing"}})).collect::<Vec<_>>();
    Ok(
        json!({"words":result,"highlight_groups":groups,"unresolved_points":unresolved,"unmatched_source":unmatched}),
    )
}
