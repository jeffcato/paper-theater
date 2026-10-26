# Agent 2 prompt: durable runtime, server and studio

You are the Paper Theater runtime agent. Read [COMMON.md](COMMON.md), [INTERFACES.md](INTERFACES.md), and your ownership row in [README.md](README.md). Implement durable execution and the thin studio around the shared generation/player boundaries.

## Own

- `crates/pt-production/src/runtime/**`
- `crates/pt-server/**`
- `apps/studio/**`
- `fixtures/runtime/**`
- `docs/implementation/runtime/**`

Agent 1 owns shared contracts, the production crate manifest/module root, reusable core crates and root lockfiles. Agent 3 supplies the generation executor and transport recipes; Agent 4 supplies the shared player and capture interface. Submit dependency/shared-schema requests to Agent 1. Do not copy player code into the studio.

## Read-only inputs

Use transient `SOURCE_ROOT` to read:

- `docs/architecture/generation/08-studio-and-server.md`
- `docs/architecture/generation/06-orchestration-and-release.md`
- `prototype/src/runner.rs`, `prototype/src/store.rs`, and the prototype HTTP entry point
- Selected `crates/atomic-journal` CAS/atomic-file helpers

Extract proven persistence, cancellation, quota and reservation patterns. Adapt code to the product's shared types and fixed stage order. Do not inherit prototype-specific visual-trial branches, hardcoded spend caps, paths or a second authoritative journal.

## Begin now

While Agent 1 freezes contracts, inspect the source implementation and prepare narrow SQLite/epoch/reservation tests in your own area. Keep preliminary types private; replace them with the published shared boundary before integration. Request only the exact missing contract fields.

First build a fixture-backed runtime and HTTP/studio slice:

1. Implement one active run with fixed stages and durable unit keys. SQLite owns requests, epochs, stage/unit results, attempts/reservations, artifact/library bindings and ordered events. Unchanged successful units survive restart and are verified before reuse.
2. Write/hash/fsync artifact bytes before database references. Commit state plus journal event atomically. SSE queries committed events after a cursor; notifications only wake readers. Do not lose events in the snapshot-to-stream race.
3. Compose Agent 3's `StageExecutor` through the shared `RunServices`. Start with a deterministic fixture executor or injected fake. Only the runtime commits results. Epoch checks guard artifact/library acceptance, stage completion and catalog activation.
4. Implement idempotent start/cancel, consistent snapshot, run history, journal and artifact APIs from `INTERFACES.md`. Browser disconnect leaves work running. Start the server on loopback with same-origin studio delivery and bounded/allowlisted artifact paths. Preserve distinct failure/wait/cancel states.
5. Build one React/Vite run list, chronological journal and artifact inspector. Link real source → recorded decision → actual artifact. Provide start/cancel, live reconnect, saved-event navigation and Agent 4's shared-player diagnostic preview. Clearly label imported/synthetic/recorded/partial data; never invent decisions, alternatives, elapsed work or global progress percentages.

## Provider dispatch and accounting

Implement the only provider/helper authorization broker. It must reserve money, requests/tokens and repair allowance durably before dispatch, with one attempt identity per operation. All models and research/review calls use it. A free token-count request can still consume request capacity.

Persist per-provider/project/model limits, cooldowns, learned daily stops and their actual retry times. Store a non-secret project/credential fingerprint where needed; never expose credentials. Separate transport retries from creative repairs. Preserve known rejections and ambiguous outcomes. Query an existing provider job where supported; an uncertain billable attempt without recovery evidence must not be repurchased automatically.

Bound helper arguments, input/output sizes, decoded media, runtime, cancellation and stderr. Only allowlisted recipes run. No arbitrary model-proposed shell command executes. Python transports receive an already authorized attempt; they cannot reserve/retry independently. Late results may record charges but cannot revive a canceled epoch.

Before any live product run, reconcile the actual inherited ledgers/reservations without resetting consumption or retry counts. Import neutral hash-bound accounting records; retain immutable original receipts outside the product if they contain forbidden identity text. Do not treat a historical experiment's scoped allowance as authorization for this product. Fixture/replay remains available without credentials or live authorization.

## Publication and recovery

Agent 3 returns a reviewed `PackageCandidate`. You validate its exact inventory, evidence/acceptance refs and current epoch, stage the immutable files, reconcile interrupted activation on restart, and update the catalog last. Do not add semantic approvals or change candidate bytes during publication. Old accepted releases survive failed export/activation. Pending/diagnostic/synthetic/failed candidates never become audience releases.

Serialize library writes or use revision checks. Cancellation invalidates pending library bindings and publication as well as stage success. Keep the server/replay interface thin; accounts, public generation hosting, distributed queues and visual editing are out of scope.

## Required checks

Use real temporary SQLite databases and a counted fake transport; do not write tests that merely mirror accessors. Cover:

- A completed unit resumes after process restart without redispatch; corrupt/stale artifact hashes fail reuse.
- A crash between reservation, dispatch and response recovery preserves uncertain charges and prevents duplicates.
- Cancel during a helper/provider call prevents late commit/publication; the underlying charge remains recorded.
- Snapshot at N plus SSE after N delivers every committed event exactly once after client deduplication, including reconnect races.
- Daily quota stops survive restart; request capacity and money are enforced independently across model calls.
- Same idempotency key/body returns the original run; conflicting body fails.
- Recorded replay issues zero provider requests and makes no budget/catalog mutation.
- Restart at publication boundaries preserves the previous accepted catalog or completes the exact verified activation safely.
- Studio uses actual persisted decisions/artifacts and the shared player; no fabricated provenance or acceptance state.

Expose targeted runtime/server checks and a studio check in your lane manifests; ask Agent 1 for root aliases/dependencies. Expected Rust entry points are `cargo test -p pt-production runtime::` and `cargo test -p pt-server`. Add meaningful browser coverage for start/cancel/reconnect/replay using fixture data. Run the product identity checker on your source and built studio output.

Return implemented paths, exact commands/results, contract revision, demonstrated persistence/dispatch guarantees, and remaining pending integration IDs. Continue independent owned work if another lane's handoff is pending.
