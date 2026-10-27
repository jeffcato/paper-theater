use super::provider_v2::*;
use super::*;
use async_trait::async_trait;
use pt_contracts as c;
use serde_json::json;
use std::sync::atomic::AtomicUsize;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

struct Time(AtomicU64);
impl Clock for Time {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn amounts(money: u64, requests: u64, tokens: u64, repairs: u64) -> c::QuotaAmounts {
    c::QuotaAmounts {
        money_microusd: money,
        requests,
        tokens,
        repairs,
    }
}
fn setup() -> (
    tempfile::TempDir,
    Store,
    Broker,
    Arc<Time>,
    String,
    i64,
    c::ProviderOperation,
) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let time = Arc::new(Time(AtomicU64::new(now() as u64)));
    let request = json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":digest(b"offline-recipe"),"profile_sha256":digest(b"profile"),"profile_id":"diagnostic"},"limits":{"money_microusd":1000,"requests":100,"tokens":1000,"repairs":100,"wall_time_ms":60000}});
    let run = store.create("provider-v2", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    // Test-only simulation of internal live authorization. HTTP still cannot create it.
    let mut request = request;
    request["mode"] = json!("live");
    store
        .transaction(|tx| {
            tx.execute(
                "UPDATE runs SET request=?2 WHERE id=?1",
                rusqlite::params![run, request.to_string()],
            )?;
            Ok(())
        })
        .unwrap();
    store
        .import_accounting(&AccountingImport {
            original_hash: digest(b"synthetic-original"),
            derivative_hash: digest(b"synthetic-derivative"),
            scope: "fake/project/model".into(),
            money: 0,
            requests: 0,
            tokens: 0,
            repairs: 0,
            window_start: 0,
            reset_at: 1,
            retry_at: 0,
            transformation: "Synthetic offline adapter fixture; no product authorization.".into(),
        })
        .unwrap();
    store
        .authorize_live(&digest(b"synthetic-proof"), 1000)
        .unwrap();
    let bytes = b"{ \"offline\": true }\n";
    let hash = store.put(bytes).unwrap();
    let payload = c::ArtifactRef {
        id: digest(b"offline-payload"),
        file: c::FileRef {
            path: "request.json".into(),
            sha256: hash,
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        },
        provenance: c::Provenance::Synthetic,
    };
    store
        .transaction(|tx| {
            tx.execute(
                "INSERT INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,'synthetic',?7)",
                rusqlite::params![
                    payload.id,
                    run,
                    payload.file.path,
                    payload.file.sha256,
                    payload.file.bytes,
                    payload.file.media_type,
                    serde_json::to_string(&payload)?
                ],
            )?;
            Ok(())
        })
        .unwrap();
    let broker = Broker::synthetic(store.clone(), time.clone());
    broker
        .register(&Binding {
            id: "primary".into(),
            context: c::ProviderContext {
                provider: "fake".into(),
                project_id: "project".into(),
                credential_context_id: "slot-a".into(),
            },
            model: "model".into(),
            recipe_sha256: digest(b"offline-recipe"),
            kind: c::OperationKind::Direction,
            recovery_estimate: c::Usage {
                requests: 1,
                ..Default::default()
            },
        })
        .unwrap();
    broker
        .authorize_unit(&run, epoch, "unit", "primary", "logical", None)
        .unwrap();
    let proposal = c::ProviderOperation {
        attempt_id: "caller-id-is-not-authority".into(),
        unit_id: "unit".into(),
        kind: c::OperationKind::Direction,
        provider: "fake".into(),
        model: "model".into(),
        recipe_sha256: digest(b"offline-recipe"),
        request_sha256: payload.file.sha256.clone(),
        payload,
        inputs: vec![],
        estimate: c::Usage {
            requests: 1,
            input_tokens: 2,
            output_tokens: 3,
            money_microusd: 0,
        },
        timeout_ms: 1000,
    };
    (temp, store, broker, time, run, epoch, proposal)
}
fn window(
    id: &str,
    scope: c::ProviderQuotaScope,
    start: u64,
    reset: u64,
    capacity: c::QuotaAmounts,
) -> Window {
    Window {
        id: id.into(),
        scope,
        start_at_ms: start,
        reset_at_ms: reset,
        capacity,
    }
}
#[test]
fn overlapping_windows_reserve_atomically_and_zero_money_still_uses_requests() {
    let (_temp, _store, broker, time, run, epoch, proposal) = setup();
    let t = time.now_ms();
    let provider = c::ProviderQuotaScope::Provider {
        provider: "fake".into(),
    };
    let project = c::ProviderQuotaScope::Project {
        provider: "fake".into(),
        project_id: "project".into(),
    };
    broker
        .configure_window(&window(
            "minute",
            provider,
            t - 1000,
            t + 60000,
            amounts(100, 2, 100, 2),
        ))
        .unwrap();
    broker
        .configure_window(&window(
            "day",
            project,
            t - 1000,
            t + 86400000,
            amounts(100, 0, 100, 2),
        ))
        .unwrap();
    assert_eq!(
        broker
            .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
            .unwrap_err()
            .code,
        "request_quota"
    );
    assert!(broker.quota_snapshot("minute").unwrap().accounted.requests == 0);
    broker
        .configure_window(&window(
            "day",
            c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            t - 1000,
            t + 86400000,
            amounts(100, 1, 100, 2),
        ))
        .unwrap();
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    issued.validate().unwrap();
    assert_ne!(issued.operation.attempt_id, proposal.attempt_id);
    assert_eq!(issued.context.credential_context_id, "slot-a");
    assert_eq!(
        broker.quota_snapshot("minute").unwrap().accounted.requests,
        1
    );
    assert_eq!(broker.quota_snapshot("day").unwrap().accounted.requests, 1);
}

fn project_window(broker: &Broker, time: &Time) {
    let t = time.now_ms();
    broker
        .configure_window(&window(
            "project-day",
            c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            t - 1000,
            t + 86400000,
            amounts(1000, 100, 1000, 100),
        ))
        .unwrap();
}
struct Unknown {
    calls: Arc<AtomicUsize>,
    time: Arc<Time>,
}
#[async_trait]
impl OfflineTransport for Unknown {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        assert_eq!(bytes, b"{ \"offline\": true }\n");
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.time.0.fetch_add(7, Ordering::SeqCst);
        let mut reply = Reply::ambiguous(op);
        reply.raw = Some(b"opaque fake provider response".to_vec());
        reply.job = Some(Job {
            id: "job-1".into(),
            status: c::ProviderJobStatus::Unknown,
        });
        Ok(reply)
    }
}
#[tokio::test]
async fn issued_descriptor_dispatches_once_with_exact_payload_and_truthful_unknown_receipt() {
    let (_temp, _store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let reserved = time.now_ms();
    time.0.fetch_add(50, Ordering::SeqCst);
    let calls = Arc::new(AtomicUsize::new(0));
    let fake = Unknown {
        calls: calls.clone(),
        time: time.clone(),
    };
    let result = broker
        .execute_issued_provider(&issued, &fake)
        .await
        .unwrap();
    result.validate_for_operation(&issued).unwrap();
    assert_eq!(result.started_at_ms, reserved + 50);
    assert_eq!(result.recorded_at_ms, reserved + 57);
    assert_eq!(result.observed_usage.requests, None);
    assert!(result.result.usage.is_none());
    assert_eq!(result.accounted_usage.requests, 1);
    assert_eq!(result.accounted_usage.tokens, 5);
    assert_eq!(
        result.recovery.as_ref().unwrap().status,
        c::ProviderJobStatus::Unknown
    );
    assert_eq!(
        broker.raw_receipt(&issued.operation.attempt_id, 1).unwrap(),
        b"opaque fake provider response"
    );
    assert_eq!(
        result.result.response_sha256,
        Some(digest(b"opaque fake provider response"))
    );
    assert_eq!(
        broker
            .execute_issued_provider(&issued, &fake)
            .await
            .unwrap_err()
            .code,
        "attempt_unresolved"
    );
    let mut forged = issued.clone();
    forged.operation.estimate.money_microusd = 1;
    assert_eq!(
        broker
            .execute_issued_provider(&forged, &fake)
            .await
            .unwrap_err()
            .code,
        "issued_descriptor_mismatch"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

fn next_run(
    store: &Store,
    key: &str,
    proposal: &c::ProviderOperation,
) -> (String, i64, c::ProviderOperation) {
    let mut request = store
        .snapshot(
            &store.api_history(i64::MAX, 1).unwrap()["runs"][0]["run_id"]
                .as_str()
                .unwrap(),
        )
        .unwrap()["request"]
        .clone();
    request["mode"] = json!("fixture");
    let run = store.create(key, &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    request["mode"] = json!("live");
    let mut proposal = proposal.clone();
    proposal.payload.id = digest(format!("{run}:payload").as_bytes());
    store
        .transaction(|tx| {
            tx.execute(
                "UPDATE runs SET request=?2 WHERE id=?1",
                rusqlite::params![run, request.to_string()],
            )?;
            tx.execute(
                "INSERT INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,'synthetic',?7)",
                rusqlite::params![
                    proposal.payload.id,
                    run,
                    proposal.payload.file.path,
                    proposal.payload.file.sha256,
                    proposal.payload.file.bytes,
                    proposal.payload.file.media_type,
                    serde_json::to_string(&proposal.payload)?
                ],
            )?;
            Ok(())
        })
        .unwrap();
    (run, epoch, proposal)
}
#[tokio::test]
async fn unresolved_work_survives_cancel_restart_elapsed_time_and_an_alternate_credential() {
    let (temp, store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let fake = Unknown {
        calls: calls.clone(),
        time: time.clone(),
    };
    broker
        .execute_issued_provider(&issued, &fake)
        .await
        .unwrap();
    store.cancel(&run).unwrap();
    time.0.fetch_add(60000, Ordering::SeqCst);
    let reopened = Store::open(temp.path()).unwrap();
    reopened.recover().unwrap();
    let broker = Broker::synthetic(reopened.clone(), time.clone());
    let (run2, epoch2, proposal2) = next_run(&reopened, "alternate-key", &proposal);
    broker
        .register(&Binding {
            id: "backup".into(),
            context: c::ProviderContext {
                provider: "fake".into(),
                project_id: "project".into(),
                credential_context_id: "slot-b".into(),
            },
            model: "model".into(),
            recipe_sha256: proposal.recipe_sha256.clone(),
            kind: proposal.kind,
            recovery_estimate: c::Usage {
                requests: 1,
                ..Default::default()
            },
        })
        .unwrap();
    broker
        .authorize_unit(
            &run2,
            epoch2,
            "unit",
            "backup",
            "different-logical-name",
            None,
        )
        .unwrap();
    assert_eq!(
        broker
            .bind_provider_proposal(&run2, epoch2, &proposal2.unit_id, &proposal2)
            .unwrap_err()
            .code,
        "attempt_unresolved"
    );
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        1
    );
    assert_eq!(
        broker
            .execute_issued_provider(&issued, &fake)
            .await
            .unwrap_err()
            .code,
        "stale_epoch"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

struct Known;
#[async_trait]
impl OfflineTransport for Known {
    async fn send(&self, op: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        let mut reply = Reply::ambiguous(op);
        reply.completion = c::ProviderCompletion::KnownRejection;
        reply.usage = c::ObservedProviderUsage {
            requests: Some(1),
            tokens: Some(5),
            input_tokens: Some(2),
            output_tokens: Some(3),
            money_microusd: Some(op.operation.estimate.money_microusd),
        };
        reply.raw = Some(b"known rejection; no provider job".to_vec());
        Ok(reply)
    }
}
#[tokio::test]
async fn product_money_cap_is_lifetime_and_does_not_reset_with_a_new_run() {
    let (_temp, store, broker, time, run, epoch, mut proposal) = setup();
    let t = time.now_ms();
    broker
        .configure_window(&window(
            "project-wide",
            c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            t - 1000,
            t + 86400000,
            amounts(10000, 100, 1000, 100),
        ))
        .unwrap();
    proposal.estimate.money_microusd = 600;
    let first = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    broker
        .execute_issued_provider(&first, &Known)
        .await
        .unwrap();
    store.cancel(&run).unwrap();
    let (run2, epoch2, proposal2) = next_run(&store, "new-budget-run", &proposal);
    broker
        .authorize_unit(&run2, epoch2, "unit", "primary", "second-logical", None)
        .unwrap();
    assert_eq!(
        broker
            .bind_provider_proposal(&run2, epoch2, &proposal2.unit_id, &proposal2)
            .unwrap_err()
            .code,
        "budget_exhausted"
    );
    assert_eq!(
        broker
            .quota_snapshot("project-wide")
            .unwrap()
            .accounted
            .money_microusd,
        600
    );
}

#[tokio::test]
async fn repair_identity_is_consumed_once_across_safe_retries_and_restart() {
    let (temp, store, broker, time, run, epoch, proposal) = setup();
    let t = time.now_ms();
    broker
        .configure_window(&window(
            "repairs",
            c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            t - 1000,
            t + 86400000,
            amounts(1000, 100, 1000, 1),
        ))
        .unwrap();
    let root = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    broker.execute_issued_provider(&root, &Known).await.unwrap();
    let repair = c::CreativeRepairIdentity {
        repair_id: "repair-1".into(),
        previous_operation_id: "logical".into(),
        repair_index: 1,
    };
    broker
        .authorize_unit(
            &run,
            epoch,
            "repair-unit",
            "primary",
            "repair-op",
            Some(repair.clone()),
        )
        .unwrap();
    let mut changed = proposal.clone();
    changed.unit_id = "repair-unit".into();
    let issued = broker
        .bind_provider_proposal(&run, epoch, &changed.unit_id, &changed)
        .unwrap();
    assert_eq!(
        broker.quota_snapshot("repairs").unwrap().accounted.repairs,
        1
    );
    broker
        .execute_issued_provider(&issued, &Known)
        .await
        .unwrap();
    let retry = broker
        .retry_provider_proposal(
            &run,
            epoch,
            &changed.unit_id,
            &issued.operation.attempt_id,
            &changed,
        )
        .unwrap();
    assert_eq!(
        retry.identity.transport_retry.as_ref().unwrap().retry_index,
        1
    );
    let result = broker
        .execute_issued_provider(&retry, &Known)
        .await
        .unwrap();
    assert_eq!(result.accounted_usage.repairs, 0);
    assert_eq!(
        broker.quota_snapshot("repairs").unwrap().accounted.repairs,
        1
    );
    store.cancel(&run).unwrap();
    let reopened = Store::open(temp.path()).unwrap();
    reopened.recover().unwrap();
    let broker = Broker::synthetic(reopened.clone(), time.clone());
    let (run2, epoch2, proposal2) = next_run(&reopened, "repair-resume", &changed);
    broker
        .authorize_unit(
            &run2,
            epoch2,
            "repair-unit",
            "primary",
            "repair-op",
            Some(repair.clone()),
        )
        .unwrap();
    let again = broker
        .retry_provider_proposal(
            &run2,
            epoch2,
            &proposal2.unit_id,
            &retry.operation.attempt_id,
            &proposal2,
        )
        .unwrap();
    assert_eq!(
        again.identity.transport_retry.as_ref().unwrap().retry_index,
        2
    );
    assert_eq!(
        broker.quota_snapshot("repairs").unwrap().accounted.repairs,
        1
    );
    broker
        .authorize_unit(
            &run2,
            epoch2,
            "stolen-unit",
            "primary",
            "stolen-repair",
            Some(repair),
        )
        .unwrap();
    let mut stolen = proposal2;
    stolen.unit_id = "stolen-unit".into();
    assert_eq!(
        broker
            .bind_provider_proposal(&run2, epoch2, &stolen.unit_id, &stolen)
            .unwrap_err()
            .code,
        "repair_conflict"
    );
}

#[test]
fn explicit_legacy_alias_preserves_overlapping_charges_windows_and_retry_deadlines() {
    let (temp, store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let t = time.now_ms();
    store.allow_recipe("legacy-recipe").unwrap();
    store
        .configure_limit(
            "fake",
            "project",
            "model",
            &Limits {
                money: 100,
                requests: 10,
                tokens: 100,
                repairs: 4,
                window_start: t as i64 - 1000,
                reset_at: t as i64 + 100000,
            },
        )
        .unwrap();
    let payload = json!({"legacy":true});
    let legacy = Operation {
        identity: "legacy-attempt".into(),
        recipe: "legacy-recipe".into(),
        provider: "fake".into(),
        project: "project".into(),
        model: "model".into(),
        request_hash: digest(&serde_json::to_vec(&payload).unwrap()),
        payload,
        money: 25,
        requests: 1,
        tokens: 7,
        repair: false,
        timeout_ms: 1000,
    };
    let old_id = match store.reserve(&run, epoch, &legacy).unwrap() {
        Dispatch::Authorized(id) => id,
        _ => panic!("legacy fixture not reserved"),
    };
    store
        .import_accounting(&AccountingImport {
            original_hash: digest(b"inherited-cost"),
            derivative_hash: digest(b"inherited-cost-neutral"),
            scope: "fake/project/model".into(),
            money: 20,
            requests: 2,
            tokens: 30,
            repairs: 1,
            window_start: t as i64 - 5000,
            reset_at: t as i64 + 10000,
            retry_at: t as i64 + 1234,
            transformation: "Synthetic inherited reservation.".into(),
        })
        .unwrap();
    assert_eq!(
        broker
            .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
            .unwrap_err()
            .code,
        "accounting_unbound"
    );
    broker
        .bind_legacy_scope(
            "fake/project/model",
            &c::ProviderQuotaScope::Model {
                provider: "fake".into(),
                project_id: "project".into(),
                model: "model".into(),
            },
        )
        .unwrap();
    let snap = broker.quota_snapshot("project-day").unwrap();
    assert_eq!(snap.accounted, amounts(45, 3, 37, 1));
    assert_eq!(snap.retry_at_ms, Some(t + 1234));
    assert_eq!(
        broker
            .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
            .unwrap_err()
            .code,
        "provider_wait"
    );
    time.0.store(t + 1233, Ordering::SeqCst);
    assert_eq!(
        broker
            .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
            .unwrap_err()
            .code,
        "provider_wait"
    );
    time.0.store(t + 1234, Ordering::SeqCst);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    assert_eq!(issued.quota_windows.len(), 2);
    assert!(
        issued
            .quota_windows
            .iter()
            .any(|w| matches!(w.scope, c::ProviderQuotaScope::Model { .. })
                && w.capacity.requests == 10)
    );
    let reopened = Store::open(temp.path()).unwrap();
    reopened.recover().unwrap();
    let broker = Broker::synthetic(reopened.clone(), time);
    assert_eq!(
        broker.quota_snapshot("project-day").unwrap().accounted,
        amounts(45, 4, 42, 1)
    );
    let legacy_state: String = reopened
        .db()
        .unwrap()
        .query_row("SELECT state FROM attempts WHERE id=?1", [old_id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(legacy_state, "uncertain");
    let state: String = reopened
        .db()
        .unwrap()
        .query_row(
            "SELECT state FROM provider_v2_attempts WHERE id=?1",
            [issued.operation.attempt_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(state, "uncertain");
}

struct Limited {
    time: Arc<Time>,
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl OfflineTransport for Limited {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut reply = Known.send(op, bytes).await?;
        reply.retry = Some(Retry {
            reason: c::ProviderRetryReason::RateLimited,
            not_before_ms: self.time.now_ms() + 60000,
            retry_after_ms: Some(60000),
            window_ids: vec!["project-day".into()],
        });
        Ok(reply)
    }
}
#[tokio::test]
async fn genuine_retry_deadline_parks_and_survives_reopen_without_automatic_resubmission() {
    let (temp, store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let result = broker
        .execute_issued_provider(
            &issued,
            &Limited {
                time: time.clone(),
                calls: calls.clone(),
            },
        )
        .await
        .unwrap();
    let deadline = result.retry.unwrap().not_before_ms;
    assert_eq!(store.snapshot(&run).unwrap()["state"], "waiting_provider");
    let persisted: u64 = store
        .db()
        .unwrap()
        .query_row("SELECT retry_at FROM runs WHERE id=?1", [&run], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(persisted, deadline);
    let reopened = Store::open(temp.path()).unwrap();
    reopened.recover().unwrap();
    assert_eq!(
        reopened.snapshot(&run).unwrap()["state"],
        "waiting_provider"
    );
    let next_epoch = reopened.snapshot(&run).unwrap()["epoch"].as_i64().unwrap();
    let broker = Broker::synthetic(reopened.clone(), time.clone());
    broker
        .authorize_unit(&run, next_epoch, "unit", "primary", "logical", None)
        .unwrap();
    time.0.store(deadline - 1, Ordering::SeqCst);
    assert_eq!(
        broker
            .retry_provider_proposal(
                &run,
                next_epoch,
                &proposal.unit_id,
                &issued.operation.attempt_id,
                &proposal
            )
            .unwrap_err()
            .code,
        "provider_wait"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    time.0.store(deadline, Ordering::SeqCst);
    let retry = broker
        .retry_provider_proposal(
            &run,
            next_epoch,
            &proposal.unit_id,
            &issued.operation.attempt_id,
            &proposal,
        )
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    broker
        .execute_issued_provider(&retry, &Known)
        .await
        .unwrap();
    assert_eq!(
        broker.quota_snapshot("project-day").unwrap().retry_at_ms,
        Some(deadline)
    );
}

struct StatusOnly {
    send: Arc<AtomicUsize>,
    recover: Arc<AtomicUsize>,
    status: c::ProviderJobStatus,
}
#[async_trait]
impl OfflineTransport for StatusOnly {
    async fn send(&self, _: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        self.send.fetch_add(1, Ordering::SeqCst);
        panic!("recovery must never submit original work")
    }
    async fn recover(&self, op: &c::ProviderOperationV2) -> Result<RecoveryReply> {
        self.recover.fetch_add(1, Ordering::SeqCst);
        assert_eq!(op.recovery.as_ref().unwrap().provider_job_id, "job-1");
        let mut reply = Reply::ambiguous(op);
        reply.completion = c::ProviderCompletion::Completed;
        reply.job = Some(Job {
            id: "job-1".into(),
            status: self.status,
        });
        reply.raw = Some(format!("status={:?}; job=job-1", self.status).into_bytes());
        Ok(RecoveryReply {
            original: reply,
            query_usage: c::ObservedProviderUsage {
                requests: Some(1),
                tokens: Some(0),
                input_tokens: Some(0),
                output_tokens: Some(0),
                money_microusd: Some(0),
            },
        })
    }
}
#[tokio::test]
async fn recovery_uses_separate_request_reservation_never_send_and_preserves_unknown_original_cost()
{
    let (temp, store, broker, time, run, epoch, mut proposal) = setup();
    project_window(&broker, &time);
    proposal.estimate.money_microusd = 10;
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let seed_calls = Arc::new(AtomicUsize::new(0));
    broker
        .execute_issued_provider(
            &issued,
            &Unknown {
                calls: seed_calls.clone(),
                time: time.clone(),
            },
        )
        .await
        .unwrap();
    store.cancel(&run).unwrap();
    let reopened = Store::open(temp.path()).unwrap();
    reopened.recover().unwrap();
    let broker = Broker::synthetic(reopened.clone(), time.clone());
    let (observer, observer_epoch, _) = next_run(&reopened, "status-observer", &proposal);
    broker
        .authorize_unit(
            &observer,
            observer_epoch,
            "poll-1",
            "primary",
            "poll-1",
            None,
        )
        .unwrap();
    let recovery = broker
        .recovery_descriptor(&issued.operation.attempt_id)
        .unwrap();
    let send = Arc::new(AtomicUsize::new(0));
    let queries = Arc::new(AtomicUsize::new(0));
    let fake = StatusOnly {
        send: send.clone(),
        recover: queries.clone(),
        status: c::ProviderJobStatus::Unknown,
    };
    let t = issued.quota_windows[0].start_at_ms;
    broker
        .configure_window(&window(
            "project-day",
            c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            t,
            issued.quota_windows[0].reset_at_ms,
            amounts(1000, 1, 1000, 100),
        ))
        .unwrap();
    assert_eq!(
        broker
            .recover_existing_provider(&observer, observer_epoch, "poll-1", &recovery, &fake)
            .await
            .unwrap_err()
            .code,
        "request_quota"
    );
    assert_eq!(queries.load(Ordering::SeqCst), 0);
    broker
        .configure_window(&window(
            "project-day",
            c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            t,
            issued.quota_windows[0].reset_at_ms,
            amounts(1000, 100, 1000, 100),
        ))
        .unwrap();
    let result = broker
        .recover_existing_provider(&observer, observer_epoch, "poll-1", &recovery, &fake)
        .await
        .unwrap();
    result.validate_for_operation(&recovery).unwrap();
    assert_eq!(result.result.completion, c::ProviderCompletion::Ambiguous);
    assert_eq!(
        result.recovery.as_ref().unwrap().status,
        c::ProviderJobStatus::Unknown
    );
    assert_eq!(result.observed_usage.money_microusd, None);
    assert!(result.result.usage.is_none());
    assert_eq!(result.accounted_usage.money_microusd, 10);
    assert_eq!(
        result.epoch_disposition,
        c::ProviderEpochDisposition::Obsolete
    );
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        2
    );
    broker
        .recover_existing_provider(&observer, observer_epoch, "poll-1", &recovery, &fake)
        .await
        .unwrap();
    assert_eq!(send.load(Ordering::SeqCst), 0);
    assert_eq!(queries.load(Ordering::SeqCst), 1);
    assert_eq!(seed_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        broker.raw_receipt(&issued.operation.attempt_id, 1).unwrap(),
        b"opaque fake provider response"
    );
    assert_eq!(
        broker.raw_receipt(&issued.operation.attempt_id, 2).unwrap(),
        b"status=Unknown; job=job-1"
    );
    broker
        .authorize_unit(
            &observer,
            observer_epoch,
            "poll-2",
            "primary",
            "poll-2",
            None,
        )
        .unwrap();
    let recovery = broker
        .recovery_descriptor(&issued.operation.attempt_id)
        .unwrap();
    let succeeded = StatusOnly {
        send: send.clone(),
        recover: queries.clone(),
        status: c::ProviderJobStatus::Succeeded,
    };
    let result = broker
        .recover_existing_provider(&observer, observer_epoch, "poll-2", &recovery, &succeeded)
        .await
        .unwrap();
    assert_eq!(
        result.recovery.unwrap().status,
        c::ProviderJobStatus::Succeeded
    );
    assert_eq!(result.observed_usage.money_microusd, None);
    assert_eq!(result.accounted_usage.money_microusd, 10);
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        3
    );
    assert_eq!(send.load(Ordering::SeqCst), 0);
    assert_eq!(queries.load(Ordering::SeqCst), 2);
    assert_eq!(reopened.snapshot(&run).unwrap()["state"], "cancelled");
    assert!(
        reopened
            .api_snapshot(&run)
            .unwrap()
            .stages
            .iter()
            .all(|s| s.artifacts.is_empty())
    );
    assert!(reopened.catalog("pg-55").unwrap().is_none());
}

struct Late {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl OfflineTransport for Late {
    async fn send(&self, op: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.started.notify_one();
        self.release.notified().await;
        let mut reply = Reply::ambiguous(op);
        reply.completion = c::ProviderCompletion::Completed;
        reply.usage = c::ObservedProviderUsage {
            requests: Some(1),
            tokens: Some(5),
            input_tokens: Some(2),
            output_tokens: Some(3),
            money_microusd: Some(17),
        };
        reply.raw = Some(b"actual late fake receipt: 17".to_vec());
        Ok(reply)
    }
}
#[tokio::test]
async fn late_receipt_keeps_original_identity_bytes_and_overage_without_reviving_cancelled_work() {
    let (_temp, store, broker, time, run, epoch, mut proposal) = setup();
    project_window(&broker, &time);
    proposal.estimate.money_microusd = 10;
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    time.0.fetch_add(50, Ordering::SeqCst);
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let fake = Arc::new(Late {
        started: started.clone(),
        release: release.clone(),
        calls: calls.clone(),
    });
    let task = {
        let broker = broker.clone();
        let issued = issued.clone();
        tokio::spawn(async move { broker.execute_issued_provider(&issued, fake.as_ref()).await })
    };
    started.notified().await;
    store.cancel(&run).unwrap();
    let stage_before = serde_json::to_value(store.api_snapshot(&run).unwrap().stages).unwrap();
    let (new_run, new_epoch, _) = next_run(&store, "new-run-during-old-transport", &proposal);
    time.0.fetch_add(137, Ordering::SeqCst);
    release.notify_one();
    let result = task.await.unwrap().unwrap();
    assert_eq!(result.identity, issued.identity);
    assert_eq!(
        result.epoch_disposition,
        c::ProviderEpochDisposition::Obsolete
    );
    assert_eq!(result.accounted_usage.money_microusd, 17);
    assert_eq!(result.recorded_at_ms - result.started_at_ms, 137);
    assert_eq!(
        broker.raw_receipt(&issued.operation.attempt_id, 1).unwrap(),
        b"actual late fake receipt: 17"
    );
    assert_eq!(store.snapshot(&run).unwrap()["state"], "cancelled");
    assert_eq!(store.snapshot(&new_run).unwrap()["state"], "running");
    assert_eq!(store.snapshot(&new_run).unwrap()["epoch"], new_epoch);
    assert_eq!(
        serde_json::to_value(store.api_snapshot(&run).unwrap().stages).unwrap(),
        stage_before
    );
    assert!(
        !store
            .journal(&run, 0, 500)
            .unwrap()
            .iter()
            .any(|e| matches!(
                e["type"].as_str(),
                Some(
                    "artifact_ready"
                        | "task_completed"
                        | "library_bound"
                        | "diagnostic_package_ready"
                )
            ))
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

fn distinct_payload(
    store: &Store,
    run: &str,
    proposal: &c::ProviderOperation,
    label: &str,
) -> c::ProviderOperation {
    let mut proposal = proposal.clone();
    let bytes = format!("{{\"offline\":\"{label}\"}}").into_bytes();
    proposal.payload.id = digest(format!("{run}:{label}").as_bytes());
    proposal.payload.file.sha256 = store.put(&bytes).unwrap();
    proposal.payload.file.bytes = bytes.len() as u64;
    proposal.request_sha256 = proposal.payload.file.sha256.clone();
    store
        .transaction(|tx| {
            tx.execute(
                "INSERT INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,'synthetic',?7)",
                rusqlite::params![
                    proposal.payload.id,
                    run,
                    proposal.payload.file.path,
                    proposal.payload.file.sha256,
                    proposal.payload.file.bytes,
                    proposal.payload.file.media_type,
                    serde_json::to_string(&proposal.payload)?
                ],
            )?;
            Ok(())
        })
        .unwrap();
    proposal
}
#[tokio::test]
async fn parallel_distinct_attempts_cannot_oversubscribe_any_applicable_window() {
    let (_temp, store, broker, time, run, epoch, proposal) = setup();
    let t = time.now_ms();
    let scopes = [
        (
            "provider-minute",
            c::ProviderQuotaScope::Provider {
                provider: "fake".into(),
            },
            60000,
            2,
        ),
        (
            "project-day",
            c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            86400000,
            1,
        ),
        (
            "model-minute",
            c::ProviderQuotaScope::Model {
                provider: "fake".into(),
                project_id: "project".into(),
                model: "model".into(),
            },
            60000,
            3,
        ),
        (
            "model-day",
            c::ProviderQuotaScope::Model {
                provider: "fake".into(),
                project_id: "project".into(),
                model: "model".into(),
            },
            86400000,
            5,
        ),
    ];
    for (name, scope, duration, requests) in scopes {
        broker
            .configure_window(&window(
                name,
                scope,
                t - 1000,
                t + duration,
                amounts(1000, requests, 1000, 10),
            ))
            .unwrap();
    }
    broker
        .authorize_unit(&run, epoch, "unit-b", "primary", "logical-b", None)
        .unwrap();
    let mut b = distinct_payload(&store, &run, &proposal, "parallel-b");
    b.unit_id = "unit-b".into();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut jobs = vec![];
    for proposal in [proposal, b] {
        let broker = broker.clone();
        let run = run.clone();
        let barrier = barrier.clone();
        jobs.push(tokio::task::spawn_blocking(move || {
            barrier.wait();
            broker.bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        }));
    }
    let mut issued = vec![];
    let mut errors = vec![];
    for job in jobs {
        match job.await.unwrap() {
            Ok(op) => issued.push(op),
            Err(e) => errors.push(e.code),
        }
    }
    assert_eq!(issued.len(), 1);
    assert_eq!(errors, vec!["request_quota"]);
    for name in [
        "provider-minute",
        "project-day",
        "model-minute",
        "model-day",
    ] {
        assert_eq!(broker.quota_snapshot(name).unwrap().accounted.requests, 1);
        assert_eq!(broker.quota_snapshot(name).unwrap().accounted.tokens, 5);
    }
    broker
        .execute_issued_provider(&issued[0], &Known)
        .await
        .unwrap();
}

#[test]
fn money_requests_and_tokens_are_independent_and_failed_reservations_do_not_charge() {
    for (capacity, money, code) in [
        (amounts(0, 100, 100, 100), 1, "budget_exhausted"),
        (amounts(1000, 0, 100, 100), 0, "request_quota"),
        (amounts(1000, 100, 4, 100), 0, "token_quota"),
    ] {
        let (_temp, _store, broker, time, run, epoch, mut proposal) = setup();
        proposal.estimate.money_microusd = money;
        let t = time.now_ms();
        broker
            .configure_window(&window(
                "limited",
                c::ProviderQuotaScope::Project {
                    provider: "fake".into(),
                    project_id: "project".into(),
                },
                t - 1000,
                t + 60000,
                capacity,
            ))
            .unwrap();
        assert_eq!(
            broker
                .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
                .unwrap_err()
                .code,
            code
        );
        assert_eq!(
            broker.quota_snapshot("limited").unwrap().accounted,
            amounts(0, 0, 0, 0)
        );
    }
}
#[tokio::test]
async fn fixture_replay_and_default_broker_reject_before_credential_context_lookup() {
    let (_temp, store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let fake = Unknown {
        calls: Arc::new(AtomicUsize::new(0)),
        time,
    };
    let mut request = store.snapshot(&run).unwrap()["request"].clone();
    // Poison registry records: entering either forbidden mode must still reject first.
    store
        .transaction(|tx| {
            tx.execute("UPDATE provider_v2_bindings SET record='not-json'", [])?;
            Ok(())
        })
        .unwrap();
    for mode in ["fixture", "replay"] {
        request["mode"] = json!(mode);
        store
            .transaction(|tx| {
                tx.execute(
                    "UPDATE runs SET request=?2 WHERE id=?1",
                    rusqlite::params![run, request.to_string()],
                )?;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            broker
                .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
                .unwrap_err()
                .code,
            "live_disabled"
        );
        assert_eq!(
            broker
                .execute_issued_provider(&issued, &fake)
                .await
                .unwrap_err()
                .code,
            "live_disabled"
        );
    }
    request["mode"] = json!("live");
    store
        .transaction(|tx| {
            tx.execute(
                "UPDATE runs SET request=?2 WHERE id=?1",
                rusqlite::params![run, request.to_string()],
            )?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        Broker::new(store.clone())
            .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
            .unwrap_err()
            .code,
        "live_disabled"
    );
    assert_eq!(
        Broker::new(store.clone())
            .execute_issued_provider(&issued, &fake)
            .await
            .unwrap_err()
            .code,
        "live_disabled"
    );
    assert_eq!(
        store.create("externally-live", &request).unwrap_err().code,
        "live_disabled"
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}
struct Never(Arc<AtomicUsize>);
#[async_trait]
impl OfflineTransport for Never {
    async fn send(&self, _: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        self.0.fetch_add(1, Ordering::SeqCst);
        std::future::pending().await
    }
}
#[tokio::test]
async fn timeout_and_restart_hold_unknown_usage_and_never_invent_a_recovery_job() {
    let (temp, _store, broker, time, run, epoch, mut proposal) = setup();
    project_window(&broker, &time);
    proposal.timeout_ms = 5;
    proposal.estimate.money_microusd = 10;
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let result = broker
        .execute_issued_provider(&issued, &Never(calls.clone()))
        .await
        .unwrap();
    assert_eq!(result.result.completion, c::ProviderCompletion::Ambiguous);
    assert_eq!(result.accounted_usage.money_microusd, 10);
    assert!(result.result.raw_response.is_none());
    assert!(result.recovery.is_none());
    assert_eq!(
        broker
            .recovery_descriptor(&issued.operation.attempt_id)
            .unwrap_err()
            .code,
        "recovery_unknown"
    );
    let reopened = Store::open(temp.path()).unwrap();
    reopened.recover().unwrap();
    let (_, epoch) = reopened.claim().unwrap().unwrap();
    let broker = Broker::synthetic(reopened.clone(), time);
    broker
        .authorize_unit(&run, epoch, "unit", "primary", "logical", None)
        .unwrap();
    assert_eq!(
        broker
            .retry_provider_proposal(
                &run,
                epoch,
                &proposal.unit_id,
                &issued.operation.attempt_id,
                &proposal
            )
            .unwrap_err()
            .code,
        "attempt_unresolved"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .money_microusd,
        10
    );
}

#[test]
fn issued_window_metadata_keeps_public_journal_summaries_bounded() {
    let (_temp, store, broker, time, run, epoch, proposal) = setup();
    let t = time.now_ms();
    for i in 0..16 {
        broker
            .configure_window(&window(
                &format!("{}-{i}", "window".repeat(20)),
                c::ProviderQuotaScope::Project {
                    provider: "fake".into(),
                    project_id: "project".into(),
                },
                t - 1000,
                t + (i + 1) * 60000,
                amounts(1000, 100, 1000, 100),
            ))
            .unwrap();
    }
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    assert_eq!(issued.quota_windows.len(), 16);
    for event in store.api_journal(&run, 0, 500).unwrap() {
        assert!(
            event.entry.summary.len() <= 4000,
            "metadata escaped the bounded journal surface"
        );
    }
}

struct MissingStatus {
    send: Arc<AtomicUsize>,
    recover: Arc<AtomicUsize>,
    time: Arc<Time>,
}
#[async_trait]
impl OfflineTransport for MissingStatus {
    async fn send(&self, _: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        self.send.fetch_add(1, Ordering::SeqCst);
        panic!("no original send during status inspection")
    }
    async fn recover(&self, op: &c::ProviderOperationV2) -> Result<RecoveryReply> {
        self.recover.fetch_add(1, Ordering::SeqCst);
        self.time.0.fetch_add(9, Ordering::SeqCst);
        let mut reply = Reply::ambiguous(op);
        reply.completion = c::ProviderCompletion::Completed;
        reply.raw = Some(b"status unavailable; usage unavailable".to_vec());
        Ok(RecoveryReply {
            original: reply,
            query_usage: Reply::ambiguous(op).usage,
        })
    }
}
#[tokio::test]
async fn successful_status_transport_does_not_invent_underlying_job_success_or_usage() {
    let (_temp, _store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    broker
        .execute_issued_provider(
            &issued,
            &Unknown {
                calls: Arc::new(AtomicUsize::new(0)),
                time: time.clone(),
            },
        )
        .await
        .unwrap();
    let op = broker
        .recovery_descriptor(&issued.operation.attempt_id)
        .unwrap();
    let observed = op.recovery.as_ref().unwrap().observed_at_ms;
    broker
        .authorize_unit(&run, epoch, "poll", "primary", "poll", None)
        .unwrap();
    let send = Arc::new(AtomicUsize::new(0));
    let queries = Arc::new(AtomicUsize::new(0));
    let result = broker
        .recover_existing_provider(
            &run,
            epoch,
            "poll",
            &op,
            &MissingStatus {
                send: send.clone(),
                recover: queries.clone(),
                time: time.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.result.completion, c::ProviderCompletion::Ambiguous);
    assert_eq!(
        result.recovery.as_ref().unwrap().status,
        c::ProviderJobStatus::Unknown
    );
    assert_eq!(result.recovery.as_ref().unwrap().observed_at_ms, observed);
    assert!(result.observed_usage.requests.is_none());
    assert!(result.result.usage.is_none());
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        2
    );
    assert_eq!(send.load(Ordering::SeqCst), 0);
    assert_eq!(queries.load(Ordering::SeqCst), 1);
}

struct Partial(bool);
#[async_trait]
impl OfflineTransport for Partial {
    async fn send(&self, op: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        let mut reply = Reply::ambiguous(op);
        if self.0 {
            reply.usage = c::ObservedProviderUsage {
                requests: Some(0),
                tokens: Some(0),
                input_tokens: Some(0),
                output_tokens: Some(0),
                money_microusd: Some(0),
            };
        } else {
            reply.completion = c::ProviderCompletion::Completed;
            reply.usage = c::ObservedProviderUsage {
                requests: Some(1),
                tokens: Some(3),
                input_tokens: None,
                output_tokens: None,
                money_microusd: None,
            };
        }
        Ok(reply)
    }
}
#[tokio::test]
async fn partial_observations_and_ambiguous_zero_usage_never_manufacture_known_splits_or_refunds() {
    for ambiguous in [false, true] {
        let (_temp, _store, broker, time, run, epoch, mut proposal) = setup();
        project_window(&broker, &time);
        proposal.estimate.money_microusd = 10;
        let issued = broker
            .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
            .unwrap();
        let result = broker
            .execute_issued_provider(&issued, &Partial(ambiguous))
            .await
            .unwrap();
        result.validate_for_operation(&issued).unwrap();
        assert_eq!(result.accounted_usage.money_microusd, 10);
        assert_eq!(result.accounted_usage.requests, 1);
        if ambiguous {
            assert_eq!(result.accounted_usage.tokens, 5);
            assert_eq!(result.observed_usage.money_microusd, Some(0));
        } else {
            assert_eq!(result.accounted_usage.tokens, 3);
            assert_eq!(result.observed_usage.tokens, Some(3));
            assert!(result.observed_usage.input_tokens.is_none());
            assert!(result.observed_usage.output_tokens.is_none());
            assert!(result.result.usage.is_none());
        }
    }
}
#[tokio::test]
async fn descriptor_conflicts_corrupt_payload_and_cancel_before_send_do_not_dispatch() {
    let (_temp, store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let mut renamed = proposal.clone();
    renamed.attempt_id = "another-caller-attempt".into();
    assert_eq!(
        broker
            .bind_provider_proposal(&run, epoch, &renamed.unit_id, &renamed)
            .unwrap()
            .operation
            .attempt_id,
        issued.operation.attempt_id
    );
    renamed.estimate.money_microusd = 1;
    assert_eq!(
        broker
            .bind_provider_proposal(&run, epoch, &renamed.unit_id, &renamed)
            .unwrap_err()
            .code,
        "operation_conflict"
    );
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        1
    );
    let fake = Unknown {
        calls: Arc::new(AtomicUsize::new(0)),
        time,
    };
    let mut forged = issued.clone();
    forged.context.credential_context_id = "unissued-slot".into();
    assert_eq!(
        broker
            .execute_issued_provider(&forged, &fake)
            .await
            .unwrap_err()
            .code,
        "issued_descriptor_mismatch"
    );
    std::fs::write(
        store.object(&proposal.payload.file.sha256).unwrap(),
        b"corrupt",
    )
    .unwrap();
    assert_eq!(
        broker
            .execute_issued_provider(&issued, &fake)
            .await
            .unwrap_err()
            .code,
        "corrupt_artifact"
    );
    store.cancel(&run).unwrap();
    assert_eq!(
        broker
            .execute_issued_provider(&issued, &fake)
            .await
            .unwrap_err()
            .code,
        "stale_epoch"
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn expired_or_new_applicable_limits_cannot_turn_an_old_reservation_into_a_send() {
    let (_temp, _store, broker, time, run, epoch, proposal) = setup();
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let t = time.now_ms();
    broker
        .configure_window(&window(
            "new-provider-window",
            c::ProviderQuotaScope::Provider {
                provider: "fake".into(),
            },
            t - 1000,
            t + 60000,
            amounts(1000, 100, 1000, 100),
        ))
        .unwrap();
    let fake = Unknown {
        calls: Arc::new(AtomicUsize::new(0)),
        time: time.clone(),
    };
    assert_eq!(
        broker
            .execute_issued_provider(&issued, &fake)
            .await
            .unwrap_err()
            .code,
        "limits_changed"
    );
    time.0
        .store(issued.quota_windows[0].reset_at_ms, Ordering::SeqCst);
    assert_eq!(
        broker
            .execute_issued_provider(&issued, &fake)
            .await
            .unwrap_err()
            .code,
        "limits_expired"
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        1
    );
}

struct Step(AtomicU64);
impl Clock for Step {
    fn now_ms(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst)
    }
}
struct Observed;
#[async_trait]
impl OfflineTransport for Observed {
    async fn send(&self, op: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        let mut reply = Reply::ambiguous(op);
        reply.raw = Some(b"genuine fake observation before storage".to_vec());
        reply.job = Some(Job {
            id: "observed-job".into(),
            status: c::ProviderJobStatus::Unknown,
        });
        reply.retry = Some(Retry {
            reason: c::ProviderRetryReason::UnknownCompletion,
            not_before_ms: 0,
            retry_after_ms: Some(25),
            window_ids: vec![],
        });
        Ok(reply)
    }
}
#[tokio::test]
async fn observation_time_precedes_storage_and_relative_retry_is_not_rebased_on_recording() {
    let (_temp, store, _broker, time, run, epoch, proposal) = setup();
    let clock = Arc::new(Step(AtomicU64::new(time.now_ms())));
    let broker = Broker::synthetic(store, clock);
    project_window(&broker, &time);
    let issued = broker
        .bind_provider_proposal(&run, epoch, &proposal.unit_id, &proposal)
        .unwrap();
    let result = broker
        .execute_issued_provider(&issued, &Observed)
        .await
        .unwrap();
    let observed = result.recovery.unwrap().observed_at_ms;
    assert!(
        observed < result.recorded_at_ms,
        "storage time was mislabeled as transport observation time"
    );
    assert_eq!(result.retry.unwrap().not_before_ms, observed + 25);
}

#[test]
fn free_status_queries_cannot_be_registered_with_zero_request_capacity() {
    let (_temp, _store, broker, _time, _run, _epoch, proposal) = setup();
    let binding = Binding {
        id: "zero-query-cost".into(),
        context: c::ProviderContext {
            provider: "fake".into(),
            project_id: "project".into(),
            credential_context_id: "slot-a".into(),
        },
        model: "model".into(),
        recipe_sha256: proposal.recipe_sha256,
        kind: proposal.kind,
        recovery_estimate: c::Usage::default(),
    };
    assert_eq!(
        broker.register(&binding).unwrap_err().code,
        "provider_binding"
    );
}

#[test]
fn caller_unit_id_cannot_select_another_runtime_authorized_unit() {
    let (_temp, _store, broker, time, run, epoch, mut proposal) = setup();
    project_window(&broker, &time);
    broker
        .authorize_unit(
            &run,
            epoch,
            "other-unit",
            "primary",
            "other-operation",
            None,
        )
        .unwrap();
    proposal.unit_id = "other-unit".into();
    assert_eq!(
        broker
            .bind_provider_proposal(&run, epoch, "unit", &proposal)
            .unwrap_err()
            .code,
        "provider_binding"
    );
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        0
    );
}
