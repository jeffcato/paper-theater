//! Bounded source-complete Speech proposals. Diagnostic Pilot never authorizes synthesis.
use super::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[path = "speech_takes.rs"]
mod takes;
#[path = "speech_transcription.rs"]
mod transcription;
#[path = "speech_units.rs"]
mod units;
#[path = "speech_verify.rs"]
mod verify;
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
const META: u64 = 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    book_id: Id,
    chapter_id: Id,
    pilot: FileRef,
    pilot_input: FileRef,
    bindings: FileRef,
    recipe: FileRef,
    speech_provider: Id,
    transcription_provider: Id,
    transcription_model: Id,
    estimate: Usage,
    timeout_ms: u64,
    limits: Limits,
    request_payloads: Vec<FileRef>,
    observations: Vec<FileRef>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    max_unit_text_bytes: u64,
    max_units: u32,
    transcription_max_wav_bytes: u64,
    transcription_max_seconds: u32,
    transcription_max_segments: u32,
    #[serde(default = "default_batches")]
    transcription_max_batches: u32,
}
fn default_batches() -> u32 {
    128
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    schema_version: u32,
    book_id: Id,
    revision: u64,
    entries: Vec<Binding>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    speaker: Id,
    canonical_id: Id,
    provider: Id,
    model: Id,
    voice_id: Id,
    catalog: ArtifactRef,
    language: String,
    accent: String,
    baseline: String,
    reference: Option<ArtifactRef>,
    established: Option<ArtifactRef>,
}
fn need(ok: bool, message: &str) -> Result<(), StageError> {
    require(ok, "speech", "evidence", message)?;
    Ok(())
}
fn bytes<'a>(files: &'a Files, f: &FileRef) -> Result<&'a [u8], StageError> {
    let (actual, b) = files
        .get(&f.path)
        .ok_or_else(|| StageError::Evidence(format!("missing speech input: {}", f.path)))?;
    need(actual == f, "speech FileRef changed")?;
    Ok(b)
}
fn artifact(input: &StageInput, f: &FileRef) -> Result<ArtifactRef, StageError> {
    input
        .inputs
        .iter()
        .find(|a| a.file == *f)
        .cloned()
        .ok_or_else(|| StageError::Evidence("speech committed ArtifactRef required".into()))
}
pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
) -> Result<StageOutput, StageError> {
    need(
        input.inputs.len() <= 64
            && input.inputs.iter().all(|a| {
                a.file.bytes
                    <= if ["raw-source.txt", "clean-source.txt"].contains(&a.file.path.as_str()) {
                        16 * META
                    } else if a.file.path.ends_with(".wav") && a.file.media_type == "audio/wav" {
                        8 * META
                    } else {
                        META
                    }
            })
            && input.inputs.iter().map(|a| a.file.bytes).sum::<u64>() <= 64 * META,
        "speech input envelope limit",
    )?;
    let files = inputs::load(input, services).await?.files;
    let Some((_, b)) = files.get("private/speech-context.json") else {
        return Ok(StageOutput {
            outcome: StageOutcome::NeedsRepair,
            artifacts: vec![],
            decisions: vec![],
            findings: vec![missing(
                "speech-context-missing",
                "Runtime-pinned Speech context is required.",
            )],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        });
    };
    let ctx: Context = parse(b)?;
    need(
        ctx.schema_version == 1
            && ctx.recipe.sha256 == input.pins.recipe_sha256
            && [
                &ctx.book_id,
                &ctx.chapter_id,
                &ctx.speech_provider,
                &ctx.transcription_provider,
                &ctx.transcription_model,
            ]
            .iter()
            .all(|s| valid_id(s)),
        "speech context identity",
    )?;
    bytes(&files, &ctx.recipe)?;
    need(
        (1..=META).contains(&ctx.limits.max_unit_text_bytes)
            && (1..=1024).contains(&ctx.limits.max_units)
            && (45..=16 * META).contains(&ctx.limits.transcription_max_wav_bytes)
            && (1..=90).contains(&ctx.limits.transcription_max_seconds)
            && (1..=64).contains(&ctx.limits.transcription_max_segments)
            && (1..=256).contains(&ctx.limits.transcription_max_batches),
        "speech configured bounds",
    )?;
    let pilot = verify::pilot(input, services, &files, &ctx).await?;
    let direction_file: FileRef = serde_json::from_value(pilot["direction"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    let direction: Value = parse(bytes(&files, &direction_file)?)?;
    need(
        direction["book_id"] == ctx.book_id && direction["chapter_id"] == ctx.chapter_id,
        "speech source/book/chapter identity",
    )?;
    let bindings: Bindings = parse(bytes(&files, &ctx.bindings)?)?;
    need(
        bindings.schema_version == 1
            && bindings.book_id == ctx.book_id
            && bindings.revision == input.library_revision
            && bindings.entries.len() <= 32,
        "speech binding registry revision",
    )?;
    let mut findings: Vec<Finding> = serde_json::from_value(pilot["findings"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    let mut supported = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for binding in &bindings.entries {
        need(
            valid_id(&binding.speaker) && seen.insert(binding.speaker.clone()),
            "duplicate/invalid speech performer",
        )?;
        let p = pilot["performers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["speaker"] == binding.speaker);
        let cast = direction["cast"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["proposal"]["id"] == binding.speaker);
        let compatible = p.is_some_and(|p| {
            p["model"] == binding.model
                && p["voice_id"] == binding.voice_id
                && p["language"] == binding.language
                && p["accent"] == binding.accent
                && p["baseline"] == binding.baseline
                && p["catalog"] == json!(binding.catalog.file)
                && p["reference_audio"] == json!(binding.reference)
                && p["retained_binding"]["artifact"] == json!(binding.established)
        }) && cast
            .is_some_and(|c| c["proposal"]["canonical_id"] == binding.canonical_id)
            && direction["speakers"]
                .as_array()
                .unwrap()
                .contains(&json!(binding.speaker))
            && binding.provider == ctx.speech_provider;
        if !compatible {
            findings.push(missing(&format!("speech-voice-binding:{}",binding.speaker),"Missing/stale/unsupported performer identity remains unresolved; no recast or invented role."));
            continue;
        }
        need(
            json!(artifact(input, &binding.catalog.file)?) == json!(binding.catalog),
            "speech catalog ArtifactRef changed",
        )?;
        supported.insert(binding.speaker.clone(), binding);
    }
    let acting = units::acting(&pilot, direction["source_text"].as_str().unwrap());
    let units = units::compile(&direction, &ctx.limits, &acting, &mut findings)?;
    let mut outputs = vec![];
    let mut registered = BTreeSet::new();
    need(
        ctx.request_payloads.len() <= 32 && ctx.observations.len() <= 32,
        "speech saved inventory limit",
    )?;
    for f in &ctx.request_payloads {
        need(
            registered.insert(f.path.clone()),
            "duplicate speech request registration",
        )?;
        bytes(&files, f)?;
    }
    let source_file: FileRef = serde_json::from_value(pilot["source"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    let base = [
        &source_file,
        &direction_file,
        &ctx.pilot,
        &ctx.pilot_input,
        &ctx.bindings,
        &ctx.recipe,
    ]
    .into_iter()
    .map(|f| artifact(input, f))
    .collect::<Result<Vec<_>, _>>()?;
    let mut handoff = vec![];
    let mut requests = BTreeMap::new();
    let mut proposals = BTreeMap::new();
    for unit in &units {
        let binding = unit.speaker.as_ref().and_then(|s| supported.get(s));
        if !unit.request_ready || binding.is_none() {
            findings.push(missing(
                &format!("speech-unit-binding:{}", unit.id),
                "Source unit retained but cannot produce a request without its verified performer.",
            ));
            continue;
        }
        let binding = binding.unwrap();
        let intents = acting
            .iter()
            .filter(|p| {
                unit.speaker.as_ref() == Some(&p.speaker)
                    && p.span[0] <= unit.span[0]
                    && unit.span[1] <= p.span[1]
            })
            .collect::<Vec<_>>();
        let styles = intents
            .iter()
            .map(|p| p.acting.as_str())
            .collect::<BTreeSet<_>>();
        if styles.len() > 1 {
            findings.push(missing(
                "speech-acting-ambiguous",
                "Conflicting source-scoped acting stays unresolved; no style is silently selected.",
            ));
            continue;
        }
        let adjustment = styles.first().copied().unwrap_or("");
        let request = json!({"schema_version":1,"task":"verbatim-speech-unit-v1","source_sha256":direction["source_sha256"],"direction":direction_file,"pilot":ctx.pilot,"bindings":ctx.bindings,"recipe":ctx.recipe,"unit":unit,"speaker":binding.speaker,"canonical_id":binding.canonical_id,"provider":binding.provider,"model":binding.model,"voice_id":binding.voice_id,"catalog":binding.catalog,"language":binding.language,"accent":binding.accent,"baseline":binding.baseline,"acting":adjustment,"acting_intents":intents,"text":unit.text,"bulk_speech_approved":false});
        let path = format!("private/speech/requests/{}.json", unit.id);
        let b = json(&request)?;
        need(b.len() as u64 <= META, "speech request bound")?;
        let file = FileRef {
            path: path.clone(),
            sha256: digest(&b),
            bytes: b.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        };
        let mut dependencies = base.clone();
        dependencies.push(binding.catalog.clone());
        for a in [&binding.reference, &binding.established]
            .into_iter()
            .flatten()
        {
            need(
                json!(artifact(input, &a.file)?) == json!(a),
                "speech reference/established ArtifactRef changed",
            )?;
            dependencies.push(a.clone());
        }
        handoff.push(json!({"request_file":file,"kind":"speech","provider":binding.provider,"model":binding.model,"recipe_sha256":ctx.recipe.sha256,"inputs":dependencies,"estimate":ctx.estimate,"timeout_ms":ctx.timeout_ms,"runtime_committed_payload_required":true,"dispatch_permitted":false}));
        if ctx.request_payloads.iter().any(|f| f.path == path) {
            need(
                ctx.request_payloads.iter().any(|f| f == &file),
                "committed speech request differs from exact compilation",
            )?;
            let p = ProviderProposalV2 {
                schema_version: 2,
                kind: OperationKind::Speech,
                provider: binding.provider.clone(),
                model: binding.model.clone(),
                recipe_sha256: ctx.recipe.sha256.clone(),
                payload: artifact(input, &file)?,
                inputs: dependencies,
                estimate: ctx.estimate.clone(),
                timeout_ms: ctx.timeout_ms,
            };
            p.validate()?;
            outputs.push((
                format!("private/speech/proposals/{}.json", unit.id),
                json!(p),
            ));
            proposals.insert(path.clone(), p);
        } else {
            findings.push(missing("speech-request-commit-pending","Runtime must commit/register exact requests before proposals can use genuine ArtifactRefs; no dispatch is permitted."));
        }
        requests.insert(path.clone(), request.clone());
        outputs.push((path, request));
    }
    need(
        ctx.request_payloads
            .iter()
            .all(|f| requests.contains_key(&f.path)),
        "registered Speech payload has no supported current unit",
    )?;
    let (saved_takes, observations) =
        takes::ingest(input, &files, &ctx, &requests, &proposals, &mut findings)?;
    let transcription_plan = transcription::plan(&saved_takes, &ctx, &mut findings)?;
    outputs.push((
        "private/speech-transcription-plan.json".into(),
        transcription_plan,
    ));
    outputs.push(("private/speech-takes.json".into(),json!({"schema_version":1,"provenance":"diagnostic","source_sha256":direction["source_sha256"],"pilot":ctx.pilot,"bindings":ctx.bindings,"recipe":ctx.recipe,"takes":saved_takes,"observations":observations,"narration_assembly":null,"delivery_master":null,"production_approved":false})));
    outputs.push(("private/speech-handoff.json".into(),json!({"schema_version":1,"caller_authority":false,"dispatch_permitted":false,"requests":handoff,"runtime_bulk_authorization_required":true})));
    for id in [
        "speech-bulk-authorization",
        "speech-runtime-execution-unavailable",
        "speech-listening-continuity",
        "speech-joins-assembly",
        "speech-measured-acoustic-evidence",
        "speech-primary-transcription-sizing-pilot",
        "production-verifier-pending",
    ] {
        findings.push(missing(id,"Planning/saved ingestion confers no pilot, bulk-synthesis, listening, join, acoustic or publication authority; no execution fallback."));
    }
    let compiler_sha256 = digest(
        &[
            include_bytes!("speech.rs").as_slice(),
            include_bytes!("speech_verify.rs").as_slice(),
            include_bytes!("speech_units.rs").as_slice(),
            include_bytes!("speech_takes.rs").as_slice(),
            include_bytes!("speech_transcription.rs").as_slice(),
        ]
        .concat(),
    );
    outputs.push(("private/speech-plan.json".into(),json!({"schema_version":1,"provenance":"diagnostic","compiler_sha256":compiler_sha256,"source_text":direction["source_text"],"source_sha256":direction["source_sha256"],"source_complete":units.iter().map(|u|u.text.as_str()).collect::<String>()==direction["source_text"],"pilot":ctx.pilot,"bindings":ctx.bindings,"recipe":ctx.recipe,"request_payloads":ctx.request_payloads,"units":units,"bulk_speech_approved":false,"production_approved":false,"reveal_permissions":[],"runtime_policy_authority":false,"findings":findings})));
    for (path, v) in &outputs {
        let b = json(&v)?;
        need(b.len() as u64 <= META, "speech candidate bound")?;
        if input.mode == ExecutionMode::Replay
            && let Some((f, _)) = files.get(path)
        {
            need(
                f.sha256 == digest(&b) && f.bytes == b.len() as u64,
                "stale recorded Speech candidate",
            )?;
        }
    }
    let key = digest(&json(&outputs)?);
    let mut artifacts = vec![];
    for (path, v) in outputs {
        artifacts.push(write(services, input, &path, &v).await?);
    }
    Ok(StageOutput {
        outcome: StageOutcome::NeedsRepair,
        artifacts,
        decisions: vec![],
        findings,
        cache_keys: vec![key],
        library_updates: vec![],
        package_candidate: None,
    })
}
