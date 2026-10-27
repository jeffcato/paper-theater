//! Private diagnostic delivery-domain fact evidence. All effects use RunServices.
use super::presentation_pcm::{
    self as pcm, Checked, Failure, Verified, absent, check, decode, failure,
};
use super::*;
use serde_json::{Value, json};
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
const OUTPUT: &str = "presentation-fact-evidence.json";
const EDIT_RECIPE: &[u8] = include_bytes!("../../../../tools/generation/readiness_correction.py");
fn implementation() -> Digest {
    digest(
        &[
            include_bytes!("presentation.rs").as_slice(),
            include_bytes!("presentation_pcm.rs").as_slice(),
        ]
        .concat(),
    )
}
fn file<'a>(files: &'a Files, path: &str) -> Checked<&'a (FileRef, Vec<u8>)> {
    files
        .get(path)
        .ok_or_else(|| absent("presentation_input_missing"))
}
fn payload<'a>(files: &'a Files, reference: &FileRef) -> Checked<&'a [u8]> {
    let mut expected = reference.clone();
    expected.path = format!("delivery/{}", reference.path);
    let actual = file(files, &expected.path)?;
    check(actual.0 == expected, "delivery_payload_binding_mismatch")?;
    Ok(&actual.1)
}
struct Narration {
    report: Value,
    findings: Vec<Finding>,
}
fn narration(input: &StageInput, files: &Files) -> Checked<Narration> {
    acoustic::verify_recorded(files).map_err(|_| failure("stale_narration_observations"))?;
    let (normalized, bytes) = file(files, "acoustic-replay.json")?;
    let matching = acoustic_facts::evaluate(input, files, bytes, normalized)
        .map_err(|_| failure("invalid_narration_facts"))?
        .ok_or_else(|| absent("source_fact_inventory_missing"))?;
    let (facts, actual) = file(files, "fact-evidence.json")?;
    check(
        json(&matching.report).map_err(|_| failure("invalid_narration_facts"))? == *actual,
        "stale_narration_facts",
    )?;
    let summary: Value = decode(&file(files, "acoustic-validation.json")?.1)?;
    check(
        summary["schema_version"] == 1
            && summary["stage"] == "acoustic"
            && summary["provenance"] == "diagnostic"
            && summary["production_approved"] == false
            && summary["provider_calls"] == 0
            && summary["normalized_report_sha256"] == normalized.sha256
            && summary["helper_recipe_sha256"] == decode::<Value>(bytes)?["recipe_sha256"]
            && summary["source_sha256"] == matching.report["source_sha256"]
            && summary["narration_sha256"] == matching.report["narration_sha256"]
            && summary["source_facts_sha256"] == matching.report["source_facts_sha256"]
            && summary["fact_matching_recipe_sha256"] == matching.report["matching_recipe_sha256"]
            && summary["fact_matching"] == serde_json::to_value(facts).unwrap(),
        "g16_fact_receipt_binding_mismatch",
    )?;
    let findings = serde_json::from_value(summary["findings"].clone())
        .map_err(|_| failure("invalid_narration_findings"))?;
    Ok(Narration {
        report: matching.report,
        findings,
    })
}
struct Context {
    mapping: Verified,
    doc: PlaybackDocument,
}
fn verify_delivery(files: &Files) -> Checked<Context> {
    let doc: PlaybackDocument = decode(&file(files, "delivery/fixture.json")?.1)?;
    validate_playback(&doc).map_err(|_| failure("invalid_delivery_document"))?;
    let normalized: Value = decode(&file(files, "acoustic-replay.json")?.1)?;
    let map_bytes = &file(files, "delivery/presentation-map.json")?.1;
    let recipe_bytes = &file(files, "presentation-recipe.json")?.1;
    let recipe: pcm::Recipe = decode(recipe_bytes)?;
    let master = payload(files, &doc.manifest.entries.audio)?;
    check(
        doc.manifest.entries.audio.media_type == "audio/wav"
            && doc.manifest.entries.audio.dimensions.is_none()
            && doc.manifest.source_sha256 == normalized["source_sha256"]
            && doc.read_along.narration_sha256 == normalized["audio_sha256"]
            && doc.manifest.sample_count == recipe.delivery_frames
            && doc.timeline.presentation_map_sha256 == digest(map_bytes),
        "delivery_document_binding_mismatch",
    )?;
    let timeline: Timeline = decode(payload(files, &doc.manifest.entries.timeline)?)?;
    let read_along: ReadAlong = decode(payload(files, &doc.manifest.entries.read_along)?)?;
    check(
        serde_json::to_value(timeline).unwrap() == serde_json::to_value(&doc.timeline).unwrap()
            && serde_json::to_value(read_along).unwrap()
                == serde_json::to_value(&doc.read_along).unwrap(),
        "delivery_document_payload_mismatch",
    )?;
    let map_refs = doc
        .manifest
        .assets
        .iter()
        .filter(|r| r.path == "presentation-map.json")
        .collect::<Vec<_>>();
    check(map_refs.len() == 1, "presentation_map_reference_ambiguous")?;
    check(
        payload(files, map_refs[0])? == map_bytes,
        "presentation_map_payload_mismatch",
    )?;
    let h = recipe
        .historical
        .as_ref()
        .ok_or_else(|| absent("documented_edit_recipe_missing"))?;
    let credits = file(files, &h.description.path)?;
    let edit = file(files, &h.edit_record.path)?;
    let code = file(files, &h.edit_implementation.path)?;
    check(
        credits.0 == h.description
            && edit.0 == h.edit_record
            && code.0 == h.edit_implementation
            && code.0.sha256 == digest(EDIT_RECIPE)
            && credits.1 == payload(files, &doc.manifest.entries.credits)?,
        "historical_recipe_binding_mismatch",
    )?;
    let credits: Value = decode(&credits.1)?;
    let edit: Value = decode(&edit.1)?;
    check(
        credits["transformation"].as_str().is_some_and(|s| {
            s.starts_with("24 kHz mono to 48 kHz stereo by exact sample duplication;")
        }) && edit["recipe_sha256"] == code.0.sha256
            && edit["new_bindings"]["delivery_audio_sha256"] == doc.manifest.entries.audio.sha256
            && edit["new_bindings"]["presentation_map_sha256"] == digest(map_bytes),
        "documented_edit_binding_mismatch",
    )?;
    let mapping = pcm::verify(
        map_bytes,
        recipe_bytes,
        &file(files, "excerpt.wav")?.1,
        master,
    )?;
    // Bind the delivery read-along to the exact original observations, preserving every measured point and atomic group.
    let alignment_ref = doc
        .manifest
        .assets
        .iter()
        .find(|r| r.path == "evidence/alignment.json")
        .ok_or_else(|| absent("alignment_binding_missing"))?;
    let alignment: Value = decode(payload(files, alignment_ref)?)?;
    check(
        doc.read_along.alignment_sha256 == alignment_ref.sha256
            && alignment["source_sha256"] == normalized["source_sha256"]
            && alignment["narration_sha256"] == normalized["audio_sha256"]
            && alignment["raw_report_sha256"]
                == file(files, "segmented-acoustic-report.json")?.0.sha256,
        "alignment_binding_mismatch",
    )?;
    let manifest: Value = decode(&file(files, "manifest.json")?.1)?;
    let base = manifest["source_span_characters"][0].as_u64().unwrap();
    let words = normalized["word_track"].as_array().unwrap();
    let groups = normalized["highlight_groups"].as_array().unwrap();
    let alignment_words = alignment["words"]
        .as_array()
        .ok_or_else(|| failure("alignment_words_missing"))?;
    check(
        words.len() == doc.read_along.words.len()
            && groups.len() == doc.read_along.groups.len()
            && alignment_words.len() == words.len(),
        "read_along_coverage_mismatch",
    )?;
    for ((raw, aligned), word) in words.iter().zip(alignment_words).zip(&doc.read_along.words) {
        let mut original = aligned.clone();
        let source_span = original
            .as_object_mut()
            .ok_or_else(|| failure("alignment_words_missing"))?
            .remove("source_span");
        check(
            original == *raw && source_span == Some(json!(word.source_span)),
            "alignment_words_mismatch",
        )?;
        let a = raw["start_sample"].as_u64().unwrap();
        let b = raw["end_sample"].as_u64().unwrap();
        if a < b {
            mapping.interval([a, b])?;
        }
        check(
            word.id == raw["id"]
                && word.text == raw["word"]
                && word.source_span
                    == [
                        raw["source_span_characters"][0].as_u64().unwrap() - base,
                        raw["source_span_characters"][1].as_u64().unwrap() - base,
                    ]
                && word.start_sample == mapping.point(a, false)?
                && word.end_sample == mapping.point(b, a < b)?,
            "read_along_measurement_mismatch",
        )?;
    }
    for (raw, group) in groups.iter().zip(&doc.read_along.groups) {
        let a = raw["start_sample"].as_u64().unwrap();
        let b = raw["end_sample"].as_u64().unwrap();
        check(
            group.id == raw["id"]
                && json!(group.word_ids) == raw["word_ids"]
                && group.timing_owner_word_id == raw["interval_word_id"]
                && group.start_sample == mapping.point(a, false)?
                && group.end_sample == mapping.point(b, true)?,
            "highlight_group_measurement_mismatch",
        )?;
    }
    Ok(Context { mapping, doc })
}
fn outcome(f: &Failure) -> Value {
    json!({"outcome":f.outcome,"reason":f.reason})
}
pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
    files: &Files,
    cache_key: Digest,
) -> Result<StageOutput, StageError> {
    let source = narration(input, files);
    let checked = source
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|_| verify_delivery(files));
    let raw = source
        .as_ref()
        .map(|s| s.report.clone())
        .unwrap_or_else(|_| {
            files
                .get("fact-evidence.json")
                .and_then(|f| serde_json::from_slice(&f.1).ok())
                .unwrap_or(Value::Null)
        });
    let mut findings = source
        .as_ref()
        .map(|s| s.findings.clone())
        .unwrap_or_default();
    if let Err(e) = &checked {
        findings.push(Finding {
            id: "presentation-placement".into(),
            field: "presentation.evidence".into(),
            outcome: e.outcome,
            message: e.reason.into(),
            evidence: input.inputs.clone(),
        });
    }
    let mut rows = Vec::new();
    let mut entries = Vec::new();
    for fact in raw["facts"].as_array().into_iter().flatten() {
        let id = fact["id"].as_str().unwrap_or("invalid");
        let result = (|| -> Checked<(u64, Vec<Value>)> {
            if fact["outcome"] != "pass" {
                return Err(Failure {
                    outcome: serde_json::from_value(fact["outcome"].clone())
                        .unwrap_or(ReviewOutcome::Fail),
                    reason: "narration_fact_unresolved",
                });
            }
            if let Some(previous) = findings
                .iter()
                .find(|f| f.id == format!("acoustic-fact:{id}") && f.outcome != ReviewOutcome::Pass)
            {
                return Err(Failure {
                    outcome: previous.outcome,
                    reason: "recorded_adverse_fact_finding",
                });
            }
            let ctx = checked.as_ref().map_err(Clone::clone)?;
            let mut bound = 0;
            let mut supports = Vec::new();
            for interval_id in fact["interval_ids"].as_array().unwrap() {
                let interval = raw["intervals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|i| i["id"] == *interval_id)
                    .ok_or_else(|| absent("supporting_interval_missing"))?;
                check(
                    interval["outcome"] == "pass",
                    "supporting_interval_unverified",
                )?;
                let start = interval["primary_start_sample"]
                    .as_u64()
                    .unwrap()
                    .min(interval["independent_start_sample"].as_u64().unwrap());
                let p = interval["primary_end_sample"].as_u64().unwrap();
                let q = interval["independent_end_sample"].as_u64().unwrap();
                let (end, segments) = ctx.mapping.interval([start, p.max(q)])?;
                let primary_end = ctx.mapping.point(p, true)?;
                let independent_end = ctx.mapping.point(q, true)?;
                check(
                    end == primary_end.max(independent_end),
                    "mapped_endpoint_inconsistent",
                )?;
                bound = bound.max(end);
                supports.push(json!({"interval_id":interval_id,"narration":interval,"segments":segments,"primary_end_sample":primary_end,"independent_end_sample":independent_end,"not_before_sample":end}));
            }
            check(!supports.is_empty(), "supporting_interval_missing")?;
            Ok((bound, supports))
        })();
        let (review, bound, reason, supports) = match result {
            Ok((bound, supports)) => (
                ReviewOutcome::Pass,
                Some(bound),
                "verified_complete_delivery_intervals",
                supports,
            ),
            Err(e) => (e.outcome, None, e.reason, vec![]),
        };
        rows.push(json!({"id":id,"narration":fact,"outcome":review,"reason":reason,"delivery_not_before_sample":bound,"supporting_intervals":supports}));
        findings.push(Finding {
            id: format!("presentation-fact:{id}"),
            field: "presentation.evidence".into(),
            outcome: review,
            message: reason.into(),
            evidence: input.inputs.clone(),
        });
        if let Ok(ctx) = &checked {
            let spans = fact["source_spans"].as_array().unwrap();
            for (i, scene) in ctx.doc.timeline.scenes.iter().enumerate().filter(|(_, s)| {
                !spans.is_empty()
                    && spans.iter().all(|p| {
                        p[0].as_u64().unwrap() >= s.source_span[0]
                            && p[1].as_u64().unwrap() <= s.source_span[1]
                    })
            }) {
                let entry = if i == 0 {
                    ctx.doc.timeline.opening.curtain_open[0]
                } else {
                    ctx.doc.timeline.transitions[i - 1].fade_in_start
                };
                let result = bound
                    .map(|b| {
                        if b <= entry {
                            "eligible_by_entry"
                        } else {
                            "would_be_late"
                        }
                    })
                    .unwrap_or("unresolved");
                entries.push(json!({"fact_id":id,"scene_id":scene.id,"entry_sample":entry,"delivery_not_before_sample":bound,"timing":result,"scope":"Source-scope timing comparison only; no image/fact requirement or reveal permission is inferred."}));
            }
        }
    }
    for id in [
        "original-resampling-execution-receipt",
        "presentation-production-review",
    ] {
        findings.push(missing(id,"Diagnostic PCM reconstruction does not establish independent production recipes, creative/audio review or reveal permission."));
    }
    let normalized: Value = files
        .get("acoustic-replay.json")
        .and_then(|f| serde_json::from_slice(&f.1).ok())
        .unwrap_or(Value::Null);
    let bindings = files
        .iter()
        .filter(|(p, _)| p.as_str() != OUTPUT)
        .map(|(p, (f, _))| (p.clone(), f.clone()))
        .collect::<BTreeMap<_, _>>();
    let missing_ids = findings
        .iter()
        .filter(|f| f.outcome != ReviewOutcome::Pass)
        .map(|f| f.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let report = json!({"schema_version":1,"provenance":"diagnostic","production_approved":false,"provider_calls":0,
        "implementation_sha256":implementation(),"bindings":bindings,"narration_verification":source.as_ref().map(|_|json!({"outcome":"pass"})).unwrap_or_else(outcome),
        "placement":checked.as_ref().map(|_|json!({"outcome":"pass","reason":"all_pcm_frames_and_declared_silence_match"})).unwrap_or_else(outcome),
        "source_sample_rate":normalized["sample_rate"],"delivery_sample_rate":48000,"word_count":normalized["word_track"].as_array().map(Vec::len),"highlight_group_count":normalized["highlight_groups"].as_array().map(Vec::len),
        "point_word_count":normalized["word_track"].as_array().map(|w|w.iter().filter(|w|w["start_sample"]==w["end_sample"]).count()),
        "endpoint_policy":"integer ceiling; interval ends own the preceding segment at a cut; points/starts own the following segment",
        "facts":rows,"entry_comparisons":entries,"reveal_permissions":[],"missing_evidence":missing_ids,
        "scope":"Exact diagnostic PCM reconstruction and arithmetic only; independent production evidence, creative review and publication remain unresolved."});
    if input.mode == ExecutionMode::Replay
        && let Some((previous, _)) = files.get(OUTPUT)
    {
        let bytes = json(&report)?;
        require(
            previous.sha256 == digest(&bytes)
                && previous.bytes == bytes.len() as u64
                && previous.media_type == "application/json",
            "presentation_replay_binding",
            OUTPUT,
            "dependent mapping inputs or implementation changed",
        )?;
    }
    let candidate = write(services, input, OUTPUT, &report).await?;
    services.check_cancelled().await?;
    Ok(StageOutput {
        outcome: StageOutcome::NeedsRepair,
        artifacts: vec![candidate.clone()],
        decisions: vec![],
        findings,
        cache_keys: vec![cache_key, implementation(), candidate.file.sha256],
        library_updates: vec![],
        package_candidate: None,
    })
}
