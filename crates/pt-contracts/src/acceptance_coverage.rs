//! Selected-scope coverage metadata. A ledger is never publication authority.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceEvidenceOwnerV1 {
    Integration,
    Generation,
    Runtime,
    Player,
    Unassigned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCoverageRowV1 {
    pub evidence_owners: Vec<AcceptanceEvidenceOwnerV1>,
    pub result: AcceptanceCheckResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCoverageLedgerV1 {
    pub schema_version: u32,
    pub scope: AcceptanceScope,
    pub matrix_version: String,
    /// Exact compact report bytes emitted by the execution, not a submitted Pass.
    pub report_sha256: Digest,
    pub checker: AcceptanceCheckerIdentity,
    pub binding: AcceptanceBinding,
    pub rows: Vec<AcceptanceCoverageRowV1>,
    pub production_approved: bool,
    pub blockers: Vec<Id>,
}

fn owners(check: &str) -> Vec<AcceptanceEvidenceOwnerV1> {
    use AcceptanceEvidenceOwnerV1::*;
    match check {
        "package-byte-integrity" | "evidence-binding-integrity" | "required-check-coverage" => {
            vec![Integration]
        }
        "autonomous-provenance" => vec![Generation, Runtime],
        "publication-recovery" | "product-spend-authorization" | "public-export-boundary" => {
            vec![Runtime]
        }
        "font-bundle"
        | "profile-r186-browser"
        | "target-device-budget"
        | "graphics-recovery-review"
        | "control-lifecycle-review"
        | "title-visibility-review" => vec![Player],
        "rendered-creative-review"
        | "opening-action-capture-coverage"
        | "incoming-picture-readiness"
        | "required-action-accounting" => vec![Generation, Player],
        "complete-chapter"
        | "complete-excerpt"
        | "complete-source-coverage"
        | "initial-reference-research"
        | "image-scope-review"
        | "image-style-review"
        | "image-reference-review"
        | "image-extraction-review"
        | "image-stage-readability"
        | "independent-acoustic-evidence"
        | "narrator-continuity"
        | "speech-joins"
        | "music-bible"
        | "mix-intelligibility"
        | "story-coverage"
        | "full-audio-review-coverage"
        | "speech-fidelity"
        | "pilot-recording-review"
        | "primary-acoustic-evidence"
        | "delivery-fact-mapping"
        | "audio-format-review" => vec![Generation],
        _ => vec![Unassigned],
    }
}

fn from_report(
    report: &AcceptanceVerificationReport,
    report_sha256: Digest,
) -> AcceptanceCoverageLedgerV1 {
    AcceptanceCoverageLedgerV1 {
        schema_version: 1,
        scope: report.scope,
        matrix_version: report.matrix_version.clone(),
        report_sha256,
        checker: report.checker.clone(),
        binding: report.binding.clone(),
        rows: report
            .checks
            .iter()
            .map(|result| AcceptanceCoverageRowV1 {
                evidence_owners: owners(&result.check_id),
                result: result.clone(),
            })
            .collect(),
        production_approved: false,
        blockers: report.blockers.clone(),
    }
}

impl AcceptanceCoverageLedgerV1 {
    /// Wire consistency only. A caller must independently select the expected
    /// checker and report bytes; this never reconstructs an execution capability.
    pub fn validate_for_report_bytes(
        &self,
        bytes: &[u8],
        expected_checker: &AcceptanceCheckerIdentity,
    ) -> ContractResult<()> {
        let report: AcceptanceVerificationReport =
            serde_json::from_slice(bytes).map_err(|e| ContractError {
                code: "acceptance_coverage".into(),
                field: "report".into(),
                message: e.to_string(),
            })?;
        report.validate(expected_checker)?;
        let expected = from_report(&report, digest(bytes));
        require(
            serde_json::to_value(self).expect("ledger")
                == serde_json::to_value(expected).expect("ledger"),
            "acceptance_coverage",
            "ledger",
            "coverage, ownership, outcome or exact report bytes differ",
        )
    }
}

impl PrivateAcceptanceVerification {
    /// Rechecks every consumed file before describing this execution's coverage.
    pub fn coverage_ledger(&self) -> ContractResult<AcceptanceCoverageLedgerV1> {
        self.recheck_inputs()?;
        self.report().validate(&acceptance_binding_checker())?;
        Ok(from_report(
            self.report(),
            digest(&serde_json::to_vec(self.report()).expect("report")),
        ))
    }
}
