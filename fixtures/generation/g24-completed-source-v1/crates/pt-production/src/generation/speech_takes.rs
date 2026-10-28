//! Verify immutable saved result/native PCM identities without synthesis or audio edits.
use super::super::pilot_provider::Observation;
use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Format {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    frames: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    schema_version: u32,
    provenance: Provenance,
    request_sha256: Digest,
    recipe_sha256: Digest,
    source_sha256: Digest,
    bindings_sha256: Digest,
    unit_id: Id,
    turn_id: Option<Id>,
    span: SourceSpan,
    text_sha256: Digest,
    speaker: Id,
    model: Id,
    voice_id: Id,
    audio: ArtifactRef,
    format: Format,
}
pub(super) fn ingest(
    input: &StageInput,
    files: &Files,
    ctx: &Context,
    requests: &BTreeMap<String, Value>,
    proposals: &BTreeMap<String, ProviderProposalV2>,
    findings: &mut Vec<Finding>,
) -> Result<(Vec<Value>, Vec<Value>), StageError> {
    let observer = ProviderObserverV2 {
        schema_version: 2,
        run_id: input.run_id.clone(),
        epoch: input.epoch,
        unit_id: input.unit_id.clone(),
    };
    let mut seen = BTreeSet::new();
    let mut takes = vec![];
    let mut observations = vec![];
    for f in &ctx.observations {
        let obs: Observation = parse(bytes(files, f)?)?;
        need(
            obs.schema_version == 1
                && matches!(
                    obs.provenance,
                    Provenance::Imported | Provenance::Synthetic | Provenance::Diagnostic
                )
                && seen.insert(obs.request.file.path.clone()),
            "ambiguous/malformed Speech observation",
        )?;
        let p = proposals.get(&obs.request.file.path).ok_or_else(|| {
            StageError::Evidence("saved take requires exact committed Speech payload".into())
        })?;
        need(
            json!(obs.request) == json!(p.payload),
            "saved Speech request ArtifactRef changed",
        )?;
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
                                .is_some_and(|raw| json!(raw) == json!(a))
                    }),
                    "Speech result/raw response is unusable or changed",
                )?;
            } else {
                need(
                    obs.response.is_none(),
                    "Speech response lacks completed issued result",
                )?;
            }
        } else {
            need(
                obs.result.is_none(),
                "Speech result lacks original issuance",
            )?;
        }
        findings.extend(obs.findings.clone());
        observations.push(json!({"file":f,"record":obs}));
        let Some(response) = &obs.response else {
            findings.push(missing("speech-saved-attempt-unresolved","Original attempt is incomplete/uncertain; retain its metadata and do not repurchase."));
            continue;
        };
        need(
            json!(artifact(input, &response.file)?) == json!(response),
            "saved Speech raw ArtifactRef changed",
        )?;
        let raw: Raw = parse(bytes(files, &response.file)?)?;
        let req = &requests[&obs.request.file.path];
        need(
            raw.schema_version == 1
                && matches!(
                    raw.provenance,
                    Provenance::Imported | Provenance::Synthetic | Provenance::Diagnostic
                )
                && raw.request_sha256 == obs.request.file.sha256
                && raw.recipe_sha256 == ctx.recipe.sha256
                && raw.source_sha256 == req["source_sha256"]
                && raw.bindings_sha256 == ctx.bindings.sha256
                && raw.unit_id == req["unit"]["id"]
                && json!(raw.turn_id) == req["unit"]["turn_id"]
                && json!(raw.span) == req["unit"]["span"]
                && raw.text_sha256 == digest(req["text"].as_str().unwrap().as_bytes())
                && raw.speaker == req["speaker"]
                && raw.model == req["model"]
                && raw.voice_id == req["voice_id"],
            "Speech raw/source/turn/span/recipe/model/voice/request mismatch",
        )?;
        need(
            raw.audio.file.media_type == "audio/wav"
                && raw.audio.file.bytes <= 8 * META
                && json!(artifact(input, &raw.audio.file)?) == json!(raw.audio),
            "native Speech WAV ArtifactRef changed",
        )?;
        if let Some(result) = &obs.result {
            need(
                result
                    .result
                    .outputs
                    .iter()
                    .any(|a| json!(a) == json!(raw.audio)),
                "Speech result lacks exact native audio output",
            )?;
        }
        let native = super::super::pilot_evidence::format(bytes(files, &raw.audio.file)?)?;
        need(
            native == json!(raw.format),
            "Speech WAV actual format/frame count differs",
        )?;
        let cache_key = digest(&json(&(
            obs.request.file.sha256.as_str(),
            &raw.audio,
            &ctx.recipe,
            &req["catalog"],
            &req["model"],
            &req["voice_id"],
        ))?);
        takes.push(json!({"unit_id":raw.unit_id,"turn_id":raw.turn_id,"source_span":raw.span,"source_sha256":raw.source_sha256,"speaker":raw.speaker,"model":raw.model,"voice_id":raw.voice_id,"bindings":ctx.bindings,"recipe":ctx.recipe,"request":obs.request,"raw_response":response,"observation":f,"audio":raw.audio,"native":native,"native_span":[0,raw.format.frames],"global_narration_span":null,"cache_key":cache_key,"speech_fidelity":"missing","production_approved":false}));
    }
    takes.sort_by(|a, b| a["unit_id"].as_str().cmp(&b["unit_id"].as_str()));
    observations.sort_by(|a, b| {
        a["record"]["request"]["file"]["path"]
            .as_str()
            .cmp(&b["record"]["request"]["file"]["path"].as_str())
    });
    for req in requests.values() {
        if !takes.iter().any(|t| t["unit_id"] == req["unit"]["id"]) {
            findings.push(missing(&format!("speech-take-missing:{}",req["unit"]["id"].as_str().unwrap()),"No compatible verified recording; preserve source and do not synthesize or repurchase."));
        }
    }
    Ok((takes, observations))
}
