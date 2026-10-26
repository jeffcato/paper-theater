//! Validate a bounded, broker-produced offline normalization candidate. No transport or storage.
use super::*;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_REPORT_BYTES: u64 = 16 * 1024 * 1024;
const HELPER: &[u8] = include_bytes!("../../../../tools/generation/helper.py");
const ADAPTERS: &[u8] = include_bytes!("../../../../tools/generation/adapters.py");

fn recipe_digest() -> Digest {
    digest(&[HELPER, ADAPTERS].concat())
}

#[derive(Deserialize)]
struct Manifest {
    schema: String,
    source_sha256: Digest,
    audio_sha256: Digest,
    sample_rate: u32,
    source_span_characters: [u64; 2],
    units: Vec<Unit>,
}
#[derive(Deserialize)]
struct Unit {
    audio: String,
}
#[derive(Deserialize)]
struct Cached {
    input_manifest_sha256: Digest,
    units: Vec<CachedUnit>,
}
#[derive(Deserialize)]
struct CachedUnit {
    request_sha256: Digest,
}
#[derive(Deserialize)]
struct Report {
    schema_version: u32,
    provenance: Provenance,
    provider_calls: u64,
    production_approved: bool,
    source_sha256: Digest,
    audio_sha256: Digest,
    sample_rate: u32,
    recipe_sha256: Digest,
    input_hashes: BTreeMap<String, Digest>,
    word_track: Vec<Word>,
    highlight_groups: Vec<Group>,
    primary_inputs: Vec<Value>,
    independent: Vec<Value>,
    reveal_permissions: Vec<Value>,
    missing_evidence: Vec<Id>,
}
#[derive(Deserialize)]
struct RecordedValidation {
    schema_version: u32,
    stage: String,
    provenance: Provenance,
    provider_calls: u64,
    production_approved: bool,
    normalized_report_sha256: Digest,
    helper_recipe_sha256: Digest,
    source_sha256: Digest,
    narration_sha256: Digest,
    findings: Vec<Finding>,
}

#[derive(Deserialize)]
struct Word {
    id: Id,
    source_span_characters: [u64; 2],
    word: String,
    start_sample: Sample,
    end_sample: Sample,
}
#[derive(Deserialize)]
struct Group {
    id: Id,
    word_ids: Vec<Id>,
    interval_word_id: Id,
    start_sample: Sample,
    end_sample: Sample,
}

type Inputs = BTreeMap<String, (FileRef, Vec<u8>)>;
fn get<'a>(files: &'a Inputs, path: &str) -> Result<&'a (FileRef, Vec<u8>), StageError> {
    files
        .get(path)
        .ok_or_else(|| StageError::Evidence(format!("missing acoustic input: {path}")))
}

fn required_inputs(files: &Inputs) -> Result<BTreeSet<String>, StageError> {
    let manifest: Manifest = parse(&get(files, "manifest.json")?.1)?;
    let cached: Cached = parse(&get(files, "segmented-acoustic-report.json")?.1)?;
    require(
        manifest.schema == "acoustic-capability-v1"
            && manifest.sample_rate > 0
            && !manifest.units.is_empty()
            && manifest.units.len() == cached.units.len()
            && manifest.source_sha256 == get(files, "source.txt")?.0.sha256
            && manifest.audio_sha256 == get(files, "excerpt.wav")?.0.sha256
            && cached.input_manifest_sha256 == get(files, "manifest.json")?.0.sha256,
        "acoustic_binding",
        "inputs",
        "source/audio/manifest scope differs before helper dispatch",
    )?;
    let mut expected = BTreeSet::from([
        "manifest.json".to_owned(),
        "source.txt".into(),
        "excerpt.wav".into(),
        "segmented-acoustic-report.json".into(),
    ]);
    for unit in &manifest.units {
        expected.insert(unit.audio.clone());
    }
    for unit in &cached.units {
        require(
            valid_digest(&unit.request_sha256),
            "acoustic_request",
            "units",
            "invalid cached request digest",
        )?;
        expected.insert(format!("transcribe/{}.json", unit.request_sha256));
        expected.insert(format!("transcribe/{}.receipt.json", unit.request_sha256));
    }
    for path in files
        .keys()
        .filter(|p| p.starts_with("meta/") && p.ends_with(".receipt.json"))
    {
        expected.insert(path.clone());
        expected.insert(path.trim_end_matches(".receipt.json").to_owned() + ".json");
    }
    for path in &expected {
        get(files, path)?;
    }
    Ok(expected)
}

fn validate(bytes: &[u8], files: &Inputs, recipe: &str) -> Result<Report, StageError> {
    let value: Value = parse(bytes)?;
    let report: Report = parse(bytes)?;
    require(
        report.schema_version == 1
            && report.provenance == Provenance::Imported
            && report.provider_calls == 0
            && !report.production_approved
            && report.reveal_permissions.is_empty(),
        "acoustic_scope",
        "report",
        "expected an imported, zero-dispatch diagnostic observation",
    )?;
    require(
        report.recipe_sha256 == recipe,
        "acoustic_recipe",
        "report.recipe_sha256",
        "normalization recipe differs",
    )?;
    let manifest: Manifest = parse(&get(files, "manifest.json")?.1)?;
    let cached: Cached = parse(&get(files, "segmented-acoustic-report.json")?.1)?;
    let saved: Value = parse(&get(files, "segmented-acoustic-report.json")?.1)?;
    let source = std::str::from_utf8(&get(files, "source.txt")?.1)
        .map_err(|_| StageError::Evidence("acoustic source must be UTF-8".into()))?;
    require(
        manifest.schema == "acoustic-capability-v1"
            && manifest.source_sha256 == report.source_sha256
            && manifest.source_sha256 == digest(source.as_bytes())
            && manifest.audio_sha256 == report.audio_sha256
            && manifest.audio_sha256 == get(files, "excerpt.wav")?.0.sha256
            && report.sample_rate > 0
            && manifest.sample_rate == report.sample_rate
            && cached.input_manifest_sha256 == get(files, "manifest.json")?.0.sha256,
        "acoustic_binding",
        "report",
        "source, audio or saved-manifest identity differs",
    )?;
    let expected = required_inputs(files)?;
    require(
        expected == report.input_hashes.keys().cloned().collect(),
        "acoustic_inputs",
        "input_hashes",
        "consumed evidence inventory is incomplete or unexpected",
    )?;
    for (path, hash) in &report.input_hashes {
        require(
            valid_digest(hash) && get(files, path)?.0.sha256 == *hash,
            "acoustic_binding",
            path,
            "consumed input digest differs",
        )?;
    }
    require(
        value["word_track"] == saved["candidate_word_track"]
            && value["highlight_groups"] == saved["candidate_highlight_groups"]
            && !report.word_track.is_empty()
            && manifest.units.len() == cached.units.len()
            && report.primary_inputs.len() == manifest.units.len(),
        "acoustic_measurements",
        "report",
        "measured words, point groups or source-turn coverage changed",
    )?;
    let scalars = source.chars().collect::<Vec<_>>();
    let base = manifest.source_span_characters[0];
    require(
        manifest.source_span_characters[1].checked_sub(base) == Some(scalars.len() as u64),
        "acoustic_source",
        "source_span_characters",
        "excerpt scalar scope differs",
    )?;
    let mut words = Vec::new();
    for (i, w) in report.word_track.iter().enumerate() {
        let a = w.source_span_characters[0].checked_sub(base);
        let b = w.source_span_characters[1].checked_sub(base);
        require(
            a.zip(b)
                .is_some_and(|(a, b)| a < b && b <= scalars.len() as u64),
            "acoustic_source",
            &w.id,
            "word outside source excerpt",
        )?;
        let a = a.unwrap() as usize;
        let b = b.unwrap() as usize;
        let next = if let Some(next) = report.word_track.get(i + 1) {
            next.source_span_characters[0]
                .checked_sub(base)
                .ok_or_else(|| StageError::Evidence("word ordering outside excerpt".into()))?
                as usize
        } else {
            scalars.len()
        };
        require(
            b <= next && next <= scalars.len(),
            "acoustic_source",
            &w.id,
            "unordered source words",
        )?;
        words.push(ReadAlongWord {
            id: w.id.clone(),
            source_span: [a as u64, b as u64],
            text: w.word.clone(),
            following_text: scalars[b..next].iter().collect(),
            start_sample: w.start_sample,
            end_sample: w.end_sample,
        });
    }
    let first = words[0].source_span[0] as usize;
    // This transient record checks original source/group structure only. Delivery hashes
    // stay unset: it is never exported as a compiled delivery read-along artifact.
    let track = ReadAlong {
        schema_version: 1,
        policy: "point-following-same-boundary-v1".into(),
        language: "und".into(),
        source_sha256: report.source_sha256.clone(),
        narration_sha256: report.audio_sha256.clone(),
        alignment_sha256: get(files, "segmented-acoustic-report.json")?
            .0
            .sha256
            .clone(),
        presentation_map_sha256: String::new(),
        delivery_audio_sha256: String::new(),
        leading_text: scalars[..first].iter().collect(),
        words,
        groups: report
            .highlight_groups
            .iter()
            .map(|g| ReadAlongGroup {
                id: g.id.clone(),
                word_ids: g.word_ids.clone(),
                timing_owner_word_id: g.interval_word_id.clone(),
                start_sample: g.start_sample,
                end_sample: g.end_sample,
            })
            .collect(),
        preferred_break_after: vec![],
    };
    validate_read_along(&track, source, MAX_SAFE_INTEGER)?;
    Ok(report)
}

pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
    files: &Inputs,
    cache_key: Digest,
) -> Result<StageOutput, StageError> {
    let recipe = recipe_digest();
    let expected = required_inputs(files)?;
    let mut findings = Vec::new();
    let mut artifacts = Vec::new();
    let (bytes, normalized_ref, selected_id) = match input.mode {
        ExecutionMode::Fixture => {
            let selected = input
                .inputs
                .iter()
                .filter(|a| expected.contains(&a.file.path))
                .fold(BTreeMap::new(), |mut refs, a| {
                    refs.entry(a.file.path.clone()).or_insert_with(|| a.clone());
                    refs
                });
            let operation = HelperOperation {
                id: digest(&json(&(
                    input.run_id.as_str(),
                    input.epoch,
                    input.unit_id.as_str(),
                    input.revision,
                    &selected,
                    &recipe,
                ))?),
                kind: HelperKind::AlignAudio,
                inputs: selected.into_values().collect(),
                recipe_sha256: recipe.clone(),
                timeout_ms: input.limits.wall_time_ms.min(30000),
                max_output_bytes: MAX_REPORT_BYTES,
            };
            require(
                operation.timeout_ms > 0,
                "acoustic_timeout",
                "limits.wall_time_ms",
                "positive bounded helper time required",
            )?;
            services.check_cancelled().await?;
            let result = services.helper(operation.clone()).await?;
            services.check_cancelled().await?;
            require(
                result.id == operation.id && result.outputs.len() == 1,
                "acoustic_helper",
                "outputs",
                "helper result must resolve the issued operation exactly once",
            )?;
            let candidate = result.outputs.into_iter().next().unwrap();
            candidate.file.validate()?;
            require(
                valid_id(&candidate.candidate_id)
                    && candidate.file.path == "acoustic-replay.json"
                    && candidate.file.media_type == "application/json"
                    && candidate.file.dimensions.is_none()
                    && candidate.file.bytes <= MAX_REPORT_BYTES
                    && candidate.provenance == Provenance::Diagnostic,
                "acoustic_candidate",
                "outputs",
                "unexpected normalization candidate identity, provenance or size",
            )?;
            let bytes = services.read_candidate(&candidate).await?;
            services.check_cancelled().await?;
            candidate.file.verify(&bytes)?;
            findings = result.findings;
            let selected_id = candidate.candidate_id.clone();
            let file = candidate.file.clone();
            artifacts.push(candidate);
            (bytes, file, selected_id)
        }
        ExecutionMode::Replay => {
            let recorded = input
                .inputs
                .iter()
                .find(|a| a.file.path == "acoustic-replay.json")
                .ok_or_else(|| {
                    StageError::Evidence(
                        "recorded acoustic-replay.json required; replay never dispatches a helper"
                            .into(),
                    )
                })?;
            require(
                recorded.file.bytes <= MAX_REPORT_BYTES,
                "acoustic_candidate",
                "inputs",
                "recorded normalization exceeds bound",
            )?;
            (
                get(files, "acoustic-replay.json")?.1.clone(),
                recorded.file.clone(),
                recorded.id.clone(),
            )
        }
        ExecutionMode::Live => {
            return Err(StageError::Evidence(
                "offline acoustic stage has no live provider adapter".into(),
            ));
        }
    };
    let report = validate(&bytes, files, &recipe)?;
    if input.mode == ExecutionMode::Replay
        && let Some((_, bytes)) = files.get("acoustic-validation.json")
    {
        let recorded: RecordedValidation = parse(bytes)?;
        require(
            recorded.schema_version == 1
                && recorded.stage == "acoustic"
                && recorded.provenance == Provenance::Diagnostic
                && recorded.provider_calls == 0
                && !recorded.production_approved
                && recorded.normalized_report_sha256 == normalized_ref.sha256
                && recorded.helper_recipe_sha256 == recipe
                && recorded.source_sha256 == report.source_sha256
                && recorded.narration_sha256 == report.audio_sha256,
            "acoustic_replay_binding",
            "acoustic-validation.json",
            "recorded findings are bound to different observations",
        )?;
        findings = recorded.findings;
    }
    let mut required = report.missing_evidence.clone();
    required.extend(
        [
            "independent-acoustic-evidence",
            "narrator-continuity",
            "speech-fidelity",
            "rendered-creative-review",
        ]
        .map(str::to_owned),
    );
    if report.independent.is_empty() {
        required.push("independent-transcription".into());
    }
    required.sort();
    required.dedup();
    for id in required {
        if findings
            .iter()
            .any(|f| f.id == id && f.outcome != ReviewOutcome::Pass)
        {
            continue;
        }
        findings.retain(|f| f.id != id);
        findings.push(Finding{id,field:"acoustic".into(),outcome:ReviewOutcome::Missing,
            message:"Saved acoustic observations do not establish complete production fact/fidelity acceptance.".into(),evidence:input.inputs.clone()});
    }
    let missing_ids = findings
        .iter()
        .filter(|f| f.outcome != ReviewOutcome::Pass)
        .map(|f| f.id.clone())
        .collect::<Vec<_>>();
    let summary = serde_json::json!({"schema_version":1,"stage":"acoustic","mode":input.mode,"provenance":"diagnostic",
        "production_approved":false,"provider_calls":0,"structural_read_along":"pass",
        "word_count":report.word_track.len(),"highlight_group_count":report.highlight_groups.len(),
        "point_word_count":report.word_track.iter().filter(|w|w.start_sample==w.end_sample).count(),
        "sample_rate":report.sample_rate,"source_sha256":report.source_sha256,"narration_sha256":report.audio_sha256,
        "normalized_report":normalized_ref,"normalized_report_sha256":normalized_ref.sha256,
        "helper_recipe_sha256":recipe,"implementation_sha256":digest(include_bytes!("acoustic.rs")),
        "contract_revision":CONTRACT_REVISION,"service_revision":SERVICE_REVISION,
        "run_id":input.run_id,"unit_id":input.unit_id,"epoch":input.epoch,"revision":input.revision,
        "pins":input.pins,"inputs":input.inputs,"input_hashes":report.input_hashes,
        "reveal_permissions":[],"missing_evidence":missing_ids,"findings":findings,
        "scope":"Original narration-domain observations; delivery mapping, full speech fidelity and independent per-fact reveal verification remain unresolved."});
    artifacts.push(write(services, input, "acoustic-validation.json", &summary).await?);
    services.check_cancelled().await?;
    if input.mode == ExecutionMode::Fixture {
        services.emit(JournalProposal{kind:EventKind::CheckCompleted,task_id:Some(input.unit_id.clone()),decision_id:None,
            artifact_ids:input.inputs.iter().map(|a|a.id.clone()).collect(),
            summary:format!("Verified offline acoustic candidate: {} source words / {} atomic groups; required production evidence remains unresolved.",report.word_track.len(),report.highlight_groups.len())}).await?;
    }
    services.check_cancelled().await?;
    Ok(StageOutput{outcome:StageOutcome::NeedsRepair,artifacts,
        decisions:vec![Decision{id:digest(&json(&(input.unit_id.as_str(),normalized_ref.sha256.as_str(),input.mode))?),
            summary:"Preserve verified saved timing and point grouping; do not grant source-fact reveal permissions.".into(),
            inputs:input.inputs.clone(),selected_ids:vec![selected_id],rejected_ids:vec![],supersedes:None}],
        findings,cache_keys:vec![cache_key,digest(&json(&(recipe,digest(include_bytes!("acoustic.rs")),normalized_ref.sha256))?)],
        library_updates:vec![],package_candidate:None})
}
