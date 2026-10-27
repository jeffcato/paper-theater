# R13: offline provider-v2 broker adapter

Prepared October 26, 2026. **R13 complete:** [handoff](../runtime/provider-v2-adapter.md), 120 Rust tests including 21 broker cases reported. Next is [R14 Presentation runtime routing](backend-next-tasks.md); public service API is integration I15. Owner: `server`. Runtime/server code, owned tests/fixtures and runtime documentation only. Consume the verified `pt-services-v1.3.0` provider-metadata-v2 records from [readiness](provider-metadata-ready.json) and the exact [server handoff](provider-metadata-server-handoff.md). Code `14821da` has successful push/pull-request CI; the public v2 dispatch method is still unavailable. Keep real dispatch disabled, load no credentials and make zero provider calls.

## Scope

Implement the handoff's **private** issuance/execution/recovery boundary with synthetic registry/accounting records and injected fake transports. Do not expose private broker structures, add a public RunServices method, wrap the disabled legacy provider method or infer production authorization from validated schemas. Any public API proposal goes to integration after the private boundary is proven.

1. Resolve provider/project/model/credential-context identity from runtime-owned registry bindings. Persist stable logical-operation, transport-retry and creative-repair lineage; reject conflicting issued descriptors. Caller-created attempt/context fields grant no authority.
2. Atomically reserve all applicable provider/project/model quota windows using the public canonical scope semantics. Roll back every window if any reservation fails. Credential slots must not multiply project capacity; a free-money request still reserves requests. Preserve inherited and unknown charges and consume each repair ID once across retries/restarts/new runs.
3. Execute only a matching runtime-issued descriptor through one fake send. Cancellation, a new run/epoch, an elapsed retry time or an alternate credential slot cannot repurchase unresolved work. Preserve active-window consumption and genuine retry deadlines through reopen/migration.
4. Recover an existing attempt/job through a separate fake recovery/status path that never calls original submission. Unknown job state/usage stays unknown; recovery queries that consume requests require their own reservation without releasing the original uncertain charge.
5. Persist late results against the original attempt, retaining actual receipt bytes, hashes, charges, retry observations and job identity after cancellation. Reject late stage/library/acceptance/catalog side effects. Record genuine dispatch/observation times rather than relabeling reservation time.

## Proof and handoff

Follow contract-first TDD. Test real SQLite transactions, parallel attempts and restart against overlapping minute/day windows; exhausted requests/money/tokens/repairs; zero-money requests; repair deduplication; descriptor conflicts; uncertain cross-run work; cancellation/late charges; exact retry waits; and recovery with an original-send counter of zero. Prove fixture/replay reject before credential access and externally reachable live mode remains unavailable without actual accounting/authorization. Synthetic test authorization is not product authorization.

Run affected offline runtime/server/contracts checks and identity scans. Preserve R11 WAV bounds/integrity/ranges and existing diagnostic helper/replay behavior. Report exact commands/results, migration behavior, fake send/recovery counters, owned paths and a concrete public-method proposal if needed. Agent 1 retains shared-directory staging/commit/push ownership; preserve concurrent generation/player/studio edits.

R12's [studio title blocker](../runtime/studio-player.md) remains separate and player-owned. Keep its strict Chrome assertion intact; rerun the complete studio check after a verified renderer fix before closing R12. Missing production evidence, actual ledger reconciliation, real transport authorization and publication remain gated.
