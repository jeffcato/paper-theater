R17 execution-bound provider service: [provider-service.md](provider-service.md), [checks/pins](provider-service-ready.json), [public-method examples](provider-service-examples.json). All three methods use R13 persistence with scoped issuance, exact once-only execution and separately authorized query-only recovery. Successful construction is test-only; credentials, live dispatch and publication remain disabled. Backend only; zero actual provider calls or UI/Git operations.

R16 bounded chapter execution: [chapter-runtime.md](chapter-runtime.md), [registration/checks](chapter-runtime-ready.json), [observed receipt](chapter-runtime-receipt.json). G21 runs through its separately pinned helper and real StageExecutor with immutable original/bundle inputs, private atomic checkpoints, conservative recovery and read-only replay. All results remain NeedsRepair; publication is blocked. Backend only; zero provider calls or UI/Git operations.

R15 trusted private acceptance: [private-acceptance.md](private-acceptance.md), [readiness](private-acceptance-ready.json). Actual independent execution, consumed-byte/context rechecks and atomic private persistence are implemented. Missing registry inputs fail closed; production verification and publication remain blocked. Backend only; no UI/Git operations.

R14 durable G18 Presentation integration: [presentation-integration.md](presentation-integration.md), [receipt](presentation-runtime-ready.json). Backend only; private NeedsRepair evidence persists, replay is read-only, publication remains blocked. R12 stays open.

R13 private offline provider-v2 adapter: [provider-v2-adapter.md](provider-v2-adapter.md). R12 remains open; see the linked unchanged-check rerun.

R12 studio consumer implementation and title-rendering blocker: [studio-player.md](studio-player.md).

Latest R11/R2-010 server capacity and compatibility handoff: [master-wav-support.md](master-wav-support.md).

Current acoustic/runtime integration and combined readiness-v2/P12 completion: [acoustic-integration.md](acoustic-integration.md).

# Runtime, HTTP, studio handoff

Implemented paths: `crates/pt-production/src/runtime/{store.rs,schema.sql,runner.rs,broker.rs,helper.rs,fixture.rs,publication.rs,tests.rs,mod.rs}`, `crates/pt-server/{Cargo.toml,src}`, `apps/studio/{src,tests,scripts,package.json,tsconfig.json}`, and this documentation directory. Root configuration, shared contracts, source evidence, lockfiles and other lanes remain owned by their assigned agents.

The CLI runs one worker against a real SQLite database, with an OS file lock per data directory. Fixed stage order, unit input keys, verified cached outputs, immutable hash-addressed files, transactionally paired state/events, cancellation epochs and revision-checked library updates are implemented. Artifacts are written/fsynced before DB references. Successful unchanged units survive reopening and an actual server process kill/restart. Missing/corrupt bytes fail reuse.

The private reservation broker retains money/request/token/repair consumption, per-provider/project/model windows, learned retry timestamps and uncertain attempts across restart. Attempt identity binds the complete descriptor. Ambiguous results retain reservations and matching unresolved provider requests cannot be repurchased by starting a different run; known late charges can be recorded after cancellation without reviving stage/library/publication acceptance. Learned daily stops park the run until its actual persisted retry time. A free-money operation still reserves request capacity. Shared live dispatch remains disabled pending R2-002/R2-008. The offline acoustic helper is registered separately by exact code digest, receives a durable authorized attempt and bounded verified scratch files, and reproduces 176 measured words/170 groups with zero provider calls. Process pipes are capped, stdin is closed after its envelope, OS CPU/file/descriptor limits apply (address-space cap additionally on supported platforms; decoded PCM inputs stay size-bounded), and cancellation kills the process group.

HTTP provides books, idempotent start/cancel, consistent shared snapshots, paginated history/journal, committed-event SSE with Last-Event-ID, verified artifacts and allowlisted diagnostic package reads. Loopback Host/Origin checks and bounded bodies/files apply. Wakeups only prompt durable queries; polling covers notification races. Browser disconnect/reconnect does not own execution. Recorded replay uses reads and makes no new run, provider, budget or catalog mutation.

The studio shows persisted chronological events, recorded source/decision/artifact links, actual artifact bytes/hashes and explicit imported/synthetic/diagnostic status. It provides start/cancel, saved navigation, reconnect and recorded replay. Its player entry point uses the official loadDiagnostic/mountPlayer/createThreeSink APIs, attaches their canvas, and shares the verified audio element clock for play/pause/restart/seek. It contains no rendering/timeline implementation. No creative candidate approval controls, fabricated decisions, global progress percentage or invented elapsed work are supplied.

Two offline recipes are distinct. The default fixture runs synthetic structural tasks. Inspect saved Oz package seeds actual raw/clean/frozen chapter sources and the frozen saved package, delegates source/acoustic/package stages to Agent 3 through shared StageExecutor/RunServices, and marks intermediate tasks synthetic. The generated diagnostic candidate retains unresolved acceptance and ends in NeedsRepair/failed, with its real media and source references inspectable. It is not a completed autonomous chapter. Shared production_blockers remains closed; diagnostic/synthetic/imported/partial candidates cannot activate the audience catalog.

Publication recovery tests inject interrupted preparation, staged files and renamed directories into real temporary SQLite/filesystem fixtures. Restart invalidates stale epochs and preserves the previous catalog pointer. These prove conservative recovery, not successful accepted live activation; the production verifier and final activation adapter remain pending.

## Verification

- `cargo test -p pt-production --locked --offline runtime::` — 32 runtime tests passed.
- `cargo test -p pt-production --test candidate_read_contract --locked --offline` — 4 passed against service revision pt-services-v1.2.0 (scoped reads and PNG dimensions).
- `cargo test -p pt-server --locked --offline` — 7 passed.
- `npm run check --workspace @paper-theater/studio` — TypeScript and 1 event-cursor test passed.
- `npm run build --workspace @paper-theater/studio` — passed; Vite reports the shared Three.js chunk size advisory.
- `cargo build -p pt-server --locked --offline` — passed.
- `node apps/studio/scripts/browser-check.mjs` — isolated Chrome/local server; start, cancel, reload, reconnect, 39 unique contiguous structural events, replay with zero POSTs, process crash/restart cache reuse, diagnostic NeedsRepair, recorded source-byte inspection, and the shared player diagnostic mount, attached renderer canvas, seek, and real verified audio playback. Screenshot: studio-browser.png. Loopback/browser execution required the tool's sandbox escalation; it was approved.
- `python3 tools/check_product_identity.py .` and `python3 tools/check_product_identity.py apps/studio/dist target/debug/pt-server` — passed.

After the AGENTS.md update, observed red→green cycles covered shared executor adoption, missing/corrupt reuse, full attempt identity, ambiguous reservation retention, shared JSON, HTTP diagnostic route, browser controls, saved candidate/acceptance display, decision links, invalid requests, uncooperative cancellation, distinct outcomes and durable quota waiting, and actual offline helper dispatch (including the initial stdin-EOF failure). The initial preliminary scaffold preceded that instruction; these cycles are not a claim it was originally test-first.

## Run

From the product root: `npm run build --workspace @paper-theater/studio`, then `cargo run -p pt-server -- --data var --port 8787`. Open http://127.0.0.1:8787. CLI options `--studio` and `--fixture` select trusted product-local directories; no experiment workspace is needed at build/runtime.

Next bounded integration: complete production evidence/profile verification and the reviewed activation adapter, reconcile actual inherited accounting, and freeze shared quota/recovery metadata before any separately authorized live dispatch. R11/R2-010 server capacity is complete up to 256 MiB; generation retains a 32 MiB cap; player P14 at `5419a3f` supports validated masters up to 256 MiB. The fixture slice is implemented; full chapter acceptance is not claimed. Pending IDs and exact owner requests are in integration-requests.md. Agent 1 alone commits remaining reviewed lane changes.

Consumed contract revision pt-contracts-v1.0.0, service revision pt-services-v1.2.0, and frozen fixture digest 5a662fb9d1687fd61a692acdd4265bc12aac9d5bfacb8e5f75e58b472dbf776c.
