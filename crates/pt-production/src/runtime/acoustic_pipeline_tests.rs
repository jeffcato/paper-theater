use super::*;
use async_trait::async_trait;
use pt_contracts as c;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
struct Audit(Arc<Mutex<Vec<c::StageInput>>>);
#[async_trait]
impl c::StageExecutor for Audit {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        self.0.lock().unwrap().push(input.clone());
        c::StageExecutor::execute(&crate::generation::GenerationExecutor, input, services).await
    }
}
fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn request() -> Value {
    let profile =
        std::fs::read(root().join("fixtures/generation/two-scene-readiness-v2/profile.json"))
            .unwrap();
    json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":digest(b"saved-diagnostic-v1"),"profile_sha256":digest(&profile),"profile_id":"paper-player-r186-v1"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}})
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
#[tokio::test]
async fn saved_pipeline_runs_real_acoustic_bundle_and_retains_missing_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let id = store.create("real-acoustic-pipeline", &request()).unwrap()["id"]
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
    let snapshot = store.api_snapshot(&id).unwrap();
    assert_eq!(snapshot.state, c::RunState::Failed);
    let acoustic = snapshot
        .stages
        .iter()
        .find(|s| s.stage == c::Stage::Acoustic)
        .unwrap();
    assert_eq!(acoustic.outcome, Some(c::StageOutcome::NeedsRepair));
    for path in ["acoustic-replay.json", "acoustic-validation.json"] {
        let a = acoustic
            .artifacts
            .iter()
            .find(|a| a.file.path == path)
            .expect("real acoustic report missing");
        let bytes = store.artifact(&a.id).unwrap().0;
        a.file.verify(&bytes).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["provider_calls"], 0);
        assert_eq!(value["production_approved"], false);
        if path == "acoustic-validation.json" {
            assert_eq!(value["word_count"], 176);
            assert_eq!(value["highlight_group_count"], 170);
            assert!(
                value["missing_evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == "independent-acoustic-evidence")
            );
            for finding in value["findings"].as_array().unwrap() {
                assert!(
                    snapshot
                        .findings
                        .iter()
                        .any(|f| serde_json::to_value(f).unwrap() == *finding),
                    "acoustic finding was overwritten by a later stage"
                );
            }
        }
    }
    assert!(
        store.diagnostic_ref(&id).unwrap().is_some(),
        "diagnostic package must remain inspectable"
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 4);
    for input in seen.iter() {
        let paths = input
            .inputs
            .iter()
            .map(|a| a.file.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(paths.iter().filter(|p| **p == "manifest.json").count(), 1);
        match input.stage {
            c::Stage::Presentation => {
                assert!(paths.contains(&"delivery/fixture.json"));
                assert!(paths.contains(&"source-facts.json"));
                assert!(!paths.contains(&"fixture.json"));
            }
            c::Stage::Acoustic => {
                assert!(paths.contains(&"segmented-acoustic-report.json"));
                assert!(!paths.contains(&"fixture.json"));
            }
            _ => {
                assert!(paths.contains(&"fixture.json"));
                assert!(!paths.contains(&"segmented-acoustic-report.json"));
            }
        }
    }
    let events = store.journal(&id, 0, 500).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "helper_authorized")
            .count(),
        1
    );
    assert!(
        events
            .iter()
            .all(|e| !e["type"].as_str().unwrap().starts_with("provider_"))
    );
}
#[tokio::test]
async fn restart_reuses_verified_acoustic_needs_repair_without_another_helper_attempt() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let id = store.create("acoustic-restart", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_fixture(
            &id,
            epoch,
            &root().join("fixtures/generation/two-scene-readiness-v2"),
            Some(&root().join("fixtures/generation/source")),
        )
        .unwrap();
    store
        .seed_acoustic_fixture(&id, epoch, &root().join("fixtures/generation/acoustic"))
        .unwrap();
    let rt = runtime(
        store.clone(),
        Arc::new(crate::generation::GenerationExecutor),
    );
    rt.execute_stage(&id, epoch, c::Stage::Source)
        .await
        .unwrap();
    assert_eq!(
        rt.execute_stage(&id, epoch, c::Stage::Acoustic)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    let before = store
        .api_snapshot(&id)
        .unwrap()
        .stages
        .into_iter()
        .find(|s| s.stage == c::Stage::Acoustic)
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.file.path == "acoustic-validation.json")
        .unwrap();
    drop(rt);
    drop(store);
    // Emulate persisted inputs written before the additive scope index existed.
    let db = rusqlite::Connection::open(temp.path().join("runtime.sqlite3")).unwrap();
    db.execute("DELETE FROM run_input_scopes", []).unwrap();
    drop(db);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    assert_eq!(
        runtime(
            store.clone(),
            Arc::new(crate::generation::GenerationExecutor)
        )
        .work_once()
        .await
        .unwrap_err()
        .code,
        "stage_needs_repair"
    );
    let snapshot = store.api_snapshot(&id).unwrap();
    assert_eq!(snapshot.state, c::RunState::Failed);
    let ac = snapshot
        .stages
        .iter()
        .find(|s| s.stage == c::Stage::Acoustic)
        .unwrap();
    assert_eq!(ac.outcome, Some(c::StageOutcome::NeedsRepair));
    assert!(
        ac.artifacts
            .iter()
            .any(|a| a.id == before.id && a.file == before.file)
    );
    let events = store.journal(&id, 0, 500).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "helper_authorized")
            .count(),
        1
    );
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "cache_reused" && e["data"]["stage"] == "acoustic")
    );
    assert!(store.diagnostic_ref(&id).unwrap().is_some());
    assert!(store.catalog("pg-55").unwrap().is_none());
}
struct HoldAcoustic {
    ready: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl c::StageExecutor for HoldAcoustic {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let stage = input.stage;
        let result =
            c::StageExecutor::execute(&crate::generation::GenerationExecutor, input, services)
                .await?;
        if stage == c::Stage::Acoustic {
            self.ready.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(result)
    }
}
#[tokio::test]
async fn cancel_after_real_helper_return_prevents_acoustic_and_package_acceptance() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let id = store.create("acoustic-cancel", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let ready = Arc::new(tokio::sync::Notify::new());
    let rt = runtime(
        store.clone(),
        Arc::new(HoldAcoustic {
            ready: ready.clone(),
        }),
    );
    let worker = tokio::spawn(async move { rt.work_once().await });
    tokio::time::timeout(std::time::Duration::from_secs(40), ready.notified())
        .await
        .unwrap();
    store.cancel(&id).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let saved = store.api_snapshot(&id).unwrap();
    assert_eq!(saved.state, c::RunState::Cancelled);
    assert!(saved.epoch > 0);
    assert!(
        !saved
            .stages
            .iter()
            .flat_map(|s| &s.artifacts)
            .any(|a| matches!(
                a.file.path.as_str(),
                "acoustic-replay.json" | "acoustic-validation.json"
            ))
    );
    assert!(store.diagnostic_ref(&id).unwrap().is_none());
    assert!(store.catalog("pg-55").unwrap().is_none());
    let journal = store.journal(&id, 0, 500).unwrap();
    assert!(journal.iter().any(|e| e["type"] == "helper_completed"));
    assert!(
        !journal
            .iter()
            .any(|e| e["type"] == "task_completed" && e["data"]["stage"] == "acoustic")
    );
}
#[tokio::test]
async fn recorded_acoustic_replay_keeps_reports_journal_and_accounting_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let id = store
        .create("acoustic-recorded-replay", &request())
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        runtime(
            store.clone(),
            Arc::new(crate::generation::GenerationExecutor)
        )
        .work_once()
        .await
        .unwrap_err()
        .code,
        "stage_needs_repair"
    );
    let before = store.api_snapshot(&id).unwrap();
    let events = store.journal(&id, 0, 500).unwrap();
    let db_hash = digest(&std::fs::read(temp.path().join("runtime.sqlite3")).unwrap());
    let acoustic = before
        .stages
        .iter()
        .find(|s| s.stage == c::Stage::Acoustic)
        .unwrap();
    for _ in 0..3 {
        for a in acoustic
            .artifacts
            .iter()
            .filter(|a| a.file.path.starts_with("acoustic-"))
        {
            a.file.verify(&store.artifact(&a.id).unwrap().0).unwrap();
        }
        assert_eq!(
            serde_json::to_value(store.api_snapshot(&id).unwrap()).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        assert_eq!(store.journal(&id, 0, 500).unwrap(), events);
        assert!(store.catalog("pg-55").unwrap().is_none());
    }
    assert_eq!(
        digest(&std::fs::read(temp.path().join("runtime.sqlite3")).unwrap()),
        db_hash
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "helper_authorized")
            .count(),
        1
    );
}
