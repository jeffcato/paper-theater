# I15: staged provider-v2 service API

Services `pt-services-v1.5.0` add provider service version 2. Playback stays `pt-contracts-v1.0.0`; provider metadata remains version 2. Existing provider/helper/candidate methods and records are unchanged. This publishes contracts and test doubles only; the server's supplied R13 private broker has **no public adapter yet**. Production construction, credentials, real transports and live dispatch remain disabled. Zero provider calls.

## Published methods

Rust exports these records from `pt_contracts::*` and adds default methods to `RunServices`:

```rust,ignore
async fn issue_provider_v2(&self, proposal: ProviderProposalV2)
    -> Result<ProviderOperationV2, ServiceError>;
async fn execute_provider_v2(&self, issued: &ProviderOperationV2)
    -> Result<ProviderResultV2, ServiceError>;
async fn recover_provider_v2(&self, request: ProviderRecoveryRequestV2)
    -> Result<ProviderRecoveryResultV2, ServiceError>;
```

All three defaults return `ServiceError::ProviderV2Unavailable` immediately, without validation, cancellation callbacks, artifact/credential/accounting access or legacy calls. Existing trait implementors need no new methods. The error enum gains this typed unavailable variant; consumers exhaustively matching `ServiceError` must handle it.

TypeScript mirrors the methods as optional on `RunServices`, preserving existing structural implementors. `providerServicesV2(services)` returns `ProviderServicesV2` with all three methods bound to the supplied service. Each absent method rejects with `ProviderV2Unavailable`, code `provider_v2_unavailable`; implemented methods retain their receiver and propagate errors. No missing or failing method falls back to `provider`, another v2 operation or a helper.

## Proposal and issuance

`ProviderProposalV2` has exactly `schema_version: 2`, `kind`, `provider`, `model`, `recipe_sha256`, `payload`, `inputs`, `estimate` and `timeout_ms`. Provider/model are registered selections proposed for checking, not credential or project authority. The request hash is derived from `payload.file.sha256`; runtime must verify the **original exact bytes**, including whitespace. Payload is bounded to 1 MiB, input inventory to 64, record to 64 KiB and timeout to 1..300000 ms. Zero money still requires a positive request estimate. Safe integer and existing artifact rules apply. Estimates do not grant spending permission.

Caller run/epoch/unit/attempt, explicit request hash, context/project/credential, logical operation, repair/retry, quota windows, mode, limits and authorization fields are rejected. Runtime gets its own trusted run/epoch/unit from the bound service instance and project/model/recipe/kind plus logical/repair/retry lineage from persisted registry/unit grants. Issuance returns the existing `ProviderOperationV2` with `recovery: None`, immutable runtime identities and every applicable quota window.

`ProviderObserverV2` describes `schema_version: 2`, `run_id`, `epoch`, `unit_id`. Runtime produces this description from the service binding; stage callers cannot supply it to a service method. Rust `issued.validate_for_proposal(&proposal, &observer)` and TS `validateProviderIssuedOperationV2(issued, proposal, observer)` check exact proposed content and service-scope consistency. The expected observer must come from trusted runtime state. These functions do not authenticate issuance or enforce quotas.

A serialized descriptor is **not an executable capability**. Execution must compare the whole typed descriptor with immutable persisted issuance, bind it to this service's active run/epoch/unit, reject recovery descriptors, recheck current cancellation/authorization/window/payload state and atomically send at most once. Completed cached results retain identity; uncertain submissions cannot be resubmitted. Waiting deadlines do not authorize a send. Known-rejection retries use runtime-selected persisted predecessor lineage; the public proposal has no retry override or reset. No transport-retry issuance helper is exposed to generation.

## Existing-job recovery

`ProviderRecoveryRequestV2` contains exactly `schema_version: 2` and `original: ProviderOperationV2`. The original descriptor must include its existing-job observation, tied to the original attempt/request. Treat the entire request as an untrusted selector: load immutable issuance and the latest genuine job observation; reject changes to descriptor, attempt/request/job or stale/conflicting status. Missing existing-job identity cannot trigger submission. The current service supplies a separately authorized active observer run/epoch/unit; it may differ from the original work.

`ProviderRecoveryResultV2` contains:

| Field | Meaning |
| --- | --- |
| `schema_version` | 2 |
| `observer` | Runtime-bound `ProviderObserverV2` for this status query |
| `query_id` | Durable runtime-issued query identity, distinct from the original attempt |
| `original` | `ProviderResultV2`, retaining original run/epoch/logical/repair/retry/attempt/request/context and job identity |
| `query_completion` | Status RPC completion, separate from the underlying job completion |
| `query_reserved_usage` | Runtime status-query reservation, including at least one request and zero creative repairs |
| `query_accounted_usage` | Separate conservative persisted query charges; never replaces original charges |
| `query_observed_usage` | Optional genuine per-metric observations; missing/null remains unknown |
| `query_started_at_ms`, `query_recorded_at_ms` | Actual status-query start and durable observation-recording time |

Rust `receipt.validate_for_request(&request, &expected_observer)` and TS `validateProviderRecoveryResultV2(receipt, request, expectedObserver)` check exact observer/original bindings, chronology, job continuity and separate conservative accounting. Unknown or ambiguous query metrics retain reservations; overages remain charged, aggregate tokens do not invent a split, and zero-cost status RPCs still consume requests. Successful status transport may leave job status/usage unknown. Original validation retains inherited uncertainty and retry timestamps.

Recovery invokes the status-query transport only. Cached completed queries add no request; unresolved queries cannot be repurchased under the same identity. An obsolete original result stays attached to its original work: observing it cannot create current stage artifacts/completion, acceptance or publication. Runtime stage acceptance must reject obsolete result artifacts rather than relabeling them with observer identity. There is no observer-issued permission, raw credential material, new provider HTTP endpoint or publication path here.

## Exact server adapter handoff

Implement these methods in the server-owned execution-bound `RunServices` implementation in a separate increment. Consume the R13 private broker without exposing its `Binding`, `Window`, transports, constructors or database objects publicly.

| Shared boundary | R13 private boundary and required adaptation |
| --- | --- |
| `issue_provider_v2(proposal)` | Validate proposal, resolve the bound service run/epoch/unit and immutable registry/unit grant. R13 `bind_provider_proposal(run, epoch, runtime_unit, &ProviderOperation)` currently accepts a legacy-shaped private projection. Prefer a private adapter accepting `ProviderProposalV2`; otherwise construct that projection inside runtime with runtime unit, derived request hash and runtime-controlled temporary attempt field which the broker replaces. Never accept caller attempt/unit authority and never call the disabled legacy service. Select `retry_provider_proposal` only from a persisted authorized known-rejection predecessor; preserve repair/retry charges. Return and validate the issued v2 descriptor. |
| `execute_provider_v2(&issued)` | Before R13 `execute_issued_provider`, compare the descriptor's run/epoch/unit to this service binding as well as the broker's immutable original. That private function currently gates the descriptor's identity; the public adapter must add the service-instance binding. Recheck mode, cancellation, grant, applicable windows/cooldowns and exact CAS payload/inputs. Refuse fixture/replay before credential lookup or reservation. Keep real transport construction unavailable. |
| `recover_provider_v2(request)` | Gate the active observer from the service instance; resolve existing original via `recovery_descriptor` and check the request against immutable issuance/latest job. Call `recover_existing_provider(observer_run, observer_epoch, observer_unit, &original, status_transport)` only. No original-send fallback. Preserve its separate transaction/query reservation and original result identity. |
| Recovery wrapper | R13 currently returns only the original `ProviderResultV2`. Add a private, persisted query-receipt adapter to return the wrapper above: query `id`, observer scope, reserved/accounted/observed usage, start and recording time, query completion and original result. The current `provider_v2_queries` row stores run/epoch but not the full observer unit/recorded timestamp; persist or recover them from immutable bound state without guessing. Cached queries must return their original receipt, never fresh timestamps or synthetic observed zeroes. Do not derive query completion from original job completion or elapsed time from reservation time. |
| Cancellation/late receipt | Preserve original charges, raw receipt history, job/retry observations and accounting under the original identity; no obsolete stage/library/acceptance/catalog writes. Recheck before returning stage-consumable results. |
| Errors | Missing capability is `ProviderV2Unavailable`; offline mode is `OfflineDispatch`; stale scope is `Cancelled`; registry/issuance conflicts fail authorization; exhausted limits remain limits. Keep private typed retry/wait evidence instead of parsing diagnostic text into authority. Never retry or route through legacy on any error. |

Required server tests: exercise the actual new methods with injected offline transports; old implementors unavailable; cross-unit/run/epoch forged issuance; original-byte changes; fixture/replay before poisoned credentials; exact cached execution; current/late canceled results; latest-job mismatch; separate observer authorization/query charges/cached receipts; unknown usage and retained original charges; no resubmission after ambiguous status. Preserve R13 transaction/window/repair/legacy reconciliation tests. R13's reported private tests are source context, not CI evidence for this unimplemented public adapter.

Generation may use the proposal type after the runtime adapter is independently verified. Until then the real execution implementation inherits unavailable defaults. Do not migrate active generation/server modules as part of I15.

## Verification and remaining gates

Contract-first red logs establish absent methods/types, absent TS defaults, and an actually accepted query/original identity collision; the minimal identity check makes the latter fail closed. Rust-produced proposal/issued/observer/recovery records exercise strict shape/authority rejection, scope/content/request conflicts, existing-job identity, cross-run observers, unsafe integers and conservative query usage. The service double exercises the signature flow, offline refusal before its fake credential lookup, exact persisted-descriptor matching and query-only recovery; it is not a production broker implementation. The TS adapter tests missing/partial capabilities, bound receiver and error propagation without legacy fallback. Native/packed strict consumers exercise the exported package.

Exact source-independent checks, code head, CI jobs/steps and source pins are recorded in `provider-service-ready.json` and `pr-ci-results.json`. Prior v1.4 acceptance receipts stay bound to their original checker/source identities; changing service/lib source changes the compiled acceptance checker fingerprint, so runtime must pin the current `acceptance_binding_checker()` rather than reuse an old report.

Remaining gates: actual runtime public adapter and persisted query wrapper; book/source/evidence binding; actual inherited accounting reconciliation; production registry/credentials/live authorization/transports; all 34 missing independent acceptance verifiers; accepted generation, target-device evidence and catalog activation. `production-verifier-pending` remains. Historical I13 recovery findings and failed R12 Studio reruns are separate owner tasks, not closed by these contracts.
