use super::*;
use pt_contracts as c;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn revision() -> String {
    digest(
        &[
            include_bytes!("../generation/chapter_import.rs").as_slice(),
            include_bytes!("../generation/mod.rs"),
            include_bytes!("../generation/inputs.rs"),
            c::acceptance_binding_checker().sha256.as_bytes(),
        ]
        .concat(),
    )
}
fn request() -> serde_json::Value {
    json!({"schema_version":1,"book_id":"pg-55","source_id":"pg-55-chapter-1","mode":"fixture","pins":{"recipe_sha256":"76e5cb432d7e3a1a91b1e59a6e27963e4bdf0c1e531f26d5368dda476132d598","profile_sha256":digest(b"diagnostic-profile-unavailable"),"profile_id":"chapter-diagnostic"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}})
}
fn runtime(store: Store) -> Runtime {
    Runtime::new(
        store,
        Arc::new(FixturePipeline::new(Arc::new(
            crate::generation::GenerationExecutor,
        ))),
    )
    .with_chapter_helper(ChapterHelper::new(root().join("tools/generation"), revision()).unwrap())
}
#[tokio::test]
async fn verified_bundle_runs_real_registered_helper_and_checkpoints_private_needs_repair() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-real", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(store.clone());
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    let units = store.chapter_units(&run).unwrap();
    assert_eq!(units.len(), 4);
    assert!(units.iter().all(|u| u.outcome.is_none()));
    assert_eq!(
        rt.execute_chapter_unit(&run, epoch, "chunk-0000")
            .await
            .unwrap(),
        c::StageOutcome::NeedsRepair
    );
    let units = store.chapter_units(&run).unwrap();
    let unit = &units[0];
    assert_eq!(unit.outcome, Some(c::StageOutcome::NeedsRepair));
    assert_eq!(unit.outputs.len(), 3);
    for a in &unit.outputs {
        assert!(
            store.artifact(&a.id).is_err(),
            "chapter metadata must stay outside the HTTP artifact allowlist"
        );
    }
    let report = store
        .chapter_artifact(&run, "chunk-0000", "private/chapter-import-validation.json")
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&report).unwrap();
    assert_eq!(report["verified_import"]["complete_source_coverage"], true);
    assert_eq!(report["production_approved"], false);
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM attempts WHERE scope='offline/product/chapter-import-v2'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    assert!(
        store
            .api_snapshot(&run)
            .unwrap()
            .findings
            .iter()
            .any(|f| f.id == "production-verifier-pending")
    );
}
struct Audit(Arc<std::sync::Mutex<Vec<c::StageInput>>>);
#[async_trait::async_trait]
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
#[tokio::test]
async fn distinct_units_restart_and_replay_exact_private_outputs_without_redispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-restart", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let rt = Runtime::new(store.clone(), Arc::new(Audit(seen.clone()))).with_chapter_helper(
        ChapterHelper::new(root().join("tools/generation"), revision()).unwrap(),
    );
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    assert_eq!(
        seen.lock().unwrap().len(),
        4,
        "every original unit record must be independently reconstructed before import"
    );
    rt.execute_chapter_unit(&run, epoch, "chunk-0000")
        .await
        .unwrap();
    let first = store.chapter_units(&run).unwrap()[0].clone();
    drop(rt);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = Runtime::new(store.clone(), Arc::new(Audit(seen.clone()))).with_chapter_helper(
        ChapterHelper::new(root().join("tools/generation"), revision()).unwrap(),
    );
    rt.import_chapter_bundle(
        &run,
        epoch,
        &temp.path().join("deleted-original"),
        "chapter-evidence.json",
        &temp.path().join("deleted-bundle"),
    )
    .await
    .unwrap();
    rt.execute_chapter_unit(&run, epoch, "chunk-0000")
        .await
        .unwrap();
    assert_eq!(seen.lock().unwrap().len(), 5);
    for label in ["chunk-0001", "chunk-0002", "chunk-0003"] {
        rt.execute_chapter_unit(&run, epoch, label).await.unwrap();
    }
    let units = store.chapter_units(&run).unwrap();
    assert_eq!(units[0].unit_key, first.unit_key);
    assert_eq!(units[0].outputs[0].id, first.outputs[0].id);
    assert_eq!(
        units
            .iter()
            .map(|u| &u.unit_key)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    store
        .finish(&run, epoch, "failed", Some("stage_needs_repair"))
        .unwrap();
    let before = store.snapshot(&run).unwrap();
    let seq = before["last_event_seq"].as_i64().unwrap();
    let db = std::fs::read(temp.path().join("runtime.sqlite3")).unwrap();
    for u in &units {
        let replay = rt.replay_chapter_unit(&run, &u.label).await.unwrap();
        assert_eq!(replay.len(), 3);
        for (file, bytes) in replay {
            assert_eq!(
                bytes,
                store.chapter_artifact(&run, &u.label, &file.path).unwrap()
            );
            file.verify(&bytes).unwrap();
        }
    }
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert!(store.journal(&run, seq, 500).unwrap().is_empty());
    assert_eq!(
        std::fs::read(temp.path().join("runtime.sqlite3")).unwrap(),
        db
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 12);
    assert!(
        seen.iter()
            .skip(8)
            .all(|i| i.mode == c::ExecutionMode::Replay && i.inputs.len() <= 64)
    );
}
struct HoldAfterGeneration(Arc<tokio::sync::Notify>);
#[async_trait::async_trait]
impl c::StageExecutor for HoldAfterGeneration {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let cold = input.mode == c::ExecutionMode::Fixture;
        let out =
            c::StageExecutor::execute(&crate::generation::GenerationExecutor, input, services)
                .await?;
        if cold {
            self.0.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(out)
    }
}
#[tokio::test]
async fn cancellation_after_helper_and_generation_unwinds_without_a_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-cancel-commit", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let ready = Arc::new(tokio::sync::Notify::new());
    let rt = Runtime::new(store.clone(), Arc::new(HoldAfterGeneration(ready.clone())))
        .with_chapter_helper(
            ChapterHelper::new(root().join("tools/generation"), revision()).unwrap(),
        );
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    let id = run.clone();
    let worker =
        tokio::spawn(async move { rt.execute_chapter_unit(&id, epoch, "chunk-0000").await });
    tokio::time::timeout(std::time::Duration::from_secs(10), ready.notified())
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
    assert!(
        store
            .chapter_units(&run)
            .unwrap()
            .iter()
            .all(|u| u.outputs.is_empty())
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM attempts WHERE state='completed'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
}
fn copy_tree(source: &std::path::Path, target: &std::path::Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let e = entry.unwrap();
        let to = target.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &to)
        } else {
            std::fs::copy(e.path(), to).unwrap();
        }
    }
}
#[tokio::test]
async fn whole_bundle_reconstruction_raw_metadata_and_pre_read_budgets_reject_before_import() {
    for kind in [
        "whole-pcm",
        "raw",
        "raw-budget",
        "ambiguity",
        "native-budget",
    ] {
        let source = tempfile::tempdir().unwrap();
        copy_tree(
            &root().join("fixtures/generation/chapter-import-v2/producer-inputs"),
            source.path(),
        );
        let script = r#"import sys,json,hashlib,base64,re
from pathlib import Path
r=Path(sys.argv[1]);kind=sys.argv[2]
enc=lambda v:(json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n').encode()
s=json.loads((r/'chapter-evidence.json').read_bytes())
if kind=='whole-pcm':
 b=bytearray((r/'chapter.wav').read_bytes());b[44]^=1;(r/'chapter.wav').write_bytes(b);s['narration']['sha256']=hashlib.sha256(b).hexdigest()
elif kind=='raw':
 p=r/s['turns'][0]['primary']['response']['path'];b=bytearray(p.read_bytes());b[-1]^=1;p.write_bytes(b)
elif kind=='raw-budget':
 s['turns'][0]['primary']['request']['bytes']=524289;s['turns'][0]['primary']['request']['path']='not-readable.json'
elif kind=='ambiguity':
 n=len(re.findall(r"[^\W_]+(?:['’][^\W_]+)*",(r/'source.txt').read_text()));s['facts'][0]['source_spans']=[[0,1]]*(2000000//n+1);s['narration']['path']='not-readable.wav'
elif kind=='native-budget':
 s['narration']['bytes']=268435457;s['narration']['path']='not-readable.wav'
if kind!='raw':(r/'chapter-evidence.json').write_bytes(enc(s))
if kind=='whole-pcm':
 p=r/'bundle/private/chapter-context.json';c=json.loads(p.read_bytes());raw=(r/'chapter-evidence.json').read_bytes();c['spec']=s;c['original_inventory']['bytes']=len(raw);c['original_inventory']['sha256']=hashlib.sha256(raw).hexdigest();c['inventory_bytes']=base64.b64encode(raw).decode();p.write_bytes(enc(c))
 p=r/'bundle/chapter-import-index.json';i=json.loads(p.read_bytes());i['narration']=s['narration'];i['context']['bytes']=len(enc(c));i['context']['sha256']=hashlib.sha256(enc(c)).hexdigest();p.write_bytes(enc(i))
 f={'path':'chapter-import-index.json','sha256':hashlib.sha256(enc(i)).hexdigest(),'bytes':len(enc(i)),'media_type':'application/json'}
 for p in (r/'bundle/units').iterdir():
  u=json.loads(p.read_bytes());u['index']=f;u['files']=[f if x['path']==f['path'] else i['context'] if x['path']==i['context']['path'] else x for x in u['files']];p.write_bytes(enc(u))
"#;
        assert!(
            std::process::Command::new("/usr/bin/python3")
                .args(["-B", "-c", script])
                .arg(source.path())
                .arg(kind)
                .status()
                .unwrap()
                .success()
        );
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = store.create(kind, &request()).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, epoch) = store.claim().unwrap().unwrap();
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let rt = Runtime::new(store.clone(), Arc::new(Audit(seen.clone()))).with_chapter_helper(
            ChapterHelper::new(root().join("tools/generation"), revision()).unwrap(),
        );
        let error = rt
            .import_chapter_bundle(
                &run,
                epoch,
                source.path(),
                "chapter-evidence.json",
                &source.path().join("bundle"),
            )
            .await
            .unwrap_err();
        if kind == "whole-pcm" {
            assert!(error.message.contains("PCM reconstruction"), "{error}");
        }
        if kind == "ambiguity" {
            assert!(error.message.contains("ambiguity work"), "{error}");
        }
        if kind.ends_with("budget") {
            assert!(
                !error.code.contains("io"),
                "payload opened before envelope rejection: {error}"
            );
        }
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM chapter_plans", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[tokio::test]
async fn cancellation_during_registered_subprocess_kills_its_group_and_preserves_the_attempt() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-cancel-process", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(store.clone());
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    let id = run.clone();
    let worker =
        tokio::spawn(async move { rt.execute_chapter_unit(&id, epoch, "chunk-0000").await });
    let pid = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let events = store.journal(&run, 0, 500).unwrap();
            if let Some(e) = events
                .iter()
                .find(|e| e["type"] == "chapter_helper_started")
            {
                break e["data"]["pid"].as_u64().unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    store.cancel(&run).unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(3), worker)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .code,
        "stale_epoch"
    );
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let state: String = store
                .db()
                .unwrap()
                .query_row("SELECT state FROM attempts", [], |r| r.get(0))
                .unwrap();
            if state != "reserved" && state != "dispatched" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        !std::process::Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        store
            .chapter_units(&run)
            .unwrap()
            .iter()
            .all(|u| u.outputs.is_empty())
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
}
#[tokio::test]
async fn helper_completion_survives_crash_and_checkpoint_rollback_without_double_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-crash", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let ready = Arc::new(tokio::sync::Notify::new());
    let rt = Runtime::new(store.clone(), Arc::new(HoldAfterGeneration(ready.clone())))
        .with_chapter_helper(
            ChapterHelper::new(root().join("tools/generation"), revision()).unwrap(),
        );
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    let id = run.clone();
    let worker =
        tokio::spawn(async move { rt.execute_chapter_unit(&id, epoch, "chunk-0000").await });
    tokio::time::timeout(std::time::Duration::from_secs(10), ready.notified())
        .await
        .unwrap();
    worker.abort();
    let _ = worker.await;
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(store.clone());
    store.db().unwrap().execute_batch("CREATE TRIGGER reject_chapter_checkpoint BEFORE INSERT ON chapter_units BEGIN SELECT RAISE(ABORT,'injected checkpoint failure'); END;").unwrap();
    assert!(
        rt.execute_chapter_unit(&run, epoch, "chunk-0000")
            .await
            .is_err()
    );
    assert!(store.chapter_units(&run).unwrap()[0].outputs.is_empty());
    assert_eq!(store.db().unwrap().query_row("SELECT COUNT(*) FROM events WHERE kind IN ('check_completed','chapter_unit_checkpoint')",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    store
        .db()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_chapter_checkpoint")
        .unwrap();
    rt.execute_chapter_unit(&run, epoch, "chunk-0000")
        .await
        .unwrap();
    assert_eq!(store.chapter_units(&run).unwrap()[0].outputs.len(), 3);
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM events WHERE kind='chapter_helper_started'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
}
#[tokio::test]
async fn corrupt_inputs_records_outputs_and_cross_unit_checkpoint_substitution_never_redispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-corrupt", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(store.clone());
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    for label in ["chunk-0000", "chunk-0001"] {
        rt.execute_chapter_unit(&run, epoch, label).await.unwrap();
    }
    let units = store.chapter_units(&run).unwrap();
    let before = store.snapshot(&run).unwrap();
    let record: String = store
        .db()
        .unwrap()
        .query_row(
            "SELECT record FROM chapter_units WHERE label='chunk-0000'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let record_ref: c::FileRef = serde_json::from_str(&record).unwrap();
    let index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original.join("bundle/chapter-import-index.json")).unwrap(),
    )
    .unwrap();
    for hash in [
        index["chunks"][0]["file"]["sha256"].as_str().unwrap(),
        &record_ref.sha256,
        &units[0].outputs[0].file.sha256,
    ] {
        let path = store.object(hash).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        for missing in [true, false] {
            if missing {
                std::fs::remove_file(&path).unwrap();
            } else {
                let mut b = bytes.clone();
                let last = b.len() - 1;
                b[last] ^= 1;
                std::fs::write(&path, b).unwrap();
            }
            assert!(
                rt.execute_chapter_unit(&run, epoch, "chunk-0000")
                    .await
                    .is_err()
            );
            assert!(rt.replay_chapter_unit(&run, "chunk-0000").await.is_err());
            assert_eq!(store.snapshot(&run).unwrap(), before);
            std::fs::write(&path, &bytes).unwrap();
        }
    }
    for column in ["record", "outputs"] {
        let saved: String = store
            .db()
            .unwrap()
            .query_row(
                &format!("SELECT {column} FROM chapter_units WHERE label='chunk-0000'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        let other: String = store
            .db()
            .unwrap()
            .query_row(
                &format!("SELECT {column} FROM chapter_units WHERE label='chunk-0001'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        store
            .db()
            .unwrap()
            .execute(
                &format!("UPDATE chapter_units SET {column}=?1 WHERE label='chunk-0000'"),
                [other],
            )
            .unwrap();
        assert_eq!(
            rt.execute_chapter_unit(&run, epoch, "chunk-0000")
                .await
                .unwrap_err()
                .code,
            "chapter_checkpoint_corrupt"
        );
        assert!(rt.replay_chapter_unit(&run, "chunk-0000").await.is_err());
        store
            .db()
            .unwrap()
            .execute(
                &format!("UPDATE chapter_units SET {column}=?1 WHERE label='chunk-0000'"),
                [saved],
            )
            .unwrap();
    }
    let saved: String = store
        .db()
        .unwrap()
        .query_row(
            "SELECT sha256 FROM chapter_plans WHERE run=?1",
            [&run],
            |r| r.get(0),
        )
        .unwrap();
    store
        .db()
        .unwrap()
        .execute(
            "UPDATE chapter_plans SET sha256=?1 WHERE run=?2",
            rusqlite::params![digest(b"substituted plan"), run],
        )
        .unwrap();
    assert!(
        rt.replay_chapter_unit(&run, "chunk-0000").await.is_err(),
        "plan row and immutable plan reference must agree"
    );
    store
        .db()
        .unwrap()
        .execute(
            "UPDATE chapter_plans SET sha256=?1 WHERE run=?2",
            rusqlite::params![saved, run],
        )
        .unwrap();
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert!(store.catalog("pg-55").unwrap().is_none());
}
#[tokio::test]
async fn fixed_acoustic_stage_resumes_only_missing_units_and_never_creates_a_package() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-pipeline", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(store.clone());
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    for stage in c::STAGES {
        if stage == c::Stage::Acoustic {
            break;
        }
        rt.execute_stage(&run, epoch, stage).await.unwrap();
    }
    rt.execute_chapter_unit(&run, epoch, "chunk-0000")
        .await
        .unwrap();
    drop(rt);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    let rt =
        runtime(store.clone()).with_chapter_inputs(temp.path().join("removed-original-inputs"));
    assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
    let units = store.chapter_units(&run).unwrap();
    assert_eq!(units.len(), 4);
    assert!(
        units
            .iter()
            .all(|u| u.outcome == Some(c::StageOutcome::NeedsRepair))
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    let snapshot = store.api_snapshot(&run).unwrap();
    assert_eq!(snapshot.state, c::RunState::Failed);
    assert_eq!(
        snapshot
            .stages
            .iter()
            .find(|s| s.stage == c::Stage::Acoustic)
            .unwrap()
            .outcome,
        Some(c::StageOutcome::NeedsRepair)
    );
    assert!(
        snapshot
            .stages
            .iter()
            .filter(|s| matches!(s.stage, c::Stage::Presentation | c::Stage::Package))
            .all(|s| s.outcome.is_none())
    );
    assert!(
        snapshot
            .findings
            .iter()
            .any(|f| f.id == "missing_source_mapping" && f.outcome == c::ReviewOutcome::Missing)
    );
    assert!(store.diagnostic_ref(&run).unwrap().is_none());
    assert!(store.catalog("pg-55").unwrap().is_none());
    if let Ok(path) = std::env::var("PT_CHAPTER_RUNTIME_RECEIPT") {
        let db = store.db().unwrap();
        let count = |kind: &str| {
            db.query_row(
                "SELECT COUNT(*) FROM events WHERE run=?1 AND kind=?2",
                rusqlite::params![run, kind],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
        };
        let receipt = json!({"schema_version":1,"test":"fixed_acoustic_stage_resumes_only_missing_units_and_never_creates_a_package","recipe_sha256":request()["pins"]["recipe_sha256"],"plan_sha256":db.query_row("SELECT sha256 FROM chapter_plans WHERE run=?1",[&run],|r|r.get::<_,String>(0)).unwrap(),"units":units,"helper_authorizations":count("chapter_helper_authorized"),"helper_starts":count("chapter_helper_started"),"helper_completions":count("chapter_helper_completed"),"helper_failures":count("chapter_helper_failed"),"checkpoint_events":count("chapter_unit_checkpoint"),"reused_units":count("chapter_unit_reused"),"provider_calls":0,"outcome":"needs_repair","run_state":snapshot.state,"production_approved":false,"publication_enabled":false});
        std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}
fn produced(variant: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let status=std::process::Command::new("/usr/bin/python3").args(["-B","-c","import sys;from pathlib import Path;sys.path.insert(0,sys.argv[1]);from test_chapter_import import build;build(Path(sys.argv[2]),sys.argv[3])"]).arg(root().join("tools/generation")).arg(temp.path()).arg(variant).status().unwrap();
    assert!(status.success());
    temp
}
#[tokio::test]
async fn cross_cut_global_ambiguity_and_original_adverse_observations_remain_unresolved() {
    for variant in ["cut", "ambiguous", "adverse", "many"] {
        let original = produced(variant);
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let run = store.create(variant, &request()).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, epoch) = store.claim().unwrap().unwrap();
        let rt = runtime(store.clone());
        rt.import_chapter_bundle(
            &run,
            epoch,
            original.path(),
            "chapter-evidence.json",
            &original.path().join("bundle"),
        )
        .await
        .unwrap();
        rt.execute_chapter_unit(&run, epoch, "chunk-0000")
            .await
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(
            &store
                .chapter_artifact(&run, "chunk-0000", "private/chapter-import-validation.json")
                .unwrap(),
        )
        .unwrap();
        let reason = match variant {
            "cut" | "many" => "cross_cut_evidence_missing",
            "ambiguous" => "global_source_or_terminal_ambiguity",
            _ => "recorded_adverse_fact_finding",
        };
        let facts = report["verified_import"]["facts"].as_array().unwrap();
        if variant != "many" {
            assert!(
                facts.iter().any(|f| f["reason"] == reason
                    && f["not_before_sample"].is_null()
                    && f["outcome"] != "pass"),
                "{variant}: {facts:?}"
            );
        }
        if variant == "cut" {
            assert!(facts.iter().any(|f| {
                f["required_chunks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|x| x == "chunk-0001")
                    && !f["present_chunks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|x| x == "chunk-0001")
            }));
        }
        if variant == "adverse" {
            assert!(
                store
                    .api_snapshot(&run)
                    .unwrap()
                    .findings
                    .iter()
                    .any(|f| f.id == "acoustic-fact:gate" && f.outcome == c::ReviewOutcome::Fail)
            );
        }
        if variant == "many" {
            let u: serde_json::Value = serde_json::from_slice(
                &std::fs::read(original.path().join("bundle/units/chunk-0000.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(u["files"].as_array().unwrap().len() + 5, 64);
        }
        assert_eq!(
            rt.replay_chapter_unit(&run, "chunk-0000")
                .await
                .unwrap()
                .len(),
            3
        );
        assert!(store.catalog("pg-55").unwrap().is_none());
    }
}
#[tokio::test]
async fn temporary_1200_second_native_chapter_freezes_and_runs_under_per_unit_bounds() {
    let original = produced("large");
    assert_eq!(
        std::fs::metadata(original.path().join("chapter.wav"))
            .unwrap()
            .len(),
        19_200_044
    );
    let spec: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original.path().join("chapter-evidence.json")).unwrap(),
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("chapter-capacity", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let rt = Runtime::new(store.clone(), Arc::new(Audit(seen.clone()))).with_chapter_helper(
        ChapterHelper::new(root().join("tools/generation"), revision()).unwrap(),
    );
    rt.import_chapter_bundle(
        &run,
        epoch,
        original.path(),
        "chapter-evidence.json",
        &original.path().join("bundle"),
    )
    .await
    .unwrap();
    let units = store.chapter_units(&run).unwrap();
    assert_eq!(units.len(), 2);
    for u in &units {
        rt.execute_chapter_unit(&run, epoch, &u.label)
            .await
            .unwrap();
    }
    let before = store.snapshot(&run).unwrap();
    for u in &units {
        assert_eq!(
            rt.replay_chapter_unit(&run, &u.label).await.unwrap().len(),
            3
        );
    }
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert!(seen.lock().unwrap().iter().all(|i| i.inputs.len() <= 64
        && i.inputs.iter().all(|a| a.file.bytes <= 16 * 1024 * 1024
            && a.file.sha256 != spec["narration"]["sha256"].as_str().unwrap())));
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    if let Ok(path) = std::env::var("PT_CHAPTER_CAPACITY_RECEIPT") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"schema_version":1,"test":"temporary_1200_second_native_chapter_freezes_and_runs_under_per_unit_bounds","native_bytes":19_200_044,"duration_seconds":1200,"sample_rate":8000,"units":units,"helper_attempts":2,"replay_helper_dispatches":0,"replay_provider_dispatches":0,"snapshot_unchanged_by_replay":true,"full_native_in_stage_inputs":false,"production_approved":false})).unwrap()).unwrap();
    }
}
#[tokio::test]
async fn cancelled_runs_and_uncertain_recovered_attempts_never_dispatch_again() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let original = root().join("fixtures/generation/chapter-import-v2/producer-inputs");
    let run = store.create("chapter-before-cancel", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(store.clone());
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    store.cancel(&run).unwrap();
    assert_eq!(
        rt.execute_chapter_unit(&run, epoch, "chunk-0000")
            .await
            .unwrap_err()
            .code,
        "stale_epoch"
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let run = store.create("chapter-uncertain", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    rt.import_chapter_bundle(
        &run,
        epoch,
        &original,
        "chapter-evidence.json",
        &original.join("bundle"),
    )
    .await
    .unwrap();
    let changed = Runtime::new(
        store.clone(),
        Arc::new(crate::generation::GenerationExecutor),
    )
    .with_chapter_helper(
        ChapterHelper::new(
            root().join("tools/generation"),
            digest(b"changed stage build"),
        )
        .unwrap(),
    );
    assert_eq!(
        changed
            .execute_chapter_unit(&run, epoch, "chunk-0000")
            .await
            .unwrap_err()
            .code,
        "chapter_plan_stale"
    );
    rt.execute_chapter_unit(&run, epoch, "chunk-0000")
        .await
        .unwrap();
    // Crash snapshot: dispatch persisted, completion/checkpoint transactions absent.
    store
        .db()
        .unwrap()
        .execute("DELETE FROM chapter_units WHERE run=?1", [&run])
        .unwrap();
    store
        .db()
        .unwrap()
        .execute(
            "UPDATE attempts SET state='dispatched',response=NULL WHERE run=?1",
            [&run],
        )
        .unwrap();
    drop(rt);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = runtime(store.clone());
    let before = store
        .db()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM events WHERE kind='chapter_helper_started'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    assert!(
        rt.execute_chapter_unit(&run, epoch, "chunk-0000")
            .await
            .is_err()
    );
    assert!(rt.replay_chapter_unit(&run, "chunk-0000").await.is_err());
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM attempts WHERE state='uncertain'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM events WHERE kind='chapter_helper_started'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        before
    );
    assert!(
        store
            .chapter_units(&run)
            .unwrap()
            .iter()
            .all(|u| u.outputs.is_empty())
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
}
