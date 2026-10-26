# Acoustic runtime integration

The saved-fixture pipeline now delegates **source, acoustic and package** to the existing GenerationExecutor through RunServices. Ordinary structural fixture runs are unchanged. The CLI configures the separate acoustic input directory; missing inputs/helper configuration fail closed.

SQLite `run_input_scopes` separates saved package/source inputs from acoustic receipts/media. Acoustic receives its exact original relative paths and no package manifest or previous synthetic stage outputs. Other stages retain saved inputs; package additionally receives the persisted acoustic reports. Existing artifact IDs deterministically restore the additive scope index without changing bytes, reservations or accounting.

The real helper produces `acoustic-replay.json`; generation verifies it through execution-scoped candidate reads and produces `acoustic-validation.json`. Both are hash-stored and committed with their original typed findings and **NeedsRepair**. Findings from different fields no longer overwrite one another when package checks reuse an ID. The original narration-domain scope and unresolved delivery/fact/fidelity/voice/rendered evidence remain explicit.

Only the pinned saved **fixture** recipe may continue from acoustic NeedsRepair to assemble an inspectable diagnostic package. Acoustic stays NeedsRepair, the final run stays failed, and publication stays closed. Failed/cancelled acoustic execution stops the pipeline. No live recipe gains this continuation.

Unchanged verified NeedsRepair checkpoints may be reused as diagnostic data; they still return NeedsRepair. Acoustic unit identity includes the registered helper digest. Restart after acoustic completion reuses both reports and the original findings with exactly one helper authorization. Corrupt artifacts still fail verification. Cancellation after a real helper return prevents report/package acceptance under the invalidated epoch. Recorded replay reads the stored journal/artifacts without helper, accounting or catalog mutation.

## Verification

- `cargo test -p pt-production --locked --offline`: 36 unit tests (24 runtime, 12 generation) and 4 candidate-contract tests passed.
- `cargo test -p pt-server --locked --offline`: 4 tests passed.
- `npm run check --workspace @paper-theater/studio`: typecheck and cursor test passed.
- `npm run build --workspace @paper-theater/studio` and `cargo build -p pt-server --locked --offline`: passed. Vite retains the shared Three.js chunk-size advisory.
- `node tools/integration/check_combined_browser.mjs --fixture fixtures/generation/two-scene-readiness-v2 --output docs/implementation/runtime/combined-acoustic-readiness-v2`: passed using an isolated Chrome profile and temporary loopback server.

Combined browser evidence covers real acoustic helper authorization, cancellation before report acceptance, persisted 176-word/170-group validation, read-only acoustic replay, structural start/cancel/reload/reconnect/process restart, source inspection, shared-player canvas and actual audio playback. Readiness-v2/P12 checks capture 11 final-composite frames, verify incoming-picture eligibility at fade start and actual nod pixel changes, and reject diagnostic production loading. Independent receipt verification matches the selected manifest, all 11 PNG hashes and three exact black final composites. See `combined-acoustic-readiness-v2/{receipt.json,verification.json,studio.png}`.

Selected input manifest: `2070bb5f88d5d68fe6156aa4df17673634fc110584ec5f7c03d82bf5bddc502b`. Contract/service: `pt-contracts-v1.0.0` / `pt-services-v1.2.0`. The immutable shared v1 fixture is unchanged. Provider calls, provider request reservations and money for the helper remain zero; credentials and actual inherited accounting were not loaded. Missing evidence still blocks publication. Target-device performance, full production evidence/activation, live authorization and larger-master envelopes remain outside this diagnostic completion.

Observed red→green cases: missing pipeline input configuration/routing; restart with missing derived scope rows; acoustic findings overwritten by package findings. Actual cancellation and replay tests exercise the real helper and persistence path. All implementation is confined to the server lane; existing generation/P12 code is reused rather than copied.

Agent 1 owns the shared Git index and final commits. Include the outstanding runtime/helper changes plus `acoustic_pipeline_tests.rs`, `fixture.rs`, `runner.rs`, `store.rs`, `schema.sql`, server CLI, studio browser harness and these receipts. No worker Git mutations occurred.
