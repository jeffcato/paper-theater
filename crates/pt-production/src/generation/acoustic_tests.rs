use super::*;
mod chapter_import;
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

fn attach_fact_inventory(input: &mut StageInput, services: &mut ProtocolServices) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/generation/acoustic-facts-v1/source-facts.json");
    let bytes = std::fs::read(path).unwrap();
    input.inputs.push(ArtifactRef {
        id: "source-facts-v1".into(),
        provenance: Provenance::Diagnostic,
        file: FileRef {
            path: "source-facts.json".into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        },
    });
    services.inputs.insert("source-facts.json".into(), bytes);
}
fn artifact_json(
    output: &StageOutput,
    services: &ProtocolServices,
    path: &str,
) -> serde_json::Value {
    let artifact = output
        .artifacts
        .iter()
        .find(|a| a.file.path == path)
        .expect("fact matching artifact was not produced");
    serde_json::from_slice(
        &services
            .candidates
            .lock()
            .unwrap()
            .get(&artifact.candidate_id)
            .unwrap()
            .1,
    )
    .unwrap()
}
#[tokio::test]
async fn explicit_facts_require_complete_intervals_and_use_later_verified_endpoints() {
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Fixture, "");
    attach_fact_inventory(&mut input, &mut services);
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    let facts = artifact_json(&output, &services, "fact-evidence.json");
    let rows = facts["facts"].as_array().unwrap();
    for (id, bound) in [
        ("cellar-instruction", 546720),
        ("Dorothy-urged", 506640),
        ("screaming", 506640),
        ("urged-cellar", 546720),
    ] {
        let fact = rows.iter().find(|r| r["id"] == id).unwrap();
        assert_eq!(fact["outcome"], "pass", "{id}: {fact}");
        assert_eq!(fact["not_before_sample"], bound);
    }
    for id in ["strange-event", "balloon-ascent", "unprovided-fact"] {
        let fact = rows.iter().find(|r| r["id"] == id).unwrap();
        assert_ne!(fact["outcome"], "pass");
        assert!(fact["not_before_sample"].is_null());
    }
    let summary = artifact_json(&output, &services, "acoustic-validation.json");
    assert_eq!(summary["word_count"], 176);
    assert_eq!(summary["highlight_group_count"], 170);
    assert_eq!(summary["point_word_count"], 6);
    let raw = artifact_json(&output, &services, "acoustic-replay.json");
    let original: serde_json::Value = serde_json::from_slice(&services.report).unwrap();
    assert_eq!(raw["word_track"], original["word_track"]);
    assert_eq!(raw["highlight_groups"], original["highlight_groups"]);
    assert!(summary["reveal_permissions"].as_array().unwrap().is_empty());
    assert!(output.findings.iter().any(|f|f.id=="acoustic-fact:unprovided-fact" && f.outcome==ReviewOutcome::Missing));
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 1);
}

fn replace_input(
    input: &mut StageInput,
    services: &mut ProtocolServices,
    path: &str,
    bytes: Vec<u8>,
) {
    let reference = input
        .inputs
        .iter_mut()
        .find(|a| a.file.path == path)
        .unwrap();
    reference.file.sha256 = digest(&bytes);
    reference.file.bytes = bytes.len() as u64;
    services.inputs.insert(path.into(), bytes);
}
fn add_recorded_report(input: &mut StageInput, services: &mut ProtocolServices) {
    let bytes = services.report.clone();
    input.inputs.push(ArtifactRef {
        id: "recorded:normalization".into(),
        provenance: Provenance::Diagnostic,
        file: FileRef {
            path: "acoustic-replay.json".into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        },
    });
    services.inputs.insert("acoustic-replay.json".into(), bytes);
}
// Synthetic in-memory mutations retain every original file and measured highlight group.
fn mutate_independent(
    input: &mut StageInput,
    services: &mut ProtocolServices,
    change: impl FnOnce(&mut serde_json::Value),
) {
    let mut report: serde_json::Value = serde_json::from_slice(&services.report).unwrap();
    let request = report["independent"][0]["request_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("meta/{request}.json");
    let receipt_path = format!("meta/{request}.receipt.json");
    let mut raw: serde_json::Value =
        serde_json::from_slice(services.inputs.get(&path).unwrap()).unwrap();
    change(&mut raw);
    let raw_bytes = serde_json::to_vec(&raw).unwrap();
    let response_sha = digest(&raw_bytes);
    replace_input(input, services, &path, raw_bytes);
    let mut receipt: serde_json::Value =
        serde_json::from_slice(services.inputs.get(&receipt_path).unwrap()).unwrap();
    receipt["response_sha256"] = serde_json::json!(response_sha);
    let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
    let receipt_sha = digest(&receipt_bytes);
    replace_input(input, services, &receipt_path, receipt_bytes);
    report["input_hashes"][&path] = serde_json::json!(response_sha);
    report["input_hashes"][&receipt_path] = serde_json::json!(receipt_sha);
    report["independent"][0]["response_sha256"] = serde_json::json!(response_sha);
    report["independent"][0]["turns"]=serde_json::Value::Array(raw["turns"].as_array().unwrap().iter().map(|t|serde_json::json!({
        "start":t["startMs"].as_f64().unwrap()/1000.0,"end":t["endMs"].as_f64().unwrap()/1000.0,"text":t["transcript"]})).collect());
    services.report = serde_json::to_vec(&report).unwrap();
    let mut facts: serde_json::Value =
        serde_json::from_slice(services.inputs.get("source-facts.json").unwrap()).unwrap();
    facts["independent_receipts"][0]["sha256"] = serde_json::json!(receipt_sha);
    replace_input(
        input,
        services,
        "source-facts.json",
        serde_json::to_vec(&facts).unwrap(),
    );
}
#[tokio::test]
async fn fact_matching_rejects_partial_coverage_hash_changes_and_missing_evidence() {
    for fault in [
        "source-hash",
        "audio-hash",
        "manifest-hash",
        "recipe-hash",
        "receipt-hash",
        "missing-interval",
        "partial-fact",
        "partial-primary",
        "missing-independent",
    ] {
        let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
        attach_fact_inventory(&mut input, &mut services);
        let mut facts: serde_json::Value =
            serde_json::from_slice(services.inputs.get("source-facts.json").unwrap()).unwrap();
        match fault {
            "source-hash" => facts["source_sha256"] = serde_json::json!("0".repeat(64)),
            "audio-hash" => facts["narration_sha256"] = serde_json::json!("0".repeat(64)),
            "manifest-hash" => {
                facts["acoustic_manifest_sha256"] = serde_json::json!("0".repeat(64))
            }
            "recipe-hash" => facts["primary_recipe_sha256"] = serde_json::json!("0".repeat(64)),
            "receipt-hash" => {
                facts["independent_receipts"][0]["sha256"] = serde_json::json!("0".repeat(64))
            }
            "missing-interval" => {
                facts["facts"][0]["interval_ids"] = serde_json::json!(["absent-interval"])
            }
            "partial-fact" => {
                facts["facts"][0]["source_spans"] = serde_json::json!([[315, 336], [779, 809]])
            }
            "partial-primary" => {
                facts["intervals"][0]["source_span"] = serde_json::json!([325, 336])
            }
            "missing-independent" => facts["independent_receipts"] = serde_json::json!([]),
            _ => unreachable!(),
        }
        replace_input(
            &mut input,
            &mut services,
            "source-facts.json",
            serde_json::to_vec(&facts).unwrap(),
        );
        add_recorded_report(&mut input, &mut services);
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
        let result = artifact_json(&output, &services, "fact-evidence.json");
        let fact = result["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == "cellar-instruction")
            .unwrap();
        assert_ne!(fact["outcome"], "pass", "accepted {fault}");
        assert!(
            fact["not_before_sample"].is_null(),
            "bound survived {fault}"
        );
        if fault == "missing-independent" {
            assert_eq!(fact["outcome"], "missing");
        }
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
        assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn fact_matching_checks_complete_independent_text_unique_terminals_and_exact_tolerance() {
    for fault in [
        "within-rule",
        "outside-rule",
        "incomplete",
        "repeated-terminal",
        "duplicate-interval",
        "unordered",
    ] {
        let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
        attach_fact_inventory(&mut input, &mut services);
        mutate_independent(&mut input, &mut services, |raw| match fault {
            "within-rule" => raw["turns"][2]["endMs"] = serde_json::json!(21970),
            "outside-rule" => raw["turns"][2]["endMs"] = serde_json::json!(21969.95),
            "incomplete" => {
                raw["turns"][2]["transcript"] = serde_json::json!("for the cellar");
                raw["transcript"] = serde_json::json!(
                    raw["turns"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|t| t["transcript"].as_str().unwrap())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            "repeated-terminal" => {
                raw["transcript"] = serde_json::json!(format!(
                    "{} for the cellar",
                    raw["transcript"].as_str().unwrap()
                ))
            }
            "duplicate-interval" => {
                raw["turns"].as_array_mut().unwrap().push(serde_json::json!({"turnId":99,"startMs":60000,"endMs":60300,"transcript":"Run for the cellar"}));
                raw["transcript"] = serde_json::json!(format!(
                    "{} Run for the cellar",
                    raw["transcript"].as_str().unwrap()
                ));
            }
            "unordered" => raw["turns"].as_array_mut().unwrap().reverse(),
            _ => unreachable!(),
        });
        add_recorded_report(&mut input, &mut services);
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        let result = artifact_json(&output, &services, "fact-evidence.json");
        let fact = result["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == "cellar-instruction")
            .unwrap();
        if fault == "within-rule" {
            assert_eq!(fact["outcome"], "pass", "{fact}");
            assert_eq!(fact["not_before_sample"], 545280);
        } else {
            assert_ne!(fact["outcome"], "pass", "accepted {fault}");
            assert!(fact["not_before_sample"].is_null());
        }
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn one_fact_can_span_multiple_fully_verified_intervals_without_skipping_prose() {
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    attach_fact_inventory(&mut input, &mut services);
    let mut inventory: serde_json::Value =
        serde_json::from_slice(services.inputs.get("source-facts.json").unwrap()).unwrap();
    inventory["facts"][3]["source_spans"] = serde_json::json!([[283, 336]]);
    replace_input(
        &mut input,
        &mut services,
        "source-facts.json",
        serde_json::to_vec(&inventory).unwrap(),
    );
    add_recorded_report(&mut input, &mut services);
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    let report = artifact_json(&output, &services, "fact-evidence.json");
    let fact = report["facts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "urged-cellar")
        .unwrap();
    assert_eq!(fact["outcome"], "pass", "{fact}");
    assert_eq!(fact["not_before_sample"], 546720);
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
}

fn recorded_fact_inputs(
    original: &StageOutput,
    previous: &ProtocolServices,
    include_fact_artifact: bool,
) -> (StageInput, ProtocolServices) {
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    attach_fact_inventory(&mut input, &mut services);
    for candidate in &original.artifacts {
        if candidate.file.path == "fact-evidence.json" && !include_fact_artifact {
            continue;
        }
        let bytes = previous
            .candidates
            .lock()
            .unwrap()
            .get(&candidate.candidate_id)
            .unwrap()
            .1
            .clone();
        input.inputs.push(ArtifactRef {
            id: format!("recorded:{}", candidate.file.path),
            file: candidate.file.clone(),
            provenance: candidate.provenance,
        });
        services.inputs.insert(candidate.file.path.clone(), bytes);
    }
    (input, services)
}
#[tokio::test]
async fn fact_replay_is_identical_without_redispatch_and_rejects_changed_inventory() {
    let (mut input, mut original) = protocol_fixture(ExecutionMode::Fixture, "");
    attach_fact_inventory(&mut input, &mut original);
    let first = GenerationExecutor.execute(input, &original).await.unwrap();
    let (input, replayed) = recorded_fact_inputs(&first, &original, true);
    let output = GenerationExecutor.execute(input, &replayed).await.unwrap();
    assert_eq!(
        artifact_json(&first, &original, "fact-evidence.json"),
        artifact_json(&output, &replayed, "fact-evidence.json")
    );
    assert_eq!(replayed.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(replayed.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(replayed.emits.load(Ordering::SeqCst), 0);
    for include_fact_artifact in [true, false] {
        let (mut input, mut replayed) =
            recorded_fact_inputs(&first, &original, include_fact_artifact);
        let mut facts: serde_json::Value =
            serde_json::from_slice(replayed.inputs.get("source-facts.json").unwrap()).unwrap();
        facts["required_fact_ids"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!("changed-required-fact"));
        replace_input(
            &mut input,
            &mut replayed,
            "source-facts.json",
            serde_json::to_vec(&facts).unwrap(),
        );
        assert!(
            GenerationExecutor.execute(input, &replayed).await.is_err(),
            "changed fact inventory was accepted as recorded replay"
        );
        assert_eq!(replayed.helper_calls.load(Ordering::SeqCst), 0);
        assert!(replayed.candidates.lock().unwrap().is_empty());
    }
}

fn rebind_recorded_hash(
    input: &mut StageInput,
    services: &mut ProtocolServices,
    path: &str,
    bytes: Vec<u8>,
) {
    let hash = digest(&bytes);
    replace_input(input, services, path, bytes);
    let mut report: serde_json::Value = serde_json::from_slice(&services.report).unwrap();
    report["input_hashes"][path] = serde_json::json!(hash);
    services.report = serde_json::to_vec(&report).unwrap();
}
#[tokio::test]
async fn fact_matching_rejects_request_response_audio_and_model_binding_errors() {
    for fault in [
        "primary-request",
        "primary-response",
        "independent-response",
        "independent-audio",
        "same-model",
        "wrong-pcm-placement",
    ] {
        let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
        attach_fact_inventory(&mut input, &mut services);
        let report: serde_json::Value = serde_json::from_slice(&services.report).unwrap();
        let primary = report["primary_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["case"] == "turn-015")
            .unwrap();
        let p_request = primary["request_sha256"].as_str().unwrap();
        let i_request = report["independent"][0]["request_sha256"].as_str().unwrap();
        match fault {
            "primary-request" => {
                let path = format!("transcribe/{p_request}.receipt.json");
                let mut receipt: serde_json::Value =
                    serde_json::from_slice(services.inputs.get(&path).unwrap()).unwrap();
                receipt["request_sha256"] = serde_json::json!("0".repeat(64));
                rebind_recorded_hash(
                    &mut input,
                    &mut services,
                    &path,
                    serde_json::to_vec(&receipt).unwrap(),
                );
            }
            "primary-response" | "independent-response" => {
                let path = if fault == "primary-response" {
                    format!("transcribe/{p_request}.json")
                } else {
                    format!("meta/{i_request}.json")
                };
                let mut bytes = services.inputs.get(&path).unwrap().clone();
                bytes.push(b' ');
                rebind_recorded_hash(&mut input, &mut services, &path, bytes);
            }
            "independent-audio" | "same-model" => {
                let path = format!("meta/{i_request}.receipt.json");
                let mut receipt: serde_json::Value =
                    serde_json::from_slice(services.inputs.get(&path).unwrap()).unwrap();
                if fault == "same-model" {
                    receipt["model"] = primary["model"].clone();
                } else {
                    receipt["audio_sha256"] = serde_json::json!("0".repeat(64));
                }
                rebind_recorded_hash(
                    &mut input,
                    &mut services,
                    &path,
                    serde_json::to_vec(&receipt).unwrap(),
                );
                let mut facts: serde_json::Value =
                    serde_json::from_slice(services.inputs.get("source-facts.json").unwrap())
                        .unwrap();
                facts["independent_receipts"][0]["sha256"] =
                    serde_json::json!(digest(services.inputs.get(&path).unwrap()));
                replace_input(
                    &mut input,
                    &mut services,
                    "source-facts.json",
                    serde_json::to_vec(&facts).unwrap(),
                );
                if fault == "same-model" {
                    let mut report: serde_json::Value =
                        serde_json::from_slice(&services.report).unwrap();
                    report["independent"][0]["model"] = primary["model"].clone();
                    services.report = serde_json::to_vec(&report).unwrap();
                }
            }
            "wrong-pcm-placement" => {
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(services.inputs.get("manifest.json").unwrap()).unwrap();
                for field in ["start_sample", "end_sample"] {
                    manifest["units"][8][field] =
                        serde_json::json!(manifest["units"][8][field].as_u64().unwrap() + 1);
                }
                rebind_recorded_hash(
                    &mut input,
                    &mut services,
                    "manifest.json",
                    serde_json::to_vec(&manifest).unwrap(),
                );
                let manifest_sha = digest(services.inputs.get("manifest.json").unwrap());
                let mut saved: serde_json::Value = serde_json::from_slice(
                    services
                        .inputs
                        .get("segmented-acoustic-report.json")
                        .unwrap(),
                )
                .unwrap();
                saved["input_manifest_sha256"] = serde_json::json!(manifest_sha);
                rebind_recorded_hash(
                    &mut input,
                    &mut services,
                    "segmented-acoustic-report.json",
                    serde_json::to_vec(&saved).unwrap(),
                );
                let mut facts: serde_json::Value =
                    serde_json::from_slice(services.inputs.get("source-facts.json").unwrap())
                        .unwrap();
                facts["acoustic_manifest_sha256"] = serde_json::json!(manifest_sha);
                replace_input(
                    &mut input,
                    &mut services,
                    "source-facts.json",
                    serde_json::to_vec(&facts).unwrap(),
                );
            }
            _ => unreachable!(),
        }
        add_recorded_report(&mut input, &mut services);
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        let matching = artifact_json(&output, &services, "fact-evidence.json");
        assert!(
            matching["facts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["outcome"] != "pass" && r["not_before_sample"].is_null()),
            "accepted {fault}"
        );
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn fact_matching_does_not_promote_read_along_name_normalization_into_fact_evidence() {
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    attach_fact_inventory(&mut input, &mut services);
    let mut facts: serde_json::Value =
        serde_json::from_slice(services.inputs.get("source-facts.json").unwrap()).unwrap();
    facts["intervals"][0]["source_span"] = serde_json::json!([32, 63]);
    facts["intervals"][0]["primary_unit_ids"] = serde_json::json!(["turn-008"]);
    facts["facts"][0]["source_spans"] = serde_json::json!([[32, 63]]);
    replace_input(
        &mut input,
        &mut services,
        "source-facts.json",
        serde_json::to_vec(&facts).unwrap(),
    );
    add_recorded_report(&mut input, &mut services);
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    let matching = artifact_json(&output, &services, "fact-evidence.json");
    let fact = matching["facts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "cellar-instruction")
        .unwrap();
    assert_eq!(fact["outcome"], "uncertain");
    assert!(fact["not_before_sample"].is_null());
    let interval = matching["intervals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "cellar-turn")
        .unwrap();
    assert_eq!(interval["reason"], "primary_complete_text_mismatch");
    let summary = artifact_json(&output, &services, "acoustic-validation.json");
    assert_eq!(summary["word_count"], 176);
    assert_eq!(summary["highlight_group_count"], 170);
}

#[tokio::test]
async fn runtime_fact_checkpoint_reconstructs_committed_artifacts_without_redispatch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    fn copy_inputs(source: &Path, destination: &Path) {
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let path = destination.join(entry.file_name());
            if entry.path().is_dir() {
                std::fs::create_dir_all(&path).unwrap();
                copy_inputs(&entry.path(), &path);
            } else {
                std::fs::copy(entry.path(), path).unwrap();
            }
        }
    }
    let fixture_inputs = tempfile::tempdir().unwrap();
    copy_inputs(
        &root.join("fixtures/generation/acoustic"),
        fixture_inputs.path(),
    );
    copy_inputs(
        &root.join("fixtures/generation/acoustic-facts-v1"),
        fixture_inputs.path(),
    );
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let request = serde_json::json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture",
        "pins":{"recipe_sha256":digest(b"offline-fact-checkpoint-v1"),"profile_sha256":digest(b"diagnostic-profile"),"profile_id":"paper-player-r186-v1"},
        "limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}});
    let run = store.create("facts-fixture", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_acoustic_fixture(&run, epoch, fixture_inputs.path())
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
    let original = artifacts
        .iter()
        .find(|a| a.file.path == "fact-evidence.json")
        .unwrap();
    let expected = store.artifact(&original.id).unwrap().0;
    assert_eq!(
        store
            .journal(&run, 0, 500)
            .unwrap()
            .iter()
            .filter(|e| e["type"] == "helper_authorized")
            .count(),
        1
    );
    let before = store.snapshot(&run).unwrap();
    let journal = store.journal(&run, 0, 500).unwrap();
    let validation = artifacts
        .iter()
        .find(|a| a.file.path == "acoustic-validation.json")
        .unwrap();
    let summary: serde_json::Value =
        serde_json::from_slice(&store.artifact(&validation.id).unwrap().0).unwrap();
    let (mut input, mut replayed) = protocol_fixture(ExecutionMode::Replay, "");
    input.inputs = serde_json::from_value(summary["inputs"].clone()).unwrap();
    for artifact in artifacts {
        if !input.inputs.iter().any(|a| a.id == artifact.id) {
            input.inputs.push(artifact.clone());
        }
    }
    replayed.inputs.clear();
    for artifact in &input.inputs {
        replayed.inputs.insert(
            artifact.file.path.clone(),
            store.artifact(&artifact.id).unwrap().0,
        );
    }
    drop(runtime);
    let output = GenerationExecutor.execute(input, &replayed).await.unwrap();
    let facts = output
        .artifacts
        .iter()
        .find(|a| a.file.path == "fact-evidence.json")
        .unwrap();
    assert_eq!(facts.file.sha256, original.file.sha256);
    assert_eq!(
        replayed.candidates.lock().unwrap()[&facts.candidate_id].1,
        expected
    );
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    assert_eq!(replayed.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(replayed.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(replayed.emits.load(Ordering::SeqCst), 0);
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert_eq!(store.journal(&run, 0, 500).unwrap(), journal);
    assert!(
        journal
            .iter()
            .all(|e| !e["type"].as_str().unwrap().starts_with("provider_"))
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
}

#[tokio::test]
async fn fact_replay_preserves_recorded_adverse_findings() {
    let (mut input, mut original) = protocol_fixture(ExecutionMode::Fixture, "");
    attach_fact_inventory(&mut input, &mut original);
    let first = GenerationExecutor.execute(input, &original).await.unwrap();
    for outcome in [ReviewOutcome::Fail, ReviewOutcome::Uncertain] {
        let (mut input, mut replayed) = recorded_fact_inputs(&first, &original, true);
        let mut summary: serde_json::Value =
            serde_json::from_slice(replayed.inputs.get("acoustic-validation.json").unwrap())
                .unwrap();
        let fact = summary["findings"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["id"] == "acoustic-fact:cellar-instruction")
            .unwrap();
        fact["outcome"] = serde_json::to_value(outcome).unwrap();
        replace_input(
            &mut input,
            &mut replayed,
            "acoustic-validation.json",
            serde_json::to_vec(&summary).unwrap(),
        );
        let output = GenerationExecutor.execute(input, &replayed).await.unwrap();
        assert_eq!(
            output
                .findings
                .iter()
                .find(|f| f.id == "acoustic-fact:cellar-instruction")
                .unwrap()
                .outcome,
            outcome
        );
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
        assert_eq!(replayed.helper_calls.load(Ordering::SeqCst), 0);
    }
}

mod chapter_bundle;
mod presentation;
mod presentation_math;
