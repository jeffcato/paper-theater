//! Staged provider-v2 service descriptions. Runtime persistence, never JSON, grants authority.
use crate::provider_metadata::{amounts, artifact, envelope, hash, id, number, usage};
use crate::*;
use serde::{Deserialize, Serialize};

pub const PROVIDER_SERVICE_VERSION: u32 = 2;
/// Stage proposes content only. All execution context and lineage are runtime-issued.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProposalV2 {
    pub schema_version: u32,
    pub kind: OperationKind,
    pub provider: Id,
    pub model: Id,
    pub recipe_sha256: Digest,
    pub payload: ArtifactRef,
    pub inputs: Vec<ArtifactRef>,
    pub estimate: Usage,
    pub timeout_ms: u64,
}
impl ProviderProposalV2 {
    pub fn validate(&self) -> ContractResult<()> {
        envelope(self.schema_version, self)?;
        id(&self.provider, "provider")?;
        id(&self.model, "model")?;
        hash(&self.recipe_sha256, "recipe_sha256")?;
        artifact(&self.payload)?;
        usage(&self.estimate, "estimate")?;
        require(
            self.estimate.requests > 0,
            "usage",
            "estimate.requests",
            "zero money does not remove request accounting",
        )?;
        require(
            self.timeout_ms > 0 && self.timeout_ms <= 300_000,
            "bounds",
            "timeout_ms",
            "expected 1..300000 milliseconds",
        )?;
        require(
            self.payload.file.bytes <= 1024 * 1024 && self.inputs.len() <= 64,
            "bounds",
            "proposal",
            "payload or input inventory exceeds bounds",
        )?;
        for a in &self.inputs {
            artifact(a)?;
        }
        Ok(())
    }
}
/// Runtime-bound service scope, also used to check returned observer metadata.
/// A serialized observer does not authorize issuance, execution or queries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderObserverV2 {
    pub schema_version: u32,
    pub run_id: Id,
    pub epoch: u64,
    pub unit_id: Id,
}
impl ProviderObserverV2 {
    pub fn validate(&self) -> ContractResult<()> {
        envelope(self.schema_version, self)?;
        id(&self.run_id, "observer.run_id")?;
        id(&self.unit_id, "observer.unit_id")?;
        number(self.epoch, "observer.epoch")
    }
}
impl ProviderOperationV2 {
    /// Consistency check only; runtime must independently recheck immutable issuance.
    pub fn validate_for_proposal(
        &self,
        proposal: &ProviderProposalV2,
        observer: &ProviderObserverV2,
    ) -> ContractResult<()> {
        self.validate()?;
        proposal.validate()?;
        observer.validate()?;
        require(
            self.recovery.is_none(),
            "recovery",
            "recovery",
            "an existing-job descriptor cannot submit new work",
        )?;
        let op = &self.operation;
        require(
            self.identity.run_id == observer.run_id
                && self.identity.epoch == observer.epoch
                && op.unit_id == observer.unit_id,
            "provider_binding",
            "observer",
            "issued work differs from the runtime-bound service scope",
        )?;
        let content = ProviderProposalV2 {
            schema_version: 2,
            kind: op.kind,
            provider: op.provider.clone(),
            model: op.model.clone(),
            recipe_sha256: op.recipe_sha256.clone(),
            payload: op.payload.clone(),
            inputs: op.inputs.clone(),
            estimate: op.estimate.clone(),
            timeout_ms: op.timeout_ms,
        };
        require(
            serde_json::to_value(content).unwrap() == serde_json::to_value(proposal).unwrap(),
            "provider_binding",
            "proposal",
            "issued descriptor differs from proposed content",
        )
    }
}
/// An untrusted existing-job selector. It cannot issue its own observer or query budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRecoveryRequestV2 {
    pub schema_version: u32,
    pub original: ProviderOperationV2,
}
impl ProviderRecoveryRequestV2 {
    pub fn validate(&self) -> ContractResult<()> {
        envelope(self.schema_version, self)?;
        self.original.validate()?;
        require(
            self.original.recovery.is_some(),
            "recovery",
            "original.recovery",
            "existing persisted job identity required",
        )
    }
}
/// Private recovery receipt: original result identity and query accounting stay separate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRecoveryResultV2 {
    pub schema_version: u32,
    pub observer: ProviderObserverV2,
    pub query_id: Id,
    pub original: ProviderResultV2,
    pub query_completion: ProviderCompletion,
    pub query_reserved_usage: QuotaAmounts,
    pub query_accounted_usage: QuotaAmounts,
    pub query_observed_usage: ObservedProviderUsage,
    pub query_started_at_ms: u64,
    pub query_recorded_at_ms: u64,
}
impl ProviderRecoveryResultV2 {
    pub fn validate_for_request(
        &self,
        request: &ProviderRecoveryRequestV2,
        expected_observer: &ProviderObserverV2,
    ) -> ContractResult<()> {
        envelope(self.schema_version, self)?;
        request.validate()?;
        self.observer.validate()?;
        expected_observer.validate()?;
        require(
            self.observer == *expected_observer,
            "provider_binding",
            "observer",
            "query observer differs from the runtime-bound service scope",
        )?;
        id(&self.query_id, "query_id")?;
        require(
            self.query_id != self.original.result.attempt_id,
            "recovery",
            "query_id",
            "status query identity must differ from the original attempt",
        )?;
        self.original.validate_for_operation(&request.original)?;
        number(self.query_started_at_ms, "query_started_at_ms")?;
        number(self.query_recorded_at_ms, "query_recorded_at_ms")?;
        require(
            self.query_started_at_ms <= self.query_recorded_at_ms
                && self.original.recorded_at_ms <= self.query_recorded_at_ms,
            "retry",
            "query_recorded_at_ms",
            "query or original result postdates query recording",
        )?;
        amounts(&self.query_reserved_usage, "query_reserved_usage")?;
        amounts(&self.query_accounted_usage, "query_accounted_usage")?;
        let (reserved, charged, obs) = (
            &self.query_reserved_usage,
            &self.query_accounted_usage,
            &self.query_observed_usage,
        );
        require(
            reserved.requests > 0
                && charged.requests > 0
                && reserved.repairs == 0
                && charged.repairs == 0,
            "usage",
            "query_accounting",
            "status queries consume requests and cannot consume creative repairs",
        )?;
        for n in [
            obs.requests,
            obs.tokens,
            obs.input_tokens,
            obs.output_tokens,
            obs.money_microusd,
        ]
        .into_iter()
        .flatten()
        {
            number(n, "query_observed_usage")?;
        }
        let split = obs
            .input_tokens
            .unwrap_or(0)
            .saturating_add(obs.output_tokens.unwrap_or(0));
        if let (Some(i), Some(o)) = (obs.input_tokens, obs.output_tokens) {
            require(
                i.checked_add(o)
                    .is_some_and(|n| n <= MAX_SAFE_INTEGER && Some(n) == obs.tokens),
                "usage",
                "query_observed_usage.tokens",
                "aggregate and split observations differ",
            )?;
        }
        if let Some(total) = obs.tokens {
            require(
                split <= total,
                "usage",
                "query_observed_usage.tokens",
                "known token components exceed observed aggregate",
            )?;
        }
        require(
            charged.tokens >= split,
            "usage",
            "query_accounted_usage.tokens",
            "known token observations exceed retained query charges",
        )?;
        for (actual, accounted, reservation) in [
            (obs.requests, charged.requests, reserved.requests),
            (obs.tokens, charged.tokens, reserved.tokens),
            (
                obs.money_microusd,
                charged.money_microusd,
                reserved.money_microusd,
            ),
        ] {
            require(
                accounted >= actual.unwrap_or(0)
                    && (!(self.query_completion == ProviderCompletion::Ambiguous
                        || actual.is_none())
                        || accounted >= reservation),
                "usage",
                "query_accounted_usage",
                "unknown or ambiguous query usage must retain reservations; observed overages stay charged",
            )?;
        }
        Ok(())
    }
}
