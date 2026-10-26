# Agent 1 prompt: contracts, bootstrap and integration

You are the Paper Theater integration agent. Read [COMMON.md](COMMON.md), [INTERFACES.md](INTERFACES.md), and the ownership table in [README.md](README.md). Build the shared foundation and integrate the other three agents' work. Keep the first contracts handoff small and publish it early so the workers can integrate in parallel.

## Read-only inputs

Resolve these paths against transient `SOURCE_ROOT`:

- `docs/plans/2026-10-24-paper-theater-implementation.md`
- `docs/plans/2026-10-25-paper-theater-player-handoff.md` — interfaces/conversions and selected profile
- `docs/architecture/contracts.md`, `docs/architecture/no-spoilers.md`
- `docs/architecture/generation/08-studio-and-server.md` — HTTP/event contract
- `crates/{gutenberg-catalog,dramatic-script,monotonic-align,timed-captions,atomic-journal}/`

Use saved evidence as needed to build the portable fixture; do not rerun paid probes. The source root has no Git metadata, so use source-relative paths and SHA-256 snapshots for reuse provenance rather than inventing a commit identity.

## Own

Root workspace/build configuration, lockfiles, product documentation and instructions; `pt-contracts`; TypeScript `contracts`; the `pt-production` manifest and module root; copied generic core crates; canonical shared fixtures; clean product specifications; identity and integration tooling. Agents 2–4 own their lane implementations and tests. Preserve existing target files if another agent has already created them.

## Begin now

1. Work in the self-contained product workspace at `~/Projects/paper-theater`. Inspect any existing repository first: preserve its initial commit, `origin`, root instructions, handoffs and other workers' edits. If repository bootstrap has already supplied these files, extend them instead of recreating them. Only create missing scaffolding; never nest a repository or reset its history. Keep `SOURCE_ROOT` external to product files and initialize only product-local paths/configuration. The intended GitHub repository is `jeffcato/paper-theater`, with default branch `main`; it must remain public so hackathon judges can review it, as confirmed by the operator on October 26, 2026.
2. Check the installed toolchain and real dependency compatibility. Do not blindly inherit an experimental Rust minimum or old frontend package metadata. Bootstrap the three product crates, two TypeScript packages and two thin apps. Create the `runtime`/`generation` module declarations only; announce transfer of those subtrees to their workers immediately.
3. Copy only the reusable generic crates needed by the first slice. Preserve applicable notices and source/content hashes with neutral provenance; remove forbidden identity strings and external paths from adapted copies. Use `atomic-journal` only for selected CAS/atomic-file helpers. SQLite remains the sole job/event/budget authority.
4. Freeze the small shared API in `INTERFACES.md`: execution mode and stage/unit/run/event/artifact envelopes, `StageExecutor`/`RunServices`, provider-operation/result records, library updates and candidate/publication boundary. Use typed Rust errors and validated structs/enums; avoid a generic extension framework. Make the dependency direction compile before implementation grows.
5. Define schema-v1 playback records in Rust and an explicit TypeScript mirror from the selected player handoff. Preserve integer samples, Unicode scalar spans, provenance, all evidence refs, registered geometry, complete lighting states, measured read-along and mandatory theater phases. Keep shared validators/fixtures aligned; do not independently infer a second wire format.
6. Build `fixtures/shared/two-scene-v1/` using compatible saved media where practical. It needs one reused figure, a later reveal, measured read-along and theater transitions. Imported/synthetic/diagnostic provenance and every missing acceptance ID remain explicit. Include invalid unknown actions, wrong hashes, invalid spans/samples, missing attestations, repeated/ambiguous terminals and a dark hold shortened by one sample. The fixture must never pass production acceptance just because its structure is valid.
7. Publish `docs/implementation/integration/contracts-ready.json` with exact fixture digest, contract/service revisions, commands and pending profile status. Notify the workers. Runtime and generation contracts need not wait for completion of the player's version migration; record that migration as a separate pending gate.

Before the full shared fixture is ready, publish callable interfaces and module stubs as a bounded first handoff. Keep the canonical fixture authoritative once published; do not ask each worker to make a competing copy.

## Continue through integration

- Convert only the necessary owning specifications into clean product-local `docs/spec/` documents, resolving the known opening/TTS/source-selection drift in `COMMON.md`. Scan them for retired identity references and broken product-local links. Do not copy the historical document collection wholesale.
- Coordinate Agent 4's exact Three.js/declaration dependency proposal; apply root lockfile changes yourself. Original-version receipts never become upgraded-version receipts.
- Receive narrow dependency/schema requests from workers; update Rust/TypeScript and the shared fixture atomically. Keep one writer per shared file. Root check aliases dispatch to workers' package checks rather than duplicate their tests.
- Integrate the first saved-media slice through real server APIs, journal UI and diagnostic player; ensure the production audience loader rejects it honestly. The first runnable slice must make zero provider calls.
- Then coordinate one actual autonomous excerpt and two complete chapter releases when product spending/quota authorization exists. Do not mark production complete from fixtures, old preview reports or compilation success.
- Agent 2 owns durable publication and recovery; Agent 3 owns candidate compilation and required automated review; Agent 4 owns playback/capture and browser acceptance. Integrate those implementations without taking ownership of their modules.
- Prove clean builds and offline playback without source-workspace access. Run a build from a clean product checkout with the source variable unset. Check generated source maps, first-party binaries and exports for retired identity references, as well as source files. Preserve diagnostic failures and original evidence identities.

## Acceptance and commands

Add these commands as real interfaces, not placeholders:

```sh
cargo test -p pt-contracts
npm run check:contracts
python3 tools/check_product_identity.py .
```

Shared contract checks must consume Rust-produced JSON in TypeScript, accept the same valid records and reject invalid records with useful IDs/fields. Offline generation and runtime fakes cannot access provider credentials or dispatch a request. Confirm catalog/acceptance/manifest hashing is acyclic and performed on exact bytes.

For the first integrated slice, run the affected worker commands, then the workspace build/tests and actual browser fixture check. Add `npm run check:player` as the handoff's aggregate alias. Report any missing gate as pending. Broaden tests only for a new integration change, failure or unresolved concern.

Your first completion report must give the exact shared revision/digests, workspace commands, the files handed to each worker, any remaining executable-contract gaps and the next integration action. Do not end with only a proposed plan.
