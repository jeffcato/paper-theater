//! Reconstruct the exact saved Direction plan without dispatch or external candidate writes.
use super::*;
use serde_json::Value;
use std::sync::Mutex;

struct Verification<'a> {
    services: &'a dyn RunServices,
    inputs: &'a [ArtifactRef],
    files: &'a BTreeMap<String, (FileRef, Vec<u8>)>,
    writes: Mutex<BTreeMap<String, Vec<u8>>>,
}
#[async_trait]
impl RunServices for Verification<'_> {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        self.check_cancelled().await?;
        if self
            .inputs
            .iter()
            .any(|v| serde_json::to_value(v).ok() == serde_json::to_value(a).ok())
        {
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
        // Candidate metadata only; never a committed ArtifactRef or public output.
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
pub(super) async fn verify(
    input: &StageInput,
    services: &dyn RunServices,
    files: &BTreeMap<String, (FileRef, Vec<u8>)>,
    saved: &FileRef,
    plan: &FileRef,
) -> Result<Value, StageError> {
    let get = |f: &FileRef| -> Result<&[u8], StageError> {
        let (actual, b) = files
            .get(&f.path)
            .ok_or_else(|| StageError::Evidence("missing saved Direction identity".into()))?;
        require(
            actual == f,
            "pilot-direction",
            "identity",
            "saved Direction reference changed",
        )?;
        Ok(b)
    };
    let mut original: StageInput = parse(get(saved)?)?;
    original.validate()?;
    require(
        original.stage == Stage::Direction && original.library_revision == input.library_revision,
        "pilot-direction",
        "stage",
        "wrong Direction stage/registry revision",
    )?;
    for a in &original.inputs {
        require(
            input
                .inputs
                .iter()
                .any(|v| serde_json::to_value(v).ok() == serde_json::to_value(a).ok()),
            "pilot-direction",
            "inputs",
            "original Direction ArtifactRef missing or changed",
        )?;
    }
    // Original observer identities stay unchanged. Replay grants no issuance authority.
    original.mode = ExecutionMode::Replay;
    let verifier = Verification {
        services,
        inputs: &original.inputs,
        files,
        writes: Mutex::new(BTreeMap::new()),
    };
    direction::execute(&original, &verifier).await?;
    let b = get(plan)?;
    require(
        verifier
            .writes
            .lock()
            .unwrap()
            .get("private/direction-plan.json")
            .is_some_and(|compiled| compiled == b),
        "pilot-direction",
        "plan",
        "stale or unsupported Direction plan",
    )?;
    parse(b)
}
