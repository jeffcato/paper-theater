use super::*;
use crate::runtime::{Runtime, Store};
use std::{path::Path, sync::Arc};

#[tokio::test]
async fn acoustic_stage_uses_real_helper_and_verified_candidate_reads_without_provider_calls() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let request = serde_json::json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture",
        "pins":{"recipe_sha256":digest(b"offline-acoustic-stage-v1"),"profile_sha256":digest(b"diagnostic-profile"),"profile_id":"paper-player-r186-v1"},
        "limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}});
    let run = store.create("generation-acoustic-stage", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_acoustic_fixture(&run, epoch, &root.join("fixtures/generation/acoustic"))
        .unwrap();
    let runtime = Runtime::new(store.clone(), Arc::new(GenerationExecutor))
        .with_offline_acoustic(root.join("tools/generation"))
        .unwrap();
    assert_eq!(
        runtime
            .execute_stage(&run, epoch, Stage::Acoustic)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    let snapshot = store.api_snapshot(&run).unwrap();
    let artifacts = &snapshot
        .stages
        .iter()
        .find(|s| s.stage == Stage::Acoustic)
        .unwrap()
        .artifacts;
    let raw_ref = artifacts
        .iter()
        .find(|a| a.file.path == "acoustic-replay.json")
        .expect("actual helper output was not proposed");
    let raw = store.artifact(&raw_ref.id).unwrap().0;
    raw_ref.file.verify(&raw).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("fixtures/generation/acoustic/segmented-acoustic-report.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["word_track"], saved["candidate_word_track"]);
    assert_eq!(
        report["highlight_groups"],
        saved["candidate_highlight_groups"]
    );
    assert_eq!(report["provider_calls"], 0);
    assert_eq!(report["production_approved"], false);
    let validation = artifacts
        .iter()
        .find(|a| a.file.path == "acoustic-validation.json")
        .expect("verified candidate validation was not produced");
    let summary: serde_json::Value =
        serde_json::from_slice(&store.artifact(&validation.id).unwrap().0).unwrap();
    assert_eq!(summary["structural_read_along"], "pass");
    assert_eq!(summary["word_count"], 176);
    assert_eq!(summary["highlight_group_count"], 170);
    assert_eq!(summary["normalized_report_sha256"], raw_ref.file.sha256);
    assert!(summary["reveal_permissions"].as_array().unwrap().is_empty());
    assert!(
        summary["missing_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "independent-acoustic-evidence")
    );
    let journal = store.journal(&run, 0, 500).unwrap();
    let attempts = journal
        .iter()
        .filter(|e| e["type"] == "helper_authorized")
        .collect::<Vec<_>>();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0]["data"]["requests"], 0);
    assert_eq!(attempts[0]["data"]["money"], 0);
    assert!(
        journal
            .iter()
            .all(|e| !e["type"].as_str().unwrap().starts_with("provider_"))
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
}

use async_trait::async_trait;
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

struct ProtocolServices {
    inputs: BTreeMap<String, Vec<u8>>,
    candidates: Mutex<BTreeMap<Id, (CandidateFile, Vec<u8>)>>,
    report: Vec<u8>,
    fault: &'static str,
    cancelled: AtomicBool,
    helper_calls: AtomicUsize,
    candidate_reads: AtomicUsize,
    emits: AtomicUsize,
}
#[async_trait]
impl RunServices for ProtocolServices {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        self.check_cancelled().await?;
        self.inputs
            .get(&a.file.path)
            .cloned()
            .ok_or_else(|| ServiceError::MissingArtifact(a.id.clone()))
    }
    async fn read_candidate(&self, c: &CandidateFile) -> Result<Vec<u8>, ServiceError> {
        self.candidate_reads.fetch_add(1, Ordering::SeqCst);
        self.check_cancelled().await?;
        if self.fault == "unknown-candidate" {
            return Err(ServiceError::MissingCandidate(c.candidate_id.clone()));
        }
        let mut bytes = self
            .candidates
            .lock()
            .unwrap()
            .get(&c.candidate_id)
            .unwrap()
            .1
            .clone();
        if self.fault == "read-hash" {
            bytes[0] ^= 1;
        }
        if self.fault == "cancel-read" {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        Ok(bytes)
    }
    async fn write_candidate(
        &self,
        r: CandidateWrite,
        bytes: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        self.check_cancelled().await?;
        assert!(bytes.len() as u64 <= r.max_bytes);
        let candidate = CandidateFile {
            candidate_id: r.candidate_id,
            provenance: r.provenance,
            file: FileRef {
                path: r.relative_path,
                sha256: digest(&bytes),
                bytes: bytes.len() as u64,
                media_type: r.media_type,
                dimensions: r.dimensions,
            },
        };
        self.candidates
            .lock()
            .unwrap()
            .insert(candidate.candidate_id.clone(), (candidate.clone(), bytes));
        Ok(candidate)
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        self.emits.fetch_add(1, Ordering::SeqCst);
        if self.fault == "cancel-emit" {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err(ServiceError::Cancelled)
        } else {
            Ok(())
        }
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("acoustic provider call")
    }
    async fn helper(&self, op: HelperOperation) -> Result<HelperResult, ServiceError> {
        self.helper_calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(op.kind, HelperKind::AlignAudio);
        assert_eq!(op.max_output_bytes, 16 * 1024 * 1024);
        let mut record: serde_json::Value = serde_json::from_slice(&self.report).unwrap();
        match self.fault {
            "claims-acceptance" => record["production_approved"] = serde_json::json!(true),
            "reveal-permission" => {
                record["reveal_permissions"] = serde_json::json!(["unverified-fact"])
            }
            "wrong-recipe" => record["recipe_sha256"] = serde_json::json!("0".repeat(64)),
            "source-hash" => record["source_sha256"] = serde_json::json!("0".repeat(64)),
            "input-hash" => {
                record["input_hashes"]["source.txt"] = serde_json::json!("0".repeat(64))
            }
            "missing-input-hash" => {
                record["input_hashes"]
                    .as_object_mut()
                    .unwrap()
                    .remove("manifest.json");
            }
            "changed-word" => record["word_track"][0]["start_sample"] = serde_json::json!(0),
            "changed-group" => record["highlight_groups"][0]["start_sample"] = serde_json::json!(0),
            "missing-groups" => record["highlight_groups"] = serde_json::json!([]),
            _ => {}
        }
        let candidate = self
            .write_candidate(
                CandidateWrite {
                    candidate_id: "helper-output".into(),
                    relative_path: "acoustic-replay.json".into(),
                    media_type: "application/json".into(),
                    dimensions: None,
                    max_bytes: op.max_output_bytes,
                    provenance: Provenance::Diagnostic,
                },
                serde_json::to_vec(&record).unwrap(),
            )
            .await?;
        let mut result = HelperResult {
            id: op.id,
            outputs: vec![candidate],
            findings: vec![],
        };
        if self.fault == "wrong-operation" {
            result.id = "unissued-operation".into();
        }
        if self.fault == "extra-output" {
            result.outputs.push(result.outputs[0].clone());
        }
        if self.fault == "unsafe-path" {
            result.outputs[0].file.path = "../candidate.json".into();
        }
        if self.fault == "uncertain" {
            result.findings.push(Finding {
                id: "complete-fact-interval-verification".into(),
                field: "acoustic".into(),
                outcome: ReviewOutcome::Uncertain,
                message: "Independent interval ambiguous".into(),
                evidence: vec![],
            });
        }
        if self.fault == "cancel-helper" {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        Ok(result)
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("unexpected acoustic library access")
    }
}

fn protocol_fixture(mode: ExecutionMode, fault: &'static str) -> (StageInput, ProtocolServices) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generation/acoustic");
    fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if path.file_name().unwrap() != "replay.json" {
                files.insert(
                    path.strip_prefix(root).unwrap().to_str().unwrap().into(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(&root, &root, &mut files);
    let inputs = files
        .iter()
        .map(|(path, bytes)| ArtifactRef {
            id: format!("saved:{path}"),
            file: FileRef {
                path: path.clone(),
                sha256: digest(bytes),
                bytes: bytes.len() as u64,
                media_type: if path.ends_with(".wav") {
                    "audio/wav"
                } else if path.ends_with(".txt") {
                    "text/plain"
                } else {
                    "application/json"
                }
                .into(),
                dimensions: None,
            },
            provenance: Provenance::Imported,
        })
        .collect();
    (
        StageInput {
            schema_version: 1,
            run_id: "acoustic-protocol".into(),
            stage: Stage::Acoustic,
            unit_id: "saved-turns".into(),
            epoch: 1,
            revision: 1,
            inputs,
            pins: Pins {
                recipe_sha256: digest(b"offline-acoustic-stage-v1"),
                profile_sha256: digest(b"profile"),
                profile_id: "paper-player-r186-v1".into(),
            },
            mode,
            limits: Limits {
                wall_time_ms: 30000,
                ..Limits::default()
            },
            library_revision: 0,
        },
        ProtocolServices {
            inputs: files,
            candidates: Mutex::new(BTreeMap::new()),
            report: std::fs::read(root.join("replay.json")).unwrap(),
            fault,
            cancelled: AtomicBool::new(false),
            helper_calls: AtomicUsize::new(0),
            candidate_reads: AtomicUsize::new(0),
            emits: AtomicUsize::new(0),
        },
    )
}

#[tokio::test]
async fn missing_cached_evidence_is_rejected_before_any_helper_attempt() {
    let (mut input, services) = protocol_fixture(ExecutionMode::Fixture, "");
    input
        .inputs
        .retain(|a| !a.file.path.ends_with(".receipt.json"));
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert!(services.candidates.lock().unwrap().is_empty());
}

#[tokio::test]
async fn acoustic_recorded_replay_has_no_helper_attempt_or_candidate_read() {
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    let bytes = services.report.clone();
    let sha = digest(&bytes);
    input.inputs.push(ArtifactRef {
        id: "recorded:normalization".into(),
        provenance: Provenance::Diagnostic,
        file: FileRef {
            path: "acoustic-replay.json".into(),
            sha256: sha.clone(),
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        },
    });
    services.inputs.insert("acoustic-replay.json".into(), bytes);
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    assert_eq!(output.artifacts.len(), 1);
    let candidates = services.candidates.lock().unwrap();
    let summary: serde_json::Value =
        serde_json::from_slice(&candidates.get(&output.artifacts[0].candidate_id).unwrap().1)
            .unwrap();
    assert_eq!(summary["mode"], "replay");
    assert_eq!(summary["normalized_report_sha256"], sha);
    assert!(output.package_candidate.is_none());
    assert!(output.library_updates.is_empty());
}

#[tokio::test]
async fn acoustic_rejects_unverified_candidates_stale_bindings_and_changed_measurements() {
    for fault in [
        "unknown-candidate",
        "read-hash",
        "wrong-operation",
        "extra-output",
        "unsafe-path",
        "claims-acceptance",
        "reveal-permission",
        "wrong-recipe",
        "source-hash",
        "input-hash",
        "missing-input-hash",
        "changed-word",
        "changed-group",
        "missing-groups",
    ] {
        let (input, services) = protocol_fixture(ExecutionMode::Fixture, fault);
        let result = GenerationExecutor.execute(input, &services).await;
        assert!(result.is_err(), "accepted {fault}");
        assert!(
            !services
                .candidates
                .lock()
                .unwrap()
                .values()
                .any(|(c, _)| c.file.path == "acoustic-validation.json"),
            "wrote a success summary for {fault}"
        );
        if fault == "unknown-candidate" {
            assert!(matches!(
                result,
                Err(StageError::Service(ServiceError::MissingCandidate(_)))
            ));
        }
    }
}

#[tokio::test]
async fn acoustic_preserves_uncertainty_and_rechecks_cancelled_execution_boundaries() {
    let (input, services) = protocol_fixture(ExecutionMode::Fixture, "uncertain");
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 1);
    assert!(
        output
            .findings
            .iter()
            .any(|f| f.id == "complete-fact-interval-verification"
                && f.outcome == ReviewOutcome::Uncertain)
    );
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    for fault in ["cancel-helper", "cancel-read", "cancel-emit"] {
        let (input, services) = protocol_fixture(ExecutionMode::Fixture, fault);
        assert!(
            matches!(
                GenerationExecutor.execute(input, &services).await,
                Err(StageError::Service(ServiceError::Cancelled))
            ),
            "lost cancellation: {fault}"
        );
    }
}

#[tokio::test]
async fn acoustic_never_dispatches_to_fill_missing_replay_or_live_inputs() {
    for mode in [ExecutionMode::Replay, ExecutionMode::Live] {
        let (input, services) = protocol_fixture(mode, "");
        assert!(GenerationExecutor.execute(input, &services).await.is_err());
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
        assert!(services.candidates.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn recorded_acoustic_findings_keep_their_original_uncertain_outcome() {
    let (input, original) = protocol_fixture(ExecutionMode::Fixture, "uncertain");
    let output = GenerationExecutor.execute(input, &original).await.unwrap();
    let saved = original.candidates.lock().unwrap();
    let (mut replay, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    for candidate in &output.artifacts {
        let bytes = saved.get(&candidate.candidate_id).unwrap().1.clone();
        replay.inputs.push(ArtifactRef {
            id: format!("recorded:{}", candidate.file.path),
            file: candidate.file.clone(),
            provenance: candidate.provenance,
        });
        services.inputs.insert(candidate.file.path.clone(), bytes);
    }
    let result = GenerationExecutor.execute(replay, &services).await.unwrap();
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.id == "complete-fact-interval-verification"
                && f.outcome == ReviewOutcome::Uncertain)
    );
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}
