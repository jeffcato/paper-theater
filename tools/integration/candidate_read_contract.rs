use async_trait::async_trait;
use pt_contracts as c;
use pt_production::runtime::{Runtime, Store};
use serde_json::json;
use std::sync::{Arc, Mutex};

struct ReadCandidate(Arc<Mutex<Option<c::CandidateFile>>>);
#[async_trait]
impl c::StageExecutor for ReadCandidate {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> Result<c::StageOutput, c::StageError> {
        let bytes = b"{\"measured\":true}".to_vec();
        let candidate = services
            .write_candidate(
                c::CandidateWrite {
                    dimensions: None,
                    candidate_id: input.unit_id,
                    relative_path: "helper/report.json".into(),
                    media_type: "application/json".into(),
                    max_bytes: 4096,
                    provenance: c::Provenance::Diagnostic,
                },
                bytes.clone(),
            )
            .await?;
        assert_eq!(services.read_candidate(&candidate).await?, bytes);
        candidate.file.verify(&bytes)?;
        // A candidate from another execution has no authority in this scratch scope.
        let previous = self.0.lock().unwrap().clone();
        if let Some(previous) = previous {
            assert!(matches!(
                services.read_candidate(&previous).await,
                Err(c::ServiceError::MissingCandidate(_))
            ));
        }
        *self.0.lock().unwrap() = Some(candidate.clone());
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![candidate],
            decisions: vec![],
            findings: vec![],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
fn request() -> serde_json::Value {
    json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":c::digest(b"candidate-read"),"profile_sha256":c::digest(b"profile"),"profile_id":"diagnostic"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}})
}
#[tokio::test]
async fn helper_candidate_bytes_are_readable_only_in_their_execution_scope() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("candidate-read", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let runtime = Runtime::new(store, Arc::new(ReadCandidate(Arc::new(Mutex::new(None)))));
    runtime
        .execute_stage(&run, epoch, c::Stage::Source)
        .await
        .unwrap();
    runtime
        .execute_stage(&run, epoch, c::Stage::Direction)
        .await
        .unwrap();
}

struct ForgedReads;
#[async_trait]
impl c::StageExecutor for ForgedReads {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> Result<c::StageOutput, c::StageError> {
        let bytes = b"{\"measured\":true}".to_vec();
        let candidate = services
            .write_candidate(
                c::CandidateWrite {
                    dimensions: None,
                    candidate_id: input.unit_id,
                    relative_path: "helper/report.json".into(),
                    media_type: "application/json".into(),
                    max_bytes: 4096,
                    provenance: c::Provenance::Diagnostic,
                },
                bytes.clone(),
            )
            .await?;
        for change in [0, 1, 2, 3] {
            let mut forged = candidate.clone();
            match change {
                0 => forged.file.sha256 = "0".repeat(64),
                1 => forged.file.bytes += 1,
                2 => forged.file.media_type = "text/plain".into(),
                _ => forged.provenance = c::Provenance::Autonomous,
            }
            assert!(matches!(
                services.read_candidate(&forged).await,
                Err(c::ServiceError::Unauthorized(_))
            ));
        }
        let mut escaped = candidate.clone();
        escaped.file.path = "../report.json".into();
        assert!(
            matches!(services.read_candidate(&escaped).await,Err(c::ServiceError::Invalid(ref e)) if e.code=="path")
        );
        let mut unknown = candidate.clone();
        unknown.candidate_id = "another-execution".into();
        assert!(matches!(
            services.read_candidate(&unknown).await,
            Err(c::ServiceError::MissingCandidate(_))
        ));
        assert_eq!(services.read_candidate(&candidate).await?, bytes);
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![candidate],
            decisions: vec![],
            findings: vec![],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
#[tokio::test]
async fn candidate_reads_reject_forged_metadata_unknown_ids_and_escaping_paths() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("forged-reads", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    Runtime::new(store, Arc::new(ForgedReads))
        .execute_stage(&run, epoch, c::Stage::Source)
        .await
        .unwrap();
}
struct CancelledRead(Store);
#[async_trait]
impl c::StageExecutor for CancelledRead {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> Result<c::StageOutput, c::StageError> {
        let candidate = services
            .write_candidate(
                c::CandidateWrite {
                    dimensions: None,
                    candidate_id: input.unit_id,
                    relative_path: "helper/report.json".into(),
                    media_type: "application/json".into(),
                    max_bytes: 4096,
                    provenance: c::Provenance::Diagnostic,
                },
                b"{}".to_vec(),
            )
            .await?;
        self.0.cancel(&input.run_id).unwrap();
        assert!(matches!(
            services.read_candidate(&candidate).await,
            Err(c::ServiceError::Cancelled)
        ));
        Err(c::ServiceError::Cancelled.into())
    }
}
#[tokio::test]
async fn candidate_read_after_cancellation_cannot_return_or_commit_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("cancelled-read", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    assert!(
        Runtime::new(store.clone(), Arc::new(CancelledRead(store.clone())))
            .execute_stage(&run, epoch, c::Stage::Source)
            .await
            .is_err()
    );
    let snapshot = store.api_snapshot(&run).unwrap();
    assert_eq!(snapshot.state, c::RunState::Cancelled);
    assert!(snapshot.stages.iter().all(|s| s.artifacts.is_empty()));
}

struct PngCandidate;
#[async_trait]
impl c::StageExecutor for PngCandidate {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> Result<c::StageOutput, c::StageError> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/shared/two-scene-v1/assets/figure.png");
        let bytes = std::fs::read(path).unwrap();
        let dimensions = [
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        ];
        let candidate = services
            .write_candidate(
                c::CandidateWrite {
                    candidate_id: input.unit_id.clone(),
                    relative_path: "candidate.png".into(),
                    media_type: "image/png".into(),
                    max_bytes: 2 * 1024 * 1024,
                    provenance: c::Provenance::Diagnostic,
                    dimensions: Some(dimensions),
                },
                bytes.clone(),
            )
            .await?;
        assert_eq!(candidate.file.dimensions, Some(dimensions));
        assert_eq!(services.read_candidate(&candidate).await?, bytes);
        for invalid in [None, Some([0, 64]), Some([4097, 64])] {
            let result = services
                .write_candidate(
                    c::CandidateWrite {
                        candidate_id: format!("{}-invalid", input.unit_id),
                        relative_path: "invalid.png".into(),
                        media_type: "image/png".into(),
                        max_bytes: 2 * 1024 * 1024,
                        provenance: c::Provenance::Diagnostic,
                        dimensions: invalid,
                    },
                    bytes.clone(),
                )
                .await;
            assert!(matches!(result,Err(c::ServiceError::Invalid(ref e)) if e.code=="dimensions"));
        }
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![candidate],
            decisions: vec![],
            findings: vec![],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
#[tokio::test]
async fn png_candidate_dimensions_survive_runtime_write_read_and_commit() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("png-candidate", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let artifacts = Runtime::new(store.clone(), Arc::new(PngCandidate))
        .execute_stage(&run, epoch, c::Stage::Source)
        .await
        .unwrap();
    assert!(artifacts[0].file.dimensions.is_some());
    let saved = store.api_snapshot(&run).unwrap();
    assert_eq!(
        saved.stages[0].artifacts[0].file.dimensions,
        artifacts[0].file.dimensions
    );
}
