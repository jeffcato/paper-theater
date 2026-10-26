# I2: shared provider quota, retry and recovery metadata

Prepared October 26, 2026. Owner: `integration`. The reviewed offline baseline is code `306059f`, with CI recorded in `4884692`. This is the next bounded shared-contract increment for runtime request **R2-002**. Use zero provider calls; do not load credentials or enable live dispatch.

## Observed boundary

`crates/pt-contracts/src/services.rs::ProviderOperation` identifies an attempt, unit, provider/model, request/recipe and estimated usage. `ProviderResult` distinguishes completed, known rejection and ambiguous outcomes, but has no structured retry/job recovery metadata. The TypeScript mirror is `packages/contracts/src/services.ts`.

The private runtime broker already records project scope, operation identity, repair consumption, quota windows, retry timestamps and job IDs. Its current scope is one provider/project/model tuple; simultaneous quota-window support needs an explicit shared description and a later runtime implementation. The actual `RunServices::provider` remains disabled. See [runtime requests](../runtime/integration-requests.md), R2-002 and R2-008.

## Required increment

1. Define matching Rust/TypeScript records for non-secret project/credential-context identity, stable logical-operation identity, creative repair identity and transport retry/attempt relationships. Specify which identifiers are runtime-issued and which are caller proposals. Never treat caller-supplied metadata as authority to spend or retry.
2. Describe simultaneous applicable provider/project/model quota scopes and windows, with exact units, start/reset/retry timestamps and estimated/observed money, request and token usage. A zero-money request still consumes request capacity where applicable. Scope identifiers must avoid collisions; validation must bound record sizes and safe integer ranges.
3. Add structured retry timing/reason and existing provider-job recovery identity/status. Specify how unknown usage, ambiguous completion, known rejection and late results are represented. Missing usage must stay unknown; a timeout or new run must not imply permission to repurchase an uncertain attempt. Recovery identifies existing work rather than silently submitting it again.
4. Publish a versioned service addendum with explicit compatibility/migration rules. Do not silently relabel old v1.2 records or grant missing legacy metadata fabricated authorization. Preserve current offline helper/candidate methods, frozen playback contracts and canonical fixture bytes. If a public signature changes, use a staged migration and exact owner handoffs rather than editing active server/generation/player modules.
5. Provide a concrete mapping from each public field to the runtime broker's existing private records, and list runtime gaps separately, including simultaneous window enforcement. Specify any required adapter/API changes for server's follow-up; retain disabled live dispatch. Do not claim that a schema alone implements the broker adapter or reconciles actual accounting.

## Validation and handoff

Use contract-first tests. Rust-produced JSON and the TypeScript consumer must agree on valid/invalid operation/result records. Include distinct transport retries versus creative repairs; overlapping quota windows; exhausted repair/request scope; free-money request accounting; absent/invalid scope; ambiguous result with unknown usage; existing-job recovery; retained retry timestamp; conflicting attempt/request identities; unsafe integers and legacy-record behavior. Contract validation can reject malformed records; quota exhaustion, credential access and redispatch behavior must be tested at an actual implemented boundary or explicitly left as server integration work.

Run contracts tests, native/packed exports and strict TypeScript consumers, then affected offline workspace/app checks and product identity scans. Preserve both v1/v2 diagnostic readiness outcomes and the exact-head CI evidence for the earlier baseline; new code needs its own commit/CI results.

Publish a provider-metadata readiness record and worker handoff with the new service revision, exact records/semantics, compatibility examples, commands/results, files and migration instructions. Work in integration-owned contracts/config/tests/docs; preserve concurrent lane edits and standing explicit-path Git rules.

Completion closes the R2-002 contract portion of I2. Keep runtime adapter enforcement, actual inherited accounting, live authorization, any unresolved book/source/acceptance bindings, production verification and catalog activation explicitly pending with their owners. Server R12 and generation G16/G17 continue independently. Player P15 is complete in local `fa361fe`; its later integration/CI is separate from this metadata increment.
