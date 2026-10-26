use super::*;
use std::sync::{Arc, atomic::{AtomicUsize,Ordering}};
use pt_contracts as c;
use async_trait::async_trait;
use serde_json::json;
struct CountedStage(Arc<AtomicUsize>);
#[async_trait]
impl c::StageExecutor for CountedStage {
    async fn execute(&self,input:c::StageInput,services:&dyn c::RunServices)->std::result::Result<c::StageOutput,c::StageError> {
        self.0.fetch_add(1,Ordering::SeqCst);
        services.check_cancelled().await?;
        let artifact=services.write_candidate(c::CandidateWrite{candidate_id:input.unit_id.clone(),relative_path:format!("{:?}.json",input.stage).to_lowercase(),media_type:"application/json".into(),max_bytes:4096,provenance:c::Provenance::Synthetic},b"{\"synthetic\":true}".to_vec()).await?;
        Ok(c::StageOutput{outcome:c::StageOutcome::Complete,artifacts:vec![artifact],decisions:vec![],findings:vec![],cache_keys:vec![],library_updates:vec![],package_candidate:None})
    }
}
fn request()->serde_json::Value {json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":digest(b"fixture-v1"),"profile_sha256":digest(b"profile-v1"),"profile_id":"diagnostic-v1"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}})}
#[tokio::test]
async fn completed_shared_stage_survives_restart_without_redispatch() {
    let temp=tempfile::tempdir().unwrap();let calls=Arc::new(AtomicUsize::new(0));
    let store=Store::open(temp.path()).unwrap();let run=store.create("resume",&request()).unwrap()["id"].as_str().unwrap().to_owned();
    let (_,epoch)=store.claim().unwrap().unwrap();
    let runtime=Runtime::new(store.clone(),Arc::new(CountedStage(calls.clone())));
    runtime.execute_stage(&run,epoch,c::Stage::Source).await.unwrap();
    drop(runtime);drop(store);
    let store=Store::open(temp.path()).unwrap();store.recover().unwrap();let (_,epoch)=store.claim().unwrap().unwrap();
    let runtime=Runtime::new(store.clone(),Arc::new(CountedStage(calls.clone())));
    runtime.execute_stage(&run,epoch,c::Stage::Source).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst),1);
    assert_eq!(store.snapshot(&run).unwrap()["stages"][0]["state"],"succeeded");
    assert!(store.journal(&run,0,500).unwrap().iter().any(|e|e["type"]=="cache_reused"));
}
#[tokio::test]
async fn missing_or_corrupt_completed_artifact_fails_reuse() {
    let temp=tempfile::tempdir().unwrap();let store=Store::open(temp.path()).unwrap();
    let run=store.create("corruption",&request()).unwrap()["id"].as_str().unwrap().to_owned();let (_,epoch)=store.claim().unwrap().unwrap();let calls=Arc::new(AtomicUsize::new(0));let runtime=Runtime::new(store.clone(),Arc::new(CountedStage(calls.clone())));
    let artifacts=runtime.execute_stage(&run,epoch,c::Stage::Source).await.unwrap();
    let path=temp.path().join("objects").join(&artifacts[0].file.sha256);
    std::fs::write(&path,b"{\"synthetic\":fals}").unwrap();
    assert_eq!(runtime.execute_stage(&run,epoch,c::Stage::Source).await.unwrap_err().code,"corrupt_artifact");
    std::fs::remove_file(path).unwrap();
    assert_eq!(runtime.execute_stage(&run,epoch,c::Stage::Source).await.unwrap_err().code,"corrupt_artifact");
    assert_eq!(calls.load(Ordering::SeqCst),1);
}
fn broker_store(temp:&tempfile::TempDir,key:&str)->(Store,String,i64) {
    let store=Store::open(temp.path()).unwrap();let run=store.create(key,&request()).unwrap()["id"].as_str().unwrap().to_owned();let (_,epoch)=store.claim().unwrap().unwrap();
    store.import_accounting(&AccountingImport{original_hash:digest(b"test-neutral-original"),derivative_hash:digest(b"test-neutral-derivative"),scope:"fake/project/model".into(),money:0,requests:0,tokens:0,repairs:0,window_start:0,reset_at:now()+86400000,retry_at:0,transformation:"Synthetic accounting test record; no live authorization.".into()}).unwrap();
    store.authorize_live(&digest(b"test-authorization-only"),100).unwrap();store.allow_recipe("test-v1").unwrap();
    store.configure_limit("fake","project","model",&Limits{money:100,requests:3,tokens:100,repairs:1,window_start:0,reset_at:now()+86400000}).unwrap();
    (store,run,epoch)
}
fn operation(id:&str)->Operation {let payload=json!({"test":true});Operation{identity:id.into(),recipe:"test-v1".into(),provider:"fake".into(),project:"project".into(),model:"model".into(),request_hash:digest(&serde_json::to_vec(&payload).unwrap()),payload,money:10,requests:1,tokens:2,repair:false,timeout_ms:1000}}
#[test]
fn attempt_identity_binds_model_recipe_and_reservation() {
    let temp=tempfile::tempdir().unwrap();let (store,run,epoch)=broker_store(&temp,"identity");let op=operation("one");
    assert!(matches!(store.reserve(&run,epoch,&op).unwrap(),Dispatch::Authorized(_)));
    let mut changed=op.clone();changed.money=1;
    assert_eq!(store.reserve(&run,epoch,&changed).unwrap_err().code,"operation_conflict");
}
struct FakeTransport {calls:Arc<AtomicUsize>}
impl Transport for FakeTransport {
    fn send<'a>(&'a self,_:&'a str,op:&'a Operation)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<Receipt>>+Send+'a>>{Box::pin(async move{self.calls.fetch_add(1,Ordering::SeqCst);Ok(Receipt{outcome:"completed".into(),actual_money:Some(op.money.min(7)),actual_tokens:Some(2),response_hash:Some(digest(b"response")),job_id:None,retry_at:None})})}
}
#[tokio::test]
async fn crash_after_reservation_preserves_charge_and_prevents_repurchase(){
    let temp=tempfile::tempdir().unwrap();let (store,run,epoch)=broker_store(&temp,"crash");let op=operation("reserved-before-crash");store.reserve(&run,epoch,&op).unwrap();drop(store);
    let store=Store::open(temp.path()).unwrap();store.recover().unwrap();let (_,epoch)=store.claim().unwrap().unwrap();let calls=Arc::new(AtomicUsize::new(0));
    assert_eq!(store.dispatch(&run,epoch,&op,&FakeTransport{calls:calls.clone()}).await.unwrap_err().code,"attempt_unresolved");assert_eq!(calls.load(Ordering::SeqCst),0);
    assert!(matches!(store.reserve(&run,epoch,&op).unwrap(),Dispatch::Existing{state,..} if state=="uncertain"));
    let mut expensive=operation("second");expensive.money=95;assert_eq!(store.reserve(&run,epoch,&expensive).unwrap_err().code,"budget_exhausted");
}
struct DelayedTransport {started:Arc<tokio::sync::Notify>,release:Arc<tokio::sync::Notify>}
impl Transport for DelayedTransport {
    fn send<'a>(&'a self,_:&'a str,_:&'a Operation)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<Receipt>>+Send+'a>>{Box::pin(async move{self.started.notify_one();self.release.notified().await;Ok(Receipt{outcome:"completed".into(),actual_money:Some(7),actual_tokens:Some(2),response_hash:Some(digest(b"late-response")),job_id:None,retry_at:None})})}
}
#[tokio::test]
async fn cancel_during_call_records_late_charge_without_reviving_epoch(){
    let temp=tempfile::tempdir().unwrap();let(store,run,epoch)=broker_store(&temp,"cancel");let started=Arc::new(tokio::sync::Notify::new());let release=Arc::new(tokio::sync::Notify::new());let transport=DelayedTransport{started:started.clone(),release:release.clone()};let clone=store.clone();let id=run.clone();
    let worker=tokio::spawn(async move{clone.dispatch(&id,epoch,&operation("late"),&transport).await});started.notified().await;store.cancel(&run).unwrap();release.notify_one();worker.await.unwrap().unwrap();
    let runtime=Runtime::new(store.clone(),Arc::new(FixtureExecutor));assert_eq!(runtime.execute_stage(&run,epoch,c::Stage::Source).await.unwrap_err().code,"stale_epoch");
    let saved=store.snapshot(&run).unwrap();assert_eq!(saved["state"],"cancelled");assert!(saved["artifacts"].as_array().unwrap().is_empty());assert!(store.catalog("pg-55").unwrap().is_none());
    assert!(store.journal(&run,0,500).unwrap().iter().any(|e|e["type"]=="provider_result" && e["data"]["money"]==7));
}
#[tokio::test]
async fn quota_stop_and_independent_request_capacity_survive_restart(){
    let temp=tempfile::tempdir().unwrap();let(store,run,epoch)=broker_store(&temp,"quota");let calls=Arc::new(AtomicUsize::new(0));let fake=FakeTransport{calls:calls.clone()};
    let mut free=operation("token-count");free.money=0;store.dispatch(&run,epoch,&free,&fake).await.unwrap();
    let wait=now()+86400000;let id=match store.reserve(&run,epoch,&operation("quota-error")).unwrap(){Dispatch::Authorized(id)=>id,_=>panic!()};
    store.record_receipt(&id,&Receipt{outcome:"rejected".into(),actual_money:Some(0),actual_tokens:Some(0),response_hash:Some(digest(b"daily-quota")),job_id:None,retry_at:Some(wait)}).unwrap();drop(store);
    let store=Store::open(temp.path()).unwrap();store.recover().unwrap();let(_,epoch)=store.claim().unwrap().unwrap();let error=store.reserve(&run,epoch,&operation("another")).unwrap_err();assert_eq!(error.code,"provider_wait");assert!(error.message.contains(&wait.to_string()));
    store.configure_limit("fake","project","other",&Limits{money:100,requests:1,tokens:100,repairs:1,window_start:0,reset_at:wait}).unwrap();let mut other=operation("other-one");other.model="other".into();other.money=0;store.dispatch(&run,epoch,&other,&fake).await.unwrap();other.identity="other-two".into();assert_eq!(store.reserve(&run,epoch,&other).unwrap_err().code,"request_quota");assert_eq!(calls.load(Ordering::SeqCst),2);
}
#[test]
fn idempotency_and_snapshot_cursor_replay_are_read_only(){
    let temp=tempfile::tempdir().unwrap();let store=Store::open(temp.path()).unwrap();let first=store.create("same",&request()).unwrap();let run=first["id"].as_str().unwrap();assert_eq!(first,store.create("same",&request()).unwrap());let mut changed=request();changed["book_id"]=json!("pg-36");changed["source_id"]=json!("pg-36-book-1-chapter-1");assert_eq!(store.create("same",&changed).unwrap_err().code,"idempotency_conflict");
    let n=first["last_event_seq"].as_i64().unwrap();store.cancel(run).unwrap();let events=store.journal(run,n,500).unwrap();assert_eq!(events.len(),1);assert_eq!(events[0]["seq"],n+1);
    let before=std::fs::read(temp.path().join("runtime.sqlite3")).unwrap();for _ in 0..5{store.snapshot(run).unwrap();store.journal(run,0,500).unwrap();store.catalog("pg-55").unwrap();}let after=std::fs::read(temp.path().join("runtime.sqlite3")).unwrap();assert_eq!(digest(&before),digest(&after));assert_eq!(store.journal(run,n+2,500).unwrap_err().code,"snapshot_required");
}
#[test]
fn public_snapshot_and_events_use_frozen_shared_contract(){
    let temp=tempfile::tempdir().unwrap();let store=Store::open(temp.path()).unwrap();let run=store.create("wire",&request()).unwrap()["id"].as_str().unwrap().to_owned();
    let snapshot=store.api_snapshot(&run).unwrap();let n=snapshot.last_event_seq;
    store.cancel(&run).unwrap();let events=store.api_journal(&run,n as i64,500).unwrap();assert_eq!(events.len(),1);assert_eq!(events[0].seq,n+1);assert_eq!(events[0].entry.kind,c::EventKind::RunTerminal);assert_eq!(events[0].run_id,run);
    let encoded=serde_json::to_value(snapshot).unwrap();let _:c::RunSnapshot=serde_json::from_value(encoded).unwrap();
}
#[tokio::test]
async fn helper_has_bounded_output_deadline_and_cancellation(){
    use helper::{Recipe,execute};
    let mut recipe=Recipe{name:"fixture-helper".into(),executable:"/usr/bin/python3".into(),arguments:vec!["-c".into(),"print('x'*10000)".into()],max_input:1024,max_output:64,max_decoded_media:1024,timeout_ms:1000};
    assert_eq!(execute(&recipe,&digest(b"test-attempt"),b"{}",0,||false).await.err().unwrap().code,"helper_output_limit");
    recipe.arguments=vec!["-c".into(),"import time; time.sleep(2)".into()];recipe.timeout_ms=30;assert_eq!(execute(&recipe,&digest(b"test-attempt"),b"{}",0,||false).await.err().unwrap().code,"helper_timeout");
    assert_eq!(execute(&recipe,&digest(b"test-attempt"),b"{}",0,||true).await.err().unwrap().code,"stale_epoch");
}
#[tokio::test]
async fn real_generation_candidate_keeps_saved_media_and_missing_acceptance_private(){
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let fixture=root.join("fixtures/shared/two-scene-v1");
    let temp=tempfile::tempdir().unwrap();let store=Store::open(temp.path()).unwrap();let profile=std::fs::read(fixture.join("profile.json")).unwrap();let mut req=request();req["pins"]["profile_sha256"]=json!(digest(&profile));req["pins"]["profile_id"]=json!("paper-player-r186-v1");
    let run=store.create("diagnostic-package",&req).unwrap()["id"].as_str().unwrap().to_owned();let(_,epoch)=store.claim().unwrap().unwrap();
    store.seed_fixture(&run,epoch,&fixture,Some(&root.join("fixtures/generation/source"))).unwrap();
    let runtime=Runtime::new(store.clone(),Arc::new(crate::generation::GenerationExecutor));runtime.execute_stage(&run,epoch,c::Stage::Source).await.unwrap();
    assert_eq!(runtime.execute_stage(&run,epoch,c::Stage::Package).await.unwrap_err().code,"stage_needs_repair");
    assert!(!store.api_snapshot(&run).unwrap().findings.is_empty());
    let reference=store.diagnostic_ref(&run).unwrap().unwrap();let (bytes,_,hash)=store.package_artifact(&run,"manifest.json").unwrap();assert_eq!(hash,reference.manifest_sha256);let manifest:c::Manifest=serde_json::from_slice(&bytes).unwrap();assert_eq!(manifest.provenance,c::Provenance::Diagnostic);
    let (audio,_,_)=store.package_artifact(&run,&manifest.entries.audio.path).unwrap();assert_eq!(digest(&audio),manifest.entries.audio.sha256);assert!(store.catalog("pg-55").unwrap().is_none());assert!(store.api_journal(&run,0,500).unwrap().iter().any(|e|e.entry.kind==c::EventKind::DirectorDecision));
}
#[test]
fn restart_at_each_publication_boundary_preserves_previous_catalog(){
    for boundary in ["prepared","files_staged","renamed"]{
        let temp=tempfile::tempdir().unwrap();let store=Store::open(temp.path()).unwrap();let run=store.create(boundary,&request()).unwrap()["id"].as_str().unwrap().to_owned();let(_,epoch)=store.claim().unwrap().unwrap();
        let old=digest(b"previous-accepted-pointer-fixture");let pending=digest(boundary.as_bytes());
        // Fault injection seeds a committed historical pointer and an interrupted activation.
        // It grants no acceptance to this synthetic test data.
        let db=rusqlite::Connection::open(temp.path().join("runtime.sqlite3")).unwrap();db.execute("INSERT INTO publications VALUES(?1,?2,?3,'pg-55','{}','active')",rusqlite::params![old,run,epoch]).unwrap();db.execute("INSERT INTO catalog VALUES('pg-55',?1)",[&old]).unwrap();db.execute("INSERT INTO publications VALUES(?1,?2,?3,'pg-55','{}','pending')",rusqlite::params![pending,run,epoch]).unwrap();
        if boundary!="prepared"{let dir=temp.path().join("releases").join(if boundary=="renamed"{pending.as_str()}else{"pending-crash"});std::fs::create_dir_all(&dir).unwrap();std::fs::write(dir.join("incomplete.json"),b"{}").unwrap();}drop(db);drop(store);
        let reopened=Store::open(temp.path()).unwrap();reopened.recover().unwrap();assert_eq!(reopened.catalog("pg-55").unwrap(),Some(old));assert!(reopened.journal(&run,0,500).unwrap().iter().any(|e|e["type"]=="publication_aborted"));
    }
}
