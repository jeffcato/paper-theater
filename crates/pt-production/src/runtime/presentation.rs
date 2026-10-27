//! Frozen private G18 inputs. Delivery paths are intentionally namespaced.
use super::store::*;
use async_trait::async_trait;
use pt_contracts as c;
use rusqlite::OptionalExtension;
use rusqlite::params;
use serde::Deserialize;
use serde_json::json;
use std::sync::Mutex;
use std::{collections::BTreeSet, path::Path};

const INDEX: &[u8] =
    include_bytes!("../../../../fixtures/generation/presentation-evidence-v1/input-index.json");
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    artifact: c::ArtifactRef,
    source: String,
}

const REPORT: &str = "presentation-fact-evidence.json";
struct Replay<'a> {
    store: &'a Store,
    inputs: Vec<c::ArtifactRef>,
    expected: c::ArtifactRef,
    candidate: Mutex<Option<c::CandidateFile>>,
}
fn denied() -> c::ServiceError {
    c::ServiceError::Unauthorized("recorded Presentation replay is read-only".into())
}
#[async_trait]
impl c::RunServices for Replay<'_> {
    async fn check_cancelled(&self) -> std::result::Result<(), c::ServiceError> {
        Ok(())
    }
    async fn read_artifact(
        &self,
        a: &c::ArtifactRef,
    ) -> std::result::Result<Vec<u8>, c::ServiceError> {
        if !self.inputs.iter().any(|saved| {
            saved.id == a.id && saved.file == a.file && saved.provenance == a.provenance
        }) {
            return Err(denied());
        }
        let bytes = self
            .store
            .artifact_bounded(&a.id, super::media::MAX_ARTIFACT_BYTES)
            .map_err(|e| c::ServiceError::Internal(e.to_string()))?
            .0;
        a.file.verify(&bytes)?;
        Ok(bytes)
    }
    async fn write_candidate(
        &self,
        r: c::CandidateWrite,
        bytes: Vec<u8>,
    ) -> std::result::Result<c::CandidateFile, c::ServiceError> {
        if !c::valid_id(&r.candidate_id)
            || r.relative_path != REPORT
            || r.media_type != self.expected.file.media_type
            || r.dimensions != self.expected.file.dimensions
            || r.provenance != self.expected.provenance
            || bytes.len() as u64 > r.max_bytes
            || r.max_bytes > super::media::MAX_ARTIFACT_BYTES as u64
        {
            return Err(denied());
        }
        self.expected.file.verify(&bytes)?;
        let result = c::CandidateFile {
            candidate_id: r.candidate_id,
            file: self.expected.file.clone(),
            provenance: r.provenance,
        };
        let mut candidate = self.candidate.lock().unwrap();
        if candidate.is_some() {
            return Err(denied());
        }
        *candidate = Some(result.clone());
        Ok(result)
    }
    async fn emit(&self, _: c::JournalProposal) -> std::result::Result<(), c::ServiceError> {
        Err(denied())
    }
    async fn provider(
        &self,
        _: c::ProviderOperation,
    ) -> std::result::Result<c::ProviderResult, c::ServiceError> {
        Err(denied())
    }
    async fn helper(
        &self,
        _: c::HelperOperation,
    ) -> std::result::Result<c::HelperResult, c::ServiceError> {
        Err(denied())
    }
    async fn library(
        &self,
        _: c::LibraryQuery,
    ) -> std::result::Result<Vec<c::LibraryBinding>, c::ServiceError> {
        Err(denied())
    }
}
pub(super) async fn replay(
    store: &Store,
    executor: &dyn c::StageExecutor,
    run: &str,
) -> Result<Vec<u8>> {
    let snapshot = store.snapshot(run)?;
    let request: c::RunRequest = serde_json::from_value(snapshot["request"].clone())?;
    if request.mode != c::ExecutionMode::Fixture
        || request.pins.recipe_sha256 != digest(b"saved-diagnostic-v1")
        || !matches!(
            snapshot["state"].as_str(),
            Some("failed" | "succeeded" | "cancelled")
        )
    {
        return fail(
            "replay_unavailable",
            "recorded Presentation requires a terminal saved diagnostic run",
        );
    }
    let saved:Option<(String,String)>=store.db()?.query_row("SELECT unit_key,output FROM units WHERE run=?1 AND stage='presentation' AND outcome='needs_repair' ORDER BY rowid DESC LIMIT 1",[run],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let (unit_id, encoded) = saved
        .ok_or_else(|| Error::new("replay_unavailable", "no committed Presentation evidence"))?;
    let output: Vec<c::ArtifactRef> = serde_json::from_str(&encoded)?;
    if output.len() != 1
        || output[0].file.path != REPORT
        || output[0].provenance != c::Provenance::Diagnostic
    {
        return fail(
            "replay_unavailable",
            "unexpected Presentation output identity",
        );
    }
    let expected = output[0].clone();
    let bytes = store
        .artifact_bounded(&expected.id, super::media::MAX_ARTIFACT_BYTES)?
        .0;
    expected
        .file
        .verify(&bytes)
        .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
    let mut inputs = store.seed_inputs_for_stage(run, c::Stage::Presentation)?;
    store.verify_presentation_inputs(&inputs)?;
    inputs.push(expected.clone());
    let input = c::StageInput {
        schema_version: 1,
        run_id: run.into(),
        stage: c::Stage::Presentation,
        unit_id,
        epoch: snapshot["epoch"].as_u64().unwrap(),
        revision: 0,
        inputs: inputs.clone(),
        pins: request.pins,
        mode: c::ExecutionMode::Replay,
        limits: request.limits,
        library_revision: 0,
    };
    let services = Replay {
        store,
        inputs,
        expected,
        candidate: Mutex::new(None),
    };
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(input.limits.wall_time_ms),
        executor.execute(input, &services),
    )
    .await
    .map_err(|_| Error::new("stage_timeout", "recorded Presentation deadline exceeded"))?
    .map_err(|e| Error::new("replay_failed", e.to_string()))?;
    let written = services.candidate.lock().unwrap();
    let candidate = written
        .as_ref()
        .ok_or_else(|| Error::new("replay_failed", "report was not reproduced"))?;
    if result.outcome != c::StageOutcome::NeedsRepair
        || result.artifacts.len() != 1
        || result.artifacts[0].candidate_id != candidate.candidate_id
        || result.artifacts[0].file != candidate.file
        || result.artifacts[0].provenance != candidate.provenance
        || !result.decisions.is_empty()
        || !result.library_updates.is_empty()
        || result.package_candidate.is_some()
    {
        return fail("replay_failed", "recorded result identity changed");
    }
    Ok(bytes)
}
fn bindings() -> Result<Vec<Binding>> {
    let bindings: Vec<Binding> = serde_json::from_slice(INDEX)?;
    let mut paths = BTreeSet::new();
    if bindings.len() != 56 {
        return fail(
            "invalid_fixture",
            "G18 requires its complete frozen input inventory",
        );
    }
    let mut total = 0;
    for binding in &bindings {
        let f = &binding.artifact.file;
        f.validate()
            .map_err(|e| Error::new("invalid_fixture", e.to_string()))?;
        total += f.bytes;
        if !c::safe_path(&binding.source)
            || !paths.insert(&f.path)
            || f.bytes > super::media::MAX_ARTIFACT_BYTES as u64
            || total > 64 * 1024 * 1024
        {
            return fail("invalid_fixture", "G18 input identity or envelope rejected");
        }
    }
    Ok(bindings)
}
impl Store {
    /// Imports the complete compiled inventory atomically from trusted local configuration.
    /// Reopening verifies the frozen durable scope; it never appends replacement inputs.
    pub fn seed_presentation_fixture(&self, run: &str, epoch: i64, root: &Path) -> Result<()> {
        Store::active(&self.db()?, run, epoch)?;
        let bindings = bindings()?;
        let existing = self.seed_inputs_for_stage(run, c::Stage::Presentation)?;
        if !existing.is_empty() {
            return self.verify_presentation_inputs(&existing);
        }
        let mut prepared = Vec::new();
        for binding in bindings {
            let reference = binding.artifact;
            let verified = super::media::VerifiedArtifact::open(
                root,
                &c::FileRef {
                    path: binding.source,
                    ..reference.file.clone()
                },
            )?;
            use std::io::Read;
            let mut bytes = Vec::with_capacity(verified.bytes as usize);
            verified
                .file
                .take(verified.bytes + 1)
                .read_to_end(&mut bytes)?;
            reference
                .file
                .verify(&bytes)
                .map_err(|e| Error::new("invalid_fixture", e.to_string()))?;
            prepared.push((
                c::ArtifactRef {
                    id: digest(
                        format!(
                            "{run}:presentation:{}:{}",
                            reference.file.path, reference.file.sha256
                        )
                        .as_bytes(),
                    ),
                    ..reference
                },
                bytes,
            ));
        }
        // Verify the whole bundle before writing any object or committing its namespace.
        for (reference, bytes) in &prepared {
            self.put_typed(bytes, &reference.file.media_type)?;
        }
        self.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            let count:i64 = tx.query_row("SELECT COUNT(*) FROM run_input_scopes WHERE run=?1 AND scope='presentation'", [run], |r|r.get(0))?;
            if count != 0 {
                return fail("input_scope_frozen", "Presentation namespace was already imported");
            }
            for (a, _) in &prepared {
                let encoded = serde_json::to_string(a)?;
                tx.execute("INSERT INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![a.id,run,a.file.path,a.file.sha256,a.file.bytes,a.file.media_type,serde_json::to_value(a.provenance)?.as_str(),encoded])?;
                tx.execute("INSERT INTO run_inputs VALUES(?1,?2,?3)",params![run,a.id,encoded])?;
                tx.execute("INSERT INTO run_input_scopes VALUES(?1,?2,'presentation')",params![run,a.id])?;
            }
            Store::emit(tx,run,"presentation_inputs_imported",&json!({"index_sha256":digest(INDEX),"artifact_ids":prepared.iter().map(|(a,_)|&a.id).collect::<Vec<_>>(),"production_approved":false}))?;
            Ok(())
        })
    }
    pub(crate) fn verify_presentation_inputs(&self, inputs: &[c::ArtifactRef]) -> Result<()> {
        let expected = bindings()?;
        if inputs.len() != expected.len()
            || expected.iter().any(|b| {
                inputs
                    .iter()
                    .filter(|a| a.file == b.artifact.file && a.provenance == b.artifact.provenance)
                    .count()
                    != 1
            })
        {
            return fail(
                "input_scope_mismatch",
                "Presentation requires every exact namespaced binding",
            );
        }
        for a in inputs {
            a.file
                .verify(
                    &self
                        .artifact_bounded(&a.id, super::media::MAX_ARTIFACT_BYTES)?
                        .0,
                )
                .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
        }
        Ok(())
    }
}
