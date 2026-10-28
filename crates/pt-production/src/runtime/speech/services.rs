use super::*;
use async_trait::async_trait;
pub(super) struct Services {
    pub store: Store,
    pub run: String,
    pub lease: Option<i64>,
    pub allowed: Vec<c::ArtifactRef>,
    pub candidates: std::sync::Mutex<BTreeMap<String, (c::CandidateFile, Vec<u8>)>>,
}
fn error(e: Error) -> c::ServiceError {
    if e.code == "stale_epoch" {
        c::ServiceError::Cancelled
    } else {
        c::ServiceError::Internal(e.code.into())
    }
}
impl Services {
    pub fn active(&self) -> std::result::Result<(), c::ServiceError> {
        if let Some(epoch) = self.lease {
            Store::active(&self.store.db().map_err(error)?, &self.run, epoch).map_err(error)?;
        }
        Ok(())
    }
}
#[async_trait]
impl c::RunServices for Services {
    async fn check_cancelled(&self) -> std::result::Result<(), c::ServiceError> {
        self.active()
    }
    async fn read_artifact(
        &self,
        a: &c::ArtifactRef,
    ) -> std::result::Result<Vec<u8>, c::ServiceError> {
        self.active()?;
        if !self.allowed.iter().any(|old| equal(old, a)) {
            return Err(c::ServiceError::Unauthorized(
                "private Speech phase input required".into(),
            ));
        }
        let b = storage::verify_owned(&self.store, &self.run, a).map_err(error)?;
        self.active()?;
        Ok(b)
    }
    async fn write_candidate(
        &self,
        r: c::CandidateWrite,
        b: Vec<u8>,
    ) -> std::result::Result<c::CandidateFile, c::ServiceError> {
        self.active()?;
        if !c::valid_id(&r.candidate_id)
            || !c::safe_path(&r.relative_path)
            || r.media_type != "application/json"
            || r.dimensions.is_some()
            || r.provenance != c::Provenance::Diagnostic
            || b.len() as u64 > META
            || b.len() as u64 > r.max_bytes
            || r.max_bytes > 32 * META
            || !(r.relative_path.starts_with("private/speech"))
        {
            return Err(c::ServiceError::Unauthorized(
                "bounded private JSON output required".into(),
            ));
        }
        let a = c::CandidateFile {
            candidate_id: r.candidate_id,
            file: file(&r.relative_path, &b, "application/json"),
            provenance: r.provenance,
        };
        let mut candidates = self.candidates.lock().unwrap();
        if candidates.len() >= 64
            || candidates.values().map(|(_, b)| b.len()).sum::<usize>() + b.len()
                > 8 * META as usize
        {
            return Err(c::ServiceError::Limit("Speech phase candidates".into()));
        }
        if candidates
            .get(&a.candidate_id)
            .is_some_and(|(old, _)| !equal(old, &a))
        {
            return Err(c::ServiceError::Unauthorized(
                "candidate identity reused".into(),
            ));
        }
        candidates.insert(a.candidate_id.clone(), (a.clone(), b));
        Ok(a)
    }
    async fn read_candidate(
        &self,
        a: &c::CandidateFile,
    ) -> std::result::Result<Vec<u8>, c::ServiceError> {
        self.active()?;
        let candidates = self.candidates.lock().unwrap();
        let (old, b) = candidates
            .get(&a.candidate_id)
            .ok_or_else(|| c::ServiceError::MissingCandidate(a.candidate_id.clone()))?;
        if !equal(old, a) {
            return Err(c::ServiceError::Unauthorized(
                "candidate substituted".into(),
            ));
        }
        a.file.verify(b)?;
        Ok(b.clone())
    }
    async fn provider(
        &self,
        _: c::ProviderOperation,
    ) -> std::result::Result<c::ProviderResult, c::ServiceError> {
        Err(c::ServiceError::OfflineDispatch)
    }
    async fn issue_provider_v2(
        &self,
        _: c::ProviderProposalV2,
    ) -> std::result::Result<c::ProviderOperationV2, c::ServiceError> {
        Err(c::ServiceError::OfflineDispatch)
    }
    async fn execute_provider_v2(
        &self,
        _: &c::ProviderOperationV2,
    ) -> std::result::Result<c::ProviderResultV2, c::ServiceError> {
        Err(c::ServiceError::OfflineDispatch)
    }
    async fn recover_provider_v2(
        &self,
        _: c::ProviderRecoveryRequestV2,
    ) -> std::result::Result<c::ProviderRecoveryResultV2, c::ServiceError> {
        Err(c::ServiceError::OfflineDispatch)
    }
    async fn helper(
        &self,
        _: c::HelperOperation,
    ) -> std::result::Result<c::HelperResult, c::ServiceError> {
        Err(c::ServiceError::OfflineDispatch)
    }
    async fn emit(&self, _: c::JournalProposal) -> std::result::Result<(), c::ServiceError> {
        Err(c::ServiceError::Unauthorized(
            "no private planning journal capability".into(),
        ))
    }
    async fn library(
        &self,
        _: c::LibraryQuery,
    ) -> std::result::Result<Vec<c::LibraryBinding>, c::ServiceError> {
        Err(c::ServiceError::Unauthorized(
            "saved Speech binding snapshot is not canonical authority".into(),
        ))
    }
}
