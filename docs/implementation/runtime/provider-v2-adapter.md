# R13: private offline provider-v2 adapter

Implemented against `pt-contracts-v1.0.0` / `pt-services-v1.3.0`, provider metadata version 2. **Live dispatch remains disabled.** No public service method or HTTP provider endpoint was added; the legacy provider method remains disabled and is never a fallback. No credentials or actual inherited ledgers were loaded. All transport/authorization/accounting fixtures are synthetic; real provider calls: **zero**.

## Boundary and persistence

The private broker resolves provider/project/model, opaque credential context, operation kind and recipe digest from immutable runtime registry and unit grants. Issuance receives the trusted run/epoch/unit separately; a proposal cannot select another authorized unit. Caller attempt IDs are replaced with persisted runtime IDs. Execution accepts only the exact issued descriptor and sends once with verified original payload bytes, including their original whitespace. Payload/input refs are checked against the issuing run and CAS hashes.

SQLite immediate transactions enforce every applicable provider/project/model window together, including overlapping minute/day windows. Canonical scope keys come from the public tagged-tuple API and exclude credential slots. Money, requests, tokens and repairs are independent limits. Run limits and the lifetime authorization cap include retained consumption; failed reservations roll back all affected records. Every status RPC reserves at least one request even when money is zero. Missing, changed or expired applicable limits block execution of an old reservation.

Logical operations, predecessor attempts and repair identities persist separately. Only a known rejection permits a transport retry with unchanged request, context, recipe and repair lineage. Each creative repair ID is consumed once, including across transport retries, reopen and new runs. Reusing it for another operation or exceeding repair capacity fails. Unresolved matching work blocks repurchase across cancellation, epoch/run changes, elapsed waits and credential slots.

Legacy tables, attempt IDs and receipts remain intact. Explicit complete model-scope aliases are required for nonzero inherited consumption and old limits; no scope is guessed. Applicable old limits become retained v2 windows, while overlapping imports/native charges and maximum cooldowns are included. Unknown legacy work is not silently upgraded. Startup marks interrupted v2 submissions and status queries uncertain without clearing charges, lineage, jobs, receipts or timestamps. Interrupted reservations lacking enough recovery evidence require reconciliation.

## Results and recovery

Unknown metrics retain their reservations; ambiguous completion retains at least the reservation even if an observation reports zero. Known aggregate tokens do not fabricate an input/output split or legacy Usage record. Overages remain charged and invalidate the active original epoch. Dispatch start, response observation, storage time and monotonic transport duration are recorded separately; relative retry delays use observation time. Retry deadlines park the run, survive reopen and do not themselves authorize another send.

Existing-job recovery is a separate transport method with its own durable query reservation. It never invokes original submission. Query usage is separate from original usage: a successful status RPC with missing job state or usage keeps those observations unknown. Job IDs remain bound to the original attempt/request; terminal observations cannot be replaced by conflicting status. Cached completed queries do not consume another request; uncertain queries cannot be repurchased under the same identity.

Actual raw receipt bytes are hash-addressed, fsynced and referenced by immutable private receipt-history records. Payloads/raw receipts are capped at 1 MiB; public metadata stays within its 64 KiB envelope and journal summaries use bounded window IDs. Late receipts retain original identity, bytes, hashes, job/retry observations and charges. They create no stage artifacts/completion, library binding, acceptance or catalog activation. Raw forensic references have a private verified reader and are not automatically published as stage artifacts. Malformed/conflicting receipts fail closed with reservations held.

## Verification

- `cargo test -p pt-production --locked --offline runtime::provider_v2_tests` — **21 pass**.
- `cargo test -p pt-contracts -p pt-production -p pt-server --locked --offline` — **120 pass**: 15 contract tests, 93 production tests, four candidate-boundary tests and eight server tests. Includes R11 WAV integrity/bounds/ranges/recovery and existing real-helper/cancellation/replay checks. An earlier broad run hit concurrent unfinished acceptance-verifier work; the final complete run above passes.
- `npm run check:contracts` — 14 playback and 80 provider-metadata cases agree across Rust/TypeScript; packed exports and strict consumer checks pass.
- Studio typecheck/cursor test, studio build and server build pass. Source and built-product identity scans and whitespace checks pass.

Executable proofs include parallel distinct attempts against four applicable windows; independent quota exhaustion; rollback; lifetime money across runs; explicit legacy migration; exact retry boundaries; repair deduplication; forged descriptors/units; corrupt payloads; expired/new limits; timeout/restart; fixture/replay rejection before poisoned credential-context lookup; default broker and HTTP live rejection; and late overage after cancellation with an unchanged canceled-stage snapshot.

Fake counters asserted by the recovery test: historical seed submission **1**, recovery-only transport submissions **0**, distinct status queries **2** (cached repeat adds **0**). Combined request accounting is **3** while the original unknown money reservation remains **10**. Missing-status recovery separately asserts **0** sends / **1** query. Matching issued dispatch asserts **1** send despite duplicate/forged execution attempts; forbidden-mode tests assert **0** sends.

## Public-method proposal and remaining gates

Send integration a separately versioned `ProviderProposalV2` containing kind, registered provider/model selection, recipe digest, payload/inputs, estimate and timeout, without caller attempt/context/run/epoch/repair authority. Proposed `RunServices` additions are issuance, issued execution and existing-attempt recovery, each defaulting to unavailable for old implementors, with **no legacy fallback**. Service instances supply their own trusted run/epoch/unit; runtime grants supply logical/repair/retry lineage. Recovery must bind a separately authorized observer unit, preserve the original result identity and reject obsolete result artifacts at stage acceptance. Private broker structures must not become public contracts.

This proposal is not implemented as a public API. Production construction is unconditionally unavailable; the synthetic-enablement constructor exists only in tests. Actual registry/credential resolution, ledger reconciliation, live authorization, real transports, remaining book/source/evidence bindings and accepted publication remain gated. Explicit window configuration/reconciliation is required; no automatic quota or uncertain-attempt reset exists.

Owned changes: runtime `provider_v2.rs`, `provider_v2/{legacy,recovery}.rs`, `provider_v2_tests.rs`, `mod.rs`, `schema.sql`, `store.rs`; server `src/tests.rs`; runtime documentation. No shared contracts, manifests/lockfiles, generation/player code or studio implementation were changed for R13. Agent 1 retains staging/commit/push ownership.

## R12 remains open

P17 now has a verified player title receipt and moves the card into the house pass. The **unchanged** complete studio check was rerun with `npm run check:preview --workspace @paper-theater/studio`; it still exits 1, now on `Diagnostic preview unavailable: Error: Nonpositive viewport` during seek to sample 115200. No assertion was weakened and R12 was not closed. [Rerun record](p17-studio-rerun.json) records the source hash, error and current artifact audit; prior R12 screenshots/verification are historical where the failed rerun replaced a capture.
