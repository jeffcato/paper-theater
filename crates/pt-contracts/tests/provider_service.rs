#[path = "support/provider_service_cases.rs"]
mod cases;
use async_trait::async_trait;
use pt_contracts::*;
use std::{
    future::Future,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};
fn ready<T>(f: impl Future<Output = T>) -> T {
    let mut f = std::pin::pin!(f);
    match f.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("test future unexpectedly pending"),
    }
}
#[derive(Default)]
struct OldServices {
    legacy: AtomicUsize,
}
#[async_trait]
impl RunServices for OldServices {
    async fn read_artifact(&self, _: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        panic!("unexpected artifact access")
    }
    async fn write_candidate(
        &self,
        _: CandidateWrite,
        _: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        panic!("unexpected candidate access")
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        panic!("unexpected journal write")
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        panic!("unavailable capability must not access old services")
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        self.legacy.fetch_add(1, Ordering::SeqCst);
        Err(ServiceError::Internal("legacy sentinel".into()))
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("unexpected helper")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("unexpected library")
    }
}
#[test]
fn old_implementors_explicitly_refuse_v2_without_legacy_fallback() {
    let s = OldServices::default();
    let c = cases::cases();
    let p: ProviderProposalV2 = serde_json::from_value(c[0]["wire"].clone()).unwrap();
    let op: ProviderOperationV2 = serde_json::from_value(c[1]["wire"].clone()).unwrap();
    let r: ProviderRecoveryRequestV2 = serde_json::from_value(c[3]["wire"].clone()).unwrap();
    assert!(matches!(
        ready(s.issue_provider_v2(p)),
        Err(ServiceError::ProviderV2Unavailable)
    ));
    assert!(matches!(
        ready(s.execute_provider_v2(&op)),
        Err(ServiceError::ProviderV2Unavailable)
    ));
    assert!(matches!(
        ready(s.recover_provider_v2(r)),
        Err(ServiceError::ProviderV2Unavailable)
    ));
    assert_eq!(s.legacy.load(Ordering::SeqCst), 0);
}
#[test]
fn provider_service_wire_cases() {
    for c in cases::cases() {
        let r = cases::check(&c);
        assert_eq!(r.is_ok(), c["valid"] == true, "{}: {r:?}", c["id"]);
        if !c["valid"].as_bool().unwrap() {
            assert_eq!(r.unwrap_err().code, c["code"], "{}", c["id"]);
        }
    }
}

// Synthetic consumer only: exercises trait data flow, not runtime authorization/quota enforcement.
struct BoundService {
    mode: ExecutionMode,
    observer: ProviderObserverV2,
    issued: std::sync::Mutex<Option<serde_json::Value>>,
    credential_lookups: AtomicUsize,
    sends: AtomicUsize,
    queries: AtomicUsize,
}
impl BoundService {
    fn gate(&self) -> Result<(), ServiceError> {
        require_live(self.mode)?;
        self.credential_lookups.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn new(mode: ExecutionMode) -> Self {
        Self {
            mode,
            observer: serde_json::from_value(cases::cases()[2]["wire"].clone()).unwrap(),
            issued: Default::default(),
            credential_lookups: AtomicUsize::new(0),
            sends: AtomicUsize::new(0),
            queries: AtomicUsize::new(0),
        }
    }
}
#[async_trait]
impl RunServices for BoundService {
    async fn read_artifact(&self, _: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        unreachable!()
    }
    async fn write_candidate(
        &self,
        _: CandidateWrite,
        _: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        unreachable!()
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        unreachable!()
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        Ok(())
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("legacy fallback forbidden")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        unreachable!()
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        unreachable!()
    }
    async fn issue_provider_v2(
        &self,
        p: ProviderProposalV2,
    ) -> Result<ProviderOperationV2, ServiceError> {
        self.gate()?;
        let issued: ProviderOperationV2 =
            serde_json::from_value(cases::cases()[1]["wire"].clone()).unwrap();
        issued.validate_for_proposal(&p, &self.observer)?;
        *self.issued.lock().unwrap() = Some(serde_json::to_value(&issued).unwrap());
        Ok(issued)
    }
    async fn execute_provider_v2(
        &self,
        op: &ProviderOperationV2,
    ) -> Result<ProviderResultV2, ServiceError> {
        self.gate()?;
        if self.issued.lock().unwrap().as_ref() != Some(&serde_json::to_value(op).unwrap())
            || op.recovery.is_some()
        {
            return Err(ServiceError::Unauthorized(
                "descriptor differs from issuance".into(),
            ));
        }
        self.sends.fetch_add(1, Ordering::SeqCst);
        let all: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/provider-metadata-v2.json"
        ))
        .unwrap();
        Ok(serde_json::from_value(
            all.iter()
                .find(|c| c["id"] == "ambiguous-unknown-usage-retains-reservation")
                .unwrap()["wire"]
                .clone(),
        )
        .unwrap())
    }
    async fn recover_provider_v2(
        &self,
        r: ProviderRecoveryRequestV2,
    ) -> Result<ProviderRecoveryResultV2, ServiceError> {
        self.gate()?;
        r.validate()?;
        let c = cases::cases();
        if serde_json::to_value(&r).unwrap() != c[3]["wire"] {
            return Err(ServiceError::Unauthorized(
                "existing descriptor differs".into(),
            ));
        }
        let mut receipt: ProviderRecoveryResultV2 =
            serde_json::from_value(c[4]["wire"].clone()).unwrap();
        receipt.observer = self.observer.clone();
        receipt.validate_for_request(&r, &self.observer)?;
        self.queries.fetch_add(1, Ordering::SeqCst);
        Ok(receipt)
    }
}
#[test]
fn staged_service_double_preserves_issuance_and_original_recovery_identity() {
    let c = cases::cases();
    let p: ProviderProposalV2 = serde_json::from_value(c[0]["wire"].clone()).unwrap();
    let op: ProviderOperationV2 = serde_json::from_value(c[1]["wire"].clone()).unwrap();
    let req: ProviderRecoveryRequestV2 = serde_json::from_value(c[3]["wire"].clone()).unwrap();
    for mode in [ExecutionMode::Fixture, ExecutionMode::Replay] {
        let s = BoundService::new(mode);
        assert!(matches!(
            ready(s.issue_provider_v2(p.clone())),
            Err(ServiceError::OfflineDispatch)
        ));
        assert!(matches!(
            ready(s.execute_provider_v2(&op)),
            Err(ServiceError::OfflineDispatch)
        ));
        assert!(matches!(
            ready(s.recover_provider_v2(req.clone())),
            Err(ServiceError::OfflineDispatch)
        ));
        assert_eq!(s.credential_lookups.load(Ordering::SeqCst), 0);
        assert_eq!(s.sends.load(Ordering::SeqCst), 0);
        assert_eq!(s.queries.load(Ordering::SeqCst), 0);
    }
    let s = BoundService::new(ExecutionMode::Live);
    assert!(matches!(
        ready(s.execute_provider_v2(&op)),
        Err(ServiceError::Unauthorized(_))
    ));
    let issued = ready(s.issue_provider_v2(p.clone())).unwrap();
    issued.validate_for_proposal(&p, &s.observer).unwrap();
    let mut forged = issued.clone();
    forged.operation.attempt_id = "forged-but-well-formed".into();
    forged.validate_for_proposal(&p, &s.observer).unwrap();
    assert!(matches!(
        ready(s.execute_provider_v2(&forged)),
        Err(ServiceError::Unauthorized(_))
    ));
    let result = ready(s.execute_provider_v2(&issued)).unwrap();
    result.validate_for_operation(&issued).unwrap();
    assert_eq!(s.sends.load(Ordering::SeqCst), 1);
    let mut observer = BoundService::new(ExecutionMode::Live);
    observer.observer.run_id = "observer-run".into();
    observer.observer.epoch = 9;
    observer.observer.unit_id = "observer-unit".into();
    let receipt = ready(observer.recover_provider_v2(req.clone())).unwrap();
    receipt
        .validate_for_request(&req, &observer.observer)
        .unwrap();
    assert_eq!(receipt.original.identity, req.original.identity);
    assert_eq!(
        receipt.original.result.attempt_id,
        req.original.operation.attempt_id
    );
    assert_eq!(receipt.query_observed_usage.money_microusd, None);
    assert_eq!(receipt.query_accounted_usage.money_microusd, 10);
    assert_eq!(observer.sends.load(Ordering::SeqCst), 0);
    assert_eq!(observer.queries.load(Ordering::SeqCst), 1);
}
