//! Reconstruct the pinned Pilot and its Source/Direction dependencies without external effects.
use super::*;
use std::sync::Mutex;
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
        let file = FileRef {
            path: r.relative_path.clone(),
            sha256: digest(&b),
            bytes: b.len() as u64,
            media_type: r.media_type,
            dimensions: r.dimensions,
        };
        self.writes.lock().unwrap().insert(r.relative_path, b);
        Ok(CandidateFile {
            candidate_id: r.candidate_id,
            file,
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
pub(super) async fn pilot(
    input: &StageInput,
    services: &dyn RunServices,
    files: &Files,
    ctx: &Context,
) -> Result<Value, StageError> {
    let mut original: StageInput = parse(bytes(files, &ctx.pilot_input)?)?;
    original.validate()?;
    need(
        original.stage == Stage::Pilot && original.library_revision == input.library_revision,
        "original Pilot stage/revision differs",
    )?;
    for a in &original.inputs {
        need(
            input.inputs.iter().any(|v| json!(v) == json!(a)),
            "original Pilot ArtifactRef missing or changed",
        )?;
    }
    original.mode = ExecutionMode::Replay;
    let verifier = Verification {
        services,
        files,
        allowed: &original.inputs,
        writes: Mutex::new(BTreeMap::new()),
    };
    super::super::pilot::execute(&original, &verifier).await?;
    let b = bytes(files, &ctx.pilot)?;
    need(
        verifier
            .writes
            .lock()
            .unwrap()
            .get("private/pilot-plan.json")
            .is_some_and(|v| v == b),
        "stale or forged Pilot plan",
    )?;
    parse(b)
}
