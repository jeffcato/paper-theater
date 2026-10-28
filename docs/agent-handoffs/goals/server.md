# Server goal

**Objective:** complete the Required implementation checklist for an ordinary, authorized, durable source-to-publication runtime. Follow [RUNBOOK](RUNBOOK.md). Owner: server. Suggested model: Sol high; medium for a fixed bounded adapter.

Own `crates/pt-production/src/runtime/**`, `crates/pt-server/**`, runtime fixtures/docs and this file. **Exclude all `apps/studio/**`**, including its tests/configuration. Review the [R19 brief](../../implementation/integration/server-speech-runtime-task.md) and [current Speech handoff](../../implementation/runtime/speech-runtime.md). R19 already has a scoped saved-flow receipt; adopt it rather than expanding another saved recipe indefinitely.

## Required implementation

- [ ] **SRV-01 — Adopt and close the current bounded seam.** Verify the R19 requests/registration/takes/PCM-copy route, exact native→batch mappings and cancellation/restart/private replay against its actual frozen source. Resolve the G25 input/receipt proposal and publish its usable private artifact mapping. Preserve R18 stop-before-Speech semantics and earlier timeout logs; do not rerun unchanged matrices without cause.
- [ ] **SRV-02 — Implement persisted production configuration and limits.** Reconcile actual inherited usage/reservations/uncertain attempts without resets. Persist explicit provider/project/model, operation, request, money and repair scopes; default absent authority to unavailable. Separate the authorized Pilot from bulk. Implement guarded ordinary construction and secret retrieval without making credentials necessary for Fixture/Replay or changing root dependencies yourself.
- [ ] **SRV-03 — Implement bounded real transports through the broker.** Support the configured Meta direction/research/review/Image/SAM/transcription and Gemini speech/primary-transcription/music/audio-review operations needed by generation. Use current verified encoders/parsers from generation and the issued exact request; preserve real raw responses, output identities and usage. Do not reuse synthetic event/provenance labels for real calls. Bound HTTP/helper execution, retrieval, output, timeout and cancellation; no model-directed arbitrary shell/network operation or independent helper retry/ledger.
- [ ] **SRV-04 — Wire actual fixed-stage inputs and commits.** Route source→direction/research→Pilot→Speech→Acoustic→preflight→visuals→presentation→music/mix→choreography→review→package through the real GenerationExecutor. Commit requests before genuine refs/issuance, recheck bytes/revision/epoch before commits and preserve per-unit content reuse. No fixture-only context seeder, fabricated ArtifactRef or generic continuation through NeedsRepair can authorize production. One active production run; no DAG engine.
- [ ] **SRV-05 — Complete real failure, recovery and replay behavior.** Keep money/request/repair reservations before dispatch and real late/unknown charges after cancel. Never resubmit an ambiguous billable attempt automatically; query an existing job only under supported separately issued recovery authority. Persist quota waits/deadlines and exact unit checkpoints. Tests prove once-only execution, cancel/rollback/restart and byte-identical replay with zero new dispatch/accounting/catalog/library effects.
- [ ] **SRV-06 — Join actual verifier execution to atomic publication.** Consume integration's current trusted policy/result, not the older envelope or serialized Pass. Recheck consumed evidence under epoch/commit boundaries, stage exact immutable export, recover interrupted activation and update catalog last. Previous accepted releases survive failures; diagnostic/synthetic/missing evidence stays unpublished. Coordinate the shared unconditional pending-block replacement with integration.
- [ ] **SRV-07 — Finish the thin Studio/audience HTTP boundary.** Provide actual idempotent starts/cancel/snapshot/journal/SSE and safe curated artifact/preview/catalog views on the agreed contracts. Snapshot N plus events after N loses no committed event; stale cursors recover. Preserve private CAS/raw/evidence HTTP rejection. Delivery previews are explicit and diagnostic; raw transcription batches remain private. Send Studio exact response/examples instead of editing its app.
- [ ] **SRV-08 — Make a real production request runnable without code edits.** Expose documented trusted startup/run configuration selecting source, pinned recipes/profile and persisted authorization. Same code supports Oz and Wells; model/source selection cannot create new authority. Run an injected full-stage sequence and real built-server process restart/export checks with no external calls. Do not leave ordinary construction permanently `None` behind a test-only success path.
- [ ] **SRV-09 — Deliver operational status and evidence.** Return the exact working launch/run/replay/export instructions, configuration schema, source/binary/recipe pins and test results. Surface genuine waiting/failed/NeedsRepair/accepted states and actionable access/limit failures. Complete all owned root/API requests through integration and preserve other agents' files. Implementation-ready is distinct from a real accepted chapter.

## Checks and dependencies

Use `cargo test --locked -p pt-production --lib runtime:: -- --test-threads=2`, `cargo test --locked -p pt-server`, relevant candidate/WAV checks, `cargo build --locked -p pt-server`, the current chapter/Direction-Pilot/Speech process probes and the identity check. Record actual scope; schedule heavy checks rather than relaxing production deadlines. INT-02 supplies shared authority/envelope changes; generation supplies actual encoders/raw decoders/candidates; INT-06 supplies publication authority. Advance independent transports/storage tests while those finalize.

## Operational validation gates

Perform the recorded small Pilot only when all actual operation/model/request/repair scopes, balances and runtime preconditions are established. A key or this list is not authorization. Bulk, additional provider spend and external deployment/submission remain separate decisions. If absent, finish all code/injected/process checks and report the exact pending gate; do not fabricate real usage or accepted media.

## Evidence and status

| Item | Owned change / handoff | Exact check and outcome | Source/contract identity / limitation |
|---|---|---|---|
| SRV-01 | In progress: durable G25 request/registration commits consume actual R19 batch/audio/copy receipt; R18/R19 original routes preserved. | Focused request registration, interrupted restart/replay, cancellation/rollback and corruption/stale-epoch checks pass; final scoped checks pending. | New source scope; historical R19 receipts retain their original pins. |

## Dependency requests

| To | Exact missing input/action and reproducer | State |
|---|---|---|
| generation / GEN-02 | Callable real provider encoders/raw decoders and genuine operation examples; see [exact requests](../../implementation/runtime/production-dependencies.md). | Pending; configuration/storage work proceeds. |
| integration / INT-02 | Approved bounded HTTPS dependency and frozen execution/config bindings; same request document. | Pending; no root dependency edits by server. |
| integration / INT-06 | Trusted executed verifier authority/current export bindings replacing the older publication envelope. | Pending; publication remains blocked. |
| operator / integration | Actual inherited accounting reconciliation and fully specified existing Pilot scopes. | Pending real validation; zero provider calls. |
