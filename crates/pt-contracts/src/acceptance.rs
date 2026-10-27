//! Private, independent acceptance plumbing. No creative verifier or publication authority.
use crate::*;
use serde::{Deserialize, Serialize};

pub const ACCEPTANCE_MATRIX_VERSION: &str = "pt-acceptance-required-v1";
pub const ACCEPTANCE_REPORT_VERSION: u32 = 1;
const MATRIX_BYTES: &[u8] =
    include_bytes!("../../../packages/contracts/src/acceptance-required-v1.json");
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceScope {
    Diagnostic,
    AutonomousExcerpt,
    ProductionChapter,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredAcceptanceCheck {
    pub id: Id,
    pub scopes: Vec<AcceptanceScope>,
    pub target: String,
    pub implemented: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCheckMatrix {
    pub version: String,
    pub checks: Vec<RequiredAcceptanceCheck>,
}
pub fn acceptance_check_matrix() -> AcceptanceCheckMatrix {
    serde_json::from_slice(MATRIX_BYTES).expect("compiled acceptance matrix")
}
pub fn required_acceptance_checks(scope: AcceptanceScope) -> Vec<RequiredAcceptanceCheck> {
    acceptance_check_matrix()
        .checks
        .into_iter()
        .filter(|c| c.scopes.contains(&scope))
        .collect()
}

pub fn declared_production_coverage_complete(d: &PlaybackDocument) -> bool {
    let ids = &d.acceptance.required_check_ids;
    let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
    ids.len() == unique.len()
        && required_acceptance_checks(AcceptanceScope::ProductionChapter)
            .iter()
            .all(|c| ids.contains(&c.id) && d.acceptance.findings.iter().any(|f| f.id == c.id))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceInputRole {
    Source,
    Narration,
    Alignment,
    PresentationMap,
    Recipe,
    Profile,
    Player,
    FactInventory,
    EvidenceInventory,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceInput {
    pub role: AcceptanceInputRole,
    pub file: FileRef,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceFact {
    pub id: Id,
    pub reveal_id: Id,
    pub source_span: SourceSpan,
    pub asset_sha256: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceFactInventory {
    pub schema_version: u32,
    pub source_scope_id: Id,
    pub source_sha256: Digest,
    pub source_span: SourceSpan,
    pub facts: Vec<AcceptanceFact>,
    pub required_cue_ids: Vec<Id>,
}
/// Runtime-owned policy. Never derive its expected identities from a candidate's claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceVerificationPolicy {
    pub schema_version: u32,
    pub scope: AcceptanceScope,
    pub inputs: Vec<AcceptanceInput>,
    pub source_span: SourceSpan,
    pub narration_sample_rate: u32,
    pub narration_sample_count: u64,
    pub source_scope_id: Id,
    pub payload_inventory_sha256: Digest,
    pub asset_sha256s: Vec<Digest>,
    pub device_id: Option<Id>,
    pub player_version: String,
    pub candidate_checker_version: String,
    pub checker_sha256: Digest,
    pub additional_required_check_ids: Vec<Id>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceSampleDomain {
    Source,
    Narration,
    Delivery,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcceptanceTarget {
    Candidate,
    Source {
        span: SourceSpan,
    },
    Asset {
        sha256: Digest,
    },
    Audio {
        domain: AcceptanceSampleDomain,
        span: Interval,
    },
    Fact {
        fact: AcceptanceFact,
        domain: AcceptanceSampleDomain,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceBinding {
    pub inventory_sha256: Digest,
    pub source_sha256: Digest,
    pub narration_sha256: Digest,
    pub alignment_sha256: Digest,
    pub presentation_map_sha256: Digest,
    pub master_sha256: Digest,
    pub recipe_sha256: Digest,
    pub profile_sha256: Digest,
    pub player_sha256: Digest,
    pub checker_sha256: Digest,
    pub fact_inventory_sha256: Digest,
    pub asset_sha256s: Vec<Digest>,
    pub source_span: SourceSpan,
    pub source_scope_id: Id,
    pub player_version: String,
    pub candidate_checker_version: String,
    pub device_id: Option<Id>,
    pub narration_sample_rate: u32,
    pub narration_sample_count: u64,
    pub delivery_sample_rate: u32,
    pub delivery_sample_count: u64,
}
/// Untrusted private proposal. A pass is never an execution receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceEvidenceProposal {
    pub schema_version: u32,
    pub id: Id,
    pub check_id: Id,
    pub target: AcceptanceTarget,
    pub binding: AcceptanceBinding,
    pub outcome: ReviewOutcome,
    pub provenance: Provenance,
    pub raw_evidence: FileRef,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceEvidenceInventory {
    pub schema_version: u32,
    pub proposals: Vec<AcceptanceEvidenceProposal>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCheckerIdentity {
    pub version: String,
    pub sha256: Digest,
}
pub fn acceptance_binding_checker() -> AcceptanceCheckerIdentity {
    AcceptanceCheckerIdentity {
        version: "pt-acceptance-bindings-v1".into(),
        sha256: digest(
            &[
                include_bytes!("acceptance.rs").as_slice(),
                include_bytes!("validate.rs").as_slice(),
                include_bytes!("common.rs").as_slice(),
                include_bytes!("playback.rs").as_slice(),
                include_bytes!("services.rs").as_slice(),
                include_bytes!("lib.rs").as_slice(),
                include_bytes!("../../../Cargo.toml").as_slice(),
                include_bytes!("../../../Cargo.lock").as_slice(),
                MATRIX_BYTES,
            ]
            .concat(),
        ),
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceReason {
    Verified,
    UnimplementedVerifier,
    MissingDeclaration,
    DuplicateCoverage,
    InvalidBinding,
    MissingBytes,
    ChangedBytes,
    StructuralFailure,
    AdverseEvidence,
    UntrustedPass,
    IneligibleProvenance,
    UnsupportedRequirement,
    PrivateEvidenceExported,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceRequirement {
    pub check_id: Id,
    pub target: AcceptanceTarget,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCheckResult {
    pub check_id: Id,
    pub target: AcceptanceTarget,
    pub outcome: ReviewOutcome,
    pub executed: bool,
    pub executed_checker: Option<AcceptanceCheckerIdentity>,
    pub reasons: Vec<AcceptanceReason>,
    pub consumed_sha256s: Vec<Digest>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceObservation {
    pub origin: String,
    pub id: Id,
    pub check_id: Id,
    pub target: AcceptanceTarget,
    pub claimed_outcome: ReviewOutcome,
    pub binding_outcome: ReviewOutcome,
    pub provenance: Provenance,
    pub raw_sha256: Option<Digest>,
    pub reasons: Vec<AcceptanceReason>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceConsumedHash {
    pub role: String,
    pub sha256: Digest,
    pub bytes: u64,
}
/// Private metadata only. Its JSON form is not an authenticated verifier capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceVerificationReport {
    pub schema_version: u32,
    pub matrix_version: String,
    pub private: bool,
    pub scope: AcceptanceScope,
    pub manifest_sha256: Option<Digest>,
    pub acceptance_sha256: Option<Digest>,
    pub binding: AcceptanceBinding,
    pub checker: AcceptanceCheckerIdentity,
    pub declared_check_ids: Vec<Id>,
    pub requirements: Vec<AcceptanceRequirement>,
    pub additional_check_ids: Vec<Id>,
    pub fact_targets: Vec<AcceptanceFact>,
    pub render_asset_sha256s: Vec<Digest>,
    pub checks: Vec<AcceptanceCheckResult>,
    pub observations: Vec<AcceptanceObservation>,
    pub consumed_hashes: Vec<AcceptanceConsumedHash>,
    pub production_approved: bool,
    pub blockers: Vec<Id>,
}

use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};
const MAX_TOTAL: u64 = 512 * 1024 * 1024;
const ROLES: [AcceptanceInputRole; 9] = [
    AcceptanceInputRole::Source,
    AcceptanceInputRole::Narration,
    AcceptanceInputRole::Alignment,
    AcceptanceInputRole::PresentationMap,
    AcceptanceInputRole::Recipe,
    AcceptanceInputRole::Profile,
    AcceptanceInputRole::Player,
    AcceptanceInputRole::FactInventory,
    AcceptanceInputRole::EvidenceInventory,
];
fn key(target: &AcceptanceTarget) -> String {
    serde_json::to_string(target).expect("target")
}
fn implemented(id: &str) -> bool {
    matches!(
        id,
        "package-byte-integrity" | "evidence-binding-integrity" | "required-check-coverage"
    )
}
fn worse(a: ReviewOutcome, b: ReviewOutcome) -> ReviewOutcome {
    let rank = |x| match x {
        ReviewOutcome::Pass => 0,
        ReviewOutcome::Missing => 1,
        ReviewOutcome::Uncertain => 2,
        ReviewOutcome::Fail => 3,
    };
    if rank(a) >= rank(b) { a } else { b }
}
fn reason_outcome(reason: AcceptanceReason) -> ReviewOutcome {
    if reason == AcceptanceReason::MissingBytes {
        ReviewOutcome::Missing
    } else {
        ReviewOutcome::Fail
    }
}
fn read_exact(root: &Path, file: &FileRef, max: u64) -> Result<Vec<u8>, AcceptanceReason> {
    if file.validate().is_err() || file.bytes > max {
        return Err(AcceptanceReason::InvalidBinding);
    }
    let root = root
        .canonicalize()
        .map_err(|_| AcceptanceReason::MissingBytes)?;
    let mut path = root.clone();
    for part in file.path.split('/') {
        path.push(part);
        if path.is_symlink() {
            return Err(AcceptanceReason::InvalidBinding);
        }
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| AcceptanceReason::MissingBytes)?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(AcceptanceReason::InvalidBinding);
    }
    let mut f = std::fs::File::open(canonical).map_err(|_| AcceptanceReason::MissingBytes)?;
    if f.metadata()
        .map_err(|_| AcceptanceReason::MissingBytes)?
        .len()
        != file.bytes
    {
        return Err(AcceptanceReason::ChangedBytes);
    }
    let mut bytes = Vec::new();
    f.by_ref()
        .take(file.bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AcceptanceReason::MissingBytes)?;
    file.verify(&bytes)
        .map_err(|_| AcceptanceReason::ChangedBytes)?;
    Ok(bytes)
}
struct ReadPin {
    root: PathBuf,
    file: FileRef,
    max: u64,
}
#[derive(Default)]
struct Reads {
    pins: Vec<ReadPin>,
    consumed: Vec<AcceptanceConsumedHash>,
    total: u64,
}
impl Reads {
    fn read(
        &mut self,
        root: &Path,
        file: &FileRef,
        role: &str,
        max: u64,
    ) -> Result<Vec<u8>, AcceptanceReason> {
        self.total = self
            .total
            .checked_add(file.bytes)
            .ok_or(AcceptanceReason::InvalidBinding)?;
        if self.total > MAX_TOTAL {
            return Err(AcceptanceReason::InvalidBinding);
        }
        let bytes = read_exact(root, file, max)?;
        self.pins.push(ReadPin {
            root: root.into(),
            file: file.clone(),
            max,
        });
        self.consumed.push(AcceptanceConsumedHash {
            role: role.into(),
            sha256: file.sha256.clone(),
            bytes: file.bytes,
        });
        Ok(bytes)
    }
}
/// Only this function can construct the capability. Deserializing report JSON cannot.
pub struct PrivateAcceptanceVerification {
    report: AcceptanceVerificationReport,
    pins: Vec<ReadPin>,
}
impl PrivateAcceptanceVerification {
    pub fn report(&self) -> &AcceptanceVerificationReport {
        &self.report
    }
    /// Recheck immediately before any later consumer uses the immutable snapshot.
    /// This still grants no production approval or publication authority.
    pub fn recheck_inputs(&self) -> ContractResult<()> {
        for p in &self.pins {
            read_exact(&p.root, &p.file, p.max).map_err(|r| ContractError {
                code: "acceptance_bytes".into(),
                field: "consumed_hashes".into(),
                message: format!("snapshot changed/unavailable: {r:?}"),
            })?;
        }
        Ok(())
    }
}
impl AcceptanceVerificationPolicy {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 1,
            "version",
            "policy.schema_version",
            "expected 1",
        )?;
        require(
            self.inputs.len() == ROLES.len()
                && ROLES
                    .iter()
                    .all(|r| self.inputs.iter().filter(|i| i.role == *r).count() == 1),
            "acceptance_policy",
            "inputs",
            "one pinned input per role is required",
        )?;
        for input in &self.inputs {
            input.file.validate()?;
        }
        require(
            valid_id(&self.source_scope_id)
                && valid_id(&self.player_version)
                && valid_id(&self.candidate_checker_version)
                && self.device_id.as_ref().is_none_or(|s| valid_id(s)),
            "acceptance_policy",
            "identities",
            "invalid scope/version/device ID",
        )?;
        require(
            valid_digest(&self.checker_sha256)
                && valid_digest(&self.payload_inventory_sha256)
                && self.asset_sha256s.len() <= 2048
                && self.asset_sha256s.iter().all(|s| valid_digest(s)),
            "acceptance_policy",
            "hashes",
            "invalid expected identity",
        )?;
        interval(
            self.source_span,
            MAX_SAFE_INTEGER,
            "policy.source_span",
            true,
        )?;
        require(
            self.narration_sample_rate > 0
                && self.narration_sample_rate <= 192000
                && self.narration_sample_count > 0
                && self.narration_sample_count <= MAX_SAFE_INTEGER
                && self.additional_required_check_ids.len() <= 128
                && self
                    .additional_required_check_ids
                    .iter()
                    .all(|s| valid_id(s)),
            "acceptance_policy",
            "limits",
            "invalid policy bounds",
        )
    }
}
fn requirements(
    scope: AcceptanceScope,
    declared: &[Id],
    extra: &[Id],
    facts: &[AcceptanceFact],
    assets: &[Digest],
    binding: &AcceptanceBinding,
    observations: &[AcceptanceObservation],
) -> Vec<AcceptanceRequirement> {
    let matrix = acceptance_check_matrix();
    let mut ids = required_acceptance_checks(scope)
        .into_iter()
        .map(|c| c.id)
        .chain(declared.iter().cloned())
        .chain(extra.iter().cloned())
        .collect::<BTreeSet<_>>();
    ids.extend(
        observations
            .iter()
            .filter(|o| o.claimed_outcome != ReviewOutcome::Pass)
            .map(|o| o.check_id.clone()),
    );
    let mut rows = Vec::new();
    for id in ids {
        let class = matrix
            .checks
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.target.as_str())
            .unwrap_or("candidate");
        let targets = match class {
            "source" => vec![AcceptanceTarget::Source {
                span: binding.source_span,
            }],
            "asset" => assets
                .iter()
                .map(|sha256| AcceptanceTarget::Asset {
                    sha256: sha256.clone(),
                })
                .collect(),
            "narration" => vec![AcceptanceTarget::Audio {
                domain: AcceptanceSampleDomain::Narration,
                span: [0, binding.narration_sample_count],
            }],
            "delivery" => vec![AcceptanceTarget::Audio {
                domain: AcceptanceSampleDomain::Delivery,
                span: [0, binding.delivery_sample_count],
            }],
            "fact_source" | "fact_narration" | "fact_delivery" => facts
                .iter()
                .map(|fact| AcceptanceTarget::Fact {
                    fact: fact.clone(),
                    domain: match class {
                        "fact_source" => AcceptanceSampleDomain::Source,
                        "fact_narration" => AcceptanceSampleDomain::Narration,
                        _ => AcceptanceSampleDomain::Delivery,
                    },
                })
                .collect(),
            _ => vec![AcceptanceTarget::Candidate],
        };
        let targets = if targets.is_empty() {
            vec![AcceptanceTarget::Candidate]
        } else {
            targets
        };
        for target in targets {
            rows.push(AcceptanceRequirement {
                check_id: id.clone(),
                target,
            });
        }
    }
    rows.sort_by_key(|r| (r.check_id.clone(), key(&r.target)));
    rows
}

pub fn verify_acceptance(
    doc: &PlaybackDocument,
    package_root: &Path,
    policy: &AcceptanceVerificationPolicy,
    evidence: &[AcceptanceEvidenceProposal],
    private_root: &Path,
) -> ContractResult<PrivateAcceptanceVerification> {
    policy.validate()?;
    require(
        evidence.len() <= 1024
            && doc.acceptance.attestations.len() <= 4096
            && doc.acceptance.findings.len() <= 4096
            && doc.acceptance.required_check_ids.len() <= 256,
        "acceptance_bounds",
        "records",
        "acceptance record bounds exceeded",
    )?;
    require(
        doc.acceptance
            .required_check_ids
            .iter()
            .all(|s| valid_id(s))
            && doc.acceptance.findings.iter().all(|f| valid_id(&f.id)),
        "id",
        "acceptance",
        "invalid check/finding ID",
    )?;
    let checker = acceptance_binding_checker();
    let mut reads = Reads::default();
    let mut package_reasons = Vec::new();
    let mut binding_reasons = Vec::new();
    let payload = doc
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(doc.manifest.assets.iter())
        .collect::<Vec<_>>();
    for f in payload.iter().copied() {
        let limit = if f.media_type == "audio/wav" {
            256 * 1024 * 1024
        } else {
            32 * 1024 * 1024
        };
        if let Err(r) = reads.read(package_root, f, "payload", limit) {
            package_reasons.push(r);
        }
    }
    if let Err(r) = reads.read(
        package_root,
        &doc.manifest.acceptance,
        "acceptance",
        32 * 1024 * 1024,
    ) {
        package_reasons.push(r);
    }
    let inventory_path = package_root.join("inventory.json");
    let inventory_ref = FileRef {
        path: "inventory.json".into(),
        sha256: doc.acceptance.inventory_sha256.clone(),
        bytes: std::fs::metadata(inventory_path).map_or(0, |m| m.len()),
        media_type: "application/json".into(),
        dimensions: None,
    };
    if let Err(r) = reads.read(package_root, &inventory_ref, "inventory", 1024 * 1024) {
        package_reasons.push(r);
    }
    let manifest_path = package_root.join("manifest.json");
    let manifest_bytes = (|| {
        if manifest_path.is_symlink()
            || !manifest_path
                .canonicalize()
                .ok()?
                .starts_with(package_root.canonicalize().ok()?)
        {
            return None;
        }
        let mut f = std::fs::File::open(&manifest_path).ok()?;
        if f.metadata().ok()?.len() > 1024 * 1024 {
            return None;
        }
        let mut b = Vec::new();
        f.by_ref().take(1024 * 1024 + 1).read_to_end(&mut b).ok()?;
        if b.len() > 1024 * 1024 {
            return None;
        }
        Some(b)
    })();
    let manifest_sha256 = manifest_bytes.as_ref().map(|b| digest(b));
    if let Some(bytes) = &manifest_bytes {
        let f = FileRef {
            path: "manifest.json".into(),
            sha256: digest(bytes),
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        };
        if reads
            .read(package_root, &f, "manifest", 1024 * 1024)
            .is_err()
            || serde_json::from_slice::<Manifest>(bytes)
                .ok()
                .and_then(|m| serde_json::to_value(m).ok())
                != serde_json::to_value(&doc.manifest).ok()
        {
            package_reasons.push(AcceptanceReason::InvalidBinding);
        }
    } else {
        package_reasons.push(AcceptanceReason::MissingBytes);
    }
    if validate_playback(doc).is_err() {
        package_reasons.push(AcceptanceReason::StructuralFailure);
    }
    if package_reasons.is_empty() && verify_package_files(doc, package_root).is_err() {
        package_reasons.push(AcceptanceReason::ChangedBytes);
    }
    let mut private_bytes = BTreeMap::new();
    for input in &policy.inputs {
        let role = serde_json::to_value(input.role)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let max = match input.role {
            AcceptanceInputRole::Narration => 256 * 1024 * 1024,
            AcceptanceInputRole::Player => 64 * 1024 * 1024,
            _ => 32 * 1024 * 1024,
        };
        match reads.read(private_root, &input.file, &role, max) {
            Ok(b) => {
                private_bytes.insert(input.role, b);
            }
            Err(r) => binding_reasons.push(r),
        };
    }
    let file = |role| &policy.inputs.iter().find(|i| i.role == role).unwrap().file;
    let mut expected_assets = policy.asset_sha256s.clone();
    expected_assets.sort();
    expected_assets.dedup();
    let binding = AcceptanceBinding {
        inventory_sha256: policy.payload_inventory_sha256.clone(),
        source_sha256: file(AcceptanceInputRole::Source).sha256.clone(),
        narration_sha256: file(AcceptanceInputRole::Narration).sha256.clone(),
        alignment_sha256: file(AcceptanceInputRole::Alignment).sha256.clone(),
        presentation_map_sha256: file(AcceptanceInputRole::PresentationMap).sha256.clone(),
        master_sha256: doc.manifest.entries.audio.sha256.clone(),
        recipe_sha256: file(AcceptanceInputRole::Recipe).sha256.clone(),
        profile_sha256: file(AcceptanceInputRole::Profile).sha256.clone(),
        player_sha256: file(AcceptanceInputRole::Player).sha256.clone(),
        checker_sha256: policy.checker_sha256.clone(),
        fact_inventory_sha256: file(AcceptanceInputRole::FactInventory).sha256.clone(),
        asset_sha256s: expected_assets,
        source_span: policy.source_span,
        source_scope_id: policy.source_scope_id.clone(),
        player_version: policy.player_version.clone(),
        candidate_checker_version: policy.candidate_checker_version.clone(),
        device_id: policy.device_id.clone(),
        narration_sample_rate: policy.narration_sample_rate,
        narration_sample_count: policy.narration_sample_count,
        delivery_sample_rate: doc.manifest.sample_rate,
        delivery_sample_count: doc.manifest.sample_count,
    };
    let mut assets = doc
        .manifest
        .assets
        .iter()
        .map(|f| f.sha256.clone())
        .collect::<Vec<_>>();
    assets.sort();
    assets.dedup();
    let mut valid = policy.checker_sha256 == checker.sha256
        && binding.inventory_sha256 == doc.acceptance.inventory_sha256
        && binding.source_sha256 == doc.manifest.source_sha256
        && binding.narration_sha256 == doc.read_along.narration_sha256
        && binding.alignment_sha256 == doc.read_along.alignment_sha256
        && binding.presentation_map_sha256 == doc.timeline.presentation_map_sha256
        && binding.presentation_map_sha256 == doc.read_along.presentation_map_sha256
        && binding.recipe_sha256 == doc.acceptance.recipe_sha256
        && binding.profile_sha256 == doc.acceptance.profile_sha256
        && policy.player_version == doc.acceptance.player_version
        && policy.candidate_checker_version == doc.acceptance.checker_version
        && assets == binding.asset_sha256s;
    valid &= private_bytes
        .get(&AcceptanceInputRole::Source)
        .is_some_and(|b| b == doc.source_text.as_bytes())
        && policy.source_span == [0, doc.source_text.chars().count() as u64];
    valid &= private_bytes
        .get(&AcceptanceInputRole::PresentationMap)
        .and_then(|b| serde_json::from_slice::<PresentationMap>(b).ok())
        .is_some_and(|m| {
            serde_json::to_value(&m).ok() == serde_json::to_value(&doc.presentation_map).ok()
                && m.source_audio_sha256 == binding.narration_sha256
                && m.source_sample_rate == policy.narration_sample_rate
                && m.delivery_sample_rate == 48000
                && m.segments
                    .iter()
                    .all(|s| s.source[1] <= policy.narration_sample_count)
        });
    valid &= private_bytes
        .get(&AcceptanceInputRole::Profile)
        .and_then(|b| serde_json::from_slice::<RenderingProfile>(b).ok())
        .is_some_and(|p| {
            serde_json::to_value(p).ok() == serde_json::to_value(&doc.shell.profile).ok()
        });
    let fact_inventory = private_bytes
        .get(&AcceptanceInputRole::FactInventory)
        .and_then(|b| serde_json::from_slice::<AcceptanceFactInventory>(b).ok());
    let facts = fact_inventory
        .as_ref()
        .map(|f| f.facts.clone())
        .unwrap_or_default();
    valid &= fact_inventory.as_ref().is_some_and(|f| {
        f.schema_version == 1
            && f.source_scope_id == policy.source_scope_id
            && f.source_sha256 == binding.source_sha256
            && f.source_span == policy.source_span
            && f.facts.len() <= 4096
            && f.facts.iter().map(|f| &f.id).collect::<BTreeSet<_>>().len() == f.facts.len()
            && {
                let mut a = f.required_cue_ids.clone();
                let mut b = doc
                    .timeline
                    .cues
                    .iter()
                    .map(|c| c.id.clone())
                    .collect::<Vec<_>>();
                a.sort();
                b.sort();
                a == b
            }
    });
    for fact in &facts {
        valid &= valid_id(&fact.id)
            && valid_digest(&fact.asset_sha256)
            && interval(
                fact.source_span,
                policy.source_span[1],
                "fact.source_span",
                true,
            )
            .is_ok()
            && doc.manifest.reveals.iter().any(|g| {
                g.id == fact.reveal_id
                    && g.facts.iter().any(|f| f.source_span == fact.source_span)
                    && doc
                        .timeline
                        .scenes
                        .iter()
                        .flat_map(|s| &s.entry_snapshot.instances)
                        .any(|i| {
                            g.contributor_ids.contains(&i.id)
                                && doc.actors.iter().any(|a| {
                                    a.id == i.asset_id && a.image.sha256 == fact.asset_sha256
                                })
                        })
            });
    }
    for g in &doc.manifest.reveals {
        for f in &g.facts {
            valid &= facts
                .iter()
                .any(|x| x.reveal_id == g.id && x.source_span == f.source_span);
        }
    }
    for a in &doc.actors {
        valid &= payload.iter().any(|f| **f == a.image);
    }
    if !valid {
        binding_reasons.push(AcceptanceReason::InvalidBinding);
    }
    if policy.scope != AcceptanceScope::Diagnostic {
        for input in &policy.inputs {
            if matches!(
                input.role,
                AcceptanceInputRole::Source
                    | AcceptanceInputRole::Narration
                    | AcceptanceInputRole::Alignment
                    | AcceptanceInputRole::Recipe
                    | AcceptanceInputRole::FactInventory
                    | AcceptanceInputRole::EvidenceInventory
            ) && payload.iter().any(|f| f.sha256 == input.file.sha256)
            {
                binding_reasons.push(AcceptanceReason::PrivateEvidenceExported);
            }
        }
    }
    let mut observations = Vec::new();
    for f in &doc.acceptance.findings {
        let mut reasons = if f.evidence.is_empty() {
            vec![AcceptanceReason::MissingBytes]
        } else {
            vec![]
        };
        let mut outcome = if f.evidence.is_empty() {
            ReviewOutcome::Missing
        } else {
            ReviewOutcome::Pass
        };
        for a in &f.evidence {
            let root = if payload.iter().any(|p| **p == a.file) {
                package_root
            } else {
                private_root
            };
            if let Err(r) = reads.read(root, &a.file, "finding-evidence", 16 * 1024 * 1024) {
                outcome = worse(outcome, reason_outcome(r));
                reasons.push(r);
            }
        }
        observations.push(AcceptanceObservation {
            origin: "candidate_finding".into(),
            id: f.id.clone(),
            check_id: f.id.clone(),
            target: AcceptanceTarget::Candidate,
            claimed_outcome: f.outcome,
            binding_outcome: outcome,
            provenance: doc.acceptance.provenance,
            raw_sha256: None,
            reasons,
        });
    }
    for a in &doc.acceptance.attestations {
        let (id, domain) = match a.kind {
            EvidenceKind::ImageScope => ("image-scope-review", AcceptanceSampleDomain::Source),
            EvidenceKind::PrimaryAnchor => (
                "primary-acoustic-evidence",
                AcceptanceSampleDomain::Narration,
            ),
            EvidenceKind::IndependentAnchor => (
                "independent-acoustic-evidence",
                AcceptanceSampleDomain::Narration,
            ),
        };
        let fact = facts
            .iter()
            .find(|f| f.source_span == a.source_span && f.asset_sha256 == a.target_sha256);
        let target = fact
            .map(|f| AcceptanceTarget::Fact {
                fact: f.clone(),
                domain,
            })
            .unwrap_or(AcceptanceTarget::Candidate);
        let mut reasons = vec![];
        let mut status = ReviewOutcome::Pass;
        if fact.is_none()
            || a.source_sha256 != binding.source_sha256
            || a.narration_sha256 != binding.narration_sha256
            || a.alignment_sha256 != binding.alignment_sha256
            || a.presentation_map_sha256 != binding.presentation_map_sha256
            || a.delivery_audio_sha256 != binding.master_sha256
            || a.recipe_sha256 != binding.recipe_sha256
        {
            reasons.push(AcceptanceReason::InvalidBinding);
            status = ReviewOutcome::Fail;
        }
        if let Some(raw) = &a.raw_evidence {
            if let Err(r) = reads.read(private_root, raw, "attestation-evidence", 16 * 1024 * 1024)
            {
                status = worse(status, reason_outcome(r));
                reasons.push(r);
            }
        } else {
            status = worse(status, ReviewOutcome::Missing);
            reasons.push(AcceptanceReason::MissingBytes);
        }
        observations.push(AcceptanceObservation {
            origin: "legacy_attestation".into(),
            id: a.id.clone(),
            check_id: id.into(),
            target,
            claimed_outcome: a.outcome,
            binding_outcome: status,
            provenance: a.provenance,
            raw_sha256: a.raw_evidence.as_ref().map(|f| f.sha256.clone()),
            reasons,
        });
    }
    let ledger = private_bytes
        .get(&AcceptanceInputRole::EvidenceInventory)
        .and_then(|b| serde_json::from_slice::<AcceptanceEvidenceInventory>(b).ok());
    if ledger
        .as_ref()
        .is_none_or(|l| l.schema_version != 1 || l.proposals.len() > 1024)
    {
        binding_reasons.push(AcceptanceReason::InvalidBinding);
    }
    let ledger_proposals = ledger
        .as_ref()
        .map(|l| l.proposals.as_slice())
        .unwrap_or(&[]);
    for (e, origin) in ledger_proposals
        .iter()
        .map(|e| (e, "ledger_proposal"))
        .chain(evidence.iter().map(|e| (e, "proposal")))
    {
        let metadata = serde_json::to_vec(e).map_err(|err| ContractError {
            code: "acceptance_evidence".into(),
            field: "proposal".into(),
            message: err.to_string(),
        })?;
        require(
            metadata.len() <= 64 * 1024,
            "acceptance_bounds",
            "proposal",
            "proposal metadata exceeds 64 KiB",
        )?;
        reads.consumed.push(AcceptanceConsumedHash {
            role: "proposal-metadata".into(),
            sha256: digest(&metadata),
            bytes: metadata.len() as u64,
        });
        require(
            e.schema_version == 1 && valid_id(&e.id) && valid_id(&e.check_id),
            "acceptance_evidence",
            "proposal",
            "invalid proposal version/ID",
        )?;
        let mut reasons = vec![];
        let mut status = ReviewOutcome::Pass;
        if e.binding != binding {
            status = ReviewOutcome::Fail;
            reasons.push(AcceptanceReason::InvalidBinding);
        }
        if let Err(r) = reads.read(
            private_root,
            &e.raw_evidence,
            "proposed-evidence",
            16 * 1024 * 1024,
        ) {
            status = worse(status, reason_outcome(r));
            reasons.push(r);
        }
        if policy.scope != AcceptanceScope::Diagnostic
            && payload.iter().any(|f| f.sha256 == e.raw_evidence.sha256)
        {
            status = ReviewOutcome::Fail;
            reasons.push(AcceptanceReason::PrivateEvidenceExported);
        }
        observations.push(AcceptanceObservation {
            origin: origin.into(),
            id: e.id.clone(),
            check_id: e.check_id.clone(),
            target: e.target.clone(),
            claimed_outcome: e.outcome,
            binding_outcome: status,
            provenance: e.provenance,
            raw_sha256: Some(e.raw_evidence.sha256.clone()),
            reasons,
        });
    }
    let render_assets = doc
        .manifest
        .assets
        .iter()
        .filter(|f| f.media_type.starts_with("image/"))
        .map(|f| f.sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let required = requirements(
        policy.scope,
        &doc.acceptance.required_check_ids,
        &policy.additional_required_check_ids,
        &facts,
        &render_assets,
        &binding,
        &observations,
    );
    let mut proposal_keys = BTreeMap::new();
    let mut proposal_ids = BTreeMap::new();
    for o in observations
        .iter()
        .filter(|o| o.origin.ends_with("proposal"))
    {
        *proposal_ids.entry(o.id.clone()).or_insert(0usize) += 1;
        *proposal_keys
            .entry((o.check_id.clone(), key(&o.target)))
            .or_insert(0usize) += 1;
    }
    for o in &mut observations {
        if o.origin.ends_with("proposal") {
            if !required
                .iter()
                .any(|r| r.check_id == o.check_id && r.target == o.target)
            {
                o.binding_outcome = ReviewOutcome::Fail;
                o.reasons.push(AcceptanceReason::InvalidBinding);
            }
            if proposal_ids[&o.id] > 1 || proposal_keys[&(o.check_id.clone(), key(&o.target))] > 1 {
                o.binding_outcome = ReviewOutcome::Fail;
                o.reasons.push(AcceptanceReason::DuplicateCoverage);
            }
        }
    }
    observations.sort_by_key(|o| {
        (
            o.check_id.clone(),
            key(&o.target),
            o.origin.clone(),
            o.id.clone(),
            serde_json::to_string(&o.claimed_outcome).unwrap(),
            o.raw_sha256.clone(),
        )
    });
    let binding_status = observations.iter().fold(
        binding_reasons
            .iter()
            .fold(ReviewOutcome::Pass, |o, r| worse(o, reason_outcome(*r))),
        |s, o| worse(s, o.binding_outcome),
    );
    let declared = &doc.acceptance.required_check_ids;
    let mut coverage_reasons = Vec::new();
    if declared.iter().collect::<BTreeSet<_>>().len() != declared.len() {
        coverage_reasons.push(AcceptanceReason::DuplicateCoverage);
    }
    if required.iter().any(|r| !declared.contains(&r.check_id)) {
        coverage_reasons.push(AcceptanceReason::MissingDeclaration);
    }
    let consumed = reads
        .consumed
        .iter()
        .map(|c| c.sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut checks = Vec::new();
    for r in &required {
        let executed = implemented(&r.check_id);
        let mut reasons = if executed {
            match r.check_id.as_str() {
                "package-byte-integrity" => package_reasons.clone(),
                "required-check-coverage" => coverage_reasons.clone(),
                _ => binding_reasons.clone(),
            }
        } else {
            vec![AcceptanceReason::UnimplementedVerifier]
        };
        let mut outcome = if executed {
            match r.check_id.as_str() {
                "package-byte-integrity" => package_reasons
                    .iter()
                    .fold(ReviewOutcome::Pass, |s, r| worse(s, reason_outcome(*r))),
                "required-check-coverage" => {
                    if coverage_reasons.is_empty() {
                        ReviewOutcome::Pass
                    } else {
                        ReviewOutcome::Fail
                    }
                }
                _ => binding_status,
            }
        } else {
            ReviewOutcome::Missing
        };
        if !acceptance_check_matrix()
            .checks
            .iter()
            .any(|c| c.id == r.check_id)
        {
            reasons.push(AcceptanceReason::UnsupportedRequirement);
        }
        if r.check_id == "evidence-binding-integrity" {
            reasons.extend(observations.iter().flat_map(|o| o.reasons.iter().copied()));
        }
        for o in observations.iter().filter(|o| {
            o.check_id == r.check_id
                && (o.target == r.target || o.target == AcceptanceTarget::Candidate)
        }) {
            outcome = worse(outcome, o.binding_outcome);
            reasons.extend(o.reasons.clone());
            if o.claimed_outcome != ReviewOutcome::Pass {
                outcome = worse(outcome, o.claimed_outcome);
                reasons.push(AcceptanceReason::AdverseEvidence);
            } else {
                reasons.push(AcceptanceReason::UntrustedPass);
            }
            if o.provenance != Provenance::Autonomous {
                reasons.push(AcceptanceReason::IneligibleProvenance);
                if policy.scope != AcceptanceScope::Diagnostic {
                    outcome = ReviewOutcome::Fail;
                }
            }
        }
        if !declared.contains(&r.check_id) {
            reasons.push(AcceptanceReason::MissingDeclaration);
            outcome = worse(outcome, ReviewOutcome::Missing);
        }
        if r.check_id == "autonomous-provenance"
            && (doc.manifest.provenance != Provenance::Autonomous
                || doc.acceptance.provenance != Provenance::Autonomous)
        {
            outcome = ReviewOutcome::Fail;
            reasons.push(AcceptanceReason::IneligibleProvenance);
        }
        if reasons.is_empty() {
            reasons.push(AcceptanceReason::Verified);
        }
        reasons.sort_by_key(|r| serde_json::to_string(r).unwrap());
        reasons.dedup();
        checks.push(AcceptanceCheckResult {
            check_id: r.check_id.clone(),
            target: r.target.clone(),
            outcome,
            executed,
            executed_checker: executed.then(|| checker.clone()),
            reasons,
            consumed_sha256s: if executed { consumed.clone() } else { vec![] },
        });
    }
    let mut blockers = checks
        .iter()
        .filter(|c| c.outcome != ReviewOutcome::Pass)
        .map(|c| c.check_id.clone())
        .collect::<Vec<_>>();
    blockers.push("production-verifier-pending".into());
    if doc.manifest.provenance != Provenance::Autonomous
        || doc.acceptance.provenance != Provenance::Autonomous
    {
        blockers.push("autonomous-provenance".into());
    }
    blockers.sort();
    blockers.dedup();
    reads
        .consumed
        .sort_by_key(|c| (c.role.clone(), c.sha256.clone(), c.bytes));
    reads
        .consumed
        .dedup_by(|a, b| a.role == b.role && a.sha256 == b.sha256 && a.bytes == b.bytes);
    let acceptance_sha256 = reads
        .consumed
        .iter()
        .find(|c| c.role == "acceptance")
        .map(|c| c.sha256.clone());
    Ok(PrivateAcceptanceVerification {
        report: AcceptanceVerificationReport {
            schema_version: 1,
            matrix_version: ACCEPTANCE_MATRIX_VERSION.into(),
            private: true,
            scope: policy.scope,
            manifest_sha256,
            acceptance_sha256,
            binding,
            checker,
            declared_check_ids: declared.clone(),
            requirements: required,
            additional_check_ids: policy.additional_required_check_ids.clone(),
            fact_targets: facts,
            render_asset_sha256s: render_assets,
            checks,
            observations,
            consumed_hashes: reads.consumed,
            production_approved: false,
            blockers,
        },
        pins: reads.pins,
    })
}

impl AcceptanceVerificationReport {
    /// Wire consistency only. JSON validation is not an execution capability.
    pub fn validate(&self, expected_checker: &AcceptanceCheckerIdentity) -> ContractResult<()> {
        require(
            self.schema_version == 1 && self.matrix_version == ACCEPTANCE_MATRIX_VERSION,
            "acceptance_report",
            "version",
            "unsupported report/matrix version",
        )?;
        require(
            self.private
                && !self.production_approved
                && self
                    .blockers
                    .iter()
                    .any(|b| b == "production-verifier-pending"),
            "acceptance_report",
            "production_approved",
            "private first-increment reports never authorize production",
        )?;
        require(
            &self.checker == expected_checker && valid_digest(&self.checker.sha256),
            "acceptance_report",
            "checker",
            "unexpected executed checker identity",
        )?;
        let binding = &self.binding;
        for h in [
            &binding.inventory_sha256,
            &binding.source_sha256,
            &binding.narration_sha256,
            &binding.alignment_sha256,
            &binding.presentation_map_sha256,
            &binding.master_sha256,
            &binding.recipe_sha256,
            &binding.profile_sha256,
            &binding.player_sha256,
            &binding.checker_sha256,
            &binding.fact_inventory_sha256,
        ]
        .into_iter()
        .chain(binding.asset_sha256s.iter())
        .chain(self.manifest_sha256.iter())
        .chain(self.acceptance_sha256.iter())
        {
            require(
                valid_digest(h),
                "acceptance_report",
                "binding",
                "invalid digest",
            )?;
        }
        interval(
            binding.source_span,
            MAX_SAFE_INTEGER,
            "binding.source_span",
            true,
        )?;
        require(
            binding.narration_sample_rate > 0
                && binding.narration_sample_rate <= 192000
                && binding.delivery_sample_rate == 48000
                && binding.narration_sample_count > 0
                && binding.narration_sample_count <= MAX_SAFE_INTEGER
                && binding.delivery_sample_count > 0
                && binding.delivery_sample_count <= MAX_SAFE_INTEGER,
            "acceptance_report",
            "sample_domain",
            "invalid sample domain",
        )?;
        let expected = requirements(
            self.scope,
            &self.declared_check_ids,
            &self.additional_check_ids,
            &self.fact_targets,
            &self.render_asset_sha256s,
            binding,
            &self.observations,
        );
        require(
            self.requirements == expected && self.checks.len() == expected.len(),
            "acceptance_report",
            "requirements",
            "missing, duplicated or conflicting required coverage",
        )?;
        let known = self
            .consumed_hashes
            .iter()
            .map(|c| &c.sha256)
            .collect::<BTreeSet<_>>();
        for c in &self.consumed_hashes {
            require(
                valid_digest(&c.sha256) && c.bytes <= MAX_SAFE_INTEGER,
                "acceptance_report",
                "consumed_hashes",
                "invalid consumed identity",
            )?;
        }
        for (r, c) in expected.iter().zip(&self.checks) {
            require(
                r.check_id == c.check_id && r.target == c.target,
                "acceptance_report",
                "checks",
                "coverage must preserve canonical order/target",
            )?;
            let runs = implemented(&c.check_id);
            require(
                c.executed == runs
                    && c.executed_checker.as_ref() == runs.then_some(expected_checker),
                "acceptance_report",
                "executed_checker",
                "candidate declarations cannot become executed checker receipts",
            )?;
            require(
                !c.reasons.is_empty()
                    && (runs
                        || c.outcome != ReviewOutcome::Pass
                            && c.reasons.contains(&AcceptanceReason::UnimplementedVerifier)),
                "acceptance_report",
                "outcome",
                "unimplemented verifiers cannot pass",
            )?;
            require(
                c.consumed_sha256s.iter().all(|s| known.contains(s))
                    && (runs || c.consumed_sha256s.is_empty()),
                "acceptance_report",
                "consumed_sha256s",
                "unexecuted checks do not consume evidence",
            )?;
            if c.outcome != ReviewOutcome::Pass {
                require(
                    self.blockers.contains(&c.check_id),
                    "acceptance_report",
                    "blockers",
                    "adverse result omitted from blockers",
                )?;
            }
            if c.check_id == "package-byte-integrity" && c.outcome == ReviewOutcome::Pass {
                require(
                    self.manifest_sha256.is_some() && self.acceptance_sha256.is_some(),
                    "acceptance_report",
                    "package",
                    "missing verified package identity",
                )?;
            }
        }
        for o in &self.observations {
            require(
                valid_id(&o.id) && valid_id(&o.check_id),
                "acceptance_report",
                "observations",
                "invalid observation identity",
            )?;
            if o.claimed_outcome != ReviewOutcome::Pass {
                require(
                    self.blockers.contains(&o.check_id),
                    "acceptance_report",
                    "observations",
                    "adverse observation must remain blocking",
                )?;
            }
        }
        Ok(())
    }
}
