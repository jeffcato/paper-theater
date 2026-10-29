use super::*;
use pt_contracts as c;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
struct Saved {
    bytes: BTreeMap<String, Vec<u8>>,
    candidates: Mutex<BTreeMap<String, Vec<u8>>>,
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
            .insert(a.candidate_id.clone(), b);
        Ok(a)
    }
    async fn check_cancelled(&self) -> std::result::Result<(), c::ServiceError> {
        Ok(())
    }
    async fn provider(
        &self,
        _: c::ProviderOperation,
    ) -> std::result::Result<c::ProviderResult, c::ServiceError> {
        panic!("zero provider calls")
    }
    async fn helper(
        &self,
        _: c::HelperOperation,
    ) -> std::result::Result<c::HelperResult, c::ServiceError> {
        panic!("zero helper calls")
    }
    async fn emit(&self, _: c::JournalProposal) -> std::result::Result<(), c::ServiceError> {
        panic!("no journal authority")
    }
    async fn library(
        &self,
        _: c::LibraryQuery,
    ) -> std::result::Result<Vec<c::LibraryBinding>, c::ServiceError> {
        panic!("no library authority")
    }
}
#[tokio::test]
async fn runtime_routes_pinned_saved_speech_to_g24_without_provider_authority() {
    let path = root().join("fixtures/generation/speech-v1/recorded-v2");
    let rows: Vec<Value> =
        serde_json::from_slice(&std::fs::read(path.join("input-index.json")).unwrap()).unwrap();
    let saved = Saved {
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
    };
    let input =
        serde_json::from_slice(&std::fs::read(path.join("stage-input.json")).unwrap()).unwrap();
    let out = c::StageExecutor::execute(
        &FixturePipeline::new(Arc::new(crate::generation::GenerationExecutor)),
        input,
        &saved,
    )
    .await
    .unwrap();
    assert!(
        out.artifacts
            .iter()
            .any(|a| a.file.path == "private/speech-takes.json")
    );
    assert_eq!(out.outcome, c::StageOutcome::NeedsRepair);
    assert!(out.package_candidate.is_none() && out.library_updates.is_empty());
}
fn registration(case: &str) -> Value {
    let r: Value = serde_json::from_slice(include_bytes!(
        "../../../../fixtures/runtime/speech-v1/registration.json"
    ))
    .unwrap();
    r["variants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["case"] == case)
        .unwrap()
        .clone()
}
fn request(case: &str) -> Value {
    let r = registration(case);
    let mut pins = r["original_input"]["pins"].clone();
    pins["recipe_sha256"] = r["recipe_sha256"].clone();
    json!({"schema_version":1,"book_id":"test-book","source_id":"chapter-1","mode":"fixture","pins":pins,"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}})
}
fn pipeline() -> Arc<dyn c::StageExecutor> {
    Arc::new(FixturePipeline::new(Arc::new(
        crate::generation::GenerationExecutor,
    )))
}
#[tokio::test]
async fn saved_speech_commits_requests_before_registration_then_ingests_original_takes() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store
        .create("speech-phases", &request("recorded-v2"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = Runtime::new(store.clone(), pipeline());
    rt.import_saved_speech(&run, epoch, &root()).unwrap();
    rt.run_saved_speech_until(&run, epoch, 1).await.unwrap();
    let p = store.speech_phases(&run).unwrap();
    assert_eq!(p.len(), 1);
    assert!(
        !p[0]
            .artifacts
            .iter()
            .any(|a| a.file.path.contains("/proposals/"))
    );
    rt.run_saved_speech_until(&run, epoch, 2).await.unwrap();
    let p = store.speech_phases(&run).unwrap();
    assert_eq!(
        p[1].artifacts
            .iter()
            .filter(|a| a.file.path.contains("/proposals/"))
            .count(),
        2
    );
    let takes: Value = serde_json::from_slice(
        &store
            .speech_artifact(&run, "registered", "private/speech-takes.json")
            .unwrap(),
    )
    .unwrap();
    assert!(takes["takes"].as_array().unwrap().is_empty());
    rt.run_saved_speech_until(&run, epoch, 3).await.unwrap();
    let p = store.speech_phases(&run).unwrap();
    assert_eq!(p.len(), 3);
    assert!(
        p.iter()
            .all(|p| p.output.outcome == c::StageOutcome::NeedsRepair)
    );
    let takes: Value = serde_json::from_slice(
        &store
            .speech_artifact(&run, "takes", "private/speech-takes.json")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(takes["takes"].as_array().unwrap().len(), 2);
    assert_eq!(
        takes["observations"][0]["record"]["issued"]["identity"]["run_id"],
        "offline-run"
    );
    assert_eq!(
        takes["observations"][0]["record"]["issued"]["identity"]["epoch"],
        1
    );
    assert_eq!(takes["production_approved"], false);
    for p in p {
        for a in p.artifacts {
            assert!(store.artifact(&a.id).is_err());
        }
    }
    assert!(store.catalog("test-book").unwrap().is_none());
}
#[tokio::test]
async fn ordinary_saved_speech_materializes_private_verified_audio_without_enabling_bulk() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store
        .create("ordinary-speech", &request("recorded-v2"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let rt = Runtime::new(store.clone(), pipeline()).with_saved_speech_inputs(root());
    assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
    assert_eq!(store.speech_phases(&run).unwrap().len(), 3);
    let batches = store.speech_batches(&run).unwrap();
    assert_eq!(batches.len(), 1);
    let bytes = store.speech_batch_audio(&run, "b0000").unwrap();
    let info = c::inspect_pcm16_wav(&mut std::io::Cursor::new(&bytes), bytes.len() as u64).unwrap();
    assert_eq!(info.frame_count, 4801);
    if let Ok(path) = std::env::var("PT_SPEECH_RUNTIME_RECEIPT") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"schema_version":1,"case":"recorded-v2","request":request("recorded-v2"),"phases":store.speech_phases(&run).unwrap(),"batches":batches,"provider_calls":0,"helper_calls":0,"bulk_speech_approved":false,"production_approved":false})).unwrap()).unwrap();
    }
    assert_eq!(batches[0].receipt["model_payload"]["mode"], "VERBATIM");
    assert_eq!(
        batches[0].receipt["model_payload"]
            .as_object()
            .unwrap()
            .len(),
        4
    );
    assert!(store.artifact(&batches[0].audio.id).is_err());
    assert!(store.catalog("test-book").unwrap().is_none());
    assert_eq!(store.db().unwrap().query_row("SELECT (SELECT COUNT(*) FROM attempts)+(SELECT COUNT(*) FROM provider_v2_attempts)+(SELECT COUNT(*) FROM library)+(SELECT COUNT(*) FROM catalog)",[],|r|r.get::<_,i64>(0)).unwrap(),0);
}
#[tokio::test]
async fn changed_prepared_scope_cannot_be_hidden_by_a_completed_speech_checkpoint() {
    let t = tempfile::tempdir().unwrap();
    let store = Store::open(t.path()).unwrap();
    let run = store
        .create("scope-tamper", &request("recorded-v2"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let rt = Runtime::new(store.clone(), pipeline()).with_saved_speech_inputs(root());
    assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
    store
        .db()
        .unwrap()
        .execute(
            "UPDATE speech_phases SET input=input||' ' WHERE ordinal=0",
            [],
        )
        .unwrap();
    assert!(rt.replay_saved_speech(&run).await.is_err());
}
struct Count(std::sync::atomic::AtomicUsize);
#[async_trait::async_trait]
impl c::StageExecutor for Count {
    async fn execute(
        &self,
        i: c::StageInput,
        s: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        c::StageExecutor::execute(&*pipeline(), i, s).await
    }
}
fn create(store: &Store, key: &str, case: &str) -> String {
    store.create(key, &request(case)).unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
#[tokio::test]
async fn every_speech_boundary_restarts_without_generation_redispatch_and_replays_exact_bytes() {
    for stop in 1..=3 {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "restart", "recorded-v2");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let count = Arc::new(Count(std::sync::atomic::AtomicUsize::new(0)));
        let rt = Runtime::new(store.clone(), count.clone());
        rt.import_saved_speech(&run, epoch, &root()).unwrap();
        rt.run_saved_speech_until(&run, epoch, stop).await.unwrap();
        drop(rt);
        drop(store);
        let store = Store::open(temp.path()).unwrap();
        store.recover().unwrap();
        let rt = Runtime::new(store.clone(), count.clone());
        assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
        assert_eq!(count.0.load(std::sync::atomic::Ordering::SeqCst), 3);
        let phases = store.speech_phases(&run).unwrap();
        assert!(phases.iter().take(stop).all(|p| p.committed_epoch == epoch));
        assert!(
            phases
                .iter()
                .skip(stop)
                .all(|p| p.committed_epoch == epoch + 1)
        );
        assert!(
            phases
                .iter()
                .all(|p| p.input.run_id == "offline-run" && p.input.epoch == 1)
        );
        let before = store.journal(&run, 0, 10000).unwrap();
        let objects = std::fs::read_dir(temp.path().join("objects"))
            .unwrap()
            .count();
        let batches = store.speech_batches(&run).unwrap();
        let replay = rt.replay_saved_speech(&run).await.unwrap();
        for (phase, files) in replay {
            for (f, b) in files {
                if phase == "pcm" {
                    let rec = batches
                        .iter()
                        .find(|v| v.audio.file == f || v.receipt_file.file == f)
                        .unwrap();
                    if rec.audio.file == f {
                        assert_eq!(store.speech_batch_audio(&run, &rec.id).unwrap(), b);
                    } else {
                        assert_eq!(serde_json::to_vec_pretty(&rec.receipt).unwrap(), b);
                    }
                } else {
                    assert_eq!(store.speech_artifact(&run, &phase, &f.path).unwrap(), b);
                }
            }
        }
        assert_eq!(before, store.journal(&run, 0, 10000).unwrap());
        assert_eq!(
            objects,
            std::fs::read_dir(temp.path().join("objects"))
                .unwrap()
                .count()
        );
        assert!(store.catalog("test-book").unwrap().is_none());
    }
}
#[tokio::test]
async fn pcm_checkpoint_restart_reuses_committed_batch_and_preserves_unplanned_and_mixed_domains() {
    for (case, n) in [
        ("bounded-transcription", 11),
        ("mixed-native", 2),
        ("unplanned", 1),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "batch-restart", case);
        let (_, epoch) = store.claim().unwrap().unwrap();
        let count = Arc::new(Count(std::sync::atomic::AtomicUsize::new(0)));
        let rt = Runtime::new(store.clone(), count.clone());
        rt.import_saved_speech(&run, epoch, &root()).unwrap();
        rt.run_saved_speech_until(&run, epoch, 3).await.unwrap();
        if n > 1 {
            store.db().unwrap().execute_batch("CREATE TRIGGER stop_copy BEFORE INSERT ON speech_batches WHEN NEW.id='b0001' BEGIN SELECT RAISE(ABORT,'injected copy checkpoint interruption'); END").unwrap();
            assert!(rt.materialize_speech_batches(&run, epoch).await.is_err());
            assert_eq!(store.speech_batches(&run).unwrap().len(), 1);
            store
                .db()
                .unwrap()
                .execute_batch("DROP TRIGGER stop_copy")
                .unwrap();
        } else {
            rt.materialize_speech_batches(&run, epoch).await.unwrap();
        }
        let old = serde_json::to_value(store.speech_batches(&run).unwrap()[0].clone()).unwrap();
        let store = Store::open(temp.path()).unwrap();
        store.recover().unwrap();
        let rt = Runtime::new(store.clone(), count.clone());
        assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
        assert_eq!(count.0.load(std::sync::atomic::Ordering::SeqCst), 3);
        let batches = store.speech_batches(&run).unwrap();
        assert_eq!(batches.len(), n);
        assert_eq!(serde_json::to_value(&batches[0]).unwrap(), old);
        let events = store.journal(&run, 0, 10000).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| e["type"] == "speech_pcm_checkpoint")
                .count(),
            n
        );
        for b in &batches {
            let audio = store.speech_batch_audio(&run, &b.id).unwrap();
            let info = c::inspect_pcm16_wav(&mut std::io::Cursor::new(&audio), audio.len() as u64)
                .unwrap();
            if case == "bounded-transcription" {
                assert!(audio.len() <= 1000);
                assert!(
                    b.receipt["segments"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|s| s["cross_cut_uncertainty"] == true)
                );
            }
            assert_eq!(b.receipt["format"]["frames"], info.frame_count);
        }
        if case == "unplanned" {
            assert_eq!(
                batches[0].receipt["unplanned_audio"]
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
        }
        if case == "mixed-native" {
            assert_ne!(
                batches[0].receipt["format"]["sample_rate"],
                batches[1].receipt["format"]["sample_rate"]
            );
        }
        rt.replay_saved_speech(&run).await.unwrap();
        if let Ok(directory) = std::env::var("PT_SPEECH_BATCH_RECEIPTS_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(std::path::Path::new(&directory).join(format!("{case}.json")),serde_json::to_vec_pretty(&json!({"schema_version":1,"case":case,"batches":batches,"provider_calls":0,"helper_calls":0})).unwrap()).unwrap();
        }
    }
}
fn copied_inputs(case: &str) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let r = registration(case);
    for row in r["files"]
        .as_array()
        .unwrap()
        .iter()
        .chain(std::iter::once(&r["original_file"]))
    {
        let relative = row["source"].as_str().unwrap();
        let path = t.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(root().join(relative), path).unwrap();
    }
    t
}
#[tokio::test]
async fn changed_source_pilot_binding_recipe_raw_audio_and_original_scope_cannot_be_imported() {
    let r = registration("recorded-v2");
    for path in [
        "clean-source.txt",
        "frozen-source.json",
        "private/pilot-plan.json",
        "private/speech-pilot-input.json",
        "private/speech-bindings.json",
        "private/speech-recipe.json",
        "private/speech-raw/u0000.json",
        "private/speech-audio/u0000.wav",
        "private/speech-observations/u0000.json",
        "private/speech/requests/u0000.json",
    ] {
        let inputs = copied_inputs("recorded-v2");
        let row = r["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["artifact"]["file"]["path"] == path)
            .unwrap();
        let file = inputs.path().join(row["source"].as_str().unwrap());
        let mut b = std::fs::read(&file).unwrap();
        b[0] ^= 1;
        std::fs::write(file, b).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = create(&store, "changed", "recorded-v2");
        let (_, epoch) = store.claim().unwrap().unwrap();
        assert!(
            Runtime::new(store.clone(), pipeline())
                .import_saved_speech(&run, epoch, inputs.path())
                .is_err(),
            "accepted changed {path}"
        );
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM speech_refs", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[tokio::test]
async fn removed_producer_inputs_reuse_frozen_bytes_but_corrupt_sources_and_pcm_block_replay() {
    let inputs = copied_inputs("recorded-v2");
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = create(&store, "corrupt", "recorded-v2");
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = Runtime::new(store.clone(), pipeline());
    rt.import_saved_speech(&run, epoch, inputs.path()).unwrap();
    drop(inputs);
    rt.run_saved_speech_until(&run, epoch, 3).await.unwrap();
    rt.materialize_speech_batches(&run, epoch).await.unwrap();
    let before = store.journal(&run, 0, 10000).unwrap();
    let r = registration("recorded-v2");
    for row in r["files"].as_array().unwrap() {
        let f: c::FileRef = serde_json::from_value(row["artifact"]["file"].clone()).unwrap();
        let path = store.object(&f.sha256).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut b = original.clone();
        b[0] ^= 1;
        std::fs::write(&path, b).unwrap();
        assert!(
            rt.replay_saved_speech(&run).await.is_err(),
            "corrupt {} reused",
            f.path
        );
        assert!(rt.run_saved_speech_until(&run, epoch, 3).await.is_err());
        std::fs::write(path, original).unwrap();
    }
    let batch = store.speech_batches(&run).unwrap().pop().unwrap();
    let path = store.object(&batch.audio.file.sha256).unwrap();
    let original = std::fs::read(&path).unwrap();
    let mut b = original.clone();
    b[44] ^= 1;
    std::fs::write(&path, b).unwrap();
    assert!(rt.materialize_speech_batches(&run, epoch).await.is_err());
    assert!(rt.replay_saved_speech(&run).await.is_err());
    std::fs::write(path, original).unwrap();
    assert_eq!(before, store.journal(&run, 0, 10000).unwrap());
    rt.replay_saved_speech(&run).await.unwrap();
}
struct Hold(Arc<tokio::sync::Notify>);
#[async_trait::async_trait]
impl c::StageExecutor for Hold {
    async fn execute(
        &self,
        i: c::StageInput,
        s: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let out = c::StageExecutor::execute(&*pipeline(), i, s).await?;
        self.0.notify_one();
        std::future::pending::<()>().await;
        Ok(out)
    }
}
#[tokio::test]
async fn cancellation_before_and_after_generation_prevents_request_and_take_commit() {
    for stop in 0..3 {
        let t = tempfile::tempdir().unwrap();
        let store = Store::open(t.path()).unwrap();
        let run = create(&store, "cancel", "recorded-v2");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let rt = Runtime::new(store.clone(), pipeline());
        rt.import_saved_speech(&run, epoch, &root()).unwrap();
        rt.run_saved_speech_until(&run, epoch, stop).await.unwrap();
        let ready = Arc::new(tokio::sync::Notify::new());
        let rt = Runtime::new(store.clone(), Arc::new(Hold(ready.clone())));
        let worker_run = run.clone();
        let worker = tokio::spawn(async move {
            rt.run_saved_speech_until(&worker_run, epoch, stop + 1)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(30), ready.notified())
            .await
            .unwrap();
        store.cancel(&run).unwrap();
        assert!(worker.await.unwrap().is_err());
        assert_eq!(store.speech_phases(&run).unwrap().len(), stop);
        assert!(
            Runtime::new(store.clone(), pipeline())
                .run_saved_speech_until(&run, epoch, stop + 1)
                .await
                .is_err()
        );
    }
}
#[tokio::test]
async fn stale_commit_and_sql_failure_roll_back_outputs_findings_and_copy_receipts() {
    for (kind, stop) in [("request", 0), ("registered", 1), ("takes", 2), ("pcm", 3)] {
        let t = tempfile::tempdir().unwrap();
        let store = Store::open(t.path()).unwrap();
        let run = create(&store, "atomic", "recorded-v2");
        let (_, epoch) = store.claim().unwrap().unwrap();
        let rt = Runtime::new(store.clone(), pipeline());
        rt.import_saved_speech(&run, epoch, &root()).unwrap();
        rt.run_saved_speech_until(&run, epoch, stop).await.unwrap();
        let trigger = if stop == 3 {
            "CREATE TRIGGER invalidate BEFORE INSERT ON speech_batches BEGIN UPDATE runs SET state='cancelled',epoch=epoch+1 WHERE id=NEW.run; END"
        } else {
            "CREATE TRIGGER invalidate BEFORE UPDATE OF result ON speech_phases BEGIN UPDATE runs SET state='cancelled',epoch=epoch+1 WHERE id=NEW.run; END"
        };
        store.db().unwrap().execute_batch(trigger).unwrap();
        let result = if stop == 3 {
            rt.materialize_speech_batches(&run, epoch).await
        } else {
            rt.run_saved_speech_until(&run, epoch, stop + 1).await
        };
        assert!(result.is_err(), "stale {kind} committed");
        assert_eq!(store.speech_phases(&run).unwrap().len(), stop);
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM speech_batches", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        store
            .db()
            .unwrap()
            .execute_batch("DROP TRIGGER invalidate")
            .unwrap();
        if stop == 3 {
            store.db().unwrap().execute_batch("CREATE TRIGGER reject_copy BEFORE INSERT ON speech_batches BEGIN SELECT RAISE(ABORT,'copy failure'); END").unwrap();
            let before = store.journal(&run, 0, 10000).unwrap();
            assert!(rt.materialize_speech_batches(&run, epoch).await.is_err());
            assert_eq!(before, store.journal(&run, 0, 10000).unwrap());
            store
                .db()
                .unwrap()
                .execute_batch("DROP TRIGGER reject_copy")
                .unwrap();
            rt.materialize_speech_batches(&run, epoch).await.unwrap();
        } else {
            rt.run_saved_speech_until(&run, epoch, stop + 1)
                .await
                .unwrap();
        }
    }
}
struct Together(Arc<tokio::sync::Barrier>);
#[async_trait::async_trait]
impl c::StageExecutor for Together {
    async fn execute(
        &self,
        i: c::StageInput,
        s: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let out = c::StageExecutor::execute(&*pipeline(), i, s).await?;
        self.0.wait().await;
        Ok(out)
    }
}
#[tokio::test]
async fn concurrent_completions_cannot_duplicate_request_findings_or_pcm_checkpoints() {
    let t = tempfile::tempdir().unwrap();
    let store = Store::open(t.path()).unwrap();
    let run = create(&store, "concurrent", "recorded-v2");
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = Runtime::new(store.clone(), pipeline());
    rt.import_saved_speech(&run, epoch, &root()).unwrap();
    let both = Runtime::new(
        store.clone(),
        Arc::new(Together(Arc::new(tokio::sync::Barrier::new(2)))),
    );
    let (a, b) = tokio::join!(
        both.run_saved_speech_until(&run, epoch, 1),
        both.run_saved_speech_until(&run, epoch, 1)
    );
    assert!(a.is_ok() || b.is_ok());
    assert_eq!(store.speech_phases(&run).unwrap().len(), 1);
    assert_eq!(
        store
            .journal(&run, 0, 10000)
            .unwrap()
            .iter()
            .filter(|e| e["type"] == "speech_phase_checkpoint")
            .count(),
        1
    );
    rt.run_saved_speech_until(&run, epoch, 3).await.unwrap();
    let (a, b) = tokio::join!(
        rt.materialize_speech_batches(&run, epoch),
        rt.materialize_speech_batches(&run, epoch)
    );
    assert!(a.is_ok() || b.is_ok());
    assert_eq!(store.speech_batches(&run).unwrap().len(), 1);
    assert_eq!(
        store
            .journal(&run, 0, 10000)
            .unwrap()
            .iter()
            .filter(|e| e["type"] == "speech_pcm_checkpoint")
            .count(),
        1
    );
}
#[tokio::test]
async fn r18_stops_before_speech_and_saved_speech_has_no_generic_bulk_or_publication_path() {
    let t = tempfile::tempdir().unwrap();
    let store = Store::open(t.path()).unwrap();
    let mut original = request("recorded-v2");
    original["pins"]["recipe_sha256"] =
        json!("b1def6b774001f5c5a991c7b4a4afefc628d9aa37d746239f14d7e567fcaf209");
    let run = store.create("r18-original", &original).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let rt = Runtime::new(store.clone(), pipeline()).with_saved_direction_pilot(root());
    assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
    assert_eq!(store.direction_pilot_phases(&run).unwrap().len(), 6);
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM speech_flows", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let run = create(&store, "no-bulk", "recorded-v2");
    let (_, epoch) = store.claim().unwrap().unwrap();
    for stage in [c::Stage::Speech, c::Stage::Acoustic, c::Stage::Package] {
        assert_eq!(
            rt.execute_stage(&run, epoch, stage).await.unwrap_err().code,
            "saved_flow_scope"
        );
    }
    let mut forged = request("recorded-v2");
    forged["mode"] = json!("live");
    assert!(store.create("live", &forged).is_err());
    forged["mode"] = json!("fixture");
    forged["pins"]["recipe_sha256"] = json!(digest(b"caller bulk authority"));
    assert_eq!(
        store.create("forged", &forged).unwrap_err().code,
        "invalid_source"
    );
}
