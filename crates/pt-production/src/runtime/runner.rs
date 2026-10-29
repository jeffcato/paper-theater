use super::store::*;
use async_trait::async_trait;
use pt_contracts as c;
use rusqlite::{OptionalExtension, params};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
#[cfg(test)]
#[path = "provider_service_tests.rs"]
mod provider_service_tests;

#[derive(Clone)]
pub struct Services {
    store: Store,
    run: String,
    epoch: i64,
    mode: c::ExecutionMode,
    unit: String,
    inputs: Arc<Vec<c::ArtifactRef>>,
    provider_v2: Option<super::provider_service::Adapter>,
    candidates: Arc<Mutex<HashMap<String, (c::CandidateFile, Vec<u8>)>>>,
    proposals: Arc<Mutex<Vec<c::JournalProposal>>>,
    offline_acoustic: Option<super::helper::OfflineAcoustic>,
}
fn service(e: Error) -> c::ServiceError {
    if e.code == "stale_epoch" {
        c::ServiceError::Cancelled
    } else {
        c::ServiceError::Internal(e.to_string())
    }
}
impl Services {
    #[cfg(test)]
    pub(super) fn injected_saved_provider(
        store: Store,
        run: String,
        epoch: i64,
        unit: String,
        inputs: Vec<c::ArtifactRef>,
        adapter: super::provider_service::Adapter,
    ) -> Self {
        Self {
            store,
            run,
            epoch,
            unit,
            mode: c::ExecutionMode::Live,
            inputs: Arc::new(inputs),
            provider_v2: Some(adapter),
            candidates: Arc::new(Mutex::new(HashMap::new())),
            proposals: Arc::new(Mutex::new(Vec::new())),
            offline_acoustic: None,
        }
    }
    async fn await_provider<T>(
        &self,
        future: impl std::future::Future<Output = std::result::Result<T, c::ServiceError>>,
    ) -> std::result::Result<T, c::ServiceError> {
        tokio::pin!(future);
        loop {
            tokio::select! {
                result=&mut future=>{self.active()?;return result;},
                _=tokio::time::sleep(std::time::Duration::from_millis(25))=>self.active()?,
            }
        }
    }
    fn provider_adapter(
        &self,
    ) -> std::result::Result<&super::provider_service::Adapter, c::ServiceError> {
        c::require_live(self.mode)?;
        self.active()?;
        self.provider_v2
            .as_ref()
            .ok_or(c::ServiceError::ProviderV2Unavailable)
    }
    fn active(&self) -> std::result::Result<(), c::ServiceError> {
        Store::active(&self.store.db().map_err(service)?, &self.run, self.epoch).map_err(service)
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
        if self.provider_v2.is_some()
            && !self
                .inputs
                .iter()
                .any(|old| old.id == a.id && old.file == a.file && old.provenance == a.provenance)
        {
            return Err(c::ServiceError::Unauthorized(
                "provider observation does not grant this execution artifact access".into(),
            ));
        }
        let bytes = self.store.artifact(&a.id).map_err(service)?.0;
        a.file.verify(&bytes)?;
        self.active()?;
        Ok(bytes)
    }
    async fn read_candidate(
        &self,
        candidate: &c::CandidateFile,
    ) -> std::result::Result<Vec<u8>, c::ServiceError> {
        self.active()?;
        candidate.file.validate()?;
        if !c::valid_id(&candidate.candidate_id) {
            return Err(c::ServiceError::Unauthorized(
                "invalid candidate identity".into(),
            ));
        }
        let bytes = {
            let candidates = self.candidates.lock().unwrap();
            let (stored, bytes) = candidates
                .get(&candidate.candidate_id)
                .ok_or_else(|| c::ServiceError::MissingCandidate(candidate.candidate_id.clone()))?;
            if stored.file != candidate.file || stored.provenance != candidate.provenance {
                return Err(c::ServiceError::Unauthorized(
                    "candidate identity mismatch".into(),
                ));
            }
            candidate.file.verify(bytes)?;
            bytes.clone()
        };
        self.active()?;
        Ok(bytes)
    }
    async fn write_candidate(
        &self,
        r: c::CandidateWrite,
        bytes: Vec<u8>,
    ) -> std::result::Result<c::CandidateFile, c::ServiceError> {
        self.active()?;
        if !matches!(
            r.media_type.as_str(),
            "application/json"
                | "text/plain"
                | "text/vtt"
                | "image/png"
                | "image/webp"
                | "audio/wav"
                | "audio/mpeg"
                | "application/octet-stream"
        ) {
            return Err(c::ServiceError::Unauthorized(
                "candidate media type is not allowlisted".into(),
            ));
        }
        if !c::valid_id(&r.candidate_id)
            || !c::safe_path(&r.relative_path)
            || bytes.len() as u64 > r.max_bytes
            || r.max_bytes > super::media::artifact_limit(&r.media_type) as u64
        {
            return Err(c::ServiceError::Unauthorized("candidate bounds".into()));
        }
        super::media::validate_artifact_bytes(&bytes, &r.media_type).map_err(service)?;
        let file = c::FileRef {
            path: r.relative_path,
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            media_type: r.media_type,
            dimensions: r.dimensions,
        };
        file.validate()?;
        let result = c::CandidateFile {
            candidate_id: r.candidate_id,
            file,
            provenance: r.provenance,
        };
        self.active()?;
        let mut candidates = self.candidates.lock().unwrap();
        let audio = |f: &c::CandidateFile| f.file.media_type == "audio/wav";
        let audio_bytes = candidates
            .values()
            .filter(|(f, _)| audio(f))
            .map(|(_, b)| b.len())
            .sum::<usize>()
            + if audio(&result) { bytes.len() } else { 0 };
        let other_bytes = candidates
            .values()
            .filter(|(f, _)| !audio(f))
            .map(|(_, b)| b.len())
            .sum::<usize>()
            + if !audio(&result) { bytes.len() } else { 0 };
        if candidates.len() >= 64
            || audio_bytes > super::media::MAX_MASTER_BYTES
            || other_bytes > super::media::MAX_ARTIFACT_BYTES
        {
            return Err(c::ServiceError::Limit("stage candidate size".into()));
        }
        if let Some((old, _)) = candidates.get(&result.candidate_id) {
            if old.file != result.file {
                return Err(c::ServiceError::Unauthorized(
                    "candidate identity reused".into(),
                ));
            }
        }
        candidates.insert(result.candidate_id.clone(), (result.clone(), bytes));
        Ok(result)
    }
    async fn emit(&self, entry: c::JournalProposal) -> std::result::Result<(), c::ServiceError> {
        self.active()?;
        if entry.summary.len() > 4000 {
            return Err(c::ServiceError::Limit("journal summary".into()));
        }
        let mut proposals = self.proposals.lock().unwrap();
        if proposals.len() >= 128 {
            return Err(c::ServiceError::Limit("journal entries".into()));
        }
        proposals.push(entry);
        Ok(())
    }
    async fn provider(
        &self,
        _: c::ProviderOperation,
    ) -> std::result::Result<c::ProviderResult, c::ServiceError> {
        self.active()?;
        c::require_live(self.mode)?;
        Err(c::ServiceError::Unauthorized(
            "live transport integration/accounting pending".into(),
        ))
    }
    async fn issue_provider_v2(
        &self,
        proposal: c::ProviderProposalV2,
    ) -> std::result::Result<c::ProviderOperationV2, c::ServiceError> {
        let adapter = self.provider_adapter()?;
        let issued = adapter.issue(&self.run, self.epoch, &self.unit, &self.inputs, &proposal)?;
        self.active()?;
        Ok(issued)
    }
    async fn execute_provider_v2(
        &self,
        issued: &c::ProviderOperationV2,
    ) -> std::result::Result<c::ProviderResultV2, c::ServiceError> {
        let adapter = self.provider_adapter()?;
        self.await_provider(adapter.execute(
            &self.run,
            self.epoch,
            &self.unit,
            &self.inputs,
            issued,
        ))
        .await
    }
    async fn recover_provider_v2(
        &self,
        request: c::ProviderRecoveryRequestV2,
    ) -> std::result::Result<c::ProviderRecoveryResultV2, c::ServiceError> {
        let adapter = self.provider_adapter()?;
        self.await_provider(adapter.recover(&self.run, self.epoch, &self.unit, request))
            .await
    }
    async fn helper(
        &self,
        operation: c::HelperOperation,
    ) -> std::result::Result<c::HelperResult, c::ServiceError> {
        self.active()?;
        let config = self
            .offline_acoustic
            .as_ref()
            .ok_or_else(|| c::ServiceError::Unauthorized("helper recipe not registered".into()))?;
        let bytes = super::helper::acoustic(
            &self.store,
            &self.run,
            self.epoch,
            self.mode,
            config,
            &operation,
        )
        .await
        .map_err(service)?;
        let record: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(Error::from)
            .map_err(service)?;
        if record["provider_calls"] != 0 || record["production_approved"] != false {
            return Err(c::ServiceError::Unauthorized(
                "offline helper response boundary".into(),
            ));
        }
        let candidate = self
            .write_candidate(
                c::CandidateWrite {
                    dimensions: None,
                    candidate_id: digest(format!("{}:{}", operation.id, digest(&bytes)).as_bytes()),
                    relative_path: "acoustic-replay.json".into(),
                    media_type: "application/json".into(),
                    max_bytes: operation.max_output_bytes,
                    provenance: c::Provenance::Diagnostic,
                },
                bytes,
            )
            .await?;
        let findings = record["missing_evidence"]
            .as_array()
            .ok_or_else(|| c::ServiceError::Internal("helper findings unavailable".into()))?
            .iter()
            .filter_map(|v| v.as_str())
            .map(|id| c::Finding {
                id: id.to_owned(),
                field: "acoustic".into(),
                outcome: c::ReviewOutcome::Missing,
                message: format!("Recorded replay leaves {id} unresolved."),
                evidence: vec![],
            })
            .collect();
        Ok(c::HelperResult {
            id: operation.id,
            outputs: vec![candidate],
            findings,
        })
    }
    async fn library(
        &self,
        q: c::LibraryQuery,
    ) -> std::result::Result<Vec<c::LibraryBinding>, c::ServiceError> {
        self.active()?;
        let db = self.store.db().map_err(service)?;
        let mut found = Vec::new();
        for entity in q.entity_ids {
            let saved: Option<String> = db
                .query_row(
                    "SELECT record FROM library WHERE book=?1 AND entity=?2",
                    params![q.book_id, entity],
                    |r| r.get(0),
                )
                .optional()
                .map_err(Error::from)
                .map_err(service)?;
            if let Some(saved) = saved {
                let binding: c::LibraryBinding = serde_json::from_str(&saved)
                    .map_err(Error::from)
                    .map_err(service)?;
                if binding.revision > q.revision {
                    return Err(c::ServiceError::RevisionConflict);
                }
                found.push(binding);
            }
        }
        Ok(found)
    }
}
#[derive(Clone)]
pub struct Runtime {
    pub store: Store,
    executor: Arc<dyn c::StageExecutor>,
    pub fixture_delay_ms: u64,
    saved_inputs: Option<(std::path::PathBuf, std::path::PathBuf)>,
    acoustic_inputs: Option<std::path::PathBuf>,
    presentation_inputs: Option<std::path::PathBuf>,
    acceptance_player: Option<(std::path::PathBuf, c::FileRef, String)>,
    chapter_helper: Option<super::chapter::ChapterHelper>,
    chapter_inputs: Option<std::path::PathBuf>,
    direction_pilot_inputs: Option<std::path::PathBuf>,
    speech_inputs: Option<std::path::PathBuf>,
    offline_acoustic: Option<super::helper::OfflineAcoustic>,
}
impl Runtime {
    pub fn new(store: Store, executor: Arc<dyn c::StageExecutor>) -> Self {
        Self {
            store,
            executor,
            fixture_delay_ms: 0,
            saved_inputs: None,
            acoustic_inputs: None,
            presentation_inputs: None,
            acceptance_player: None,
            chapter_helper: None,
            chapter_inputs: None,
            direction_pilot_inputs: None,
            speech_inputs: None,
            offline_acoustic: None,
        }
    }
    pub fn with_offline_acoustic(mut self, directory: std::path::PathBuf) -> Result<Self> {
        self.offline_acoustic = Some(super::helper::OfflineAcoustic::new(directory)?);
        Ok(self)
    }
    pub fn with_chapter_helper(mut self, config: super::chapter::ChapterHelper) -> Self {
        self.chapter_helper = Some(config);
        self
    }
    /// Trusted producer root containing chapter-evidence.json and bundle/.
    pub fn with_chapter_inputs(mut self, root: std::path::PathBuf) -> Self {
        self.chapter_inputs = Some(root);
        self
    }
    pub async fn import_chapter_bundle(
        &self,
        run: &str,
        epoch: i64,
        original: &std::path::Path,
        evidence: &str,
        bundle: &std::path::Path,
    ) -> Result<()> {
        let config = self.chapter_helper.as_ref().ok_or_else(|| {
            Error::new(
                "chapter_unregistered",
                "G21 helper registration is unavailable",
            )
        })?;
        super::chapter::import(
            &self.store,
            self.executor.as_ref(),
            config,
            run,
            epoch,
            original,
            evidence,
            bundle,
        )
        .await
    }
    pub async fn execute_chapter_unit(
        &self,
        run: &str,
        epoch: i64,
        label: &str,
    ) -> Result<c::StageOutcome> {
        let config = self.chapter_helper.as_ref().ok_or_else(|| {
            Error::new(
                "chapter_unregistered",
                "G21 helper registration is unavailable",
            )
        })?;
        super::chapter::execute_unit(
            &self.store,
            self.executor.as_ref(),
            config,
            run,
            epoch,
            label,
        )
        .await
    }
    pub async fn replay_chapter_unit(
        &self,
        run: &str,
        label: &str,
    ) -> Result<Vec<(c::FileRef, Vec<u8>)>> {
        let config = self
            .chapter_helper
            .as_ref()
            .ok_or_else(|| Error::new("chapter_unregistered", "G21 registration is unavailable"))?;
        super::chapter::replay(&self.store, self.executor.as_ref(), config, run, label).await
    }
    pub fn with_acoustic_inputs(mut self, root: std::path::PathBuf) -> Self {
        self.acoustic_inputs = Some(root);
        self
    }
    /// Trusted product root containing the frozen G18 input index's sources.
    pub fn with_presentation_inputs(mut self, root: std::path::PathBuf) -> Self {
        self.presentation_inputs = Some(root);
        self
    }
    /// Trusted startup registry only; no generation/request-supplied expected identities.
    pub fn with_acceptance_player(
        mut self,
        root: std::path::PathBuf,
        file: c::FileRef,
        version: String,
    ) -> Self {
        self.acceptance_player = Some((root, file, version));
        self
    }
    /// Recomputes private evidence against its recorded bytes, without durable writes.
    pub async fn replay_presentation(&self, run: &str) -> Result<Vec<u8>> {
        super::presentation::replay(&self.store, self.executor.as_ref(), run).await
    }
    pub fn with_saved_inputs(
        mut self,
        fixture: std::path::PathBuf,
        sources: std::path::PathBuf,
    ) -> Self {
        self.saved_inputs = Some((fixture, sources));
        self
    }
    pub fn with_saved_speech_inputs(mut self, root: std::path::PathBuf) -> Self {
        self.speech_inputs = Some(root);
        self
    }
    pub async fn materialize_speech_batches(&self, run: &str, epoch: i64) -> Result<()> {
        super::speech::materialize(&self.store, run, epoch).await
    }
    pub fn import_saved_speech(&self, run: &str, epoch: i64, root: &std::path::Path) -> Result<()> {
        super::speech::freeze(&self.store, run, epoch, root)
    }
    pub async fn run_saved_speech_until(&self, run: &str, epoch: i64, phases: usize) -> Result<()> {
        super::speech::run(&self.store, self.executor.as_ref(), run, epoch, phases).await
    }
    pub async fn replay_saved_speech(
        &self,
        run: &str,
    ) -> Result<Vec<(String, Vec<(c::FileRef, Vec<u8>)>)>> {
        super::speech::replay(&self.store, self.executor.as_ref(), run).await
    }
    pub fn with_saved_direction_pilot(mut self, root: std::path::PathBuf) -> Self {
        self.direction_pilot_inputs = Some(root);
        self
    }
    pub fn import_saved_direction_pilot(
        &self,
        run: &str,
        epoch: i64,
        root: &std::path::Path,
    ) -> Result<()> {
        super::direction_pilot::freeze(&self.store, run, epoch, root)
    }
    pub async fn run_saved_direction_pilot_until(
        &self,
        run: &str,
        epoch: i64,
        phases: usize,
    ) -> Result<()> {
        super::direction_pilot::run(&self.store, self.executor.as_ref(), run, epoch, phases).await
    }
    pub async fn replay_saved_direction_pilot(
        &self,
        run: &str,
    ) -> Result<Vec<(String, Vec<(c::FileRef, Vec<u8>)>)>> {
        super::direction_pilot::replay(&self.store, self.executor.as_ref(), run).await
    }
    pub async fn work_once(&self) -> Result<bool> {
        self.store.wake_due()?;
        let Some((run, epoch)) = self.store.claim()? else {
            return Ok(false);
        };
        if self.store.snapshot(&run)?["request"]["pins"]["recipe_sha256"]
            .as_str()
            .is_some_and(super::speech::is_recipe)
        {
            let result=async{
                let exists:bool=self.store.db()?.query_row("SELECT EXISTS(SELECT 1 FROM speech_flows WHERE run=?1)",[&run],|r|r.get(0))?;
                if !exists{let root=self.speech_inputs.as_ref().ok_or_else(||Error::new("speech_unavailable","trusted Speech inputs are not configured"))?;self.import_saved_speech(&run,epoch,root)?;}
                self.run_saved_speech_until(&run,epoch,3).await?;self.materialize_speech_batches(&run,epoch).await?;fail("stage_needs_repair","saved Speech/PCM cannot authorize bulk synthesis, transcription or publication")
            }.await;
            if self.store.is_active(&run, epoch)? {
                self.store
                    .finish(&run, epoch, "failed", result.as_ref().err().map(|e| e.code))?;
            }
            return result;
        }
        if self.store.snapshot(&run)?["request"]["pins"]["recipe_sha256"]
            == super::direction_pilot::RECIPE
        {
            let result = async {
                let exists: bool = self.store.db()?.query_row(
                    "SELECT EXISTS(SELECT 1 FROM saved_flow_plans WHERE run=?1)",
                    [&run],
                    |r| r.get(0),
                )?;
                if !exists {
                    let root = self.direction_pilot_inputs.as_ref().ok_or_else(|| {
                        Error::new(
                            "saved_flow_unavailable",
                            "trusted saved Direction/Pilot inputs are not configured",
                        )
                    })?;
                    self.import_saved_direction_pilot(&run, epoch, root)?;
                }
                self.run_saved_direction_pilot_until(&run, epoch, 6).await?;
                fail(
                    "stage_needs_repair",
                    "saved Direction/Pilot evidence cannot approve bulk Speech or publication",
                )
            }
            .await;
            if self.store.is_active(&run, epoch)? {
                self.store
                    .finish(&run, epoch, "failed", result.as_ref().err().map(|e| e.code))?;
            }
            return result;
        }
        if self.store.snapshot(&run)?["request"]["pins"]["recipe_sha256"] == super::chapter::RECIPE
        {
            let exists: bool = self.store.db()?.query_row(
                "SELECT EXISTS(SELECT 1 FROM chapter_plans WHERE run=?1)",
                [&run],
                |r| r.get(0),
            )?;
            if !exists {
                let imported = match &self.chapter_inputs {
                    Some(root) => {
                        self.import_chapter_bundle(
                            &run,
                            epoch,
                            root,
                            "chapter-evidence.json",
                            &root.join("bundle"),
                        )
                        .await
                    }
                    None => fail(
                        "chapter_inputs_unavailable",
                        "trusted original chapter/bundle inputs are not configured",
                    ),
                };
                if let Err(e) = imported {
                    if self.store.is_active(&run, epoch)? {
                        self.store.finish(&run, epoch, "failed", Some(e.code))?;
                    }
                    return Err(e);
                }
            }
        }
        if self.store.snapshot(&run)?["request"]["pins"]["recipe_sha256"]
            == digest(b"saved-diagnostic-v1")
        {
            let prepared = match &self.saved_inputs {
                Some((fixture, sources)) => self
                    .store
                    .seed_fixture(&run, epoch, fixture, Some(sources))
                    .and_then(|_| match &self.acoustic_inputs {
                        Some(root) => self.store.seed_acoustic_fixture(&run, epoch, root),
                        None => fail(
                            "fixture_unavailable",
                            "stage-specific acoustic inputs are not configured",
                        ),
                    })
                    .and_then(|_| match &self.presentation_inputs {
                        Some(root) => self.store.seed_presentation_fixture(&run, epoch, root),
                        None => fail(
                            "fixture_unavailable",
                            "Presentation inputs are not configured",
                        ),
                    }),
                None => fail(
                    "fixture_unavailable",
                    "saved fixture inputs are not configured",
                ),
            };
            if let Err(e) = prepared {
                self.store.finish(&run, epoch, "failed", Some(e.code))?;
                return Err(e);
            }
        }
        if let Some((root, file, version)) = &self.acceptance_player {
            if let Err(e) = self
                .store
                .register_acceptance_player(&run, epoch, root, file, version)
            {
                self.store.finish(&run, epoch, "failed", Some(e.code))?;
                return Err(e);
            }
        }
        self.execute_claimed(&run, epoch).await?;
        Ok(true)
    }
    pub async fn execute_claimed(&self, run: &str, epoch: i64) -> Result<()> {
        let request = self.store.snapshot(run)?["request"].clone();
        let diagnostic = request["mode"] == "fixture"
            && request["pins"]["recipe_sha256"] == digest(b"saved-diagnostic-v1");
        let mut needs_repair = false;
        let mut result = Ok(());
        let mut package_visited = false;
        for stage in c::STAGES {
            if stage == c::Stage::Package {
                package_visited = true;
            }
            if let Err(e) = self.execute_stage(run, epoch, stage).await {
                if diagnostic
                    && matches!(stage, c::Stage::Acoustic | c::Stage::Presentation)
                    && e.code == "stage_needs_repair"
                {
                    needs_repair = true;
                    continue;
                }
                result = Err(e);
                break;
            }
        }
        if diagnostic
            && package_visited
            && (result.is_ok()
                || result
                    .as_ref()
                    .is_err_and(|e| e.code == "stage_needs_repair"))
            && self.store.diagnostic_ref(run)?.is_some()
        {
            match self.store.verify_private_acceptance(run, epoch, &[]) {
                Ok(_) => needs_repair = true,
                Err(e) if e.code == "acceptance_registry_missing" => {
                    self.store
                        .private_acceptance_unavailable(run, epoch, e.code)?;
                    needs_repair = true;
                }
                Err(e) => result = Err(e),
            }
        }
        if needs_repair && result.is_ok() {
            result = fail(
                "stage_needs_repair",
                "required diagnostic evidence remains unresolved",
            );
        }
        if self.store.snapshot(run)?["state"] == "waiting_provider" {
            return Ok(());
        }
        if self.store.is_active(run, epoch)? {
            match &result {
                Ok(()) => self.store.finish(run, epoch, "succeeded", None)?,
                Err(e) => self.store.finish(run, epoch, "failed", Some(e.code))?,
            }
        }
        if result.as_ref().is_err_and(|e| e.code == "stale_epoch") {
            return Ok(());
        }
        result
    }
    pub async fn execute_stage(
        &self,
        run: &str,
        epoch: i64,
        stage: c::Stage,
    ) -> Result<Vec<c::ArtifactRef>> {
        Store::active(&self.store.db()?, run, epoch)?;
        let snapshot = self.store.snapshot(run)?;
        let request: c::RunRequest = serde_json::from_value(snapshot["request"].clone())?;
        if request.pins.recipe_sha256 == super::direction_pilot::RECIPE
            || super::speech::is_recipe(&request.pins.recipe_sha256)
        {
            return fail(
                "saved_flow_scope",
                "saved phases require their private checkpoint path; bulk stages have no grant",
            );
        }
        if stage == c::Stage::Acoustic && request.pins.recipe_sha256 == super::chapter::RECIPE {
            let units = self.store.chapter_units(run)?;
            for unit in units {
                self.execute_chapter_unit(run, epoch, &unit.label).await?;
            }
            return fail(
                "stage_needs_repair",
                "bounded private chapter units retain missing semantic/source-mapping evidence",
            );
        }
        let stage_name = serde_json::to_value(stage)?.as_str().unwrap().to_owned();
        let mut inputs = self.store.seed_inputs_for_stage(run, stage)?;
        let db = self.store.db()?;
        for prior in c::STAGES {
            if matches!(stage, c::Stage::Acoustic | c::Stage::Presentation) {
                break;
            }
            if prior == stage {
                break;
            }
            let name = serde_json::to_value(prior)?.as_str().unwrap().to_owned();
            let saved:Option<String>=db.query_row("SELECT output FROM units WHERE run=?1 AND stage=?2 ORDER BY rowid DESC LIMIT 1",params![run,name],|r|r.get(0)).optional()?;
            if let Some(saved) = saved {
                inputs.extend(serde_json::from_str::<Vec<c::ArtifactRef>>(&saved)?);
            }
        }
        if stage == c::Stage::Presentation
            && request.pins.recipe_sha256 == digest(b"saved-diagnostic-v1")
        {
            // Cache reuse must verify dependencies as well as the saved report.
            self.store.verify_presentation_inputs(&inputs)?;
        }
        let mut identity = json!({"request":snapshot["request"],"inputs":inputs,"stage":stage,"contracts":c::CONTRACT_REVISION,"services":c::SERVICE_REVISION});
        if stage == c::Stage::Acoustic {
            identity["helper_recipe_sha256"] =
                serde_json::to_value(self.offline_acoustic.as_ref().map(|helper| &helper.sha256))?;
        }
        let input_hash = digest(&serde_json::to_vec(&identity)?);
        let key = digest(format!("{stage_name}:{input_hash}").as_bytes());
        if let Some((outcome, saved)) = self.store.cached(run, &stage_name, &key, &input_hash)? {
            self.store.transaction(|tx|{Store::active(tx,run,epoch)?;Store::emit(tx,run,"cache_reused",&json!({"stage":stage_name,"unit_key":key,"outcome":outcome,"artifact_ids":saved.iter().map(|a|&a.id).collect::<Vec<_>>()}))?;Ok(())})?;
            if outcome == "needs_repair" {
                return fail(
                    "stage_needs_repair",
                    "recorded diagnostic findings remain unresolved",
                );
            }
            return Ok(saved);
        }
        let input = c::StageInput {
            schema_version: 1,
            run_id: run.into(),
            stage,
            unit_id: key.clone(),
            epoch: epoch as u64,
            revision: 0,
            inputs,
            pins: request.pins,
            mode: request.mode,
            limits: request.limits,
            library_revision: 0,
        };
        input
            .validate()
            .map_err(|e| Error::new("invalid_input", e.to_string()))?;
        self.store.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            tx.execute(
                "UPDATE stages SET state='running' WHERE run=?1 AND name=?2",
                params![run, stage_name],
            )?;
            Store::emit(
                tx,
                run,
                "task_started",
                &json!({"stage":stage_name,"unit_key":key}),
            )?;
            Ok(())
        })?;
        let services = Services {
            store: self.store.clone(),
            run: run.into(),
            epoch,
            mode: request.mode,
            unit: input.unit_id.clone(),
            inputs: Arc::new(input.inputs.clone()),
            provider_v2: None,
            candidates: Arc::new(Mutex::new(HashMap::new())),
            proposals: Arc::new(Mutex::new(Vec::new())),
            offline_acoustic: self.offline_acoustic.clone(),
        };
        if self.fixture_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.fixture_delay_ms)).await;
        }
        let deadline =
            tokio::time::sleep(std::time::Duration::from_millis(input.limits.wall_time_ms));
        tokio::pin!(deadline);
        let future = self.executor.execute(input, &services);
        tokio::pin!(future);
        let output = loop {
            tokio::select! {
                result=&mut future=>break result.map_err(|e|match e {c::StageError::Service(c::ServiceError::Cancelled)=>Error::new("stale_epoch","executor cancelled"),_=>Error::new("stage_failed",e.to_string())})?,
                _=&mut deadline=>return fail("stage_timeout","executor exceeded its wall-time allowance"),
                _=tokio::time::sleep(std::time::Duration::from_millis(25))=>Store::active(&self.store.db()?,run,epoch)?,
            }
        };
        let outcome = output.outcome;
        if outcome == c::StageOutcome::Cancelled {
            self.store.cancel(run)?;
            return fail("stale_epoch", "executor cancelled the run");
        }
        let artifacts = self.store.commit_shared(
            run,
            epoch,
            &stage_name,
            &key,
            &input_hash,
            output,
            &services,
        )?;
        match outcome {
            c::StageOutcome::Complete => {}
            c::StageOutcome::NeedsRepair => {
                return fail(
                    "stage_needs_repair",
                    "diagnostic artifacts preserved; required evidence remains unresolved",
                );
            }
            c::StageOutcome::Failed => return fail("stage_failed", "required stage failed"),
            c::StageOutcome::Cancelled => return fail("stale_epoch", "executor cancelled"),
        };
        Ok(artifacts)
    }
}
impl Store {
    fn cached(
        &self,
        run: &str,
        stage: &str,
        key: &str,
        input_hash: &str,
    ) -> Result<Option<(String, Vec<c::ArtifactRef>)>> {
        let saved:Option<(String,String,String)>=self.db()?.query_row("SELECT input_hash,output,outcome FROM units WHERE run=?1 AND stage=?2 AND unit_key=?3 AND outcome IN ('complete','needs_repair')",params![run,stage,key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some((hash, output, outcome)) = saved {
            if hash != input_hash {
                return fail("stale_unit", "unit inputs changed");
            }
            let artifacts: Vec<c::ArtifactRef> = serde_json::from_str(&output)?;
            for a in &artifacts {
                a.file
                    .verify(&self.artifact(&a.id)?.0)
                    .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
            }
            return Ok(Some((outcome, artifacts)));
        }
        Ok(None)
    }
    fn commit_shared(
        &self,
        run: &str,
        epoch: i64,
        stage: &str,
        key: &str,
        input_hash: &str,
        output: c::StageOutput,
        services: &Services,
    ) -> Result<Vec<c::ArtifactRef>> {
        if output.artifacts.len() > 64 || output.decisions.len() > 64 || output.findings.len() > 128
        {
            return fail(
                "stage_boundary",
                "bounded output required; shared package activation pending R2-005",
            );
        }
        let mut artifacts = Vec::new();
        {
            let candidates = services.candidates.lock().unwrap();
            for a in &output.artifacts {
                let (stored, bytes) = candidates.get(&a.candidate_id).ok_or_else(|| {
                    Error::new(
                        "missing_candidate",
                        "candidate was not written through services",
                    )
                })?;
                if stored.file != a.file || stored.provenance != a.provenance {
                    return fail(
                        "invalid_candidate",
                        "returned candidate differs from written bytes",
                    );
                }
                a.file
                    .verify(bytes)
                    .map_err(|e| Error::new("invalid_candidate", e.to_string()))?;
                self.put_typed(bytes, &a.file.media_type)?;
                artifacts.push(c::ArtifactRef {
                    id: digest(format!("{run}:{key}:{}:{}", a.file.path, a.file.sha256).as_bytes()),
                    file: a.file.clone(),
                    provenance: a.provenance,
                });
            }
        }
        let snapshot = self.snapshot(run)?;
        let pins: c::Pins = serde_json::from_value(snapshot["request"]["pins"].clone())?;
        let candidates = if output.package_candidate.is_some() {
            services
                .candidates
                .lock()
                .unwrap()
                .values()
                .map(|(f, b)| (f.file.path.clone(), b.clone()))
                .collect()
        } else {
            std::collections::BTreeMap::new()
        };
        let mapping = super::fixture::candidate_mapping(
            &output,
            &self.seed_inputs(run)?,
            &artifacts,
            &candidates,
            &pins,
        )?;
        self.transaction(|tx|{
            Store::active(tx,run,epoch)?;
            for a in &artifacts {tx.execute("INSERT INTO artifacts(id,run,name,sha256,bytes,media_type,provenance,reference) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![a.id,run,a.file.path,a.file.sha256,a.file.bytes,a.file.media_type,serde_json::to_value(a.provenance)?.as_str(),serde_json::to_string(a)?])?;Store::emit(tx,run,"artifact_ready",&json!({"stage":stage,"unit_key":key,"artifact":a}))?;}
            for d in &output.decisions {
                if d.summary.is_empty() || d.summary.len()>4000 || d.inputs.is_empty(){return fail("invalid_decision","source/input references and recorded explanation required");}
                let mut links=Vec::new();
                for a in &d.inputs{a.file.verify(&self.artifact(&a.id)?.0).map_err(|e|Error::new("invalid_decision",e.to_string()))?;links.push(a.id.clone());}
                for selected in &d.selected_ids {if let Some(index)=output.artifacts.iter().position(|a|a.candidate_id==*selected){links.push(artifacts[index].id.clone());}else if tx.query_row("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=?1)",[selected],|r|r.get::<_,bool>(0))?{links.push(selected.clone());}}
                links.sort();links.dedup();let mut record=serde_json::to_value(d)?;record["artifact_ids"]=json!(links);Store::emit(tx,run,"director_decision",&record)?;
            }
            for f in &output.findings{Store::emit(tx,run,"check_completed",&serde_json::to_value(f)?)?;}
            for update in &output.library_updates {if update.artifacts.is_empty(){return fail("invalid_library","binding needs verified artifact refs");}let old:Option<i64>=tx.query_row("SELECT revision FROM library WHERE book=?1 AND entity=?2",params![update.book_id,update.entity_id],|r|r.get(0)).optional()?;if old.unwrap_or(0) as u64!=update.expected_revision{return fail("library_conflict","binding revision changed");}for a in &update.artifacts{let known:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=?1 AND sha256=?2)",params![a.id,a.file.sha256],|r|r.get(0))?;if !known{return fail("invalid_library","unknown binding artifact");}}
                let binding=c::LibraryBinding{book_id:update.book_id.clone(),entity_id:update.entity_id.clone(),binding:update.binding,revision:update.expected_revision+1,artifacts:update.artifacts.clone()};tx.execute("INSERT INTO library VALUES(?1,?2,?3,?4) ON CONFLICT(book,entity) DO UPDATE SET revision=excluded.revision,record=excluded.record",params![update.book_id,update.entity_id,binding.revision,serde_json::to_string(&binding)?])?;Store::emit(tx,run,"library_bound",&serde_json::to_value(binding)?)?;
            }
            for proposal in services.proposals.lock().unwrap().iter(){Store::emit(tx,run,"journal_entry",&serde_json::to_value(proposal)?)?;}
            if let Some((manifest_hash,mapping))=&mapping {tx.execute("INSERT INTO diagnostic_packages VALUES(?1,?2,?3,?4,?5) ON CONFLICT(run) DO UPDATE SET epoch=excluded.epoch,manifest_hash=excluded.manifest_hash,mapping=excluded.mapping,candidate=excluded.candidate",params![run,epoch,manifest_hash,serde_json::to_string(mapping)?,serde_json::to_string(&output.package_candidate)?])?;Store::emit(tx,run,"diagnostic_package_ready",&json!({"manifest_sha256":manifest_hash,"production_acceptance":"unresolved"}))?;}
            let (outcome,stage_state)=match output.outcome {c::StageOutcome::Complete=>("complete","succeeded"),c::StageOutcome::NeedsRepair=>("needs_repair","repairing"),c::StageOutcome::Failed=>("failed","failed"),c::StageOutcome::Cancelled=>("cancelled","cancelled")};
            tx.execute("INSERT INTO units VALUES(?1,?2,?3,?4,?5,?6)",params![run,stage,key,input_hash,serde_json::to_string(&artifacts)?,outcome])?;tx.execute("UPDATE stages SET state=?3 WHERE run=?1 AND name=?2",params![run,stage,stage_state])?;Store::emit(tx,run,"task_completed",&json!({"stage":stage,"unit_key":key,"outcome":outcome,"artifact_ids":artifacts.iter().map(|a|&a.id).collect::<Vec<_>>()}))?;Ok(artifacts)
        })
    }
    pub fn wake_due(&self) -> Result<()> {
        self.transaction(|tx| {
            let ids = tx
                .prepare("SELECT id FROM runs WHERE state='waiting_provider' AND retry_at<=?1")?
                .query_map([now()], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for id in ids {
                tx.execute("UPDATE runs SET state='recovering' WHERE id=?1", [&id])?;
                Store::emit(tx, &id, "provider_wait_ended", &json!({}))?;
            }
            Ok(())
        })
    }
}
pub struct FixtureExecutor;
#[async_trait]
impl c::StageExecutor for FixtureExecutor {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        services.check_cancelled().await?;
        let name = serde_json::to_value(input.stage)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let bytes=serde_json::to_vec(&json!({"schema_version":1,"provenance":"synthetic","diagnostic":true,"audience_eligible":false,"stage":input.stage,"missing_acceptance":["actual-source-media","independent-review","render-evidence"],"note":"Structural fixture; no creative decisions or measured work are claimed."})).unwrap();
        let artifact = services
            .write_candidate(
                c::CandidateWrite {
                    dimensions: None,
                    candidate_id: input.unit_id,
                    relative_path: format!("{name}-fixture.json"),
                    media_type: "application/json".into(),
                    max_bytes: 4096,
                    provenance: c::Provenance::Synthetic,
                },
                bytes,
            )
            .await?;
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![artifact],
            decisions: vec![],
            findings: vec![],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}

/// The server supplies generation through its shared trait; runtime never imports its sibling.
pub struct FixturePipeline {
    generation: Arc<dyn c::StageExecutor>,
}
impl FixturePipeline {
    pub fn new(generation: Arc<dyn c::StageExecutor>) -> Self {
        Self { generation }
    }
}
#[async_trait]
impl c::StageExecutor for FixturePipeline {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        if super::direction_pilot::registered(input.stage, &input.pins.recipe_sha256)
            || (input.stage == c::Stage::Speech
                && input.pins.recipe_sha256
                    == "9f36e9d1bf5718c1c0dab46f1d24971b73cc686636d8aee748763a8620f3a956")
            || (input.stage == c::Stage::Acoustic
                && input.pins.recipe_sha256 == super::chapter::RECIPE)
            || input.pins.recipe_sha256 == digest(b"saved-diagnostic-v1")
                && matches!(
                    input.stage,
                    c::Stage::Source
                        | c::Stage::Acoustic
                        | c::Stage::Presentation
                        | c::Stage::Package
                )
        {
            self.generation.execute(input, services).await
        } else {
            FixtureExecutor.execute(input, services).await
        }
    }
}
