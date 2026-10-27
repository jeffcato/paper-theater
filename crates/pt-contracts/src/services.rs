use crate::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Fixture,
    Replay,
    Live,
}
impl ExecutionMode {
    pub fn permits_provider(self) -> bool {
        self == Self::Live
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Source,
    Direction,
    Pilot,
    Speech,
    Acoustic,
    VisualPreflight,
    Visuals,
    Presentation,
    MusicMix,
    Choreography,
    Review,
    Package,
}
pub const STAGES: [Stage; 12] = [
    Stage::Source,
    Stage::Direction,
    Stage::Pilot,
    Stage::Speech,
    Stage::Acoustic,
    Stage::VisualPreflight,
    Stage::Visuals,
    Stage::Presentation,
    Stage::MusicMix,
    Stage::Choreography,
    Stage::Review,
    Stage::Package,
];
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Queued,
    Running,
    WaitingProvider,
    Recovering,
    Succeeded,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub recipe_sha256: Digest,
    pub profile_sha256: Digest,
    pub profile_id: Id,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub money_microusd: u64,
    pub requests: u64,
    pub tokens: u64,
    pub repairs: u32,
    pub wall_time_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub id: Id,
    pub file: FileRef,
    pub provenance: Provenance,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageInput {
    pub schema_version: u32,
    pub run_id: Id,
    pub stage: Stage,
    pub unit_id: Id,
    pub epoch: u64,
    pub revision: u64,
    pub inputs: Vec<ArtifactRef>,
    pub pins: Pins,
    pub mode: ExecutionMode,
    pub limits: Limits,
    pub library_revision: u64,
}
impl StageInput {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 1,
            "version",
            "schema_version",
            "expected 1",
        )?;
        for (name, value) in [
            ("run_id", &self.run_id),
            ("unit_id", &self.unit_id),
            ("profile_id", &self.pins.profile_id),
        ] {
            require(valid_id(value), "id", name, "invalid ID")?;
        }
        for (name, value) in [
            ("recipe_sha256", &self.pins.recipe_sha256),
            ("profile_sha256", &self.pins.profile_sha256),
        ] {
            require(valid_digest(value), "digest", name, "invalid SHA-256")?;
        }
        for n in [
            self.epoch,
            self.revision,
            self.library_revision,
            self.limits.money_microusd,
            self.limits.requests,
            self.limits.tokens,
            self.limits.wall_time_ms,
        ] {
            require(
                n <= MAX_SAFE_INTEGER,
                "integer",
                "stage_input",
                "integer exceeds JSON safe range",
            )?;
        }
        for a in &self.inputs {
            require(valid_id(&a.id), "id", "inputs.id", "invalid artifact ID")?;
            a.file.validate()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: Id,
    pub field: String,
    pub outcome: ReviewOutcome,
    pub message: String,
    pub evidence: Vec<ArtifactRef>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    Pass,
    Fail,
    Uncertain,
    Missing,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StageOutcome {
    Complete,
    NeedsRepair,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub id: Id,
    pub summary: String,
    pub inputs: Vec<ArtifactRef>,
    pub selected_ids: Vec<Id>,
    pub rejected_ids: Vec<Id>,
    pub supersedes: Option<Id>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateFile {
    pub candidate_id: Id,
    pub file: FileRef,
    pub provenance: Provenance,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageOutput {
    pub outcome: StageOutcome,
    pub artifacts: Vec<CandidateFile>,
    pub decisions: Vec<Decision>,
    pub findings: Vec<Finding>,
    pub cache_keys: Vec<Digest>,
    pub library_updates: Vec<LibraryUpdate>,
    pub package_candidate: Option<PackageCandidate>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryUpdate {
    pub book_id: Id,
    pub entity_id: Id,
    pub binding: BindingKind,
    pub expected_revision: u64,
    pub artifacts: Vec<ArtifactRef>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BindingKind {
    CharacterSprite,
    Voice,
    SubjectReference,
    ArtProfile,
    MusicalBible,
    Shell,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryQuery {
    pub book_id: Id,
    pub entity_ids: Vec<Id>,
    pub revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryBinding {
    pub book_id: Id,
    pub entity_id: Id,
    pub binding: BindingKind,
    pub revision: u64,
    pub artifacts: Vec<ArtifactRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCandidate {
    pub candidate_id: Id,
    pub provenance: Provenance,
    pub payload_inventory: Vec<FileRef>,
    pub inventory_sha256: Digest,
    pub acceptance: CandidateFile,
    pub manifest: CandidateFile,
    pub required_artifacts: Vec<ArtifactRef>,
    pub findings: Vec<Finding>,
    pub pins: Pins,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Research,
    Direction,
    Speech,
    Transcription,
    AcousticVerification,
    Image,
    Segmentation,
    Music,
    Review,
    TokenCount,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderOperation {
    pub attempt_id: Id,
    pub unit_id: Id,
    pub kind: OperationKind,
    pub provider: Id,
    pub model: Id,
    pub recipe_sha256: Digest,
    pub request_sha256: Digest,
    pub payload: ArtifactRef,
    pub inputs: Vec<ArtifactRef>,
    pub estimate: Usage,
    pub timeout_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub money_microusd: u64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCompletion {
    Completed,
    KnownRejection,
    Ambiguous,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderResult {
    pub attempt_id: Id,
    pub completion: ProviderCompletion,
    pub request_sha256: Digest,
    pub response_sha256: Option<Digest>,
    pub raw_response: Option<ArtifactRef>,
    pub outputs: Vec<ArtifactRef>,
    pub usage: Option<Usage>,
    pub started_at: String,
    pub elapsed_ms: u64,
    pub diagnostic: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HelperKind {
    DecodeAudio,
    AlignAudio,
    MixAudio,
    RegisterImage,
    Capture,
    ValidatePackage,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelperOperation {
    pub id: Id,
    pub kind: HelperKind,
    pub inputs: Vec<ArtifactRef>,
    pub recipe_sha256: Digest,
    pub timeout_ms: u64,
    pub max_output_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelperResult {
    pub id: Id,
    pub outputs: Vec<CandidateFile>,
    pub findings: Vec<Finding>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateWrite {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<[u32; 2]>,
    pub candidate_id: Id,
    pub relative_path: String,
    pub media_type: String,
    pub max_bytes: u64,
    pub provenance: Provenance,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    TaskStarted,
    TaskCompleted,
    DirectorDecision,
    ArtifactReady,
    CheckCompleted,
    RepairPlanned,
    CacheReused,
    ProviderWait,
    BudgetUpdate,
    RunTerminal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalProposal {
    pub kind: EventKind,
    pub task_id: Option<Id>,
    pub decision_id: Option<Id>,
    pub artifact_ids: Vec<Id>,
    pub summary: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEvent {
    pub schema_version: u32,
    pub run_id: Id,
    pub seq: u64,
    pub epoch: u64,
    pub revision: u64,
    pub timestamp: String,
    #[serde(flatten)]
    pub entry: JournalProposal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    pub schema_version: u32,
    pub source_id: Id,
    pub book_id: Id,
    pub mode: ExecutionMode,
    pub pins: Pins,
    pub limits: Limits,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageSnapshot {
    pub stage: Stage,
    pub unit_id: Id,
    pub outcome: Option<StageOutcome>,
    pub artifacts: Vec<ArtifactRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSnapshot {
    pub schema_version: u32,
    pub run_id: Id,
    pub request_sha256: Digest,
    pub epoch: u64,
    pub revision: u64,
    pub state: RunState,
    pub last_event_seq: u64,
    pub stages: Vec<StageSnapshot>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("provider-v2 service unavailable; no legacy fallback")]
    ProviderV2Unavailable,
    #[error(transparent)]
    Invalid(#[from] ContractError),
    #[error("run cancelled or epoch stale")]
    Cancelled,
    #[error("provider dispatch forbidden in fixture/replay mode")]
    OfflineDispatch,
    #[error("operation not authorized: {0}")]
    Unauthorized(String),
    #[error("limit exhausted: {0}")]
    Limit(String),
    #[error("artifact missing: {0}")]
    MissingArtifact(Id),
    #[error("candidate missing from this execution: {0}")]
    MissingCandidate(Id),
    #[error("library revision conflict")]
    RevisionConflict,
    #[error("service failed: {0}")]
    Internal(String),
}
#[derive(Debug, thiserror::Error)]
pub enum StageError {
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    Invalid(#[from] ContractError),
    #[error("required stage evidence failed: {0}")]
    Evidence(String),
}
/// The runtime binds an instance to one run/epoch. Methods must recheck cancellation
/// before committing; only provider charges may be recorded for an obsolete epoch.
#[async_trait]
pub trait RunServices: Send + Sync {
    async fn read_artifact(&self, artifact: &ArtifactRef) -> Result<Vec<u8>, ServiceError>;
    /// Read only candidates owned by this active execution. Verify the full
    /// candidate identity, safe scratch-relative path, size and exact SHA-256;
    /// check cancellation/epoch before and after copying bytes. This never
    /// grants a committed artifact ID or any publication permission.
    /// Implementations without a candidate store fail closed for compatibility.
    async fn read_candidate(&self, candidate: &CandidateFile) -> Result<Vec<u8>, ServiceError> {
        self.check_cancelled().await?;
        candidate.file.validate()?;
        Err(ServiceError::MissingCandidate(
            candidate.candidate_id.clone(),
        ))
    }

    async fn write_candidate(
        &self,
        request: CandidateWrite,
        bytes: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError>;
    async fn emit(&self, entry: JournalProposal) -> Result<(), ServiceError>;
    async fn check_cancelled(&self) -> Result<(), ServiceError>;
    async fn provider(&self, operation: ProviderOperation) -> Result<ProviderResult, ServiceError>;
    /// Propose content only. Runtime binds this service's run/epoch/unit and
    /// registry/grant lineage, verifies bytes and persists issuance/reservations.
    /// Fixture/replay must refuse before credential or reservation access.
    async fn issue_provider_v2(
        &self,
        _proposal: ProviderProposalV2,
    ) -> Result<ProviderOperationV2, ServiceError> {
        Err(ServiceError::ProviderV2Unavailable)
    }
    /// Descriptor JSON is not a capability. Runtime must match immutable issuance
    /// and this active service scope; recovery descriptors cannot submit work.
    async fn execute_provider_v2(
        &self,
        _issued: &ProviderOperationV2,
    ) -> Result<ProviderResultV2, ServiceError> {
        Err(ServiceError::ProviderV2Unavailable)
    }
    /// Query only existing persisted work with a separately authorized observer.
    /// Preserve original result identity/charges; never invoke original submission.
    async fn recover_provider_v2(
        &self,
        _request: ProviderRecoveryRequestV2,
    ) -> Result<ProviderRecoveryResultV2, ServiceError> {
        Err(ServiceError::ProviderV2Unavailable)
    }
    async fn helper(&self, operation: HelperOperation) -> Result<HelperResult, ServiceError>;
    async fn library(&self, query: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError>;
}
#[async_trait]
pub trait StageExecutor: Send + Sync {
    async fn execute(
        &self,
        input: StageInput,
        services: &dyn RunServices,
    ) -> Result<StageOutput, StageError>;
}
/// Call before reading credentials, reserving or dispatching. Live still needs
/// the runtime's persisted money, quota, repair and attempt authorization.
pub fn require_live(mode: ExecutionMode) -> Result<(), ServiceError> {
    if mode.permits_provider() {
        Ok(())
    } else {
        Err(ServiceError::OfflineDispatch)
    }
}
