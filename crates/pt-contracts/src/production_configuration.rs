//! Administrative wire input. Runtime's persisted reconciliation grants authority.
//! Validation, hashes and deserialization never grant issuance or publication.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

fn bounded_limits(limits: &Limits) -> ContractResult<()> {
    for n in [
        limits.money_microusd,
        limits.requests,
        limits.tokens,
        limits.wall_time_ms,
    ] {
        require(
            n <= MAX_SAFE_INTEGER,
            "production_config",
            "limits",
            "unsafe JSON integer",
        )?;
    }
    require(
        limits.wall_time_ms > 0 && limits.wall_time_ms <= 300_000,
        "production_config",
        "limits.wall_time_ms",
        "expected 1..300000 milliseconds",
    )
}
fn within(a: &Limits, b: &Limits) -> bool {
    a.money_microusd <= b.money_microusd
        && a.requests <= b.requests
        && a.tokens <= b.tokens
        && a.repairs <= b.repairs
        && a.wall_time_ms <= b.wall_time_ms
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductionPurposeV1 {
    Pilot,
    Bulk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionOperationScopeV1 {
    pub id: Id,
    pub unit_id: Id,
    pub stage: Stage,
    pub purpose: ProductionPurposeV1,
    pub kind: OperationKind,
    pub context: ProviderContext,
    pub model: Id,
    pub recipe_sha256: Digest,
    pub request_sha256: Digest,
    /// Environment name only; contents never enter the wire record.
    pub secret_env: String,
    pub limits: Limits,
    pub allow_recovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionConfigurationV1 {
    pub schema_version: u32,
    pub id: Id,
    pub book_id: Id,
    pub source_id: Id,
    pub pins: Pins,
    pub authorization_proof_sha256: Digest,
    pub accounting_original_sha256s: Vec<Digest>,
    pub limits: Limits,
    pub operations: Vec<ProductionOperationScopeV1>,
}

impl ProductionConfigurationV1 {
    /// Content/scope consistency only. Runtime must check durable authority,
    /// immutable request bytes, active epoch, reservations and existing attempts.
    pub fn validate_proposal(
        &self,
        scope_id: &str,
        input: &StageInput,
        proposal: &ProviderProposalV2,
    ) -> ContractResult<()> {
        self.validate()?;
        input.validate()?;
        proposal.validate()?;
        let op = self
            .operations
            .iter()
            .find(|o| o.id == scope_id)
            .ok_or_else(|| ContractError {
                code: "production_config".into(),
                field: "scope_id".into(),
                message: "operation is not configured".into(),
            })?;
        require(
            input.mode == ExecutionMode::Live
                && input.stage == op.stage
                && input.unit_id == op.unit_id
                && input.pins.profile_id == self.pins.profile_id
                && input.pins.profile_sha256 == self.pins.profile_sha256
                && input.pins.recipe_sha256 == self.pins.recipe_sha256
                && within(&input.limits, &op.limits)
                && proposal.kind == op.kind
                && proposal.provider == op.context.provider
                && proposal.model == op.model
                && proposal.recipe_sha256 == op.recipe_sha256
                && proposal.payload.file.sha256 == op.request_sha256
                && proposal.timeout_ms <= input.limits.wall_time_ms
                && proposal.estimate.requests <= input.limits.requests
                && proposal.estimate.money_microusd <= input.limits.money_microusd
                && proposal
                    .estimate
                    .input_tokens
                    .checked_add(proposal.estimate.output_tokens)
                    .is_some_and(|n| n <= input.limits.tokens),
            "production_config",
            "proposal",
            "request/stage/pins/estimate differ from configured scope",
        )
    }

    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 1,
            "production_config",
            "schema_version",
            "expected 1",
        )?;
        StageInput {
            schema_version: 1,
            run_id: self.id.clone(),
            stage: Stage::Source,
            unit_id: self.id.clone(),
            epoch: 0,
            revision: 0,
            inputs: vec![],
            pins: self.pins.clone(),
            mode: ExecutionMode::Live,
            limits: self.limits.clone(),
            library_revision: 0,
        }
        .validate()?;
        bounded_limits(&self.limits)?;
        require(
            matches!(
                (self.book_id.as_str(), self.source_id.as_str()),
                ("pg-55", "pg-55-chapter-1") | ("pg-36", "pg-36-book-1-chapter-1")
            ),
            "production_config",
            "source_id",
            "book and selected chapter differ",
        )?;
        require(
            valid_digest(&self.authorization_proof_sha256)
                && !self.accounting_original_sha256s.is_empty()
                && self.accounting_original_sha256s.len() <= 256
                && self
                    .accounting_original_sha256s
                    .iter()
                    .all(|h| valid_digest(h))
                && self
                    .accounting_original_sha256s
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    == self.accounting_original_sha256s.len(),
            "production_config",
            "accounting_original_sha256s",
            "exact nonempty unique accounting and authorization identities required",
        )?;
        require(
            !self.operations.is_empty()
                && self.operations.len() <= 64
                && serde_json::to_vec(self).map_or(false, |b| b.len() <= 64 * 1024),
            "production_config",
            "operations",
            "bounded nonempty operation inventory required",
        )?;
        let mut ids = BTreeSet::new();
        for op in &self.operations {
            bounded_limits(&op.limits)?;
            require(
                [
                    &op.id,
                    &op.unit_id,
                    &op.context.provider,
                    &op.context.project_id,
                    &op.context.credential_context_id,
                    &op.model,
                ]
                .iter()
                .all(|v| valid_id(v))
                    && ids.insert(&op.id)
                    && valid_digest(&op.recipe_sha256)
                    && valid_digest(&op.request_sha256),
                "production_config",
                "operations",
                "unique operation and exact provider/model/recipe/request identities required",
            )?;
            require(
                !op.secret_env.is_empty()
                    && op.secret_env.len() <= 128
                    && !op.secret_env.as_bytes()[0].is_ascii_digit()
                    && op
                        .secret_env
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'),
                "production_config",
                "secret_env",
                "environment name only; credential contents are forbidden",
            )?;
            require(
                within(&op.limits, &self.limits)
                    && op.limits.requests > 0
                    && (op.purpose != ProductionPurposeV1::Pilot || op.stage == Stage::Pilot),
                "production_config",
                "operations.limits",
                "operation cannot widen limits or relabel Pilot scope",
            )?;
        }
        Ok(())
    }
}
