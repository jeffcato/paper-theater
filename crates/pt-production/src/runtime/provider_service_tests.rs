use super::*;
use crate::runtime::provider_v2::{Binding, Broker, Clock, OfflineTransport, Reply, Window};
use c::RunServices;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
struct Time(AtomicU64);
impl Clock for Time {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
struct Fake {
    sends: AtomicUsize,
    queries: AtomicUsize,
    time: Arc<Time>,
}
#[async_trait]
impl OfflineTransport for Fake {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        assert_eq!(bytes, b"{ \"offline\": true }\n");
        self.sends.fetch_add(1, Ordering::SeqCst);
        self.time.0.fetch_add(7, Ordering::SeqCst);
        let mut reply = Reply::ambiguous(op);
        reply.completion = c::ProviderCompletion::Completed;
        reply.raw = Some(b"offline receipt".to_vec());
        reply.usage = c::ObservedProviderUsage {
            requests: Some(1),
            tokens: Some(5),
            input_tokens: Some(2),
            output_tokens: Some(3),
            money_microusd: Some(0),
        };
        Ok(reply)
    }
}
struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    broker: Broker,
    time: Arc<Time>,
    services: Services,
    proposal: c::ProviderProposalV2,
    transport: Arc<Fake>,
}
fn setup() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let mut request = json!({"schema_version":1,"book_id":"pg-55","source_id":"pg-55-chapter-1","mode":"fixture","pins":{"recipe_sha256":digest(b"offline-recipe"),"profile_sha256":digest(b"profile"),"profile_id":"diagnostic"},"limits":{"money_microusd":1000,"requests":100,"tokens":1000,"repairs":10,"wall_time_ms":60000}});
    let run = store.create("provider-service-v2", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    // Isolated test authorization and synthetic accounting only. No public/live constructor.
    request["mode"] = json!("live");
    store
        .db()
        .unwrap()
        .execute(
            "UPDATE runs SET request=?1 WHERE id=?2",
            params![request.to_string(), run],
        )
        .unwrap();
    store
        .import_accounting(&super::super::broker::AccountingImport {
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
            transformation: "Offline injected service test; no actual product authorization."
                .into(),
        })
        .unwrap();
    store
        .authorize_live(&digest(b"synthetic-proof"), 1000)
        .unwrap();
    let body = b"{ \"offline\": true }\n";
    let hash = store.put(body).unwrap();
    let payload = c::ArtifactRef {
        id: digest(b"service-payload"),
        file: c::FileRef {
            path: "request.json".into(),
            sha256: hash,
            bytes: body.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        },
        provenance: c::Provenance::Synthetic,
    };
    store
        .db()
        .unwrap()
        .execute(
            "INSERT INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,'synthetic',?7)",
            params![
                payload.id,
                run,
                payload.file.path,
                payload.file.sha256,
                payload.file.bytes,
                payload.file.media_type,
                serde_json::to_string(&payload).unwrap()
            ],
        )
        .unwrap();
    let time = Arc::new(Time(AtomicU64::new(now() as u64)));
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
                input_tokens: 2,
                output_tokens: 1,
                money_microusd: 4,
            },
        })
        .unwrap();
    broker
        .authorize_unit(&run, epoch, "unit", "primary", "logical", None)
        .unwrap();
    let t = time.now_ms();
    broker
        .configure_window(&Window {
            id: "project-day".into(),
            scope: c::ProviderQuotaScope::Project {
                provider: "fake".into(),
                project_id: "project".into(),
            },
            start_at_ms: t - 1000,
            reset_at_ms: t + 86400000,
            capacity: c::QuotaAmounts {
                money_microusd: 1000,
                requests: 100,
                tokens: 1000,
                repairs: 10,
            },
        })
        .unwrap();
    let proposal = c::ProviderProposalV2 {
        schema_version: 2,
        kind: c::OperationKind::Direction,
        provider: "fake".into(),
        model: "model".into(),
        recipe_sha256: digest(b"offline-recipe"),
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
    let transport = Arc::new(Fake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: time.clone(),
    });
    let services = Services {
        store: store.clone(),
        run,
        epoch,
        mode: c::ExecutionMode::Live,
        unit: "unit".into(),
        inputs: Arc::new(vec![proposal.payload.clone()]),
        provider_v2: Some(super::super::provider_service::Adapter::injected(
            broker.clone(),
            transport.clone(),
        )),
        candidates: Arc::new(Mutex::new(HashMap::new())),
        proposals: Arc::new(Mutex::new(Vec::new())),
        offline_acoustic: None,
    };
    Fixture {
        _temp: temp,
        store,
        broker,
        time,
        services,
        proposal,
        transport,
    }
}
#[tokio::test]
async fn actual_service_issues_exact_persisted_descriptor_without_dispatch() {
    let f = setup();
    let issued = f
        .services
        .issue_provider_v2(f.proposal.clone())
        .await
        .expect("actual Services must issue through the persistent broker");
    issued
        .validate_for_proposal(
            &f.proposal,
            &c::ProviderObserverV2 {
                schema_version: 2,
                run_id: f.services.run.clone(),
                epoch: f.services.epoch as u64,
                unit_id: "unit".into(),
            },
        )
        .unwrap();
    let saved: String = f
        .store
        .db()
        .unwrap()
        .query_row("SELECT descriptor FROM provider_v2_attempts", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(saved, serde_json::to_string(&issued).unwrap());
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 0);
    assert_eq!(f.transport.queries.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        1
    );
}
#[tokio::test]
async fn exact_issued_execution_sends_original_bytes_once_and_returns_the_cached_receipt() {
    let f = setup();
    let issued = f
        .services
        .issue_provider_v2(f.proposal.clone())
        .await
        .unwrap();
    let first = f
        .services
        .execute_provider_v2(&issued)
        .await
        .expect("actual Services must execute exact persisted issuance");
    first.validate_for_operation(&issued).unwrap();
    let second = f.services.execute_provider_v2(&issued).await.unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&second).unwrap()
    );
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 1);
    assert_eq!(f.transport.queries.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .tokens,
        5
    );
    assert_eq!(
        f.broker
            .raw_receipt(&issued.operation.attempt_id, 1)
            .unwrap(),
        b"offline receipt"
    );
    assert!(f.services.candidates.lock().unwrap().is_empty());
    assert!(f.store.catalog("pg-55").unwrap().is_none());
}
struct Poison(Arc<AtomicUsize>);
#[async_trait]
impl OfflineTransport for Poison {
    async fn send(&self, _: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("offline mode reached a poisoned credential lookup")
    }
    async fn recover(
        &self,
        _: &c::ProviderOperationV2,
    ) -> Result<super::super::provider_v2::RecoveryReply> {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("offline recovery reached a poisoned credential lookup")
    }
}
#[tokio::test]
async fn offline_modes_and_absent_configuration_preserve_typed_errors_before_any_access() {
    let f = setup();
    let issued = f
        .services
        .issue_provider_v2(f.proposal.clone())
        .await
        .unwrap();
    let poison = Arc::new(AtomicUsize::new(0));
    let mut service = f.services.clone();
    service.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        Arc::new(Poison(poison.clone())),
    ));
    let before = f.store.snapshot(&service.run).unwrap();
    for mode in [c::ExecutionMode::Fixture, c::ExecutionMode::Replay] {
        service.mode = mode;
        assert!(matches!(
            service.issue_provider_v2(f.proposal.clone()).await,
            Err(c::ServiceError::OfflineDispatch)
        ));
        assert!(matches!(
            service.execute_provider_v2(&issued).await,
            Err(c::ServiceError::OfflineDispatch)
        ));
        assert!(matches!(
            service
                .recover_provider_v2(c::ProviderRecoveryRequestV2 {
                    schema_version: 2,
                    original: issued.clone()
                })
                .await,
            Err(c::ServiceError::OfflineDispatch)
        ));
    }
    service.mode = c::ExecutionMode::Live;
    service.provider_v2 = None;
    assert!(matches!(
        service.issue_provider_v2(f.proposal).await,
        Err(c::ServiceError::ProviderV2Unavailable)
    ));
    assert!(matches!(
        service.execute_provider_v2(&issued).await,
        Err(c::ServiceError::ProviderV2Unavailable)
    ));
    assert!(matches!(
        service
            .recover_provider_v2(c::ProviderRecoveryRequestV2 {
                schema_version: 2,
                original: issued
            })
            .await,
        Err(c::ServiceError::ProviderV2Unavailable)
    ));
    assert_eq!(poison.load(Ordering::SeqCst), 0);
    assert_eq!(f.store.snapshot(&service.run).unwrap(), before);
}
#[tokio::test]
async fn full_descriptor_and_execution_scope_reject_forged_authority_without_fallback() {
    let f = setup();
    let issued = f
        .services
        .issue_provider_v2(f.proposal.clone())
        .await
        .unwrap();
    let before = f.store.snapshot(&f.services.run).unwrap();
    for change in 0..9 {
        let mut bad = issued.clone();
        match change {
            0 => bad.identity.run_id = "another-run".into(),
            1 => bad.identity.epoch += 1,
            2 => bad.operation.unit_id = "another-unit".into(),
            3 => bad.identity.operation_id = "caller-logical".into(),
            4 => bad.context.credential_context_id = "caller-slot".into(),
            5 => bad.operation.estimate.money_microusd = 1,
            6 => bad.quota_windows[0].capacity.requests += 1,
            7 => bad.operation.attempt_id = digest(b"never issued"),
            _ => {
                bad.recovery = Some(c::ExistingProviderJob {
                    attempt_id: bad.operation.attempt_id.clone(),
                    request_sha256: bad.operation.request_sha256.clone(),
                    provider_job_id: "caller-job".into(),
                    status: c::ProviderJobStatus::Running,
                    observed_at_ms: f.time.now_ms(),
                })
            }
        }
        assert!(
            matches!(
                f.services.execute_provider_v2(&bad).await,
                Err(c::ServiceError::Unauthorized(_))
            ),
            "field {change}"
        );
    }
    let mut other = f.services.clone();
    other.unit = "another-unit".into();
    assert!(matches!(
        other.issue_provider_v2(f.proposal.clone()).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    assert!(matches!(
        other.execute_provider_v2(&issued).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    other = f.services.clone();
    other.epoch += 1;
    assert!(matches!(
        other.execute_provider_v2(&issued).await,
        Err(c::ServiceError::Cancelled)
    ));
    let mut bad = f.proposal.clone();
    bad.inputs.push(c::ArtifactRef {
        id: digest(b"ungranted"),
        ..bad.payload.clone()
    });
    assert!(matches!(
        f.services.issue_provider_v2(bad).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 0);
    assert_eq!(f.transport.queries.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(f.store.snapshot(&f.services.run).unwrap(), before);
}
#[tokio::test]
async fn changed_original_bytes_and_foreign_committed_artifacts_cannot_issue_send_or_redeem_cache()
{
    let f = setup();
    let issued = f
        .services
        .issue_provider_v2(f.proposal.clone())
        .await
        .unwrap();
    let path = f.store.object(&f.proposal.payload.file.sha256).unwrap();
    let saved = std::fs::read(&path).unwrap();
    let mut changed = saved.clone();
    changed[0] = b'[';
    std::fs::write(&path, &changed).unwrap();
    assert!(
        f.services
            .issue_provider_v2(f.proposal.clone())
            .await
            .is_err()
    );
    assert!(f.services.execute_provider_v2(&issued).await.is_err());
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 0);
    std::fs::write(&path, &saved).unwrap();
    f.services.execute_provider_v2(&issued).await.unwrap();
    std::fs::write(&path, &changed).unwrap();
    assert!(f.services.execute_provider_v2(&issued).await.is_err());
    std::fs::write(&path, saved).unwrap();
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 1);
    // An artifact can exist in SQLite and still be outside this run's authority.
    let mut request = f.store.snapshot(&f.services.run).unwrap()["request"].clone();
    request["mode"] = json!("fixture");
    let other = f.store.create("foreign-owner", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    f.store
        .db()
        .unwrap()
        .execute(
            "UPDATE artifacts SET run=?1 WHERE id=?2",
            params![other, f.proposal.payload.id],
        )
        .unwrap();
    assert!(matches!(
        f.services.issue_provider_v2(f.proposal).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM provider_v2_attempts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 1);
    assert_eq!(f.transport.queries.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn current_run_and_lifetime_authorization_limits_are_rechecked_before_send() {
    for lifetime in [false, true] {
        let mut f = setup();
        f.proposal.estimate.money_microusd = 10;
        let issued = f
            .services
            .issue_provider_v2(f.proposal.clone())
            .await
            .unwrap();
        if lifetime {
            f.store
                .db()
                .unwrap()
                .execute("UPDATE authorization SET cap=5", [])
                .unwrap();
        } else {
            let mut request = f.store.snapshot(&f.services.run).unwrap()["request"].clone();
            request["limits"]["requests"] = json!(0);
            f.store
                .db()
                .unwrap()
                .execute(
                    "UPDATE runs SET request=?1 WHERE id=?2",
                    params![request.to_string(), f.services.run],
                )
                .unwrap();
        }
        assert!(
            matches!(
                f.services.execute_provider_v2(&issued).await,
                Err(c::ServiceError::Limit(_))
            ),
            "changed authorization must stop dispatch"
        );
        assert_eq!(f.transport.sends.load(Ordering::SeqCst), 0);
        assert_eq!(
            f.broker
                .quota_snapshot("project-day")
                .unwrap()
                .accounted
                .money_microusd,
            10
        );
    }
}
struct Late {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    calls: AtomicUsize,
}
#[async_trait]
impl OfflineTransport for Late {
    async fn send(&self, op: &c::ProviderOperationV2, _: &[u8]) -> Result<Reply> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        self.release.notified().await;
        let mut result = Reply::ambiguous(op);
        result.raw = Some(b"late private receipt".to_vec());
        Ok(result)
    }
}
#[tokio::test]
async fn cancellation_returns_promptly_while_late_receipts_and_charges_remain_original() {
    let f = setup();
    let late = Arc::new(Late {
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
        calls: AtomicUsize::new(0),
    });
    let mut service = f.services.clone();
    service.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        late.clone(),
    ));
    let issued = service.issue_provider_v2(f.proposal).await.unwrap();
    let operation = issued.clone();
    let mut worker = tokio::spawn(async move { service.execute_provider_v2(&operation).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), late.entered.notified())
        .await
        .unwrap();
    f.store.cancel(&f.services.run).unwrap();
    let stopped = tokio::time::timeout(std::time::Duration::from_millis(250), &mut worker).await;
    late.release.notify_one();
    assert!(
        matches!(stopped, Ok(Ok(Err(c::ServiceError::Cancelled)))),
        "cancellation must not await the transport"
    );
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if f.broker
                .raw_receipt(&issued.operation.attempt_id, 1)
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let saved: String = f
        .store
        .db()
        .unwrap()
        .query_row("SELECT result FROM provider_v2_attempts", [], |r| r.get(0))
        .unwrap();
    let result: c::ProviderResultV2 = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        result.epoch_disposition,
        c::ProviderEpochDisposition::Obsolete
    );
    assert_eq!(result.identity.run_id, f.services.run);
    assert_eq!(result.accounted_usage.requests, 1);
    assert_eq!(result.accounted_usage.tokens, 5);
    assert_eq!(late.calls.load(Ordering::SeqCst), 1);
    assert!(f.services.candidates.lock().unwrap().is_empty());
    assert!(f.store.catalog("pg-55").unwrap().is_none());
}
struct QueryFake {
    sends: AtomicUsize,
    queries: AtomicUsize,
    time: Arc<Time>,
}
#[async_trait]
impl OfflineTransport for QueryFake {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        assert_eq!(bytes, b"{ \"offline\": true }\n");
        self.sends.fetch_add(1, Ordering::SeqCst);
        self.time.0.fetch_add(7, Ordering::SeqCst);
        let mut reply = Reply::ambiguous(op);
        reply.job = Some(super::super::provider_v2::Job {
            id: "job-1".into(),
            status: c::ProviderJobStatus::Unknown,
        });
        reply.raw = Some(b"original uncertain receipt".to_vec());
        Ok(reply)
    }
    async fn recover(
        &self,
        op: &c::ProviderOperationV2,
    ) -> Result<super::super::provider_v2::RecoveryReply> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        self.time.0.fetch_add(9, Ordering::SeqCst);
        let mut reply = Reply::ambiguous(op);
        reply.job = Some(super::super::provider_v2::Job {
            id: "job-1".into(),
            status: c::ProviderJobStatus::Unknown,
        });
        reply.raw = Some(b"unknown job status".to_vec());
        Ok(super::super::provider_v2::RecoveryReply {
            query_usage: reply.usage.clone(),
            original: reply,
        })
    }
}
#[tokio::test]
async fn separately_bound_recovery_queries_once_and_caches_the_exact_private_wrapper() {
    let f = setup();
    let fake = Arc::new(QueryFake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: f.time.clone(),
    });
    let mut origin = f.services.clone();
    origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let issued = origin.issue_provider_v2(f.proposal.clone()).await.unwrap();
    let executed = origin.execute_provider_v2(&issued).await.unwrap();
    f.broker
        .authorize_unit(
            &origin.run,
            origin.epoch,
            "observer",
            "primary",
            "status-logical",
            None,
        )
        .unwrap();
    f.broker
        .authorize_observer(
            &origin.run,
            origin.epoch,
            "observer",
            &issued.operation.attempt_id,
        )
        .unwrap();
    let mut observer = origin.clone();
    observer.unit = "observer".into();
    observer.inputs = Arc::new(vec![]);
    let request = c::ProviderRecoveryRequestV2 {
        schema_version: 2,
        original: {
            let mut selector = issued.clone();
            selector.recovery = executed.recovery.clone();
            selector
        },
    };
    let receipt = observer
        .recover_provider_v2(request.clone())
        .await
        .expect("actual Services must expose the durable query receipt");
    receipt
        .validate_for_request(
            &request,
            &c::ProviderObserverV2 {
                schema_version: 2,
                run_id: observer.run.clone(),
                epoch: observer.epoch as u64,
                unit_id: observer.unit.clone(),
            },
        )
        .unwrap();
    let again = observer.recover_provider_v2(request.clone()).await.unwrap();
    assert_eq!(
        serde_json::to_vec(&receipt).unwrap(),
        serde_json::to_vec(&again).unwrap()
    );
    assert_eq!(fake.sends.load(Ordering::SeqCst), 1);
    assert_eq!(fake.queries.load(Ordering::SeqCst), 1);
    assert_eq!(
        receipt.original.identity.operation_id,
        issued.identity.operation_id
    );
    assert_eq!(
        receipt.original.result.attempt_id,
        issued.operation.attempt_id
    );
    assert_eq!(receipt.query_completion, c::ProviderCompletion::Completed);
    assert_eq!(
        receipt.original.result.completion,
        c::ProviderCompletion::Ambiguous
    );
    assert_eq!(receipt.query_observed_usage.requests, None);
    assert_eq!(receipt.query_accounted_usage.requests, 1);
    assert_eq!(receipt.query_accounted_usage.tokens, 3);
    assert_eq!(receipt.query_accounted_usage.money_microusd, 4);
    let accounted = f.broker.quota_snapshot("project-day").unwrap().accounted;
    assert_eq!(accounted.requests, 2);
    assert_eq!(accounted.tokens, 8);
    assert_eq!(accounted.money_microusd, 4);
    assert!(observer.candidates.lock().unwrap().is_empty());
    assert!(
        f.store
            .artifact(
                receipt
                    .original
                    .result
                    .raw_response
                    .as_ref()
                    .unwrap()
                    .id
                    .as_str()
            )
            .is_err()
    );
    if let Ok(path) = std::env::var("PT_PROVIDER_SERVICE_RECEIPT") {
        let example = json!({"schema_version":1,"task":"R17","provenance":"synthetic","test":"separately_bound_recovery_queries_once_and_caches_the_exact_private_wrapper","issue_provider_v2":{"proposal":f.proposal,"issued":issued},"execute_provider_v2":{"issued":issued,"result":executed},"recover_provider_v2":{"request":request,"result":receipt},"cached_query_receipt_identical":true,"fake_submission_calls":fake.sends.load(Ordering::SeqCst),"fake_query_calls":fake.queries.load(Ordering::SeqCst),"external_provider_calls":0,"production_approved":false,"publication_enabled":false});
        std::fs::write(path, serde_json::to_vec_pretty(&example).unwrap()).unwrap();
    }
}
#[tokio::test]
async fn recovery_requires_a_separate_observer_grant_and_latest_genuine_selector() {
    let f = setup();
    let fake = Arc::new(QueryFake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: f.time.clone(),
    });
    let mut origin = f.services.clone();
    origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let issued = origin.issue_provider_v2(f.proposal.clone()).await.unwrap();
    origin.execute_provider_v2(&issued).await.unwrap();
    let request = c::ProviderRecoveryRequestV2 {
        schema_version: 2,
        original: f
            .broker
            .recovery_descriptor(&issued.operation.attempt_id)
            .unwrap(),
    };
    assert!(matches!(
        origin.recover_provider_v2(request.clone()).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    f.broker
        .authorize_unit(
            &origin.run,
            origin.epoch,
            "observer",
            "primary",
            "status-logical",
            None,
        )
        .unwrap();
    let mut observer = origin.clone();
    observer.unit = "observer".into();
    observer.inputs = Arc::new(vec![]);
    assert!(matches!(
        observer.recover_provider_v2(request.clone()).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    f.broker
        .authorize_observer(
            &origin.run,
            origin.epoch,
            "observer",
            &issued.operation.attempt_id,
        )
        .unwrap();
    let before = f.store.snapshot(&origin.run).unwrap();
    for change in 0..5 {
        let mut bad = request.clone();
        match change {
            0 => bad.original.recovery.as_mut().unwrap().provider_job_id = "caller-job".into(),
            1 => bad.original.recovery.as_mut().unwrap().status = c::ProviderJobStatus::Running,
            2 => bad.original.recovery.as_mut().unwrap().observed_at_ms -= 1,
            3 => bad.original.identity.operation_id = "caller-logical".into(),
            _ => bad.original.operation.payload.id = digest(b"other payload"),
        }
        assert!(observer.recover_provider_v2(bad).await.is_err());
    }
    assert_eq!(f.store.snapshot(&origin.run).unwrap(), before);
    assert_eq!(fake.queries.load(Ordering::SeqCst), 0);
    observer.recover_provider_v2(request.clone()).await.unwrap();
    assert!(origin.execute_provider_v2(&request.original).await.is_err());
    let cached = origin.issue_provider_v2(f.proposal).await.unwrap();
    assert!(origin.execute_provider_v2(&cached).await.is_err());
    assert_eq!(fake.sends.load(Ordering::SeqCst), 1);
    assert_eq!(fake.queries.load(Ordering::SeqCst), 1);
}
fn observer_run(
    f: &Fixture,
    issued: &c::ProviderOperationV2,
    transport: Arc<dyn OfflineTransport>,
) -> Services {
    let mut request = f.store.snapshot(&f.services.run).unwrap()["request"].clone();
    request["mode"] = json!("fixture");
    let run = f.store.create("separate-observer", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = f.store.claim().unwrap().unwrap();
    request["mode"] = json!("live");
    f.store
        .db()
        .unwrap()
        .execute(
            "UPDATE runs SET request=?1 WHERE id=?2",
            params![request.to_string(), run],
        )
        .unwrap();
    f.broker
        .authorize_unit(&run, epoch, "observer", "primary", "status-logical", None)
        .unwrap();
    f.broker
        .authorize_observer(&run, epoch, "observer", &issued.operation.attempt_id)
        .unwrap();
    let mut service = f.services.clone();
    service.run = run;
    service.epoch = epoch;
    service.unit = "observer".into();
    service.inputs = Arc::new(vec![]);
    service.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        transport,
    ));
    service
}
struct OverOriginal(QueryFake);
#[async_trait]
impl OfflineTransport for OverOriginal {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        let mut reply = self.0.send(op, bytes).await?;
        reply.usage.requests = Some(2);
        reply.usage.tokens = Some(30);
        reply.usage.money_microusd = Some(20);
        Ok(reply)
    }
    async fn recover(
        &self,
        op: &c::ProviderOperationV2,
    ) -> Result<super::super::provider_v2::RecoveryReply> {
        self.0.recover(op).await
    }
}
#[tokio::test]
async fn unknown_recovery_preserves_original_overages_and_cross_run_observer_identity() {
    let f = setup();
    let fake = Arc::new(OverOriginal(QueryFake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: f.time.clone(),
    }));
    let mut origin = f.services.clone();
    origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let issued = origin.issue_provider_v2(f.proposal.clone()).await.unwrap();
    assert!(matches!(
        origin.execute_provider_v2(&issued).await,
        Err(c::ServiceError::Cancelled)
    ));
    let observer = observer_run(&f, &issued, fake.clone());
    let request = c::ProviderRecoveryRequestV2 {
        schema_version: 2,
        original: f
            .broker
            .recovery_descriptor(&issued.operation.attempt_id)
            .unwrap(),
    };
    let receipt = observer.recover_provider_v2(request).await.unwrap();
    assert_eq!(
        receipt.original.accounted_usage.money_microusd, 20,
        "unknown query usage cannot refund an observed original overage"
    );
    assert_eq!(receipt.original.accounted_usage.requests, 2);
    assert_eq!(receipt.original.accounted_usage.tokens, 30);
    assert_eq!(receipt.original.identity.run_id, origin.run);
    assert_eq!(receipt.original.identity.epoch, origin.epoch as u64);
    assert_eq!(receipt.observer.run_id, observer.run);
    assert_ne!(receipt.observer.run_id, receipt.original.identity.run_id);
    assert_eq!(
        receipt.original.epoch_disposition,
        c::ProviderEpochDisposition::Obsolete
    );
    assert_eq!(
        f.broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .money_microusd,
        24
    );
    assert_eq!(fake.0.sends.load(Ordering::SeqCst), 1);
    assert_eq!(fake.0.queries.load(Ordering::SeqCst), 1);
    assert!(observer.candidates.lock().unwrap().is_empty());
    assert!(f.store.catalog("pg-55").unwrap().is_none());
}
struct OldOutputs(QueryFake);
#[async_trait]
impl OfflineTransport for OldOutputs {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        self.0.send(op, bytes).await
    }
    async fn recover(
        &self,
        op: &c::ProviderOperationV2,
    ) -> Result<super::super::provider_v2::RecoveryReply> {
        let mut r = self.0.recover(op).await?;
        r.original.outputs.push(op.operation.payload.clone());
        Ok(r)
    }
}
#[tokio::test]
async fn obsolete_original_outputs_never_become_observer_artifacts_or_candidate_authority() {
    let f = setup();
    let fake = Arc::new(OldOutputs(QueryFake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: f.time.clone(),
    }));
    let mut origin = f.services.clone();
    origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let issued = origin.issue_provider_v2(f.proposal.clone()).await.unwrap();
    origin.execute_provider_v2(&issued).await.unwrap();
    f.store.cancel(&origin.run).unwrap();
    let observer = observer_run(&f, &issued, fake.clone());
    let receipt = observer
        .recover_provider_v2(c::ProviderRecoveryRequestV2 {
            schema_version: 2,
            original: f
                .broker
                .recovery_descriptor(&issued.operation.attempt_id)
                .unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(
        receipt.original.epoch_disposition,
        c::ProviderEpochDisposition::Obsolete
    );
    assert_eq!(receipt.original.result.outputs[0].id, f.proposal.payload.id);
    assert!(
        matches!(
            observer
                .read_artifact(&receipt.original.result.outputs[0])
                .await,
            Err(c::ServiceError::Unauthorized(_))
        ),
        "a query receipt must not grant artifact access to its observer"
    );
    assert!(observer.candidates.lock().unwrap().is_empty());
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE run=?1",
                [&observer.run],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert!(f.store.catalog("pg-55").unwrap().is_none());
}
#[tokio::test]
async fn cached_query_wrapper_is_checked_against_durable_identity_accounting_and_raw_bytes() {
    let f = setup();
    let fake = Arc::new(QueryFake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: f.time.clone(),
    });
    let mut origin = f.services.clone();
    origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let issued = origin.issue_provider_v2(f.proposal).await.unwrap();
    origin.execute_provider_v2(&issued).await.unwrap();
    f.broker
        .authorize_unit(
            &origin.run,
            origin.epoch,
            "observer",
            "primary",
            "status-logical",
            None,
        )
        .unwrap();
    f.broker
        .authorize_observer(
            &origin.run,
            origin.epoch,
            "observer",
            &issued.operation.attempt_id,
        )
        .unwrap();
    let mut observer = origin;
    observer.unit = "observer".into();
    observer.inputs = Arc::new(vec![]);
    let request = c::ProviderRecoveryRequestV2 {
        schema_version: 2,
        original: f
            .broker
            .recovery_descriptor(&issued.operation.attempt_id)
            .unwrap(),
    };
    let receipt = observer.recover_provider_v2(request.clone()).await.unwrap();
    let saved = serde_json::to_string(&receipt).unwrap();
    for field in ["query_id", "query_accounted_usage", "query_started_at_ms"] {
        let mut bad = serde_json::to_value(&receipt).unwrap();
        match field {
            "query_id" => bad[field] = json!("forged-query"),
            "query_accounted_usage" => bad[field]["requests"] = json!(2),
            _ => bad[field] = json!(receipt.query_started_at_ms - 1),
        }
        f.store
            .db()
            .unwrap()
            .execute(
                "UPDATE provider_v2_query_scopes SET receipt=?1",
                [bad.to_string()],
            )
            .unwrap();
        assert!(
            observer.recover_provider_v2(request.clone()).await.is_err(),
            "cached {field} changed without durable authority"
        );
        f.store
            .db()
            .unwrap()
            .execute("UPDATE provider_v2_query_scopes SET receipt=?1", [&saved])
            .unwrap();
    }
    let raw = &receipt.original.result.raw_response.as_ref().unwrap().file;
    let path = f.store.object(&raw.sha256).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let mut bad = bytes.clone();
    bad[0] ^= 1;
    std::fs::write(&path, bad).unwrap();
    assert!(observer.recover_provider_v2(request.clone()).await.is_err());
    std::fs::write(&path, bytes).unwrap();
    let cached = observer.recover_provider_v2(request.clone()).await.unwrap();
    assert_eq!(serde_json::to_string(&cached).unwrap(), saved);
    // Old rows without observer/timestamp metadata remain held; never invent a wrapper.
    f.store
        .db()
        .unwrap()
        .execute("DELETE FROM provider_v2_query_scopes", [])
        .unwrap();
    assert!(matches!(
        observer.recover_provider_v2(request).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    assert_eq!(fake.sends.load(Ordering::SeqCst), 1);
    assert_eq!(fake.queries.load(Ordering::SeqCst), 1);
}
struct AmbiguousQuery(QueryFake);
#[async_trait]
impl OfflineTransport for AmbiguousQuery {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        self.0.send(op, bytes).await
    }
    async fn recover(
        &self,
        _: &c::ProviderOperationV2,
    ) -> Result<super::super::provider_v2::RecoveryReply> {
        self.0.queries.fetch_add(1, Ordering::SeqCst);
        Err(Error::new("transport_unknown", "retry_at=9007199254740991"))
    }
}
#[tokio::test]
async fn ambiguous_queries_keep_separate_reservations_and_cannot_be_repurchased_after_restart() {
    let f = setup();
    let fake = Arc::new(AmbiguousQuery(QueryFake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: f.time.clone(),
    }));
    let mut origin = f.services.clone();
    origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let issued = origin.issue_provider_v2(f.proposal).await.unwrap();
    origin.execute_provider_v2(&issued).await.unwrap();
    f.broker
        .authorize_unit(
            &origin.run,
            origin.epoch,
            "observer",
            "primary",
            "status-logical",
            None,
        )
        .unwrap();
    f.broker
        .authorize_observer(
            &origin.run,
            origin.epoch,
            "observer",
            &issued.operation.attempt_id,
        )
        .unwrap();
    let mut observer = origin.clone();
    observer.unit = "observer".into();
    observer.inputs = Arc::new(vec![]);
    let request = c::ProviderRecoveryRequestV2 {
        schema_version: 2,
        original: f
            .broker
            .recovery_descriptor(&issued.operation.attempt_id)
            .unwrap(),
    };
    let receipt = observer.recover_provider_v2(request.clone()).await.unwrap();
    assert_eq!(receipt.query_completion, c::ProviderCompletion::Ambiguous);
    assert_eq!(receipt.query_observed_usage.money_microusd, None);
    assert_eq!(receipt.query_reserved_usage, receipt.query_accounted_usage);
    assert!(observer.recover_provider_v2(request.clone()).await.is_err());
    assert!(origin.execute_provider_v2(&issued).await.is_err());
    assert_eq!(
        f.store.snapshot(&origin.run).unwrap()["state"],
        "running",
        "transport diagnostic text must not create a retry deadline"
    );
    let reopened = Store::open(f._temp.path()).unwrap();
    reopened.recover().unwrap();
    let (_, epoch) = reopened.claim().unwrap().unwrap();
    let broker = Broker::synthetic(reopened.clone(), f.time.clone());
    assert!(matches!(
        observer.recover_provider_v2(request.clone()).await,
        Err(c::ServiceError::Cancelled)
    ));
    broker
        .authorize_unit(
            &origin.run,
            epoch,
            "observer",
            "primary",
            "status-logical",
            None,
        )
        .unwrap();
    broker
        .authorize_observer(&origin.run, epoch, "observer", &issued.operation.attempt_id)
        .unwrap();
    observer.epoch = epoch;
    observer.store = reopened.clone();
    observer.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        broker.clone(),
        fake.clone(),
    ));
    assert!(observer.recover_provider_v2(request).await.is_err());
    assert_eq!(fake.0.sends.load(Ordering::SeqCst), 1);
    assert_eq!(fake.0.queries.load(Ordering::SeqCst), 1);
    assert_eq!(
        broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .money_microusd,
        4
    );
    assert_eq!(
        reopened
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM provider_v2_queries WHERE state='uncertain'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}
struct LateQuery {
    base: QueryFake,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
#[async_trait]
impl OfflineTransport for LateQuery {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        self.base.send(op, bytes).await
    }
    async fn recover(
        &self,
        op: &c::ProviderOperationV2,
    ) -> Result<super::super::provider_v2::RecoveryReply> {
        self.entered.notify_one();
        self.release.notified().await;
        self.base.recover(op).await
    }
}
#[tokio::test]
async fn canceled_or_dropped_query_await_keeps_late_receipt_under_original_and_observer_scopes() {
    for drop_stage in [false, true] {
        let f = setup();
        let fake = Arc::new(LateQuery {
            base: QueryFake {
                sends: AtomicUsize::new(0),
                queries: AtomicUsize::new(0),
                time: f.time.clone(),
            },
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
        });
        let mut origin = f.services.clone();
        origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
            f.broker.clone(),
            fake.clone(),
        ));
        let issued = origin.issue_provider_v2(f.proposal).await.unwrap();
        origin.execute_provider_v2(&issued).await.unwrap();
        f.broker
            .authorize_unit(
                &origin.run,
                origin.epoch,
                "observer",
                "primary",
                "status-logical",
                None,
            )
            .unwrap();
        f.broker
            .authorize_observer(
                &origin.run,
                origin.epoch,
                "observer",
                &issued.operation.attempt_id,
            )
            .unwrap();
        let mut observer = origin.clone();
        observer.unit = "observer".into();
        observer.inputs = Arc::new(vec![]);
        let scope_epoch = observer.epoch;
        let request = c::ProviderRecoveryRequestV2 {
            schema_version: 2,
            original: f
                .broker
                .recovery_descriptor(&issued.operation.attempt_id)
                .unwrap(),
        };
        let mut worker = tokio::spawn(async move { observer.recover_provider_v2(request).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), fake.entered.notified())
            .await
            .unwrap();
        if drop_stage {
            worker.abort();
        }
        f.store.cancel(&origin.run).unwrap();
        if drop_stage {
            assert!(worker.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(
                tokio::time::timeout(std::time::Duration::from_millis(250), &mut worker).await,
                Ok(Ok(Err(c::ServiceError::Cancelled)))
            ));
        }
        fake.release.notify_one();
        let saved = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let value: Option<String> = f
                    .store
                    .db()
                    .unwrap()
                    .query_row("SELECT receipt FROM provider_v2_query_scopes", [], |r| {
                        r.get(0)
                    })
                    .unwrap();
                if let Some(v) = value {
                    break v;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let receipt: c::ProviderRecoveryResultV2 = serde_json::from_str(&saved).unwrap();
        assert_eq!(receipt.observer.epoch, scope_epoch as u64);
        assert_eq!(receipt.original.identity, issued.identity);
        assert_eq!(
            receipt.original.epoch_disposition,
            c::ProviderEpochDisposition::Obsolete
        );
        assert_eq!(receipt.query_accounted_usage.money_microusd, 4);
        assert_eq!(fake.base.sends.load(Ordering::SeqCst), 1);
        assert_eq!(fake.base.queries.load(Ordering::SeqCst), 1);
        assert!(f.store.catalog("pg-55").unwrap().is_none());
    }
}
struct WrongReceipt(Arc<Fake>);
#[async_trait]
impl OfflineTransport for WrongReceipt {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        let mut r = self.0.send(op, bytes).await?;
        r.attempt_id = "wrong-attempt".into();
        Ok(r)
    }
}
#[tokio::test]
async fn invalid_transport_result_keeps_typed_error_and_reservation_without_any_fallback() {
    let f = setup();
    let mut service = f.services.clone();
    service.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        Arc::new(WrongReceipt(f.transport.clone())),
    ));
    let issued = service.issue_provider_v2(f.proposal).await.unwrap();
    assert!(
        matches!(service.execute_provider_v2(&issued).await,Err(c::ServiceError::Internal(ref code)) if code=="receipt_conflict")
    );
    assert!(matches!(
        service.execute_provider_v2(&issued).await,
        Err(c::ServiceError::Unauthorized(_))
    ));
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 1);
    assert_eq!(f.transport.queries.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        1
    );
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM provider_v2_receipts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
struct QueryCharges {
    base: QueryFake,
    over: bool,
}
#[async_trait]
impl OfflineTransport for QueryCharges {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        self.base.send(op, bytes).await
    }
    async fn recover(
        &self,
        op: &c::ProviderOperationV2,
    ) -> Result<super::super::provider_v2::RecoveryReply> {
        let mut reply = self.base.recover(op).await?;
        reply.query_usage = c::ObservedProviderUsage {
            requests: Some(if self.over { 2 } else { 0 }),
            tokens: Some(if self.over { 9 } else { 0 }),
            input_tokens: None,
            output_tokens: None,
            money_microusd: Some(if self.over { 10 } else { 0 }),
        };
        Ok(reply)
    }
}
#[tokio::test]
async fn query_zero_money_still_charges_a_request_and_observed_overages_remain_charged() {
    for over in [false, true] {
        let f = setup();
        let fake = Arc::new(QueryCharges {
            base: QueryFake {
                sends: AtomicUsize::new(0),
                queries: AtomicUsize::new(0),
                time: f.time.clone(),
            },
            over,
        });
        let mut origin = f.services.clone();
        origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
            f.broker.clone(),
            fake.clone(),
        ));
        let issued = origin.issue_provider_v2(f.proposal).await.unwrap();
        origin.execute_provider_v2(&issued).await.unwrap();
        f.broker
            .authorize_unit(
                &origin.run,
                origin.epoch,
                "observer",
                "primary",
                "status-logical",
                None,
            )
            .unwrap();
        f.broker
            .authorize_observer(
                &origin.run,
                origin.epoch,
                "observer",
                &issued.operation.attempt_id,
            )
            .unwrap();
        let mut observer = origin.clone();
        observer.unit = "observer".into();
        observer.inputs = Arc::new(vec![]);
        let request = c::ProviderRecoveryRequestV2 {
            schema_version: 2,
            original: f
                .broker
                .recovery_descriptor(&issued.operation.attempt_id)
                .unwrap(),
        };
        let result = observer.recover_provider_v2(request.clone()).await;
        if over {
            assert!(matches!(result, Err(c::ServiceError::Cancelled)));
        } else {
            result
                .unwrap()
                .validate_for_request(
                    &request,
                    &c::ProviderObserverV2 {
                        schema_version: 2,
                        run_id: observer.run,
                        epoch: observer.epoch as u64,
                        unit_id: observer.unit,
                    },
                )
                .unwrap();
        }
        let encoded: String = f
            .store
            .db()
            .unwrap()
            .query_row("SELECT receipt FROM provider_v2_query_scopes", [], |r| {
                r.get(0)
            })
            .unwrap();
        let receipt: c::ProviderRecoveryResultV2 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(
            receipt.query_accounted_usage.requests,
            if over { 2 } else { 1 }
        );
        assert_eq!(
            receipt.query_accounted_usage.money_microusd,
            if over { 10 } else { 0 }
        );
        assert_eq!(
            receipt.query_accounted_usage.tokens,
            if over { 9 } else { 0 }
        );
        assert_eq!(receipt.query_accounted_usage.repairs, 0);
        assert_eq!(receipt.query_observed_usage.input_tokens, None);
        assert_eq!(
            receipt.query_observed_usage.requests,
            Some(if over { 2 } else { 0 })
        );
        assert_eq!(fake.base.sends.load(Ordering::SeqCst), 1);
        assert_eq!(fake.base.queries.load(Ordering::SeqCst), 1);
    }
}
#[tokio::test]
async fn query_wrapper_commit_rolls_back_atomically_and_keeps_the_unresolved_reservation() {
    let f = setup();
    let fake = Arc::new(QueryFake {
        sends: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        time: f.time.clone(),
    });
    let mut origin = f.services.clone();
    origin.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let issued = origin.issue_provider_v2(f.proposal).await.unwrap();
    origin.execute_provider_v2(&issued).await.unwrap();
    f.broker
        .authorize_unit(
            &origin.run,
            origin.epoch,
            "observer",
            "primary",
            "status-logical",
            None,
        )
        .unwrap();
    f.broker
        .authorize_observer(
            &origin.run,
            origin.epoch,
            "observer",
            &issued.operation.attempt_id,
        )
        .unwrap();
    let mut observer = origin;
    observer.unit = "observer".into();
    observer.inputs = Arc::new(vec![]);
    let request = c::ProviderRecoveryRequestV2 {
        schema_version: 2,
        original: f
            .broker
            .recovery_descriptor(&issued.operation.attempt_id)
            .unwrap(),
    };
    f.store.db().unwrap().execute_batch("CREATE TRIGGER reject_query_receipt BEFORE UPDATE OF receipt ON provider_v2_query_scopes BEGIN SELECT RAISE(ABORT,'injected receipt commit failure'); END;").unwrap();
    assert!(matches!(
        observer.recover_provider_v2(request.clone()).await,
        Err(c::ServiceError::Internal(_))
    ));
    let (state,receipt):(String,Option<String>)=f.store.db().unwrap().query_row("SELECT q.state,s.receipt FROM provider_v2_queries q JOIN provider_v2_query_scopes s ON s.id=q.id",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(state, "dispatched");
    assert!(receipt.is_none());
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM provider_v2_receipts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM events WHERE kind='provider_result'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    f.store
        .db()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_query_receipt")
        .unwrap();
    assert!(observer.recover_provider_v2(request).await.is_err());
    assert_eq!(fake.sends.load(Ordering::SeqCst), 1);
    assert_eq!(fake.queries.load(Ordering::SeqCst), 1);
    assert_eq!(
        f.broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .money_microusd,
        4
    );
    assert!(f.store.catalog("pg-55").unwrap().is_none());
}
struct Unconfigured {
    proposal: c::ProviderProposalV2,
    issued: c::ProviderOperationV2,
}
#[async_trait]
impl c::StageExecutor for Unconfigured {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        assert_ne!(input.unit_id, "unit");
        assert_eq!(input.mode, c::ExecutionMode::Live);
        assert!(matches!(
            services.issue_provider_v2(self.proposal.clone()).await,
            Err(c::ServiceError::ProviderV2Unavailable)
        ));
        assert!(matches!(
            services.execute_provider_v2(&self.issued).await,
            Err(c::ServiceError::ProviderV2Unavailable)
        ));
        assert!(matches!(
            services
                .recover_provider_v2(c::ProviderRecoveryRequestV2 {
                    schema_version: 2,
                    original: self.issued.clone()
                })
                .await,
            Err(c::ServiceError::ProviderV2Unavailable)
        ));
        Ok(c::StageOutput {
            outcome: c::StageOutcome::NeedsRepair,
            artifacts: vec![],
            decisions: vec![],
            findings: vec![c::Finding {
                id: "production-verifier-pending".into(),
                field: "provider".into(),
                outcome: c::ReviewOutcome::Missing,
                message: "Offline service verification grants no production capability.".into(),
                evidence: vec![],
            }],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
#[tokio::test]
async fn ordinary_runtime_construction_has_no_provider_capability_even_with_valid_private_issuance()
{
    let f = setup();
    let issued = f
        .services
        .issue_provider_v2(f.proposal.clone())
        .await
        .unwrap();
    let runtime = Runtime::new(
        f.store.clone(),
        Arc::new(Unconfigured {
            proposal: f.proposal,
            issued,
        }),
    );
    assert_eq!(
        runtime
            .execute_stage(&f.services.run, f.services.epoch, c::Stage::Source)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 0);
    assert_eq!(f.transport.queries.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM provider_v2_attempts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(f.store.catalog("pg-55").unwrap().is_none());
}
#[tokio::test]
async fn persisted_issuance_cannot_override_a_changed_current_unit_grant() {
    let f = setup();
    let issued = f.services.issue_provider_v2(f.proposal).await.unwrap();
    let db = f.store.db().unwrap();
    let saved: String = db
        .query_row("SELECT record FROM provider_v2_units", [], |r| r.get(0))
        .unwrap();
    let mut grant: serde_json::Value = serde_json::from_str(&saved).unwrap();
    grant["operation_id"] = json!("changed-logical");
    db.execute(
        "UPDATE provider_v2_units SET record=?1",
        [grant.to_string()],
    )
    .unwrap();
    assert!(
        matches!(
            f.services.execute_provider_v2(&issued).await,
            Err(c::ServiceError::Unauthorized(_))
        ),
        "issuance must still match the active immutable grant"
    );
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 0);
}
struct KnownReject(Arc<Fake>);
#[async_trait]
impl OfflineTransport for KnownReject {
    async fn send(&self, op: &c::ProviderOperationV2, bytes: &[u8]) -> Result<Reply> {
        let mut r = self.0.send(op, bytes).await?;
        r.completion = c::ProviderCompletion::KnownRejection;
        Ok(r)
    }
}
#[tokio::test]
async fn transport_retry_issuance_uses_only_the_persisted_runtime_predecessor_grant() {
    let f = setup();
    let fake = Arc::new(KnownReject(f.transport.clone()));
    let mut service = f.services.clone();
    service.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake.clone(),
    ));
    let original = service.issue_provider_v2(f.proposal.clone()).await.unwrap();
    service.execute_provider_v2(&original).await.unwrap();
    f.store.recover().unwrap();
    let (_, epoch) = f.store.claim().unwrap().unwrap();
    f.broker
        .authorize_retry_unit(
            &service.run,
            epoch,
            "unit",
            "primary",
            "logical",
            None,
            &original.operation.attempt_id,
        )
        .unwrap();
    service.epoch = epoch;
    let retry = service
        .issue_provider_v2(f.proposal.clone())
        .await
        .expect("only runtime-persisted predecessor authority may issue a transport retry");
    assert_eq!(
        retry
            .identity
            .transport_retry
            .as_ref()
            .unwrap()
            .previous_attempt_id,
        original.operation.attempt_id
    );
    assert_eq!(
        retry.identity.transport_retry.as_ref().unwrap().retry_index,
        1
    );
    assert!(retry.identity.repair.is_none());
    service.execute_provider_v2(&retry).await.unwrap();
    let same = service.issue_provider_v2(f.proposal).await.unwrap();
    assert_eq!(
        serde_json::to_string(&same).unwrap(),
        serde_json::to_string(&retry).unwrap()
    );
    service.execute_provider_v2(&same).await.unwrap();
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 2);
    assert_eq!(
        f.broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .requests,
        2
    );
    assert_eq!(
        f.broker
            .quota_snapshot("project-day")
            .unwrap()
            .accounted
            .repairs,
        0
    );
}
#[tokio::test]
async fn public_repair_issuance_and_its_authorized_retry_charge_the_repair_only_once() {
    let f = setup();
    let fake = Arc::new(KnownReject(f.transport.clone()));
    let mut service = f.services.clone();
    service.provider_v2 = Some(super::super::provider_service::Adapter::injected(
        f.broker.clone(),
        fake,
    ));
    let original = service.issue_provider_v2(f.proposal.clone()).await.unwrap();
    service.execute_provider_v2(&original).await.unwrap();
    let repair = c::CreativeRepairIdentity {
        repair_id: "repair-1".into(),
        previous_operation_id: "logical".into(),
        repair_index: 1,
    };
    f.broker
        .authorize_unit(
            &service.run,
            service.epoch,
            "repair-unit",
            "primary",
            "repair-logical",
            Some(repair.clone()),
        )
        .unwrap();
    service.unit = "repair-unit".into();
    let revised = service.issue_provider_v2(f.proposal.clone()).await.unwrap();
    assert_eq!(revised.identity.repair, Some(repair.clone()));
    service.execute_provider_v2(&revised).await.unwrap();
    f.store.recover().unwrap();
    let (_, epoch) = f.store.claim().unwrap().unwrap();
    f.broker
        .authorize_retry_unit(
            &service.run,
            epoch,
            "repair-unit",
            "primary",
            "repair-logical",
            Some(repair),
            &revised.operation.attempt_id,
        )
        .unwrap();
    service.epoch = epoch;
    let retry = service.issue_provider_v2(f.proposal).await.unwrap();
    service.execute_provider_v2(&retry).await.unwrap();
    service.execute_provider_v2(&retry).await.unwrap();
    let usage = f.broker.quota_snapshot("project-day").unwrap().accounted;
    assert_eq!(usage.repairs, 1);
    assert_eq!(usage.requests, 3);
    assert_eq!(f.transport.sends.load(Ordering::SeqCst), 3);
}
