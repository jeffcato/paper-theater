use super::*;
use async_trait::async_trait;
use pt_contracts as c;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn request() -> Value {
    let profile =
        std::fs::read(root().join("fixtures/generation/two-scene-readiness-v2/profile.json"))
            .unwrap();
    json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":digest(b"saved-diagnostic-v1"),"profile_sha256":digest(&profile),"profile_id":"paper-player-r186-v1"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}})
}
struct Audit(Arc<Mutex<Vec<(c::StageInput, c::StageOutput)>>>);
#[async_trait]
impl c::StageExecutor for Audit {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let output = c::StageExecutor::execute(
            &crate::generation::GenerationExecutor,
            input.clone(),
            services,
        )
        .await?;
        self.0.lock().unwrap().push((input, output.clone()));
        Ok(output)
    }
}
fn runtime(store: Store, executor: Arc<dyn c::StageExecutor>) -> Runtime {
    Runtime::new(store, Arc::new(FixturePipeline::new(executor)))
        .with_saved_inputs(
            root().join("fixtures/generation/two-scene-readiness-v2"),
            root().join("fixtures/generation/source"),
        )
        .with_acoustic_inputs(root().join("fixtures/generation/acoustic"))
        .with_presentation_inputs(root())
        .with_offline_acoustic(root().join("tools/generation"))
        .unwrap()
}
fn report(store: &Store, run: &str) -> (c::ArtifactRef, Vec<u8>) {
    let snapshot = store.api_snapshot(run).unwrap();
    let stage = snapshot
        .stages
        .iter()
        .find(|s| s.stage == c::Stage::Presentation)
        .unwrap();
    assert_eq!(stage.outcome, Some(c::StageOutcome::NeedsRepair));
    let reference = stage
        .artifacts
        .iter()
        .find(|a| a.file.path == "presentation-fact-evidence.json")
        .expect("real Presentation report missing")
        .clone();
    let bytes = store.artifact(&reference.id).unwrap().0;
    reference.file.verify(&bytes).unwrap();
    (reference, bytes)
}
#[tokio::test]
async fn real_presentation_persists_exact_inputs_bounds_and_findings_through_package() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("presentation-pipeline", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let seen = Arc::new(Mutex::new(Vec::new()));
    assert_eq!(
        runtime(store.clone(), Arc::new(Audit(seen.clone())))
            .work_once()
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    let (_, bytes) = report(&store, &run);
    assert_eq!(
        bytes,
        std::fs::read(
            root().join(
                "fixtures/generation/presentation-evidence-v1/presentation-fact-evidence.json"
            )
        )
        .unwrap()
    );
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    for (id, bound) in [
        ("cellar-instruction", 1189440),
        ("Dorothy-urged", 1109280),
        ("screaming", 1109280),
        ("urged-cellar", 1189440),
    ] {
        assert_eq!(
            value["facts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["id"] == id)
                .unwrap()["delivery_not_before_sample"],
            bound
        );
    }
    let snapshot = store.api_snapshot(&run).unwrap();
    assert_eq!(snapshot.state, c::RunState::Failed);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 4);
    let (input, output) = seen
        .iter()
        .find(|(i, _)| i.stage == c::Stage::Presentation)
        .unwrap();
    let index: Vec<Value> = serde_json::from_slice(
        &std::fs::read(
            root().join("fixtures/generation/presentation-evidence-v1/input-index.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(input.inputs.len(), 56);
    for entry in index {
        let expected: c::ArtifactRef = serde_json::from_value(entry["artifact"].clone()).unwrap();
        assert!(
            input
                .inputs
                .iter()
                .any(|a| a.file == expected.file && a.provenance == expected.provenance)
        );
    }
    for finding in &output.findings {
        assert!(snapshot.findings.iter().any(|f|serde_json::to_value(f).unwrap()==serde_json::to_value(finding).unwrap()));
    }
    assert!(store.diagnostic_ref(&run).unwrap().is_some());
    assert!(
        store
            .package_artifact(&run, "presentation-fact-evidence.json")
            .is_err()
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    let journal = store.journal(&run, 0, 1000).unwrap();
    assert_eq!(
        journal
            .iter()
            .filter(|e| e["type"] == "helper_authorized")
            .count(),
        1
    );
    assert!(
        !journal
            .iter()
            .any(|e| e["type"].as_str().unwrap().starts_with("provider_"))
    );
}

#[tokio::test]
async fn presentation_restarts_and_replays_identically_without_redispatch_or_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("presentation-restart", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_presentation_fixture(&run, epoch, &root())
        .unwrap();
    // Presentation works with no registered helper at all.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let rt = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(Audit(seen.clone())))),
    );
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Presentation)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    let expected = report(&store, &run);
    drop(rt);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(Audit(seen.clone())))),
    );
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Presentation)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    let reused = report(&store, &run);
    assert_eq!(reused.0.id, expected.0.id);
    assert_eq!(reused.0.file, expected.0.file);
    assert_eq!(reused.1, expected.1);
    assert_eq!(
        seen.lock().unwrap().len(),
        1,
        "restart redispatched generation"
    );
    store
        .finish(&run, epoch, "failed", Some("stage_needs_repair"))
        .unwrap();
    let before = store.snapshot(&run).unwrap();
    let journal = store.journal(&run, 0, 1000).unwrap();
    let db = std::fs::read(temp.path().join("runtime.sqlite3")).unwrap();
    for _ in 0..2 {
        assert_eq!(rt.replay_presentation(&run).await.unwrap(), expected.1);
        assert_eq!(store.snapshot(&run).unwrap(), before);
        assert_eq!(store.journal(&run, 0, 1000).unwrap(), journal);
        assert!(store.catalog("pg-55").unwrap().is_none());
    }
    assert_eq!(
        std::fs::read(temp.path().join("runtime.sqlite3")).unwrap(),
        db
    );
    let seen = seen.lock().unwrap();
    assert!(
        seen.iter()
            .skip(1)
            .all(|(i, o)| i.mode == c::ExecutionMode::Replay
                && i.inputs.len() == 57
                && o.outcome == c::StageOutcome::NeedsRepair)
    );
    assert!(
        !journal
            .iter()
            .any(|e| e["type"].as_str().unwrap().starts_with("helper_")
                || e["type"].as_str().unwrap().starts_with("provider_")
                || e["type"] == "journal_entry")
    );
}

struct HoldPresentation(Arc<tokio::sync::Notify>);
#[async_trait]
impl c::StageExecutor for HoldPresentation {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let output =
            c::StageExecutor::execute(&crate::generation::GenerationExecutor, input, services)
                .await?;
        assert_eq!(
            output.artifacts[0].file.path,
            "presentation-fact-evidence.json"
        );
        self.0.notify_one();
        std::future::pending::<()>().await;
        Ok(output)
    }
}
#[tokio::test]
async fn cancellation_after_real_presentation_return_prevents_all_result_commits() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("presentation-cancel", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_presentation_fixture(&run, epoch, &root())
        .unwrap();
    let ready = Arc::new(tokio::sync::Notify::new());
    let rt = Runtime::new(store.clone(), Arc::new(HoldPresentation(ready.clone())));
    let task_run = run.clone();
    let worker = tokio::spawn(async move {
        rt.execute_stage(&task_run, epoch, c::Stage::Presentation)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(40), ready.notified())
        .await
        .unwrap();
    store.cancel(&run).unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), worker)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .code,
        "stale_epoch"
    );
    let snapshot = store.api_snapshot(&run).unwrap();
    assert_eq!(snapshot.state, c::RunState::Cancelled);
    let presentation = snapshot
        .stages
        .iter()
        .find(|s| s.stage == c::Stage::Presentation)
        .unwrap();
    assert!(presentation.artifacts.is_empty());
    assert!(snapshot.findings.is_empty());
    assert!(store.diagnostic_ref(&run).unwrap().is_none());
    assert!(store.catalog("pg-55").unwrap().is_none());
    let journal = store.journal(&run, 0, 1000).unwrap();
    assert!(!journal.iter().any(|e| e["type"] == "task_completed"
        || e["type"] == "artifact_ready"
        || e["type"] == "check_completed"));
}

#[tokio::test]
async fn synthetic_recipe_does_not_require_the_saved_presentation_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let mut request = request();
    request["pins"]["recipe_sha256"] = json!(digest(b"fixture-v1"));
    let run = store.create("synthetic-unchanged", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(
            crate::generation::GenerationExecutor,
        ))),
    )
    .work_once()
    .await
    .unwrap();
    assert_eq!(
        store.api_snapshot(&run).unwrap().state,
        c::RunState::Succeeded
    );
    assert!(store.diagnostic_ref(&run).unwrap().is_none());
    assert!(store.catalog("pg-55").unwrap().is_none());
}

#[tokio::test]
async fn missing_and_corrupt_dependencies_or_reports_fail_cache_and_replay_without_redispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("presentation-corruption", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_presentation_fixture(&run, epoch, &root())
        .unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let rt = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(Audit(seen.clone())))),
    );
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Presentation)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    let (reference, bytes) = report(&store, &run);
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    let hashes = [
        value["bindings"]["source-facts.json"]["sha256"]
            .as_str()
            .unwrap(),
        value["bindings"]["delivery/audio/master.wav"]["sha256"]
            .as_str()
            .unwrap(),
        &reference.file.sha256,
    ];
    for replay in [false, true] {
        if replay {
            store
                .finish(&run, epoch, "failed", Some("stage_needs_repair"))
                .unwrap();
        }
        let snapshot = store.snapshot(&run).unwrap();
        let journal = store.journal(&run, 0, 1000).unwrap();
        for hash in hashes {
            let path = store.object(hash).unwrap();
            let original = std::fs::read(&path).unwrap();
            for missing in [true, false] {
                if missing {
                    std::fs::remove_file(&path).unwrap();
                } else {
                    let mut corrupt = original.clone();
                    let last = corrupt.len() - 1;
                    corrupt[last] ^= 1;
                    std::fs::write(&path, corrupt).unwrap();
                }
                let error = if replay {
                    rt.replay_presentation(&run).await.unwrap_err()
                } else {
                    rt.execute_stage(&run, epoch, c::Stage::Presentation)
                        .await
                        .unwrap_err()
                };
                assert_eq!(error.code, "corrupt_artifact");
                assert_eq!(store.snapshot(&run).unwrap(), snapshot);
                assert_eq!(store.journal(&run, 0, 1000).unwrap(), journal);
                assert_eq!(seen.lock().unwrap().len(), 1);
                assert!(store.catalog("pg-55").unwrap().is_none());
                std::fs::write(&path, &original).unwrap();
            }
        }
    }
}

fn index() -> Vec<Value> {
    serde_json::from_slice(
        &std::fs::read(
            root().join("fixtures/generation/presentation-evidence-v1/input-index.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn acoustic_namespace_cannot_substitute_even_the_complete_presentation_bundle() {
    let exported = tempfile::tempdir().unwrap();
    for binding in index() {
        let path = exported
            .path()
            .join(binding["artifact"]["file"]["path"].as_str().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(root().join(binding["source"].as_str().unwrap()), path).unwrap();
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("presentation-scope", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_acoustic_fixture(&run, epoch, exported.path())
        .unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let rt = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(Audit(seen.clone())))),
    );
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Presentation)
            .await
            .unwrap_err()
            .code,
        "input_scope_mismatch"
    );
    assert!(seen.lock().unwrap().is_empty());
    store
        .seed_presentation_fixture(&run, epoch, &root())
        .unwrap();
    // The frozen scope can be reused with its source directory unavailable.
    store
        .seed_presentation_fixture(&run, epoch, &temp.path().join("absent"))
        .unwrap();
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Presentation)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    let (_, bytes) = report(&store, &run);
    assert_eq!(
        bytes,
        std::fs::read(
            root().join(
                "fixtures/generation/presentation-evidence-v1/presentation-fact-evidence.json"
            )
        )
        .unwrap()
    );
    let journal = store.journal(&run, 0, 1000).unwrap();
    assert_eq!(
        journal
            .iter()
            .filter(|e| e["type"] == "presentation_inputs_imported")
            .count(),
        1
    );
    assert!(
        !journal
            .iter()
            .any(|e| e["type"].as_str().unwrap().starts_with("helper_")
                || e["type"].as_str().unwrap().starts_with("provider_"))
    );
}

#[test]
fn missing_corrupt_and_symlink_source_inputs_cannot_partially_freeze_the_namespace() {
    let sources = tempfile::tempdir().unwrap();
    let index = index();
    for binding in &index {
        let source = binding["source"].as_str().unwrap();
        let path = sources.path().join(source);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(root().join(source), path).unwrap();
    }
    let last = sources
        .path()
        .join(index.last().unwrap()["source"].as_str().unwrap());
    let original = std::fs::read(&last).unwrap();
    for mode in ["missing", "corrupt", "symlink"] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = store.create(mode, &request()).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, epoch) = store.claim().unwrap().unwrap();
        std::fs::remove_file(&last).unwrap();
        match mode {
            "corrupt" => {
                let mut bytes = original.clone();
                bytes[0] ^= 1;
                std::fs::write(&last, bytes).unwrap();
            }
            "symlink" => std::os::unix::fs::symlink(
                root().join(index.last().unwrap()["source"].as_str().unwrap()),
                &last,
            )
            .unwrap(),
            _ => {}
        }
        let before = store.snapshot(&run).unwrap();
        let journal = store.journal(&run, 0, 1000).unwrap();
        assert!(
            store
                .seed_presentation_fixture(&run, epoch, sources.path())
                .is_err()
        );
        assert_eq!(store.snapshot(&run).unwrap(), before);
        assert_eq!(store.journal(&run, 0, 1000).unwrap(), journal);
        assert_eq!(
            std::fs::read_dir(temp.path().join("objects"))
                .unwrap()
                .count(),
            0
        );
        if last.symlink_metadata().is_ok() {
            std::fs::remove_file(&last).unwrap();
        }
        std::fs::write(&last, &original).unwrap();
        store
            .seed_presentation_fixture(&run, epoch, sources.path())
            .unwrap();
    }
}

struct PresentationOnly;
#[async_trait]
impl c::StageExecutor for PresentationOnly {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        if input.stage == c::Stage::Presentation {
            c::StageExecutor::execute(&crate::generation::GenerationExecutor, input, services).await
        } else {
            c::StageExecutor::execute(&FixtureExecutor, input, services).await
        }
    }
}
#[tokio::test]
async fn a_different_recipe_stops_at_real_presentation_needs_repair_before_packaging() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let mut request = request();
    request["pins"]["recipe_sha256"] = json!(digest(b"another-recipe"));
    let run = store.create("presentation-no-bypass", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_presentation_fixture(&run, epoch, &root())
        .unwrap();
    let rt = Runtime::new(store.clone(), Arc::new(PresentationOnly));
    assert_eq!(
        rt.execute_claimed(&run, epoch).await.unwrap_err().code,
        "stage_needs_repair"
    );
    let (_, bytes) = report(&store, &run);
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["production_approved"], false);
    assert!(value["reveal_permissions"].as_array().unwrap().is_empty());
    assert_eq!(store.api_snapshot(&run).unwrap().state, c::RunState::Failed);
    assert!(store.diagnostic_ref(&run).unwrap().is_none());
    assert!(store.catalog("pg-55").unwrap().is_none());
    assert!(
        !store
            .journal(&run, 0, 1000)
            .unwrap()
            .iter()
            .any(|e| e["type"] == "task_started" && e["data"]["stage"] == "package")
    );
}
