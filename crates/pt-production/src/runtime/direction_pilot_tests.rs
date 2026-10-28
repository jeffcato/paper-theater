use super::*;
use pt_contracts as c;
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
struct Saved {
    input: c::StageInput,
    bytes: BTreeMap<String, Vec<u8>>,
    candidates: Mutex<BTreeMap<String, (c::CandidateFile, Vec<u8>)>>,
}
impl Saved {
    fn load(case: &str) -> Self {
        let base = root().join("fixtures/generation").join(case);
        let input =
            serde_json::from_slice(&std::fs::read(base.join("stage-input.json")).unwrap()).unwrap();
        let rows: Vec<serde_json::Value> =
            serde_json::from_slice(&std::fs::read(base.join("input-index.json")).unwrap()).unwrap();
        Self {
            input,
            bytes: rows
                .iter()
                .map(|r| {
                    (
                        r["artifact"]["id"].as_str().unwrap().into(),
                        std::fs::read(root().join(r["source"].as_str().unwrap())).unwrap(),
                    )
                })
                .collect(),
            candidates: Mutex::new(BTreeMap::new()),
        }
    }
}
#[async_trait::async_trait]
impl c::RunServices for Saved {
    async fn read_artifact(
        &self,
        a: &c::ArtifactRef,
    ) -> std::result::Result<Vec<u8>, c::ServiceError> {
        let b = self
            .bytes
            .get(&a.id)
            .ok_or_else(|| c::ServiceError::MissingArtifact(a.id.clone()))?;
        a.file.verify(b)?;
        Ok(b.clone())
    }
    async fn write_candidate(
        &self,
        r: c::CandidateWrite,
        b: Vec<u8>,
    ) -> std::result::Result<c::CandidateFile, c::ServiceError> {
        let a = c::CandidateFile {
            candidate_id: r.candidate_id,
            file: c::FileRef {
                path: r.relative_path,
                sha256: digest(&b),
                bytes: b.len() as u64,
                media_type: r.media_type,
                dimensions: r.dimensions,
            },
            provenance: r.provenance,
        };
        self.candidates
            .lock()
            .unwrap()
            .insert(a.candidate_id.clone(), (a.clone(), b));
        Ok(a)
    }
    async fn check_cancelled(&self) -> std::result::Result<(), c::ServiceError> {
        Ok(())
    }
    async fn emit(&self, _: c::JournalProposal) -> std::result::Result<(), c::ServiceError> {
        panic!("private planning must not emit")
    }
    async fn provider(
        &self,
        _: c::ProviderOperation,
    ) -> std::result::Result<c::ProviderResult, c::ServiceError> {
        panic!("no legacy dispatch")
    }
    async fn helper(
        &self,
        _: c::HelperOperation,
    ) -> std::result::Result<c::HelperResult, c::ServiceError> {
        panic!("no helper dispatch")
    }
    async fn library(
        &self,
        _: c::LibraryQuery,
    ) -> std::result::Result<Vec<c::LibraryBinding>, c::ServiceError> {
        panic!("no library authority")
    }
}
#[tokio::test]
async fn pinned_direction_and_pilot_recipes_reach_the_real_generation_stage() {
    for (case, path) in [
        ("direction-v1/narration-only", "private/direction-plan.json"),
        ("pilot-v1/saved-audio", "private/pilot-plan.json"),
    ] {
        let saved = Saved::load(case);
        let pipeline = FixturePipeline::new(Arc::new(crate::generation::GenerationExecutor));
        let out = c::StageExecutor::execute(&pipeline, saved.input.clone(), &saved)
            .await
            .unwrap();
        assert!(
            out.artifacts.iter().any(|a| a.file.path == path),
            "real {path} missing from runtime pipeline"
        );
        assert_eq!(out.outcome, c::StageOutcome::NeedsRepair);
        assert!(out.library_updates.is_empty() && out.package_candidate.is_none());
    }
}
fn request() -> serde_json::Value {
    json!({"schema_version":1,"book_id":"test-book","source_id":"chapter-1","mode":"fixture","pins":{"recipe_sha256":"b1def6b774001f5c5a991c7b4a4afefc628d9aa37d746239f14d7e567fcaf209","profile_sha256":"164a7280541f54d675d85ee8fdc6febec39b226cb827e4c2aceff9b239d74fec","profile_id":"paper-player-r186-v1"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}})
}
#[tokio::test]
async fn ordinary_saved_flow_commits_each_request_phase_and_keeps_pilot_private_needs_repair() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("direction-pilot", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let rt = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(
            crate::generation::GenerationExecutor,
        ))),
    )
    .with_saved_direction_pilot(root());
    assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
    let phases = store.direction_pilot_phases(&run).unwrap();
    assert_eq!(phases.len(), 6);
    assert_eq!(phases[0].stage, c::Stage::Source);
    assert!(
        phases
            .iter()
            .skip(1)
            .all(|p| p.outcome == c::StageOutcome::NeedsRepair)
    );
    let plan = store
        .direction_pilot_artifact(&run, "pilot-review", "private/pilot-plan.json")
        .unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&plan).unwrap();
    assert_eq!(plan["takes"].as_array().unwrap().len(), 2);
    assert_eq!(plan["reviews"].as_array().unwrap().len(), 1);
    assert_eq!(plan["bulk_speech_approved"], false);
    assert_eq!(plan["production_approved"], false);
    if let Ok(path) = std::env::var("PT_DIRECTION_PILOT_RECEIPT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"schema_version":1,"run_id":run,"request":request(),"phases":phases,"provider_requests":0,"helper_calls":0,"production_approved":false,"bulk_speech_approved":false})).unwrap()).unwrap();
    }
    for phase in &phases {
        for a in &phase.outputs {
            assert!(store.artifact(&a.id).is_err());
        }
    }
    let snapshot = store.api_snapshot(&run).unwrap();
    assert!(
        snapshot
            .findings
            .iter()
            .any(|f| f.id == "retained-pilot-adverse")
    );
    assert!(
        snapshot
            .findings
            .iter()
            .any(|f| f.id == "production-verifier-pending")
    );
    assert!(
        snapshot
            .stages
            .iter()
            .find(|s| s.stage == c::Stage::Speech)
            .unwrap()
            .outcome
            .is_none()
    );
    assert!(store.catalog("test-book").unwrap().is_none());
    assert_eq!(store.snapshot(&run).unwrap()["state"], "failed");
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM provider_v2_attempts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

fn create(store: &Store, key: &str) -> String {
    store.create(key, &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn runtime(store: &Store, executor: Arc<dyn c::StageExecutor>) -> Runtime {
    Runtime::new(store.clone(), executor).with_saved_direction_pilot(root())
}
fn pipeline() -> Arc<dyn c::StageExecutor> {
    Arc::new(FixturePipeline::new(Arc::new(
        crate::generation::GenerationExecutor,
    )))
}
struct Count(std::sync::atomic::AtomicUsize);
#[async_trait::async_trait]
impl c::StageExecutor for Count {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        c::StageExecutor::execute(
            &FixturePipeline::new(Arc::new(crate::generation::GenerationExecutor)),
            input,
            services,
        )
        .await
    }
}
#[tokio::test]
async fn restart_reuses_each_committed_phase_and_replay_preserves_bytes_events_and_accounting() {
    for stop in 1..6 {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "restart");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let count = Arc::new(Count(std::sync::atomic::AtomicUsize::new(0)));
        let rt = runtime(&store, count.clone());
        rt.import_saved_direction_pilot(&run, epoch, &root())
            .unwrap();
        rt.run_saved_direction_pilot_until(&run, epoch, stop)
            .await
            .unwrap();
        drop(rt);
        drop(store);
        let store = Store::open(temp.path()).unwrap();
        store.recover().unwrap();
        let rt = Runtime::new(store.clone(), count.clone());
        assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
        assert_eq!(count.0.load(std::sync::atomic::Ordering::SeqCst), 6);
        let phases = store.direction_pilot_phases(&run).unwrap();
        assert!(phases.iter().take(stop).all(|p| p.committed_epoch == epoch));
        assert!(
            phases
                .iter()
                .skip(stop)
                .all(|p| p.original_epoch == epoch as u64 && p.committed_epoch == epoch + 1)
        );
        let before = store.journal(&run, 0, 10000).unwrap();
        let replay = rt.replay_saved_direction_pilot(&run).await.unwrap();
        for (phase, files) in replay {
            for (file, b) in files {
                assert_eq!(
                    store
                        .direction_pilot_artifact(&run, &phase, &file.path)
                        .unwrap(),
                    b
                );
            }
        }
        assert_eq!(before, store.journal(&run, 0, 10000).unwrap());
        assert!(store.catalog("test-book").unwrap().is_none());
        assert_eq!(store.db().unwrap().query_row("SELECT (SELECT COUNT(*) FROM provider_v2_attempts)+(SELECT COUNT(*) FROM provider_v2_queries)+(SELECT COUNT(*) FROM library)+(SELECT COUNT(*) FROM publications)",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}
#[tokio::test]
async fn epoch_change_inside_request_or_result_commit_rolls_back_all_phase_effects() {
    for stop in [1, 2, 3, 4, 5] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "commit-cancel");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let rt = runtime(&store, pipeline());
        rt.import_saved_direction_pilot(&run, epoch, &root())
            .unwrap();
        rt.run_saved_direction_pilot_until(&run, epoch, stop)
            .await
            .unwrap();
        store.db().unwrap().execute_batch("CREATE TRIGGER cancel_phase BEFORE UPDATE OF result ON saved_flow_phases WHEN NEW.result IS NOT NULL BEGIN UPDATE runs SET state='cancelled',epoch=epoch+1 WHERE id=NEW.run; END;").unwrap();
        assert!(
            rt.run_saved_direction_pilot_until(&run, epoch, stop + 1)
                .await
                .is_err(),
            "stale epoch committed phase {stop}"
        );
        assert_eq!(store.direction_pilot_phases(&run).unwrap().len(), stop);
        store
            .db()
            .unwrap()
            .execute_batch("DROP TRIGGER cancel_phase")
            .unwrap();
        rt.run_saved_direction_pilot_until(&run, epoch, stop + 1)
            .await
            .unwrap();
        assert_eq!(store.direction_pilot_phases(&run).unwrap().len(), stop + 1);
    }
}

fn registered_rows() -> Vec<serde_json::Value> {
    serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(root().join("fixtures/runtime/direction-pilot-v1/registration.json"))
            .unwrap(),
    )
    .unwrap()["files"]
        .as_array()
        .unwrap()
        .clone()
}
fn copied_inputs() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    for row in registered_rows() {
        let relative = row["source"].as_str().unwrap();
        let path = temp.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(root().join(relative), path).unwrap();
    }
    temp
}
#[tokio::test]
async fn changed_source_registry_catalog_recipe_observation_and_audio_fail_before_freeze() {
    for role in [
        "raw-source.txt",
        "clean-source.txt",
        "frozen-source.json",
        "private/book-registry.json",
        "private/voice-catalog.json",
        "private/direction-recipe.json",
        "private/pilot-recipe.json",
        "private/pilot-selection.json",
        "private/direction-response.json",
        "private/pilot-raw/t0.json",
        "private/pilot-audio/t0.wav",
    ] {
        let inputs = copied_inputs();
        let row = registered_rows()
            .into_iter()
            .find(|r| r["artifact"]["file"]["path"] == role)
            .unwrap();
        let path = inputs.path().join(row["source"].as_str().unwrap());
        let mut b = std::fs::read(&path).unwrap();
        b[0] ^= 1;
        std::fs::write(path, b).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "changed");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let rt = runtime(&store, pipeline());
        assert!(
            rt.import_saved_direction_pilot(&run, epoch, inputs.path())
                .is_err(),
            "accepted changed {role}"
        );
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM saved_flow_artifacts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(store.catalog("test-book").unwrap().is_none());
    }
}
#[tokio::test]
async fn frozen_inputs_survive_removed_source_but_corrupt_cas_and_checkpoints_block_restart_and_replay()
 {
    let inputs = copied_inputs();
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = create(&store, "frozen");
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(&store, pipeline());
    rt.import_saved_direction_pilot(&run, epoch, inputs.path())
        .unwrap();
    drop(inputs);
    rt.run_saved_direction_pilot_until(&run, epoch, 6)
        .await
        .unwrap();
    for row in registered_rows() {
        let file: c::FileRef = serde_json::from_value(row["artifact"]["file"].clone()).unwrap();
        let path = store.object(&file.sha256).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut corrupt = original.clone();
        corrupt[0] ^= 1;
        std::fs::write(&path, corrupt).unwrap();
        assert!(
            rt.replay_saved_direction_pilot(&run).await.is_err(),
            "corrupt {} reused",
            file.path
        );
        assert!(
            rt.run_saved_direction_pilot_until(&run, epoch, 6)
                .await
                .is_err()
        );
        std::fs::write(&path, original).unwrap();
    }
    store
        .db()
        .unwrap()
        .execute(
            "UPDATE saved_flow_phases SET result=result||' ' WHERE phase='pilot-takes'",
            [],
        )
        .unwrap();
    assert!(rt.replay_saved_direction_pilot(&run).await.is_err());
    assert!(
        rt.run_saved_direction_pilot_until(&run, epoch, 6)
            .await
            .is_err()
    );
}
struct Hold {
    ready: Arc<tokio::sync::Notify>,
}
#[async_trait::async_trait]
impl c::StageExecutor for Hold {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let out = c::StageExecutor::execute(
            &FixturePipeline::new(Arc::new(crate::generation::GenerationExecutor)),
            input,
            services,
        )
        .await?;
        self.ready.notify_one();
        std::future::pending::<()>().await;
        Ok(out)
    }
}
#[tokio::test]
async fn cancellation_before_and_during_requests_and_results_prevents_late_checkpoint() {
    for stop in [1, 2, 3, 4, 5] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "cancel");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let rt = runtime(&store, pipeline());
        rt.import_saved_direction_pilot(&run, epoch, &root())
            .unwrap();
        rt.run_saved_direction_pilot_until(&run, epoch, stop)
            .await
            .unwrap();
        let ready = Arc::new(tokio::sync::Notify::new());
        let rt = runtime(
            &store,
            Arc::new(Hold {
                ready: ready.clone(),
            }),
        );
        let worker_run = run.clone();
        let worker = tokio::spawn(async move {
            rt.run_saved_direction_pilot_until(&worker_run, epoch, stop + 1)
                .await
        });
        ready.notified().await;
        store.cancel(&run).unwrap();
        assert!(worker.await.unwrap().is_err());
        assert_eq!(store.direction_pilot_phases(&run).unwrap().len(), stop);
        assert!(
            runtime(&store, pipeline())
                .run_saved_direction_pilot_until(&run, epoch, stop + 1)
                .await
                .is_err()
        );
        assert!(store.catalog("test-book").unwrap().is_none());
    }
}
#[tokio::test]
async fn saved_pilot_never_grants_bulk_speech_and_caller_receipts_cannot_select_expected_scope() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let mut forged = request();
    forged["pins"]["recipe_sha256"] = json!(digest(b"caller-recipe"));
    assert_eq!(
        store.create("forged", &forged).unwrap_err().code,
        "invalid_source"
    );
    let run = create(&store, "scope");
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(&store, pipeline());
    rt.import_saved_direction_pilot(&run, epoch, &root())
        .unwrap();
    rt.run_saved_direction_pilot_until(&run, epoch, 6)
        .await
        .unwrap();
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Speech)
            .await
            .unwrap_err()
            .code,
        "saved_flow_scope"
    );
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Package)
            .await
            .unwrap_err()
            .code,
        "saved_flow_scope"
    );
    let mut plan: serde_json::Value = {
        let f: String = store
            .db()
            .unwrap()
            .query_row("SELECT reference FROM saved_flow_plans", [], |r| r.get(0))
            .unwrap();
        let f: c::FileRef = serde_json::from_str(&f).unwrap();
        serde_json::from_slice(&store.read_object(&f.sha256, f.bytes as usize).unwrap()).unwrap()
    };
    plan["registration"]["library_revision"] = json!(999);
    let b = serde_json::to_vec(&plan).unwrap();
    let sha = store.put(&b).unwrap();
    let f = json!({"path":"private/saved-flow-plan.json","sha256":sha,"bytes":b.len(),"media_type":"application/json"});
    store
        .db()
        .unwrap()
        .execute("UPDATE saved_flow_plans SET reference=?1", [f.to_string()])
        .unwrap();
    assert_eq!(
        rt.replay_saved_direction_pilot(&run)
            .await
            .unwrap_err()
            .code,
        "saved_flow_stale"
    );
}

struct Time;
impl super::provider_v2::Clock for Time {
    fn now_ms(&self) -> u64 {
        now() as u64
    }
}
struct PilotFake {
    sends: std::sync::atomic::AtomicUsize,
    cancel: Option<(Store, String)>,
}
#[async_trait::async_trait]
impl super::provider_v2::OfflineTransport for PilotFake {
    async fn send(
        &self,
        op: &c::ProviderOperationV2,
        b: &[u8],
    ) -> Result<super::provider_v2::Reply> {
        op.operation.payload.file.verify(b).unwrap();
        self.sends.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some((store, run)) = &self.cancel {
            store.cancel(run)?;
        }
        let mut reply = super::provider_v2::Reply::ambiguous(op);
        reply.completion = c::ProviderCompletion::Completed;
        reply.raw = Some(b"unchanged offline raw result".to_vec());
        reply.usage = c::ObservedProviderUsage {
            requests: Some(1),
            tokens: None,
            input_tokens: None,
            output_tokens: None,
            money_microusd: None,
        };
        Ok(reply)
    }
}
#[tokio::test]
async fn committed_pilot_requests_use_r17_once_only_while_bulk_and_late_acceptance_stay_denied() {
    use super::provider_v2::{Binding, Broker, Window};
    for cancel in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "pilot-r17");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let rt = runtime(&store, pipeline());
        rt.import_saved_direction_pilot(&run, epoch, &root())
            .unwrap();
        rt.run_saved_direction_pilot_until(&run, epoch, 5)
            .await
            .unwrap();
        let input = store
            .direction_pilot_phases(&run)
            .unwrap()
            .pop()
            .unwrap()
            .inputs;
        // Consume the generation proposal only as content. The grant below is test-owned.
        let p: c::ProviderProposalV2 = serde_json::from_slice(
            &store
                .direction_pilot_artifact(
                    &run,
                    "pilot-takes",
                    "private/pilot/proposals/p000-t00.json",
                )
                .unwrap(),
        )
        .unwrap();
        // Phase 4 registers the exact committed request; no saved observation is authority.
        let inputs = input;
        assert!(inputs.iter().any(|a| a.id == p.payload.id));
        let mut live = request();
        live["mode"] = json!("live");
        live["limits"] = json!({"money_microusd":1000000,"requests":100,"tokens":1000000,"repairs":10,"wall_time_ms":30000});
        store
            .db()
            .unwrap()
            .execute(
                "UPDATE runs SET request=?1 WHERE id=?2",
                rusqlite::params![live.to_string(), run],
            )
            .unwrap();
        store
            .import_accounting(&super::broker::AccountingImport {
                original_hash: digest(b"offline original"),
                derivative_hash: digest(b"offline derivative"),
                scope: "offline/pilot".into(),
                money: 0,
                requests: 0,
                tokens: 0,
                repairs: 0,
                window_start: 0,
                reset_at: 1,
                retry_at: 0,
                transformation: "Synthetic offline pilot integration; no product authorization"
                    .into(),
            })
            .unwrap();
        store
            .authorize_live(&digest(b"test-only"), 1000000)
            .unwrap();
        let broker = Broker::synthetic(store.clone(), Arc::new(Time));
        broker
            .register(&Binding {
                id: "pilot-only".into(),
                context: c::ProviderContext {
                    provider: p.provider.clone(),
                    project_id: "offline-pilot".into(),
                    credential_context_id: "offline-slot".into(),
                },
                model: p.model.clone(),
                recipe_sha256: p.recipe_sha256.clone(),
                kind: p.kind,
                recovery_estimate: c::Usage {
                    requests: 1,
                    ..Default::default()
                },
            })
            .unwrap();
        broker
            .authorize_unit(
                &run,
                epoch,
                "saved-flow-pilot",
                "pilot-only",
                "pilot-take-0",
                None,
            )
            .unwrap();
        broker
            .configure_window(&Window {
                id: "pilot-project".into(),
                scope: c::ProviderQuotaScope::Project {
                    provider: p.provider.clone(),
                    project_id: "offline-pilot".into(),
                },
                start_at_ms: now() as u64 - 1000,
                reset_at_ms: now() as u64 + 86400000,
                capacity: c::QuotaAmounts {
                    money_microusd: 1000000,
                    requests: 100,
                    tokens: 1000000,
                    repairs: 10,
                },
            })
            .unwrap();
        let fake = Arc::new(PilotFake {
            sends: std::sync::atomic::AtomicUsize::new(0),
            cancel: cancel.then(|| (store.clone(), run.clone())),
        });
        let adapter = super::provider_service::Adapter::injected(broker.clone(), fake.clone());
        assert!(
            adapter
                .issue(&run, epoch, "bulk-speech", &inputs, &p)
                .is_err()
        );
        let mut bulk = p.clone();
        bulk.payload.id = "caller-bulk-payload".into();
        assert!(
            adapter
                .issue(&run, epoch, "saved-flow-pilot", &inputs, &bulk)
                .is_err()
        );
        let services = Services::injected_saved_provider(
            store.clone(),
            run.clone(),
            epoch,
            "saved-flow-pilot".into(),
            inputs.clone(),
            adapter.clone(),
        );
        let issued = c::RunServices::issue_provider_v2(&services, p.clone())
            .await
            .unwrap();
        let result = c::RunServices::execute_provider_v2(&services, &issued).await;
        if cancel {
            assert!(matches!(result, Err(c::ServiceError::Cancelled)));
        } else {
            let result = result.unwrap();
            result.validate_for_operation(&issued).unwrap();
            let again = c::RunServices::execute_provider_v2(&services, &issued)
                .await
                .unwrap();
            assert_eq!(
                serde_json::to_value(result).unwrap(),
                serde_json::to_value(again).unwrap()
            );
        }
        assert_eq!(fake.sends.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            broker
                .quota_snapshot("pilot-project")
                .unwrap()
                .accounted
                .requests,
            1
        );
        assert!(store.catalog("test-book").unwrap().is_none());
        assert!(store.artifact(&p.payload.id).is_err());
        let raw: String = store
            .db()
            .unwrap()
            .query_row("SELECT result FROM provider_v2_attempts", [], |r| r.get(0))
            .unwrap();
        let persisted: c::ProviderResultV2 = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            persisted.epoch_disposition,
            if cancel {
                c::ProviderEpochDisposition::Obsolete
            } else {
                c::ProviderEpochDisposition::Current
            }
        );
        assert_eq!(
            persisted.accounted_usage.tokens,
            p.estimate.input_tokens + p.estimate.output_tokens
        );
        let raw = persisted.result.raw_response.unwrap();
        assert_eq!(
            store
                .read_object(&raw.file.sha256, raw.file.bytes as usize)
                .unwrap(),
            b"unchanged offline raw result"
        );
        assert!(store.artifact(&raw.id).is_err());
        let original = store
            .db()
            .unwrap()
            .query_row("SELECT descriptor FROM provider_v2_attempts", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&original).unwrap(),
            serde_json::to_value(&issued).unwrap()
        );
    }
}

#[tokio::test]
async fn two_phase_mapping_preserves_original_receipts_and_only_registers_committed_requests() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = create(&store, "mapping");
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(&store, pipeline());
    rt.import_saved_direction_pilot(&run, epoch, &root())
        .unwrap();
    rt.run_saved_direction_pilot_until(&run, epoch, 6)
        .await
        .unwrap();
    let phases = store.direction_pilot_phases(&run).unwrap();
    for (request_phase, result_phase, paths) in [
        (1, 2, vec!["private/direction-request.json"]),
        (
            3,
            4,
            vec![
                "private/pilot/requests/p000-t00.json",
                "private/pilot/requests/p000-t01.json",
            ],
        ),
        (4, 5, vec!["private/pilot/requests/p000-review.json"]),
    ] {
        for path in paths {
            assert!(
                !phases[request_phase]
                    .inputs
                    .iter()
                    .any(|a| a.file.path == path)
            );
            let (original, current) = phases[request_phase]
                .original_mappings
                .iter()
                .find(|(a, _)| a.file.path == path)
                .unwrap();
            assert_eq!(original.file, current.file);
            assert_ne!(original.id, current.id);
            assert_eq!(original.provenance, c::Provenance::Synthetic);
            assert_eq!(current.provenance, c::Provenance::Diagnostic);
            assert_eq!(
                serde_json::to_value(
                    phases[result_phase]
                        .inputs
                        .iter()
                        .find(|a| a.file.path == path)
                        .unwrap()
                )
                .unwrap(),
                serde_json::to_value(original).unwrap()
            );
        }
    }
    for row in registered_rows() {
        let original: c::ArtifactRef = serde_json::from_value(row["artifact"].clone()).unwrap();
        if let Some(a) = phases[5].inputs.iter().find(|a| a.id == original.id) {
            assert_eq!(
                serde_json::to_value(a).unwrap(),
                serde_json::to_value(&original).unwrap()
            );
            assert_eq!(
                store
                    .read_object(&a.file.sha256, a.file.bytes as usize)
                    .unwrap(),
                std::fs::read(root().join(row["source"].as_str().unwrap())).unwrap()
            );
        }
    }
}
struct ChangeConsumed {
    store: Store,
}
#[async_trait::async_trait]
impl c::StageExecutor for ChangeConsumed {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let out = c::StageExecutor::execute(
            &FixturePipeline::new(Arc::new(crate::generation::GenerationExecutor)),
            input.clone(),
            services,
        )
        .await?;
        let path = self.store.object(&input.inputs[0].file.sha256).unwrap();
        let mut b = std::fs::read(&path).unwrap();
        b[0] ^= 1;
        std::fs::write(path, b).unwrap();
        Ok(out)
    }
}
#[tokio::test]
async fn changed_consumed_bytes_and_failed_checkpoint_transaction_leave_no_accepted_result() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = create(&store, "rollback");
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(&store, pipeline());
    rt.import_saved_direction_pilot(&run, epoch, &root())
        .unwrap();
    rt.run_saved_direction_pilot_until(&run, epoch, 1)
        .await
        .unwrap();
    let rows_before: i64 = store
        .db()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM saved_flow_artifacts", [], |r| {
            r.get(0)
        })
        .unwrap();
    let events_before = store.journal(&run, 0, 10000).unwrap();
    store.db().unwrap().execute_batch("CREATE TRIGGER reject_phase BEFORE UPDATE OF result ON saved_flow_phases BEGIN SELECT RAISE(ABORT,'injected checkpoint failure'); END").unwrap();
    assert!(
        rt.run_saved_direction_pilot_until(&run, epoch, 2)
            .await
            .is_err()
    );
    assert_eq!(store.direction_pilot_phases(&run).unwrap().len(), 1);
    assert_eq!(
        rows_before,
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM saved_flow_artifacts", [], |r| r
                .get::<_, i64>(0))
            .unwrap()
    );
    assert_eq!(
        store
            .journal(&run, 0, 10000)
            .unwrap()
            .iter()
            .filter(|e| e["type"] == "saved_flow_phase_checkpoint")
            .count(),
        events_before
            .iter()
            .filter(|e| e["type"] == "saved_flow_phase_checkpoint")
            .count()
    );
    store
        .db()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_phase")
        .unwrap();
    let bad = runtime(
        &store,
        Arc::new(ChangeConsumed {
            store: store.clone(),
        }),
    );
    assert!(
        bad.run_saved_direction_pilot_until(&run, epoch, 2)
            .await
            .is_err()
    );
    assert!(
        store
            .direction_pilot_artifact(&run, "direction-request", "private/direction-request.json")
            .is_err()
    );
    assert!(store.catalog("test-book").unwrap().is_none());
}

struct Together(Arc<tokio::sync::Barrier>);
#[async_trait::async_trait]
impl c::StageExecutor for Together {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let out = c::StageExecutor::execute(
            &FixturePipeline::new(Arc::new(crate::generation::GenerationExecutor)),
            input,
            services,
        )
        .await?;
        self.0.wait().await;
        Ok(out)
    }
}
#[tokio::test]
async fn concurrent_phase_completion_cannot_duplicate_the_atomic_checkpoint_or_findings() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = create(&store, "once");
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(&store, pipeline());
    rt.import_saved_direction_pilot(&run, epoch, &root())
        .unwrap();
    rt.run_saved_direction_pilot_until(&run, epoch, 1)
        .await
        .unwrap();
    let both = runtime(
        &store,
        Arc::new(Together(Arc::new(tokio::sync::Barrier::new(2)))),
    );
    let (a, b) = tokio::join!(
        both.run_saved_direction_pilot_until(&run, epoch, 2),
        both.run_saved_direction_pilot_until(&run, epoch, 2)
    );
    assert!(a.is_ok() || b.is_ok());
    assert_eq!(
        store
            .journal(&run, 0, 10000)
            .unwrap()
            .iter()
            .filter(|e| e["type"] == "saved_flow_phase_checkpoint"
                && e["data"]["phase"] == "direction-request")
            .count(),
        1
    );
}
