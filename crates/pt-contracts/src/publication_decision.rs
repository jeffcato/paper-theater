//! Accepted-publication decision. Runtime persistence grants authority.
//!
//! Executing verifier connecting executed verification to publication
//! decisions. A diagnostic package is accepted for publication only when:
//! - the coverage ledger re-validates against the exact report bytes and the
//!   pinned checker, with every required check covered (forged, changed or
//!   incomplete coverage fails),
//! - the published payload carries the exact report bytes it claims, so a
//!   ledger joined to another execution's payload fails as stale context,
//! - every published file is exactly allowlisted, safe, public and
//!   byte-identical, with no private bytes under any name,
//! - the published catalog travels inside the payload.
//! Adverse or missing evidence stays honestly recorded in the ledger; this
//! decision never approves production and grants no release, dispatch or
//! media authority. The runtime persists the returned record.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedPublicationV1 {
    pub schema_version: u32,
    pub report_sha256: Digest,
    /// Exact executed checker that produced the report.
    pub checker: AcceptanceCheckerIdentity,
    pub scope: AcceptanceScope,
    pub matrix_version: String,
    /// Digest of the canonical allowlist JSON this decision bound.
    pub allowlist_sha256: Digest,
    pub catalog_sha256: Digest,
    pub file_count: u64,
}

/// Independent shared-verifier entry point. `files` carries the exact bytes
/// offered for each requested path. Verifies, never publishes.
pub fn verify_accepted_publication(
    ledger: &AcceptanceCoverageLedgerV1,
    report_bytes: &[u8],
    expected_checker: &AcceptanceCheckerIdentity,
    allowlist: &PublicExportAllowlistV1,
    catalog_sha256: &Digest,
    files: &[(&str, &[u8])],
    known_private_digests: &[Digest],
) -> ContractResult<AcceptedPublicationV1> {
    ledger.validate_for_report_bytes(report_bytes, expected_checker)?;
    allowlist.verify_public_export(
        &files.iter().copied().collect::<Vec<_>>(),
        known_private_digests,
    )?;
    for entry in &allowlist.entries {
        require(
            files.iter().any(|(p, _)| *p == entry.path),
            "publication_decision",
            &entry.path,
            "allowlisted file missing from the published payload",
        )?;
    }
    require(
        files.iter().any(|(_, b)| *b == report_bytes),
        "publication_decision",
        "report",
        "published payload must carry the exact executed report bytes",
    )?;
    hash(catalog_sha256, "publication_decision.catalog")?;
    require(
        files.iter().any(|(_, b)| digest(*b) == *catalog_sha256),
        "publication_decision",
        "catalog",
        "published catalog must travel inside the payload",
    )?;
    Ok(AcceptedPublicationV1 {
        schema_version: 1,
        report_sha256: ledger.report_sha256.clone(),
        checker: ledger.checker.clone(),
        scope: ledger.scope,
        matrix_version: ledger.matrix_version.clone(),
        allowlist_sha256: digest(&serde_json::to_vec(allowlist).expect("allowlist")),
        catalog_sha256: catalog_sha256.clone(),
        file_count: files.len() as u64,
    })
}
