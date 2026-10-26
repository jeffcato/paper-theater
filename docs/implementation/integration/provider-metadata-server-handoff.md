# Server adapter handoff — I2 / R2-002

Consume `pt-services-v1.3.0`, provider envelope version 2, with unchanged playback v1. Validate `ProviderOperationV2` and validate every result against its original operation. Records and semantics are in [provider-metadata-v2.md](provider-metadata-v2.md); exact files/checks are in [provider-metadata-ready.json](provider-metadata-ready.json). This handoff does not implement or enable an adapter.

## Field mapping

Private sources below are in `crates/pt-production/src/runtime/{broker.rs,schema.sql,runner.rs,store.rs}`. `Operation`, `Limits`, `Receipt` and administrative broker methods remain private. Never expose them by changing visibility to make this handoff compile.

| Public field | Existing private source | Required adapter behavior or gap |
| --- | --- | --- |
| `context.provider`, `context.project_id` | `Operation.provider`, `.project` | Resolve from runtime's configured project registry; compare with the requested provider/model. No caller-created scope. |
| `context.credential_context_id` | No persisted equivalent | Add a non-secret registry binding. Multiple slots share project limits; never store a key or use a slot to multiply quota. |
| `operation.attempt_id` | `attempts.id` | Issue the actual persisted attempt ID. Existing reserve computes `digest("{run}:{Operation.identity}")`; retain that mapping for existing records. Do not put a caller attempt ID into `Operation.identity` and accidentally derive a different ID. |
| `identity.run_id`, `.epoch` | `attempts.run`, `.epoch`; active run binding | Use the original attempt identity for late results. Compare active epoch only to gate stage side effects. |
| `identity.operation_id` | No stable cross-attempt equivalent | Add persisted logical-operation identity. Existing `Operation.identity` / `attempts.operation` is a run-local idempotency key, not sufficient logical/repair lineage. Keep its existing uniqueness and descriptor conflict checks. |
| `identity.repair.*` | `Operation.repair`, `attempts.repair`, run/scope repair limits | Add repair ID, previous operation and positive ordinal; consume a repair ID once durably across transport retries/restarts/new runs. Do not map every retry with a repair identity to a new `repair=true` charge. |
| `identity.transport_retry.*` | Existing attempts/request hashes; no lineage columns | Persist previous attempt/request and retry ordinal. Verify unchanged logical operation, repair, context, request and recipe against stored predecessors. An unresolved/ambiguous predecessor blocks repurchase. |
| `operation.unit_id`, `.kind` | Active Services unit; no complete broker persistence | Bind and persist unit/kind to the issued descriptor. A model/caller-selected string is insufficient. |
| `operation.provider`, `.model`, `.recipe_sha256` | `Operation.provider`, `.model`, `.recipe`; recipes allowlist | Resolve digest to the allowlisted runtime recipe; preserve its exact meaning rather than treating an arbitrary digest as an allowed recipe name. |
| `operation.request_sha256`, `.payload`, `.inputs` | `Operation.request_hash`, `.payload`; artifact store | Verify exact payload bytes and artifact refs before dispatch. Existing private validation reserializes JSON; require matching canonical bytes or change the private adapter to retain exact bytes. Never silently rehash a different request. |
| `operation.estimate` | `Operation.money`, `.requests`, `.tokens` | Map money/requests directly, tokens to checked input+output sum. Keep the original split in the issued record. Estimate is a proposal until runtime reserves it. |
| `operation.timeout_ms` | `Operation.timeout_ms` | Keep bounded deadline/cancellation behavior. Timeout creates ambiguous completion, retaining charges. |
| `quota_windows[].scope` | Existing `provider/project/model` scope string | Use tagged tuple keys. Migrate known legacy tuples by explicit aliases without resetting limits/attempts/imports; provider/project-wide scopes need new enforcement/storage. Never guess an unbound legacy project. |
| `quota_windows[].window_id` | No stable ID; one `limits` row per scope today | Add durable scope/interval identities; allow multiple simultaneous windows for the same scope. |
| `.start_at_ms`, `.reset_at_ms`, `.retry_at_ms` | `limits.window_start`, `.reset_at`, `.retry_at`; imported windows/retry | Values already use Unix milliseconds. Retain MAX cooldown across persisted/imported observations. Preserve old windows and overlapping inherited consumption. |
| `.capacity.{money_microusd,requests,tokens,repairs}` | `Limits.{money,requests,tokens,repairs}` | Exact unit mapping; each applicable capacity must be enforced atomically. |
| `.accounted.*` | Conservative sums of `attempts.{money,requests,tokens,repair}` plus overlapping accounting imports | Keep reservations for missing/ambiguous usage and inherited uncertain charges. Do not zero consumption at migration, cancellation, a key change or a new run. Preserve truthful over-capacity snapshots. |
| `result.attempt_id`, `.request_sha256` | `attempts.id`, `.request_hash` | Must match issued operation and immutable stored descriptor. Reject conflicting receipt identity. |
| `result.completion` | `Receipt.outcome`, `attempts.state` | `completed` → completed, `rejected` → known_rejection, `ambiguous`/uncertain → ambiguous. Recovery-pending is not successful completion of the underlying job. |
| `result.response_sha256`, `.raw_response`, `.outputs` | `Receipt.response_hash`; response/artifact store | Hash field maps directly; raw refs/outputs need the bounded verified artifact adapter. Do not invent refs from a hash alone. |
| `observed_usage.money_microusd`, `.tokens` | `Receipt.actual_money`, `.actual_tokens` | Preserve each Option exactly, including None. Aggregate token observations do not fabricate an input/output split. |
| `observed_usage.requests`, `.input_tokens`, `.output_tokens` | No authoritative per-metric fields today | Keep unknown until genuine provider receipts supply them. Reserved request count is not observed usage. |
| `result.usage` | No exact split in current Receipt | Leave null unless all legacy metrics are genuinely known and match observations. Never fill missing values with zero. |
| `accounted_usage` | Persisted conservative `attempts.money`, `.requests`, `.tokens`, and this attempt's distinct repair consumption | Match actual retained charges, not invented observed usage. Ambiguous usage retains max(reserved, observed); missing metrics retain their reservation. Repair zero on a retry does not undo its previously consumed repair ID. |
| `started_at_ms`, legacy `.started_at`, `.elapsed_ms` | `attempts.created` is reservation time, not a proven transport start | Persist actual dispatch-start and monotonic duration; do not relabel reservation time as a known transport timestamp. |
| `recorded_at_ms`, `epoch_disposition` | Result journal timestamp; original attempt epoch vs current run epoch | Persist recording time; late billing/results remain attached to original work without current-epoch artifact/library/publication commits. |
| `retry.reason`, `.observed_at_ms`, `.not_before_ms`, `.retry_after_ms`, `.window_ids` | `Receipt.retry_at`, limits/imported cooldown, `park_provider` reason and run retry time | Preserve absolute deadlines and map known reasons; relative delay/observation fields require transport instrumentation. Do not fabricate an original Retry-After value from an absolute deadline. Park until the maximum applicable lower bound; waking reevaluates authorization rather than resubmitting. |
| `recovery.attempt_id`, `.request_sha256`, `.provider_job_id` | `attempts.id`, `.request_hash`, `.job_id`; `Receipt.job_id` | Look up existing work; never allocate another original submission because a job is still unknown or pending. Preserve job IDs/attempt correlations through restart. |
| `recovery.status`, `.observed_at_ms` | No independent provider-job status/timestamp | Record genuine status observations, otherwise unknown. Transport completed/rejected is not proof of job succeeded/failed. Retain terminal observations and identity; no `send` fallback. |

## Staged adapter/API work

No existing shared method changes in I2. Keep `RunServices::provider(ProviderOperation) -> ProviderResult` disabled as it is today; no wrapper may delegate to it as a compatibility fallback.

The server follow-up should first implement these **private**, offline-testable boundaries (proposed signatures, not available APIs):

```rust,ignore
fn bind_provider_proposal(run: &str, epoch: u64, proposal: ProviderOperation)
    -> Result<ProviderOperationV2, ServiceError>;
async fn execute_issued_provider(operation: ProviderOperationV2)
    -> Result<ProviderResultV2, ServiceError>;
async fn recover_existing_provider(operation: ProviderOperationV2)
    -> Result<ProviderResultV2, ServiceError>;
```

Binding resolves registry/allowlists and persisted lineage, atomically reserves all applicable windows, then issues the authoritative descriptor. Executing compares that descriptor with the store before its one permitted send. Recovery requires `recovery: Some`, identifies the existing attempt/job and never invokes original submission. A provider status-query may consume a separately reserved request; it must not release or replace the original uncertain charge. Legacy uncertain attempts lacking enough metadata remain held/unbound until explicitly reconciled; no automatic v2 upgrade.

After these private boundaries and their gates are proven, send Agent 1 a concrete proposal for a separately versioned public method/issuance boundary. Do not change StageInput or the public trait opportunistically. A future method must default to unavailable for old implementors and must not fall back to legacy dispatch. Schema support is not public adapter availability.

## Required server proofs and remaining gates

- Test real reservation transactions across overlapping provider/project/model minute/day windows, including parallel attempts, free-money request exhaustion, independent money/tokens/repair exhaustion and cumulative unknown/inherited charges. Roll back every window on failed reservation.
- Prove cancel/new run/credential slot/restart/elapsed retry deadline cannot repurchase an unresolved request. Distinguish a known safe transport retry from a creative repair, and charge each repair ID once. Persist lineage and descriptor conflicts.
- Prove late results record truthful charges/job/receipt bytes after cancellation but cannot mutate stage/library/acceptance/catalog state. Retain original attempt identity.
- Prove exact retry timestamps and conservative reservations survive restart/migration, existing-job recovery never calls original `send`, and missing job/usage/status stays unknown.
- Prove fixture/replay reject before credentials, and live rejects without actual inherited-accounting reconciliation, explicit product authorization and every required adapter gate. Use fake transports only for this follow-up.

I2's schema cases deliberately accept truthful exhausted descriptions; those tests do **not** prove broker enforcement. Simultaneous-window enforcement, lineage/repair persistence, registry/issuance, raw receipt and job adapters remain server-owned. Actual accounting reconciliation, live authorization, unresolved book/source/acceptance bindings, production verification and activation remain pending with their owners. Live dispatch stays disabled and I2 makes zero provider calls.
