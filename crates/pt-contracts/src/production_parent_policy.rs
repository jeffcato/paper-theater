//! Compiler-bound policy descriptions. Runtime persistence grants authority.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionCompilerScopeV1 {
    pub id: Id,
    pub stage: Stage,
    pub purpose: ProductionPurposeV1,
    pub kind: OperationKind,
    pub context: ProviderContext,
    pub model: Id,
    pub recipe_sha256: Digest,
    pub secret_env: String,
    pub limits: Limits,
    pub allow_recovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionParentPolicyV1 {
    pub schema_version: u32,
    pub id: Id,
    /// Stable inherited ledger identity; run/unit/attempt aliases never reset it.
    pub accounting_scope_id: Id,
    pub book_id: Id,
    pub source_id: Id,
    pub source_sha256: Digest,
    pub compiler_sha256: Digest,
    pub pins: Pins,
    pub authorization_proof_sha256: Digest,
    pub accounting_original_sha256s: Vec<Digest>,
    /// Explicit immutable reuse receipts, including imported source provenance.
    pub imported_receipt_sha256s: Vec<Digest>,
    pub limits: Limits,
    pub operations: Vec<ProductionCompilerScopeV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionInputLineageV1 {
    pub artifact: ArtifactRef,
    pub source_sha256: Digest,
    pub compiler_sha256: Option<Digest>,
    pub producer_receipt_sha256: Digest,
}

/// Private audit proposal. Runtime reconstructs this from committed inputs and
/// verified compiler provenance; deserialization never grants derived execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionDerivationV1 {
    pub schema_version: u32,
    pub parent_policy: FileRef,
    pub scope_id: Id,
    pub compiler_sha256: Digest,
    pub source_sha256: Digest,
    pub purpose: ProductionPurposeV1,
    pub input: StageInput,
    pub proposal: ProviderProposalV2,
    pub input_lineage: Vec<ProductionInputLineageV1>,
}

impl ProductionParentPolicyV1 {
    pub fn validate(&self) -> ContractResult<()> {
        let hashes = |values: &[Digest]| {
            values.len() <= 256
                && values.iter().all(|h| valid_digest(h))
                && values.iter().collect::<BTreeSet<_>>().len() == values.len()
        };
        require(
            self.schema_version == 1
                && valid_id(&self.id)
                && valid_id(&self.accounting_scope_id)
                && valid_digest(&self.compiler_sha256)
                && valid_digest(&self.authorization_proof_sha256)
                && valid_digest(&self.pins.recipe_sha256)
                && valid_digest(&self.pins.profile_sha256)
                && valid_id(&self.pins.profile_id)
                && !self.accounting_original_sha256s.is_empty()
                && hashes(&self.accounting_original_sha256s)
                && hashes(&self.imported_receipt_sha256s)
                && registered_source_catalog_v1().sources.iter().any(|s| {
                    s.book_id == self.book_id
                        && s.source_id == self.source_id
                        && s.canonical_source_sha256 == self.source_sha256
                })
                && !self.operations.is_empty()
                && self.operations.len() <= 64
                && serde_json::to_vec(self).is_ok_and(|b| b.len() <= 64 * 1024),
            "production_parent",
            "policy",
            "bounded registered source/compiler/pins and original accounting identities required",
        )?;
        crate::production_configuration::bounded_limits(&self.limits)?;
        let mut ids = BTreeSet::new();
        for scope in &self.operations {
            crate::production_configuration::bounded_limits(&scope.limits)?;
            require(
                [
                    &scope.id,
                    &scope.context.provider,
                    &scope.context.project_id,
                    &scope.context.credential_context_id,
                    &scope.model,
                ]
                .iter()
                .all(|id| valid_id(id))
                    && ids.insert(&scope.id)
                    && valid_digest(&scope.recipe_sha256)
                    && !scope.secret_env.is_empty()
                    && scope.secret_env.len() <= 128
                    && !scope.secret_env.as_bytes()[0].is_ascii_digit()
                    && scope
                        .secret_env
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                    && crate::production_configuration::within(&scope.limits, &self.limits)
                    && scope.limits.requests > 0
                    && (scope.purpose != ProductionPurposeV1::Pilot || scope.stage == Stage::Pilot),
                "production_parent",
                "operations",
                "unique bounded compiler scope cannot widen limits or relabel Pilot",
            )?;
        }
        Ok(())
    }

    pub fn validate_derivation(
        &self,
        derived: &ProductionDerivationV1,
        original_policy_bytes: &[u8],
        logical_request_bytes: &[u8],
    ) -> ContractResult<()> {
        self.validate()?;
        require(
            derived.parent_policy.bytes <= 64 * 1024
                && derived.parent_policy.media_type == "application/json"
                && derived.input_lineage.len() <= 128
                && serde_json::to_vec(derived).is_ok_and(|b| b.len() <= 64 * 1024),
            "production_parent",
            "bounds",
            "bounded policy and derivation metadata required",
        )?;
        derived.parent_policy.verify(original_policy_bytes)?;
        let original: Self =
            serde_json::from_slice(original_policy_bytes).map_err(|_| ContractError {
                code: "production_parent".into(),
                field: "parent_policy".into(),
                message: "invalid original parent policy".into(),
            })?;
        require(
            serde_json::to_value(original).unwrap() == serde_json::to_value(self).unwrap(),
            "production_parent",
            "parent_policy",
            "original parent policy differs",
        )?;
        derived.input.validate()?;
        derived.proposal.validate()?;
        derived
            .proposal
            .payload
            .file
            .verify(logical_request_bytes)?;
        let scope = self
            .operations
            .iter()
            .find(|o| o.id == derived.scope_id)
            .ok_or_else(|| ContractError {
                code: "production_parent".into(),
                field: "scope_id".into(),
                message: "unconfigured compiler scope".into(),
            })?;
        let input = &derived.input;
        let proposal = &derived.proposal;
        require(
            derived.schema_version == 1
                && valid_digest(&self.compiler_sha256)
                && derived.compiler_sha256 == self.compiler_sha256
                && derived.source_sha256 == self.source_sha256
                && input.mode == ExecutionMode::Live
                && input.stage == scope.stage
                && derived.purpose == scope.purpose
                && input.pins.profile_id == self.pins.profile_id
                && input.pins.profile_sha256 == self.pins.profile_sha256
                && input.pins.recipe_sha256 == self.pins.recipe_sha256
                && proposal.kind == scope.kind
                && proposal.provider == scope.context.provider
                && proposal.model == scope.model
                && proposal.recipe_sha256 == scope.recipe_sha256
                && proposal.payload.provenance == Provenance::Autonomous
                && crate::production_configuration::within(&scope.limits, &self.limits)
                && crate::production_configuration::within(&input.limits, &scope.limits)
                && proposal.timeout_ms <= input.limits.wall_time_ms
                && proposal.estimate.requests <= input.limits.requests
                && proposal.estimate.money_microusd <= input.limits.money_microusd
                && proposal
                    .estimate
                    .input_tokens
                    .checked_add(proposal.estimate.output_tokens)
                    .is_some_and(|n| n <= input.limits.tokens),
            "production_parent",
            "derivation",
            "derived request differs from frozen compiler scope",
        )?;
        let mut expected = BTreeMap::new();
        for artifact in input.inputs.iter().chain(&proposal.inputs) {
            if let Some(previous) = expected.insert(&artifact.id, artifact) {
                require(
                    serde_json::to_value(previous).unwrap()
                        == serde_json::to_value(artifact).unwrap(),
                    "production_parent",
                    "input_lineage",
                    "input identity has conflicting bytes or provenance",
                )?;
            }
        }
        require(
            !expected.is_empty()
                && expected.len() <= 128
                && derived.input_lineage.len() == expected.len(),
            "production_parent",
            "input_lineage",
            "complete nonempty input provenance required",
        )?;
        let mut seen = BTreeSet::new();
        for lineage in &derived.input_lineage {
            let artifact = &lineage.artifact;
            require(
                seen.insert(&artifact.id)
                    && expected.get(&artifact.id).is_some_and(|expected| {
                        serde_json::to_value(expected).unwrap()
                            == serde_json::to_value(artifact).unwrap()
                    })
                    && lineage.source_sha256 == self.source_sha256
                    && valid_digest(&lineage.producer_receipt_sha256)
                    && lineage
                        .compiler_sha256
                        .as_ref()
                        .is_none_or(|h| valid_digest(h))
                    && match artifact.provenance {
                        Provenance::Autonomous => {
                            lineage.compiler_sha256.as_ref() == Some(&self.compiler_sha256)
                        }
                        Provenance::Imported => self
                            .imported_receipt_sha256s
                            .contains(&lineage.producer_receipt_sha256),
                        _ => false,
                    },
                "production_parent",
                "input_lineage",
                "unregistered, diagnostic or stale input provenance",
            )?;
        }
        Ok(())
    }
}
