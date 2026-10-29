//! Read-only reconstruction and exact PCM comparison. Runtime alone creates and commits batches.
use super::*;
use std::{io::Cursor, sync::Mutex};
struct Verification<'a> {
    services: &'a dyn RunServices,
    files: &'a Files,
    allowed: &'a [ArtifactRef],
    writes: Mutex<BTreeMap<String, Vec<u8>>>,
}
#[async_trait]
impl RunServices for Verification<'_> {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        self.check_cancelled().await?;
        if self.allowed.iter().any(|v| json!(v) == json!(a)) {
            return Ok(self
                .files
                .get(&a.file.path)
                .ok_or_else(|| ServiceError::MissingArtifact(a.id.clone()))?
                .1
                .clone());
        }
        Err(ServiceError::MissingArtifact(a.id.clone()))
    }
    async fn write_candidate(
        &self,
        r: CandidateWrite,
        b: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        self.check_cancelled().await?;
        // Verification-only candidate metadata; no committed IDs, CAS or runtime effects.
        let f = FileRef {
            path: r.relative_path.clone(),
            sha256: digest(&b),
            bytes: b.len() as u64,
            media_type: r.media_type,
            dimensions: r.dimensions,
        };
        self.writes.lock().unwrap().insert(r.relative_path, b);
        Ok(CandidateFile {
            candidate_id: r.candidate_id,
            file: f,
            provenance: r.provenance,
        })
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        self.services.check_cancelled().await
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        Err(ServiceError::OfflineDispatch)
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        Err(ServiceError::OfflineDispatch)
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        Err(ServiceError::OfflineDispatch)
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        Err(ServiceError::OfflineDispatch)
    }
}
pub(super) struct Verified {
    pub speech: Value,
    pub manifest: Value,
    pub plan: Value,
    pub batch: Value,
    pub info: Pcm16WavInfo,
}
pub(super) fn pcm(b: &[u8]) -> Result<Pcm16WavInfo, StageError> {
    Ok(inspect_pcm16_wav(&mut Cursor::new(b), b.len() as u64)?)
}
pub(super) async fn batch(
    input: &StageInput,
    services: &dyn RunServices,
    files: &Files,
    ctx: &Context,
    speech_context: &Value,
) -> Result<Verified, StageError> {
    let mut original: StageInput = parse(bytes(files, &ctx.speech_input)?)?;
    original.validate()?;
    need(
        original.stage == Stage::Speech
            && original.library_revision == input.library_revision
            && original.inputs.iter().any(|a| a.file == ctx.speech_context),
        "original Speech stage/context/revision",
    )?;
    for a in &original.inputs {
        need(
            input.inputs.iter().any(|v| json!(v) == json!(a)),
            "original Speech ArtifactRef missing or changed",
        )?;
    }
    original.mode = ExecutionMode::Replay;
    let verifier = Verification {
        services,
        files,
        allowed: &original.inputs,
        writes: Mutex::new(BTreeMap::new()),
    };
    super::super::speech::execute(&original, &verifier).await?;
    for (path, f) in [
        ("private/speech-plan.json", &ctx.speech_plan),
        ("private/speech-takes.json", &ctx.take_manifest),
        ("private/speech-transcription-plan.json", &ctx.copy_plan),
    ] {
        need(
            verifier
                .writes
                .lock()
                .unwrap()
                .get(path)
                .is_some_and(|v| v == bytes(files, f).unwrap_or(&[])),
            "stale or forged committed G24 output",
        )?;
    }
    let speech: Value = parse(bytes(files, &ctx.speech_plan)?)?;
    let manifest: Value = parse(bytes(files, &ctx.take_manifest)?)?;
    let plan: Value = parse(bytes(files, &ctx.copy_plan)?)?;
    need(
        plan["provider"] == ctx.provider && plan["model"] == ctx.model,
        "primary provider/model changed from planned context",
    )?;
    let batch = plan["batches"]
        .as_array()
        .and_then(|v| v.iter().find(|b| b["id"] == ctx.batch_id))
        .ok_or_else(|| StageError::Evidence("unplanned primary batch".into()))?
        .clone();
    let audio = bytes(files, &ctx.batch_audio.file)?;
    let info = pcm(audio)?;
    let rate = info.sample_rate as u64;
    need(
        batch["sample_rate"] == info.sample_rate
            && batch["channels"] == info.channels
            && batch["bits_per_sample"] == 16
            && batch["frames"] == info.frame_count
            && info.frame_count
                <= rate
                    * speech_context["limits"]["transcription_max_seconds"]
                        .as_u64()
                        .unwrap()
                        .min(90),
        "physical batch format/duration/frame domain",
    )?;
    let receipt: Value = parse(bytes(files, &ctx.copy_receipt)?)?;
    need(
        matches!(
            receipt["provenance"].as_str(),
            Some("synthetic" | "imported" | "diagnostic")
        ),
        "unsupported copy provenance",
    )?;
    let expected = json!({"schema_version":1,"provenance":receipt["provenance"],"recipe":"native-pcm16-frame-copy-v1","recipe_sha256":ctx.copy_recipe_sha256,"take_manifest":artifact(input,&ctx.take_manifest)?,"copy_plan":artifact(input,&ctx.copy_plan)?,"source_sha256":speech["source_sha256"],"batch_id":ctx.batch_id,"format":{"sample_rate":info.sample_rate,"channels":info.channels,"bits_per_sample":16,"frames":info.frame_count},"segments":batch["segments"],"unplanned_audio":plan["unplanned_audio"],"audio":ctx.batch_audio,"sample_origin":"materialized-batch-local","narration_assembly":null,"measured_word_intervals":null,"fact_bounds":null,"model_payload":{"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":ctx.batch_audio},"dispatch_permitted":false,"bulk_speech_approved":false,"production_approved":false});
    need(
        receipt == expected,
        "copy receipt identity/mapping/authority differs",
    )?;
    let segments = batch["segments"]
        .as_array()
        .ok_or_else(|| StageError::Evidence("copy segments missing".into()))?;
    need(
        !segments.is_empty() && segments.len() <= 64,
        "copy segment bound",
    )?;
    let mut cursor = 0;
    for s in segments {
        services.check_cancelled().await?;
        let a: ArtifactRef = serde_json::from_value(s["audio"].clone())
            .map_err(|e| StageError::Evidence(e.to_string()))?;
        need(
            a.file.bytes <= 8 * META && json!(artifact(input, &a.file)?) == json!(a),
            "native recording identity/bound",
        )?;
        let native = bytes(files, &a.file)?;
        let ni = pcm(native)?;
        let ns: Interval = serde_json::from_value(s["native_span"].clone())
            .map_err(|e| StageError::Evidence(e.to_string()))?;
        let bs: Interval = serde_json::from_value(s["batch_span"].clone())
            .map_err(|e| StageError::Evidence(e.to_string()))?;
        need(
            ni.sample_rate == info.sample_rate
                && ni.channels == info.channels
                && ns[0] < ns[1]
                && ns[1] <= ni.frame_count
                && bs[0] == cursor
                && bs[0] < bs[1]
                && bs[1] <= info.frame_count
                && ns[1] - ns[0] == bs[1] - bs[0],
            "native/batch copy format or coverage mismatch",
        )?;
        let take = manifest["takes"]
            .as_array()
            .and_then(|v| {
                v.iter()
                    .find(|t| t["unit_id"] == s["unit_id"] && t["audio"] == json!(a))
            })
            .ok_or_else(|| StageError::Evidence("segment lacks original take".into()))?;
        need(
            take["native"]["frames"] == ni.frame_count,
            "native take frame identity",
        )?;
        let block = info.block_align as u64;
        let na = (ni.data_offset + ns[0] * block) as usize;
        let nb = (ni.data_offset + ns[1] * block) as usize;
        let ba = (info.data_offset + bs[0] * block) as usize;
        let bb = (info.data_offset + bs[1] * block) as usize;
        need(
            native[na..nb] == audio[ba..bb],
            "materialized PCM differs from selected original native frames",
        )?;
        cursor = bs[1];
    }
    need(cursor == info.frame_count, "incomplete batch coverage")?;
    Ok(Verified {
        speech,
        manifest,
        plan,
        batch,
        info,
    })
}
