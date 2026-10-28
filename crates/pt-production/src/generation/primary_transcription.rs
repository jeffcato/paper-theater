//! Private unhinted primary evidence. No materialization, execution or reveal authority.
use super::*;
use serde::Deserialize;
use serde_json::{Value, json};
#[path = "primary_mapping.rs"]
mod mapping;
#[path = "primary_match.rs"]
mod matching;
#[path = "primary_verify.rs"]
mod verify;
#[path = "primary_words.rs"]
mod words;
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
const META: u64 = 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    speech_input: FileRef,
    speech_context: FileRef,
    speech_plan: FileRef,
    take_manifest: FileRef,
    copy_plan: FileRef,
    batch_id: Id,
    batch_audio: ArtifactRef,
    copy_receipt: FileRef,
    copy_recipe_sha256: Digest,
    recipe: FileRef,
    provider: Id,
    model: Id,
    estimate: Usage,
    timeout_ms: u64,
    request_payload: Option<FileRef>,
    observation: Option<FileRef>,
    limits: Limits,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    max_words: u32,
    max_source_tokens: u32,
    max_match_cells: u64,
    max_candidate_edges: u32,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    schema_version: u32,
    provenance: Provenance,
    provider: Id,
    model: Id,
    request: ArtifactRef,
    audio: ArtifactRef,
    copy_recipe_sha256: Digest,
    recipe_sha256: Digest,
    response: Option<ArtifactRef>,
    issued: Option<ProviderOperationV2>,
    result: Option<ProviderResultV2>,
    findings: Vec<Finding>,
}
fn need(ok: bool, msg: &str) -> Result<(), StageError> {
    require(ok, "primary-transcription", "evidence", msg)?;
    Ok(())
}
fn bytes<'a>(files: &'a Files, f: &FileRef) -> Result<&'a [u8], StageError> {
    let (actual, b) = files
        .get(&f.path)
        .ok_or_else(|| StageError::Evidence(format!("missing primary input: {}", f.path)))?;
    need(actual == f, "primary FileRef changed")?;
    Ok(b)
}
fn artifact(input: &StageInput, f: &FileRef) -> Result<ArtifactRef, StageError> {
    input
        .inputs
        .iter()
        .find(|a| a.file == *f)
        .cloned()
        .ok_or_else(|| {
            StageError::Evidence("primary genuine committed ArtifactRef required".into())
        })
}
async fn read(
    input: &StageInput,
    services: &dyn RunServices,
    f: &FileRef,
) -> Result<Vec<u8>, StageError> {
    need(f.bytes <= META, "primary preflight metadata limit")?;
    services.check_cancelled().await?;
    let b = services.read_artifact(&artifact(input, f)?).await?;
    f.verify(&b)?;
    services.check_cancelled().await?;
    Ok(b)
}
pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
) -> Result<StageOutput, StageError> {
    need(
        input.inputs.len() <= 64
            && input.inputs.iter().all(|a| a.file.bytes <= 16 * META)
            && input.inputs.iter().map(|a| a.file.bytes).sum::<u64>() <= 64 * META,
        "primary input envelope limit",
    )?;
    let context = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/primary-transcription-context.json")
        .ok_or_else(|| StageError::Evidence("missing primary context".into()))?;
    let ctx: Context = parse(&read(input, services, &context.file).await?)?;
    need(
        ctx.schema_version == 1
            && ctx.recipe.sha256 == input.pins.recipe_sha256
            && valid_digest(&ctx.copy_recipe_sha256)
            && [&ctx.batch_id, &ctx.provider, &ctx.model]
                .iter()
                .all(|x| valid_id(x)),
        "primary context identity",
    )?;
    need(
        (1..=300000).contains(&ctx.timeout_ms)
            && ctx.estimate.requests > 0
            && [
                ctx.estimate.requests,
                ctx.estimate.input_tokens,
                ctx.estimate.output_tokens,
                ctx.estimate.money_microusd,
            ]
            .iter()
            .all(|n| *n <= MAX_SAFE_INTEGER),
        "primary planned usage/timeout bounds",
    )?;
    need(
        (1..=2048).contains(&ctx.limits.max_words)
            && (1..=8192).contains(&ctx.limits.max_source_tokens)
            && (1..=2000000).contains(&ctx.limits.max_match_cells)
            && (1..=16384).contains(&ctx.limits.max_candidate_edges),
        "primary matching limits",
    )?;
    let speech_context: Value = parse(&read(input, services, &ctx.speech_context).await?)?;
    let cap = speech_context["limits"]["transcription_max_wav_bytes"]
        .as_u64()
        .ok_or_else(|| StageError::Evidence("missing original Speech bounds".into()))?
        .min(16 * META);
    need(
        ctx.batch_audio.file.media_type == "audio/wav"
            && ctx.batch_audio.file.bytes <= cap
            && json!(artifact(input, &ctx.batch_audio.file)?) == json!(ctx.batch_audio),
        "primary materialized audio role/bound",
    )?;
    for a in &input.inputs {
        let limit = if json!(a) == json!(ctx.batch_audio) {
            cap
        } else if ["raw-source.txt", "clean-source.txt"].contains(&a.file.path.as_str()) {
            16 * META
        } else if a.file.path.ends_with(".wav") && a.file.media_type == "audio/wav" {
            8 * META
        } else {
            META
        };
        need(a.file.bytes <= limit, "primary artifact role bound")?;
    }
    let files = inputs::load(input, services).await?.files;
    bytes(&files, &ctx.recipe)?;
    let verified = verify::batch(input, services, &files, &ctx, &speech_context).await?;
    let mut findings: Vec<Finding> = serde_json::from_value(verified.speech["findings"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    let request = json!({"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":ctx.batch_audio});
    let b = json(&request)?;
    let request_file = FileRef {
        path: "private/primary/request.json".into(),
        sha256: digest(&b),
        bytes: b.len() as u64,
        media_type: "application/json".into(),
        dimensions: None,
    };
    let mut outputs: Vec<(String, Value)> = vec![("private/primary/request.json".into(), request)];
    let mut proposal = None;
    if let Some(f) = &ctx.request_payload {
        need(
            *f == request_file,
            "committed primary request differs from exact audio-only compilation",
        )?;
        bytes(&files, f)?;
        let p = ProviderProposalV2 {
            schema_version: 2,
            kind: OperationKind::Transcription,
            provider: ctx.provider.clone(),
            model: ctx.model.clone(),
            recipe_sha256: ctx.recipe.sha256.clone(),
            payload: artifact(input, f)?,
            inputs: vec![ctx.batch_audio.clone()],
            estimate: ctx.estimate.clone(),
            timeout_ms: ctx.timeout_ms,
        };
        p.validate()?;
        outputs.push(("private/primary/proposal.json".into(), json!(p)));
        proposal = Some(p);
    } else {
        findings.push(missing("primary-request-commit-pending","Runtime must commit/register exact unhinted request bytes before any content-only v2 proposal can name its ArtifactRef."));
    }
    let mut parsed = words::Parsed::empty("observation-missing");
    let mut observation = None;
    if let Some(f) = &ctx.observation {
        let obs: Observation = parse(bytes(&files, f)?)?;
        let p = proposal.as_ref().ok_or_else(|| {
            StageError::Evidence(
                "primary observation requires exact registered committed request".into(),
            )
        })?;
        need(
            obs.schema_version == 1
                && matches!(
                    obs.provenance,
                    Provenance::Synthetic | Provenance::Imported | Provenance::Diagnostic
                )
                && obs.provider == ctx.provider
                && obs.model == ctx.model
                && json!(obs.request) == json!(p.payload)
                && json!(obs.audio) == json!(ctx.batch_audio)
                && obs.copy_recipe_sha256 == ctx.copy_recipe_sha256
                && obs.recipe_sha256 == ctx.recipe.sha256,
            "saved primary request/audio/recipe/provider/model identity",
        )?;
        let observer = ProviderObserverV2 {
            schema_version: 2,
            run_id: input.run_id.clone(),
            epoch: input.epoch,
            unit_id: input.unit_id.clone(),
        };
        if let Some(issued) = &obs.issued {
            issued.validate_for_proposal(p, &observer)?;
            if let Some(result) = &obs.result {
                result.validate_for_operation(issued)?;
                need(
                    obs.response.as_ref().is_none_or(|a| {
                        result.result.completion == ProviderCompletion::Completed
                            && result.epoch_disposition == ProviderEpochDisposition::Current
                            && result.retry.is_none()
                            && result.recovery.is_none()
                            && result
                                .result
                                .raw_response
                                .as_ref()
                                .is_some_and(|r| json!(a) == json!(r))
                    }),
                    "unusable or changed primary result/raw identity",
                )?;
            } else {
                need(
                    obs.response.is_none(),
                    "primary response without completed result",
                )?;
            }
        } else {
            need(
                obs.result.is_none(),
                "primary result without original issuance",
            )?;
        }
        findings.extend(obs.findings.clone());
        if let Some(raw) = &obs.response {
            need(
                json!(artifact(input, &raw.file)?) == json!(raw),
                "original raw primary ArtifactRef changed",
            )?;
            parsed = words::parse_response(
                bytes(&files, &raw.file)?,
                &ctx.model,
                &verified.info,
                &ctx.limits,
            )?;
        } else {
            parsed = words::Parsed::empty("original-attempt-unresolved");
        }
        observation = Some(obs);
    }
    if parsed.status != "completed" {
        findings.push(missing(
            "primary-response-unresolved",
            &format!(
                "Original measured response remains unresolved: {}",
                parsed.status
            ),
        ));
    }
    let normalized = mapping::normalize(&verified, &parsed, &ctx.limits)?;
    if !normalized["unmatched_source"]
        .as_array()
        .unwrap()
        .is_empty()
    {
        findings.push(missing(
            "primary-source-coverage",
            "Complete global source coverage is not established by these measured observations.",
        ));
    }
    if normalized["words"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["source_span"].is_null())
    {
        findings.push(missing("primary-mapping-uncertainty","Global ambiguity, unmatched recognition, native scope or artificial join/cut remains unresolved; no word timing is invented."));
    }
    if !normalized["unresolved_points"]
        .as_array()
        .unwrap()
        .is_empty()
    {
        findings.push(missing("primary-point-grouping","Measured points lack a following compatible positive interval at the same original/native boundary; no duration is assigned."));
    }
    for id in [
        "primary-execution-unavailable",
        "primary-independent-acoustic-evidence",
        "speech-fidelity",
        "voice-pilot",
        "production-verifier-pending",
    ] {
        findings.push(missing(id,"Private primary mechanics grant no execution, independent verification, listening, fidelity, reveal or publication authority."));
    }
    let compiler_sha256 = digest(
        &[
            include_bytes!("primary_transcription.rs").as_slice(),
            include_bytes!("primary_verify.rs").as_slice(),
            include_bytes!("primary_words.rs").as_slice(),
            include_bytes!("primary_match.rs").as_slice(),
            include_bytes!("primary_mapping.rs").as_slice(),
            include_bytes!("../../../monotonic-align/src/normalize.rs").as_slice(),
            include_bytes!("../../../pt-contracts/src/wav.rs").as_slice(),
        ]
        .concat(),
    );
    outputs.push(("private/primary/handoff.json".into(),json!({"schema_version":1,"request_file":request_file,"kind":"transcription","provider":ctx.provider,"model":ctx.model,"recipe_sha256":ctx.recipe.sha256,"inputs":[ctx.batch_audio],"estimate":ctx.estimate,"timeout_ms":ctx.timeout_ms,"runtime_committed_payload_required":true,"caller_authority":false,"dispatch_permitted":false})));
    outputs.push(("private/primary/normalized.json".into(),json!({"schema_version":1,"provenance":"diagnostic","compiler_sha256":compiler_sha256,"batch_id":ctx.batch_id,"batch_format":{"sample_rate":verified.info.sample_rate,"channels":verified.info.channels,"frames":verified.info.frame_count},"native_segments":verified.batch["segments"],"unobserved_planned_batches":verified.plan["batches"].as_array().unwrap().iter().filter(|b|b["id"]!=ctx.batch_id).map(|b|b["id"].clone()).collect::<Vec<_>>(),"stage_provider_dispatch_calls":0,"stage_helper_dispatch_calls":0,"fact_bounds":null,"independent_evidence":null,"source_sha256":verified.speech["source_sha256"],"source_plan":ctx.speech_plan,"take_manifest":ctx.take_manifest,"copy_plan":ctx.copy_plan,"batch_audio":ctx.batch_audio,"copy_receipt":ctx.copy_receipt,"copy_recipe_sha256":ctx.copy_recipe_sha256,"recipe":ctx.recipe,"request":request_file,"observation_file":ctx.observation,"observation":observation,"parser_status":parsed.status,"invalid_word_index":parsed.invalid_word,"words":normalized["words"],"highlight_groups":normalized["highlight_groups"],"unresolved_points":normalized["unresolved_points"],"unmatched_source":normalized["unmatched_source"],"unplanned_audio":verified.plan["unplanned_audio"],"matching_policy":"global-optimal-monotonic-occurrences-v1/monotonic-exact-v1-tokenization","rounding":"exact-decimal-ceiling-v1","grouping_policy":"point-following-same-boundary-v1","copy_verified":true,"native_domains_preserved":true,"global_narration":null,"delivery":null,"reveal_permissions":[],"production_approved":false,"findings":findings})));
    let key = digest(&json(&outputs)?);
    for (path, v) in &outputs {
        let b = json(v)?;
        need(b.len() as u64 <= META, "primary candidate bound")?;
        if input.mode == ExecutionMode::Replay
            && let Some((f, _)) = files.get(path)
        {
            need(
                f.sha256 == digest(&b) && f.bytes == b.len() as u64,
                "stale primary replay candidate",
            )?;
        }
    }
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
