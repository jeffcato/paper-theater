//! Runtime-owned private verification. Serialized reports confer no authority.
use super::store::*;
use pt_contracts as c;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};
mod compilation;
const REPORT_LIMIT: usize = 2 * 1024 * 1024;
const METADATA_LIMIT: usize = 4 * 1024 * 1024;
#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct Compilation {
    schema_version: u32,
    run_id: String,
    request_sha256: String,
    policy: c::AcceptanceVerificationPolicy,
    document: c::FileRef,
    package: Vec<c::FileRef>,
    private: Vec<c::FileRef>,
    observations_sha256: String,
}
// No Clone, Deserialize, public fields or report-to-capability constructor.
pub(crate) struct Prepared {
    store: Store,
    run: String,
    epoch: i64,
    compilation: Compilation,
    compilation_ref: c::FileRef,
    verification: c::PrivateAcceptanceVerification,
    _package_root: tempfile::TempDir,
    _private_root: tempfile::TempDir,
}
fn reference(path: &str, bytes: &[u8], mime: &str) -> c::FileRef {
    c::FileRef {
        path: path.into(),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
        media_type: mime.into(),
        dimensions: None,
    }
}
fn metadata(store: &Store, path: &str, value: &impl Serialize) -> Result<c::FileRef> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > METADATA_LIMIT {
        return fail("acceptance_bounds", "private metadata envelope exceeded");
    }
    store.put(&bytes)?;
    Ok(reference(path, &bytes, "application/json"))
}
fn open_object(store: &Store, file: &c::FileRef) -> Result<super::media::VerifiedArtifact> {
    file.validate()
        .map_err(|e| Error::new("acceptance_input", e.to_string()))?;
    let path = store.object(&file.sha256)?;
    if !std::fs::symlink_metadata(&path)
        .map_err(|_| Error::new("corrupt_artifact", "private object missing"))?
        .is_file()
    {
        return fail("corrupt_artifact", "private object must be a regular file");
    }
    super::media::VerifiedArtifact::checked(
        std::fs::File::open(path)?,
        file.bytes,
        file.media_type.clone(),
        file.sha256.clone(),
    )
}
fn read_object(store: &Store, file: &c::FileRef, limit: usize) -> Result<Vec<u8>> {
    if file.bytes > limit as u64 {
        return fail("acceptance_bounds", "private input envelope exceeded");
    }
    let mut verified = open_object(store, file)?;
    let mut bytes = Vec::with_capacity(file.bytes as usize);
    Read::by_ref(&mut verified.file)
        .take(file.bytes + 1)
        .read_to_end(&mut bytes)?;
    file.verify(&bytes)
        .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
    Ok(bytes)
}
fn materialize(
    store: &Store,
    run: &str,
    epoch: i64,
    files: &[c::FileRef],
    root: &std::path::Path,
) -> Result<()> {
    let mut total = 0u64;
    let mut paths = BTreeMap::new();
    for file in files {
        Store::active(&store.db()?, run, epoch)?;
        if let Some(previous) = paths.insert(&file.path, file) {
            if previous != file {
                return fail("acceptance_input", "conflicting snapshot path");
            }
            continue;
        }
        total = total
            .checked_add(file.bytes)
            .ok_or_else(|| Error::new("acceptance_bounds", "size overflow"))?;
        if files.len() > 2048 || total > 512 * 1024 * 1024 {
            return fail("acceptance_bounds", "snapshot envelope exceeded");
        }
        let mut verified = open_object(store, file)?;
        let path = root.join(&file.path);
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut out = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        std::io::copy(
            &mut Read::by_ref(&mut verified.file).take(file.bytes + 1),
            &mut out,
        )?;
        out.flush()?;
        super::media::VerifiedArtifact::open(root, file)?;
    }
    Ok(())
}
fn observations(store: &Store, run: &str) -> Result<Vec<c::Finding>> {
    Ok(store
        .api_snapshot(run)?
        .findings
        .into_iter()
        .filter(|f| f.field != "acceptance.independent")
        .collect())
}
fn ledger(store: &Store, run: &str) -> Result<Vec<c::FileRef>> {
    store
        .db()?
        .prepare("SELECT reference FROM acceptance_observations WHERE run=?1 ORDER BY sha256")?
        .query_map([run], |r| r.get::<_, String>(0))?
        .map(|s| Ok(serde_json::from_str(&s?)?))
        .collect()
}
fn observation_state(store: &Store, run: &str) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&(
        observations(store, run)?,
        ledger(store, run)?,
    ))?))
}
impl Store {
    /// Trusted backend configuration only; generation and HTTP cannot register players.
    pub fn register_acceptance_player(
        &self,
        run: &str,
        epoch: i64,
        root: &std::path::Path,
        file: &c::FileRef,
        version: &str,
    ) -> Result<()> {
        Store::active(&self.db()?, run, epoch)?;
        if file.bytes > super::media::MAX_ARTIFACT_BYTES as u64
            || version.is_empty()
            || version.len() > 128
        {
            return fail(
                "acceptance_registry_invalid",
                "bounded frozen player registration required",
            );
        }
        file.validate()
            .map_err(|e| Error::new("acceptance_registry_invalid", e.to_string()))?;
        let mut private = file.clone();
        private.path = "inputs/player.bin".into();
        let encoded = serde_json::to_string(&private)?;
        let old: Option<(String, String)> = self
            .db()?
            .query_row(
                "SELECT reference,version FROM acceptance_players WHERE run=?1",
                [run],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some(old) = old {
            if old != (encoded.clone(), version.into()) {
                return fail(
                    "acceptance_registry_frozen",
                    "player registration cannot be replaced",
                );
            }
            open_object(self, &private)?;
            return Ok(());
        }
        let verified = super::media::VerifiedArtifact::open(root, file)?;
        let mut bytes = Vec::new();
        verified.file.take(file.bytes + 1).read_to_end(&mut bytes)?;
        file.verify(&bytes)
            .map_err(|e| Error::new("acceptance_registry_invalid", e.to_string()))?;
        self.put(&bytes)?;
        self.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            let existing: Option<(String, String)> = tx
                .query_row(
                    "SELECT reference,version FROM acceptance_players WHERE run=?1",
                    [run],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some(old) = existing {
                if old != (encoded.clone(), version.into()) {
                    return fail(
                        "acceptance_registry_frozen",
                        "player registration cannot be replaced",
                    );
                }
                return Ok(());
            }
            tx.execute(
                "INSERT INTO acceptance_players VALUES(?1,?2,?3)",
                params![run, encoded, version],
            )?;
            Store::emit(
                tx,
                run,
                "acceptance_player_registered",
                &json!({"sha256":private.sha256,"version":version,"production_approved":false}),
            )?;
            Ok(())
        })
    }
    pub(crate) fn prepare_private_acceptance(
        &self,
        run: &str,
        epoch: i64,
        additional: &[c::AcceptanceEvidenceProposal],
    ) -> Result<Prepared> {
        Store::active(&self.db()?, run, epoch)?;
        let player: Option<(String, String)> = self
            .db()?
            .query_row(
                "SELECT reference,version FROM acceptance_players WHERE run=?1",
                [run],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (player, version) = player.ok_or_else(|| {
            Error::new(
                "acceptance_registry_missing",
                "trusted frozen player role is unavailable",
            )
        })?;
        if additional.len() > 256 {
            return fail("acceptance_bounds", "additional proposal envelope exceeded");
        }
        let mut additions = Vec::new();
        for proposal in additional {
            let f = &proposal.raw_evidence;
            f.validate()
                .map_err(|e| Error::new("acceptance_input", e.to_string()))?;
            let redeemed: bool = self.db()?.query_row(
                "SELECT EXISTS(SELECT 1 FROM private_acceptance_reports WHERE report_sha256=?1)",
                [&f.sha256],
                |r| r.get(0),
            )?;
            if redeemed {
                return fail(
                    "acceptance_report_redemption",
                    "stored report JSON is not execution evidence",
                );
            }
            let rows = self
                .db()?
                .prepare("SELECT reference FROM artifacts WHERE run=?1 AND sha256=?2 AND bytes=?3")?
                .query_map(params![run, f.sha256, f.bytes], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            if !rows
                .iter()
                .any(|s| serde_json::from_str::<c::ArtifactRef>(s).is_ok_and(|a| a.file == *f))
            {
                return fail(
                    "acceptance_input",
                    "additional evidence is not an exact runtime-owned reference",
                );
            }
            read_object(self, f, 16 * 1024 * 1024)?;
            let bytes = serde_json::to_vec(proposal)?;
            if bytes.len() > 64 * 1024 {
                return fail("acceptance_bounds", "proposal metadata envelope exceeded");
            }
            additions.push(metadata(
                self,
                &format!("additional/{}.json", digest(&bytes)),
                proposal,
            )?);
        }
        self.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            for f in &additions {
                tx.execute(
                    "INSERT OR IGNORE INTO acceptance_observations VALUES(?1,?2,?3)",
                    params![run, f.sha256, serde_json::to_string(f)?],
                )?;
            }
            let count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM acceptance_observations WHERE run=?1",
                [run],
                |r| r.get(0),
            )?;
            if count > 256 {
                return fail("acceptance_bounds", "retained proposal envelope exceeded");
            }
            Ok(())
        })?;
        let compilation =
            compilation::construct(self, run, epoch, serde_json::from_str(&player)?, &version)?;
        let compilation_ref = metadata(self, "compilation.json", &compilation)?;
        self.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            let old: Option<String> = tx
                .query_row(
                    "SELECT reference FROM acceptance_compilations WHERE run=?1 AND sha256=?2",
                    params![run, compilation_ref.sha256],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(old) = old {
                let old: c::FileRef = serde_json::from_str(&old)?;
                read_object(self, &old, METADATA_LIMIT)?;
                if old != compilation_ref {
                    return fail(
                        "acceptance_compilation_changed",
                        "immutable runtime policy/evidence changed",
                    );
                }
            } else {
                tx.execute(
                    "INSERT INTO acceptance_compilations VALUES(?1,?2,?3)",
                    params![
                        run,
                        compilation_ref.sha256,
                        serde_json::to_string(&compilation_ref)?
                    ],
                )?;
            }
            Ok(())
        })?;
        let private = compilation.private.clone();
        let package_root = tempfile::Builder::new()
            .prefix("acceptance-package-")
            .tempdir_in(self.root.join("scratch"))?;
        let private_root = tempfile::Builder::new()
            .prefix("acceptance-private-")
            .tempdir_in(self.root.join("scratch"))?;
        materialize(self, run, epoch, &compilation.package, package_root.path())?;
        materialize(self, run, epoch, &private, private_root.path())?;
        let document: c::PlaybackDocument = serde_json::from_slice(&read_object(
            self,
            &compilation.document,
            super::media::MAX_ARTIFACT_BYTES,
        )?)?;
        // All additional records are retained in the independently pinned inventory.
        let verification = c::verify_acceptance(
            &document,
            package_root.path(),
            &compilation.policy,
            &[],
            private_root.path(),
        )
        .map_err(|e| Error::new("acceptance_verification", e.to_string()))?;
        verification
            .recheck_inputs()
            .map_err(|e| Error::new("acceptance_changed", e.to_string()))?;
        verification
            .report()
            .validate(&c::acceptance_binding_checker())
            .map_err(|e| Error::new("acceptance_report_invalid", e.to_string()))?;
        let mut compilation = compilation;
        compilation.private = private;
        Ok(Prepared {
            store: self.clone(),
            run: run.into(),
            epoch,
            compilation,
            compilation_ref,
            verification,
            _package_root: package_root,
            _private_root: private_root,
        })
    }
    /// Always executes the independent verifier, including after process recovery.
    pub fn verify_private_acceptance(
        &self,
        run: &str,
        epoch: i64,
        additional: &[c::AcceptanceEvidenceProposal],
    ) -> Result<c::FileRef> {
        self.prepare_private_acceptance(run, epoch, additional)?
            .commit()
    }
    pub(super) fn private_acceptance_unavailable(
        &self,
        run: &str,
        epoch: i64,
        reason: &str,
    ) -> Result<()> {
        self.transaction(|tx| {
            Store::active(tx,run,epoch)?;
            Store::emit(tx,run,"private_acceptance_unavailable",&json!({"reason":reason,"production_approved":false,"status":"needs_repair"}))?;
            for id in ["private-acceptance-inputs","production-verifier-pending"] {
                Store::emit(tx,run,"check_completed",&json!({"id":id,"field":"acceptance.independent","outcome":"missing","message":"Trusted private verification inputs are unavailable; publication remains blocked.","evidence":[]}))?;
            }Ok(())
        })
    }
    /// Test inspection only; production decisions must execute verification again.
    #[cfg(test)]
    pub(crate) fn private_acceptance_report(
        &self,
        run: &str,
    ) -> Result<Option<(c::FileRef, c::AcceptanceVerificationReport)>> {
        let encoded:Option<String>=self.db()?.query_row("SELECT reference FROM private_acceptance_reports WHERE run=?1 ORDER BY last_seq DESC LIMIT 1",[run],|r|r.get(0)).optional()?;
        encoded
            .map(|encoded| {
                let f: c::FileRef = serde_json::from_str(&encoded)?;
                let report: c::AcceptanceVerificationReport =
                    serde_json::from_slice(&read_object(self, &f, REPORT_LIMIT)?)?;
                report
                    .validate(&c::acceptance_binding_checker())
                    .map_err(|e| Error::new("acceptance_report_invalid", e.to_string()))?;
                Ok((f, report))
            })
            .transpose()
    }
}
impl Prepared {
    pub(crate) fn commit(self) -> Result<c::FileRef> {
        let report = self.verification.report();
        if report.production_approved
            || !report
                .blockers
                .iter()
                .any(|b| b == "production-verifier-pending")
        {
            return fail(
                "acceptance_report_invalid",
                "this verifier cannot grant production authority",
            );
        }
        let bytes = serde_json::to_vec(report)?;
        if bytes.len() > REPORT_LIMIT {
            return fail("acceptance_bounds", "private report envelope exceeded");
        }
        let file = reference(
            "private/acceptance-verification.json",
            &bytes,
            "application/json",
        );
        self.store.put(&bytes)?;
        self.store.transaction(|tx| {
            Store::active(tx,&self.run,self.epoch)?;
            let context:Option<(String,String)>=tx.query_row("SELECT r.request,p.manifest_hash FROM runs r JOIN diagnostic_packages p ON p.run=r.id WHERE r.id=?1",[&self.run],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            let (request,manifest)=context.ok_or_else(||Error::new("acceptance_context_changed","intended package is unavailable"))?;
            let request:serde_json::Value=serde_json::from_str(&request)?;
            if digest(&serde_json::to_vec(&request)?)!=self.compilation.request_sha256 || report.manifest_sha256.as_deref()!=Some(manifest.as_str()) {
                return fail("acceptance_context_changed","intended source or package changed after verification");
            }
            self.verification.recheck_inputs().map_err(|e|Error::new("acceptance_changed",e.to_string()))?;
            report.validate(&c::acceptance_binding_checker()).map_err(|e|Error::new("acceptance_report_invalid",e.to_string()))?;
            for f in self.compilation.package.iter().chain(&self.compilation.private).chain([&self.compilation.document,&self.compilation_ref]) {open_object(&self.store,f)?;}
            if observation_state(&self.store,&self.run)?!=self.compilation.observations_sha256 {return fail("acceptance_evidence_changed","new adverse evidence requires a new trusted compilation");}
            let seq=Store::emit(tx,&self.run,"private_acceptance_verified",&json!({"report_sha256":file.sha256,"manifest_sha256":report.manifest_sha256,"checker_sha256":report.checker.sha256,"status":"needs_repair","production_approved":false}))?;
            tx.execute("INSERT INTO private_acceptance_reports VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'needs_repair',?10,1) ON CONFLICT(run,report_sha256) DO UPDATE SET epoch=excluded.epoch,last_seq=excluded.last_seq,executions=executions+1",params![self.run,file.sha256,serde_json::to_string(&file)?,self.epoch,report.manifest_sha256,report.acceptance_sha256,report.checker.sha256,self.compilation_ref.sha256,digest(&serde_json::to_vec(&report.consumed_hashes)?),seq])?;
            for blocker in report.blockers.iter().filter(|id|!report.checks.iter().any(|c|c.check_id==**id)) {Store::emit(tx,&self.run,"check_completed",&json!({"id":blocker,"field":"acceptance.independent","outcome":"missing","message":"Independent verification retains this publication blocker.","evidence":[]}))?;}
            for check in report.checks.iter().filter(|c|c.outcome!=c::ReviewOutcome::Pass) {
                let summary=format!("Independent result for {:?}: {:?}",check.target,check.reasons).chars().take(1000).collect::<String>();
                Store::emit(tx,&self.run,"check_completed",&json!({"id":check.check_id,"field":"acceptance.independent","outcome":check.outcome,"message":summary,"evidence":[]}))?;
            }
            tx.execute("UPDATE stages SET state='repairing' WHERE run=?1 AND name='package'",[&self.run])?;Ok(())
        })?;
        Ok(file)
    }
    #[cfg(test)]
    pub(super) fn private_root(&self) -> &std::path::Path {
        self._private_root.path()
    }
}

/// Migration interface: decide an accepted diagnostic publication from live
/// executed verification. Rechecks inputs through the coverage ledger, binds
/// the exact report bytes, and returns the persisted decision record.
/// Callers persist the returned record (replacing `unresolved`); serving
/// paths adopt it separately. Grants no production approval.
pub fn decide_accepted_publication(
    verification: &c::PrivateAcceptanceVerification,
    allowlist: &c::PublicExportAllowlistV1,
    catalog_sha256: &c::Digest,
    files: &[(&str, &[u8])],
    known_private_digests: &[c::Digest],
) -> Result<c::AcceptedPublicationV1> {
    let ledger = verification
        .coverage_ledger()
        .map_err(|e| Error::new("publication_inputs_changed", e.to_string()))?;
    let report_bytes = serde_json::to_vec(verification.report())?;
    c::verify_accepted_publication(
        &ledger,
        &report_bytes,
        &c::acceptance_binding_checker(),
        allowlist,
        catalog_sha256,
        files,
        known_private_digests,
    )
    .map_err(|e| Error::new("publication_decision", e.to_string()))
}
