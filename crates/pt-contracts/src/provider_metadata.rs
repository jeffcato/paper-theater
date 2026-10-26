//! Versioned descriptions, never spending/retry authorization. Legacy methods stay unchanged.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PROVIDER_METADATA_VERSION: u32 = 2;
pub const MAX_PROVIDER_RECORD_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderContext {
    pub provider: Id,
    pub project_id: Id,
    pub credential_context_id: Id,
}

/// Credential contexts do not partition quota: keys for one project share its limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProviderQuotaScope {
    Provider {
        provider: Id,
    },
    Project {
        provider: Id,
        project_id: Id,
    },
    Model {
        provider: Id,
        project_id: Id,
        model: Id,
    },
}
impl ProviderQuotaScope {
    /// SHA-256 of the compact JSON string tuple, including the scope kind.
    pub fn key(&self) -> Digest {
        let tuple = match self {
            Self::Provider { provider } => vec!["provider", provider],
            Self::Project {
                provider,
                project_id,
            } => vec!["project", provider, project_id],
            Self::Model {
                provider,
                project_id,
                model,
            } => vec!["model", provider, project_id, model],
        };
        digest(&serde_json::to_vec(&tuple).expect("string tuple"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuotaAmounts {
    pub money_microusd: u64,
    pub requests: u64,
    /// Input plus output tokens, not a money conversion.
    pub tokens: u64,
    pub repairs: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderQuotaWindow {
    pub window_id: Id,
    pub scope: ProviderQuotaScope,
    pub start_at_ms: u64,
    pub reset_at_ms: u64,
    pub retry_at_ms: Option<u64>,
    pub capacity: QuotaAmounts,
    /// Conservative persisted consumption, including unresolved reservations.
    pub accounted: QuotaAmounts,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreativeRepairIdentity {
    pub repair_id: Id,
    pub previous_operation_id: Id,
    pub repair_index: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransportRetryIdentity {
    pub previous_attempt_id: Id,
    pub previous_request_sha256: Digest,
    pub retry_index: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderAttemptIdentity {
    pub run_id: Id,
    pub epoch: u64,
    pub operation_id: Id,
    pub repair: Option<CreativeRepairIdentity>,
    pub transport_retry: Option<TransportRetryIdentity>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderJobStatus {
    Submitted,
    Running,
    Succeeded,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExistingProviderJob {
    pub attempt_id: Id,
    pub request_sha256: Digest,
    pub provider_job_id: String,
    pub status: ProviderJobStatus,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderOperationV2 {
    pub schema_version: u32,
    pub operation: ProviderOperation,
    pub context: ProviderContext,
    pub identity: ProviderAttemptIdentity,
    pub quota_windows: Vec<ProviderQuotaWindow>,
    /// Some describes inspection/recovery of this existing attempt, never resubmission.
    pub recovery: Option<ExistingProviderJob>,
}
impl ProviderOperationV2 {
    pub fn validate(&self) -> ContractResult<()> {
        envelope(self.schema_version, self)?;
        context(&self.context, &self.operation.provider)?;
        let op = &self.operation;
        id(&op.attempt_id, "operation.attempt_id")?;
        id(&op.unit_id, "operation.unit_id")?;
        id(&op.provider, "operation.provider")?;
        id(&op.model, "operation.model")?;
        hash(&op.recipe_sha256, "operation.recipe_sha256")?;
        hash(&op.request_sha256, "operation.request_sha256")?;
        usage(&op.estimate, "operation.estimate")?;
        require(
            op.estimate.requests > 0,
            "usage",
            "operation.estimate.requests",
            "zero money does not remove request accounting",
        )?;
        require(
            op.timeout_ms > 0 && op.timeout_ms <= 300_000,
            "bounds",
            "operation.timeout_ms",
            "expected 1..300000 milliseconds",
        )?;
        artifact(&op.payload)?;
        require(
            op.payload.file.sha256 == op.request_sha256,
            "provider_binding",
            "operation.payload",
            "payload and request must bind the same exact bytes",
        )?;
        require(
            op.payload.file.bytes <= 1024 * 1024 && op.inputs.len() <= 64,
            "bounds",
            "operation",
            "payload or input inventory exceeds bounds",
        )?;
        for a in &op.inputs {
            artifact(a)?;
        }
        identity(&self.identity, &op.attempt_id, &op.request_sha256)?;
        if let Some(job) = &self.recovery {
            recovery(job, &op.attempt_id, &op.request_sha256)?;
        }
        windows(&self.quota_windows, &self.context, &op.model)
    }
}

/// Each absent metric is unknown. It is never an observed zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedProviderUsage {
    pub requests: Option<u64>,
    /// Provider-reported aggregate, even when the input/output split is unknown.
    pub tokens: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub money_microusd: Option<u64>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRetryReason {
    RateLimited,
    QuotaExhausted,
    RepairExhausted,
    TransientTransport,
    ProviderUnavailable,
    UnknownCompletion,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRetryTiming {
    pub reason: ProviderRetryReason,
    pub observed_at_ms: u64,
    pub not_before_ms: u64,
    pub retry_after_ms: Option<u64>,
    pub window_ids: Vec<Id>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderEpochDisposition {
    Current,
    Obsolete,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderResultV2 {
    pub schema_version: u32,
    pub result: ProviderResult,
    pub context: ProviderContext,
    pub identity: ProviderAttemptIdentity,
    pub quota_windows: Vec<ProviderQuotaWindow>,
    pub observed_usage: ObservedProviderUsage,
    /// Persisted conservative charges, not another claim of observed usage.
    pub accounted_usage: QuotaAmounts,
    pub started_at_ms: u64,
    pub recorded_at_ms: u64,
    pub epoch_disposition: ProviderEpochDisposition,
    pub retry: Option<ProviderRetryTiming>,
    pub recovery: Option<ExistingProviderJob>,
}
impl ProviderResultV2 {
    pub fn validate_for_operation(&self, op: &ProviderOperationV2) -> ContractResult<()> {
        op.validate()?;
        envelope(self.schema_version, self)?;
        let result = &self.result;
        require(
            self.context == op.context
                && self.identity == op.identity
                && result.attempt_id == op.operation.attempt_id
                && result.request_sha256 == op.operation.request_sha256,
            "provider_binding",
            "result",
            "result must match the runtime-issued context, logical operation, attempt and request",
        )?;
        number(self.started_at_ms, "started_at_ms")?;
        number(self.recorded_at_ms, "recorded_at_ms")?;
        number(result.elapsed_ms, "result.elapsed_ms")?;
        require(
            self.started_at_ms <= self.recorded_at_ms,
            "retry",
            "recorded_at_ms",
            "result recording predates start",
        )?;
        require(
            !result.started_at.is_empty()
                && result.started_at.len() <= 64
                && !result.started_at.chars().any(char::is_control)
                && result.diagnostic.len() <= 4096
                && result.outputs.len() <= 64,
            "bounds",
            "result",
            "result text or output inventory exceeds bounds",
        )?;
        if let Some(h) = &result.response_sha256 {
            hash(h, "result.response_sha256")?;
        }
        if let Some(a) = &result.raw_response {
            artifact(a)?;
            require(
                result.response_sha256.as_ref() == Some(&a.file.sha256),
                "provider_binding",
                "result.raw_response",
                "raw response digest differs",
            )?;
        }
        for a in &result.outputs {
            artifact(a)?;
        }
        require(
            result.completion != ProviderCompletion::KnownRejection || result.outputs.is_empty(),
            "provider_binding",
            "result.outputs",
            "known rejection cannot deliver outputs",
        )?;
        amounts(&self.accounted_usage, "accounted_usage")?;
        let observed = &self.observed_usage;
        for n in [
            observed.requests,
            observed.tokens,
            observed.input_tokens,
            observed.output_tokens,
            observed.money_microusd,
        ]
        .into_iter()
        .flatten()
        {
            number(n, "observed_usage")?;
        }
        if let (Some(input), Some(output)) = (observed.input_tokens, observed.output_tokens) {
            require(
                input
                    .checked_add(output)
                    .is_some_and(|n| n <= MAX_SAFE_INTEGER && observed.tokens == Some(n)),
                "usage",
                "observed_usage.tokens",
                "aggregate and split observations differ",
            )?;
        }
        if let Some(total) = observed.tokens {
            require(
                observed
                    .input_tokens
                    .unwrap_or(0)
                    .checked_add(observed.output_tokens.unwrap_or(0))
                    .is_some_and(|n| n <= total),
                "usage",
                "observed_usage.tokens",
                "known token components exceed observed aggregate",
            )?;
        }
        for (actual, charged, reserved) in [
            (
                observed.requests,
                self.accounted_usage.requests,
                op.operation.estimate.requests,
            ),
            (
                observed.tokens,
                self.accounted_usage.tokens,
                op.operation.estimate.input_tokens + op.operation.estimate.output_tokens,
            ),
            (
                observed.money_microusd,
                self.accounted_usage.money_microusd,
                op.operation.estimate.money_microusd,
            ),
        ] {
            if let Some(n) = actual {
                number(n, "observed_usage")?;
            }
            require(
                charged >= actual.unwrap_or(0)
                    && (!(result.completion == ProviderCompletion::Ambiguous || actual.is_none())
                        || charged >= reserved),
                "usage",
                "accounted_usage",
                "unknown or ambiguous usage must retain at least the original reservation; observed overages remain charged",
            )?;
        }
        require(
            self.accounted_usage.tokens
                >= observed
                    .input_tokens
                    .unwrap_or(0)
                    .saturating_add(observed.output_tokens.unwrap_or(0)),
            "usage",
            "accounted_usage.tokens",
            "known token observations exceed retained charges",
        )?;
        require(
            self.accounted_usage.repairs <= 1
                && (op.identity.repair.is_some() || self.accounted_usage.repairs == 0),
            "usage",
            "accounted_usage.repairs",
            "an attempt records at most one distinct creative-repair consumption",
        )?;
        let all_known = observed.requests.is_some()
            && observed.input_tokens.is_some()
            && observed.output_tokens.is_some()
            && observed.money_microusd.is_some();
        require(
            result.usage.is_some() == all_known,
            "usage",
            "result.usage",
            "legacy usage is present exactly when every observed metric is known",
        )?;
        if let Some(known) = &result.usage {
            usage(known, "result.usage")?;
            require(
                observed.requests == Some(known.requests)
                    && observed.input_tokens == Some(known.input_tokens)
                    && observed.output_tokens == Some(known.output_tokens)
                    && observed.money_microusd == Some(known.money_microusd),
                "usage",
                "result.usage",
                "legacy and per-metric observations differ",
            )?;
        }
        windows(&self.quota_windows, &self.context, &op.operation.model)?;
        require(
            self.quota_windows.len() == op.quota_windows.len(),
            "quota_window",
            "quota_windows",
            "result must retain each original quota window",
        )?;
        for old in &op.quota_windows {
            let w = self
                .quota_windows
                .iter()
                .find(|w| w.window_id == old.window_id)
                .ok_or_else(|| ContractError {
                    code: "quota_window".into(),
                    field: "quota_windows".into(),
                    message: "original quota window missing".into(),
                })?;
            require(
                w.scope == old.scope
                    && w.start_at_ms == old.start_at_ms
                    && w.reset_at_ms == old.reset_at_ms
                    && w.capacity == old.capacity,
                "quota_window",
                "quota_windows",
                "original window identity/bounds/capacity changed",
            )?;
            require(
                old.retry_at_ms
                    .is_none_or(|n| w.retry_at_ms.is_some_and(|next| next >= n)),
                "retry",
                "quota_windows.retry_at_ms",
                "persisted retry timestamp cannot be cleared or shortened",
            )?;
            require(
                w.accounted.money_microusd >= self.accounted_usage.money_microusd
                    && w.accounted.requests >= self.accounted_usage.requests
                    && w.accounted.tokens >= self.accounted_usage.tokens
                    && w.accounted.repairs >= self.accounted_usage.repairs,
                "usage",
                "quota_windows.accounted",
                "window accounting omits this attempt's conservative charges",
            )?;
        }
        if let Some(timing) = &self.retry {
            for n in [timing.observed_at_ms, timing.not_before_ms] {
                number(n, "retry")?;
            }
            require(
                timing.observed_at_ms <= self.recorded_at_ms
                    && timing.not_before_ms >= timing.observed_at_ms,
                "retry",
                "retry",
                "invalid observation/retry chronology",
            )?;
            if let Some(delay) = timing.retry_after_ms {
                number(delay, "retry.retry_after_ms")?;
                require(
                    timing
                        .observed_at_ms
                        .checked_add(delay)
                        .is_some_and(|n| n <= timing.not_before_ms),
                    "retry",
                    "retry.not_before_ms",
                    "retry deadline shortens the observed relative delay",
                )?;
            }
            let ids = timing.window_ids.iter().collect::<BTreeSet<_>>();
            require(
                ids.len() == timing.window_ids.len()
                    && ids.len() <= 16
                    && ids
                        .iter()
                        .all(|id| self.quota_windows.iter().any(|w| &w.window_id == *id)),
                "retry",
                "retry.window_ids",
                "unknown or duplicate retry window",
            )?;
            require(
                !matches!(
                    timing.reason,
                    ProviderRetryReason::QuotaExhausted
                        | ProviderRetryReason::RepairExhausted
                        | ProviderRetryReason::RateLimited
                ) || !ids.is_empty(),
                "retry",
                "retry.window_ids",
                "quota/rate/repair reason requires its applicable window",
            )?;
            require(
                self.quota_windows
                    .iter()
                    .filter_map(|w| w.retry_at_ms)
                    .all(|n| timing.not_before_ms >= n),
                "retry",
                "retry.not_before_ms",
                "retry deadline shortens a persisted window cooldown",
            )?;
        }
        if let Some(job) = &self.recovery {
            recovery(job, &result.attempt_id, &result.request_sha256)?;
            require(
                job.observed_at_ms <= self.recorded_at_ms,
                "recovery",
                "recovery.observed_at_ms",
                "job observation postdates this record",
            )?;
        }
        if let Some(old) = &op.recovery {
            require(
                self.recovery.as_ref().is_some_and(|job| {
                    job.provider_job_id == old.provider_job_id
                        && job.observed_at_ms >= old.observed_at_ms
                        && (!matches!(
                            old.status,
                            ProviderJobStatus::Succeeded | ProviderJobStatus::Failed
                        ) || job.status == old.status)
                }),
                "recovery",
                "recovery",
                "existing job identity/observation or terminal state changed",
            )?;
        }
        Ok(())
    }
}

fn envelope(version: u32, value: &impl Serialize) -> ContractResult<()> {
    require(
        version == PROVIDER_METADATA_VERSION,
        "version",
        "schema_version",
        "expected provider metadata version 2",
    )?;
    require(
        serde_json::to_vec(value).map_or(false, |v| v.len() <= MAX_PROVIDER_RECORD_BYTES),
        "bounds",
        "record",
        "provider record exceeds 64 KiB",
    )
}
fn id(value: &str, field: &str) -> ContractResult<()> {
    require(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c)),
        "id",
        field,
        "expected a bounded ASCII identifier",
    )
}
fn number(value: u64, field: &str) -> ContractResult<()> {
    require(
        value <= MAX_SAFE_INTEGER,
        "integer",
        field,
        "expected a nonnegative JSON safe integer",
    )
}
fn hash(value: &str, field: &str) -> ContractResult<()> {
    require(
        valid_digest(value),
        "digest",
        field,
        "expected lowercase SHA-256",
    )
}
fn usage(value: &Usage, field: &str) -> ContractResult<()> {
    for n in [
        value.money_microusd,
        value.requests,
        value.input_tokens,
        value.output_tokens,
    ] {
        number(n, field)?;
    }
    require(
        value
            .input_tokens
            .checked_add(value.output_tokens)
            .is_some_and(|n| n <= MAX_SAFE_INTEGER),
        "integer",
        field,
        "combined token count exceeds safe integer range",
    )
}
fn artifact(a: &ArtifactRef) -> ContractResult<()> {
    id(&a.id, "artifact.id")?;
    a.file.validate()
}
fn identity(value: &ProviderAttemptIdentity, attempt: &str, request: &str) -> ContractResult<()> {
    id(&value.run_id, "identity.run_id")?;
    id(&value.operation_id, "identity.operation_id")?;
    number(value.epoch, "identity.epoch")?;
    if let Some(repair) = &value.repair {
        id(&repair.repair_id, "identity.repair.repair_id")?;
        id(
            &repair.previous_operation_id,
            "identity.repair.previous_operation_id",
        )?;
        number(repair.repair_index, "identity.repair.repair_index")?;
        require(
            repair.repair_index > 0 && repair.previous_operation_id != value.operation_id,
            "repair",
            "identity.repair",
            "creative repair needs a new logical operation and positive repair index",
        )?;
    }
    if let Some(retry) = &value.transport_retry {
        id(
            &retry.previous_attempt_id,
            "identity.transport_retry.previous_attempt_id",
        )?;
        hash(
            &retry.previous_request_sha256,
            "identity.transport_retry.previous_request_sha256",
        )?;
        number(retry.retry_index, "identity.transport_retry.retry_index")?;
        require(
            retry.retry_index > 0 && retry.previous_attempt_id != attempt,
            "retry",
            "identity.transport_retry",
            "transport retry needs a distinct previous attempt and positive index",
        )?;
        require(
            retry.previous_request_sha256 == request,
            "provider_binding",
            "identity.transport_retry",
            "transport retry cannot change request bytes",
        )?;
    }
    Ok(())
}
fn recovery(job: &ExistingProviderJob, attempt: &str, request: &str) -> ContractResult<()> {
    id(&job.attempt_id, "recovery.attempt_id")?;
    hash(&job.request_sha256, "recovery.request_sha256")?;
    require(
        job.attempt_id == attempt && job.request_sha256 == request,
        "recovery",
        "recovery",
        "recovery must identify this existing attempt/request",
    )?;
    require(
        !job.provider_job_id.is_empty()
            && job.provider_job_id.len() <= 256
            && job.provider_job_id.bytes().all(|b| (33..=126).contains(&b)),
        "recovery",
        "recovery.provider_job_id",
        "expected a bounded opaque printable ASCII job ID",
    )?;
    number(job.observed_at_ms, "recovery.observed_at_ms")
}
fn context(value: &ProviderContext, provider: &str) -> ContractResult<()> {
    id(&value.provider, "context.provider")?;
    id(&value.project_id, "context.project_id")?;
    id(
        &value.credential_context_id,
        "context.credential_context_id",
    )?;
    require(
        value.provider == provider,
        "provider_binding",
        "context.provider",
        "provider identity differs",
    )
}
fn amounts(value: &QuotaAmounts, field: &str) -> ContractResult<()> {
    for n in [
        value.money_microusd,
        value.requests,
        value.tokens,
        value.repairs,
    ] {
        number(n, field)?;
    }
    Ok(())
}
fn windows(
    values: &[ProviderQuotaWindow],
    ctx: &ProviderContext,
    model: &str,
) -> ContractResult<()> {
    require(
        !values.is_empty() && values.len() <= 16,
        "quota_scope",
        "quota_windows",
        "expected 1..16 applicable windows",
    )?;
    let mut ids = BTreeSet::new();
    let mut tuples = BTreeSet::new();
    let mut project = false;
    for w in values {
        id(&w.window_id, "quota_windows.window_id")?;
        let applicable = match &w.scope {
            ProviderQuotaScope::Provider { provider } => provider == &ctx.provider,
            ProviderQuotaScope::Project {
                provider,
                project_id,
            } => {
                project |= provider == &ctx.provider && project_id == &ctx.project_id;
                provider == &ctx.provider && project_id == &ctx.project_id
            }
            ProviderQuotaScope::Model {
                provider,
                project_id,
                model: m,
            } => provider == &ctx.provider && project_id == &ctx.project_id && m == model,
        };
        require(
            applicable,
            "quota_scope",
            "quota_windows.scope",
            "scope does not match provider/project/model",
        )?;
        number(w.start_at_ms, "quota_windows.start_at_ms")?;
        number(w.reset_at_ms, "quota_windows.reset_at_ms")?;
        if let Some(n) = w.retry_at_ms {
            number(n, "quota_windows.retry_at_ms")?;
        }
        require(
            w.start_at_ms < w.reset_at_ms,
            "quota_window",
            "quota_windows.reset_at_ms",
            "expected a positive half-open UTC-millisecond window",
        )?;
        require(
            ids.insert(&w.window_id)
                && tuples.insert((w.scope.key(), w.start_at_ms, w.reset_at_ms)),
            "quota_window",
            "quota_windows",
            "duplicate window ID or scope/interval",
        )?;
        amounts(&w.capacity, "quota_windows.capacity")?;
        amounts(&w.accounted, "quota_windows.accounted")?;
    }
    require(
        project,
        "quota_scope",
        "quota_windows",
        "project quota scope is required; credentials do not create new capacity",
    )
}
