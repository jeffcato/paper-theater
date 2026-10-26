# Paper Theater TODO and work log

Last reviewed: October 26, 2026, 11:35 MDT. Integrated baseline: `41061a6`; clean-checkout and saved-generation browser checks passed.

Deliver two complete, accepted first-chapter performances: *The Wonderful Wizard of Oz* (Gutenberg #55, Chapter 1) and *The War of the Worlds* (Gutenberg #36, Book One Chapter I, “The Eve of the War”). Generation runs autonomously through the local Rust server and studio; the static audience app uses the same player.

This is a snapshot of the four implementation lanes. `[x]` means the stated increment is implemented with the evidence described; it does not certify a production release. `[ ]` includes pending work and work in progress awaiting verification. Uncommitted changes are being made concurrently, so refresh this log at each integration handoff.

## Requirements and evidence

The build baseline is the external read-only source workspace's `docs/plans/2026-10-24-paper-theater-implementation.md`, `docs/architecture/system.md`, and owning generation/playback specifications. Consult that workspace through transient `SOURCE_ROOT`; do not persist its absolute path or introduce a build/runtime dependency on it. Current user requirements and the product ownership instructions resolve source-document drift.

Product-local references:

- [Ownership and integration gates](docs/agent-handoffs/README.md), [shared requirements](docs/agent-handoffs/COMMON.md), and [interfaces](docs/agent-handoffs/INTERFACES.md).
- [Contracts](docs/spec/contracts.md), [playback](docs/spec/playback.md), and [server](docs/spec/server.md).
- [Frozen contract/fixture handoff](docs/implementation/integration/contracts-ready.json), [verification report](docs/implementation/integration/verification.md), and [worker updates](docs/implementation/integration/worker-updates.md).

## Current position

The contracts, offline diagnostic package, durable fixture runner, journal UI, and player component checks are implemented. The complete browser → real generation candidate → stored package → shared-player rendering slice is still open. No accepted autonomous excerpt or complete chapter is evidenced by the reviewed handoffs.

The candidate-path regression is fixed in `8ac0e5b`; the exact committed snapshot passed a clean offline checkout with SOURCE_ROOT unset. The operator requires the repository to remain public so hackathon judges can review it; the visibility decision is resolved.

| Milestone | Status | Remaining exit condition |
|---|---|---|
| M0 — Workspace and executable contracts | Implemented; clean snapshot verified | Keep all lanes on the frozen contract/fixture revision and finish the pending player/browser integration. |
| M1 — Durable runner | Fixture foundation implemented | Finish real generation wiring, helper/provider integration, accounting reconciliation, and shared-package publication recovery. |
| M2 — Thin studio | Journal flow browser-verified | Mount the shared player and inspect/replay the actual generated candidate and its artifacts. |
| M3 — Player/package loading | Component checks pass; browser acceptance pending | Finish mounting/rendering, both app integrations, and numerical Chrome/device acceptance. |
| M4 — Production adapters | Offline source/evidence/geometry/preflight increments implemented | Complete the remaining stage adapters, independent reviews, and broker integration. |
| M5 — Autonomous excerpt | Pending | Browser Start produces an accepted immutable excerpt, final shell, and real journal without creative intervention. |
| M6 — Both full chapters | Pending | Two complete accepted packages and production journals with full prose, evidence, music, and read-along. |
| M7 — Delivery/rehearsal | Pending | Both performances and recorded studio walkthroughs work without provider availability on the delivery computer. |

## Owners

| Owner | Responsibility |
|---|---|
| Agent 1 | Shared Rust/TypeScript contracts, generic core crates, canonical fixtures, root configuration/lockfiles, integration, and Git operations. |
| Agent 2 | Runtime/storage/accounting/publication, HTTP/SSE server, and studio. |
| Agent 3 | Source, direction/research, speech/evidence, visuals, presentation/music/cues, automated review, and candidate assembly. |
| Agent 4 | Shared player, rendering profile/capture, audience app, and browser/device playback evidence. |
| Project manager / operator | Track dependencies, settle operational inputs, coordinate milestone acceptance and delivery. |

Only Agent 1 stages/commits or changes branches in this shared working directory. Shared-schema and lockfile changes go through Agent 1; workers preserve other lanes' edits.

## Completed increments

### Agent 1 — Contracts and integration

- [x] Bootstrap the self-contained Rust/TypeScript workspace, four lane handoffs, repository rules, and product identity checker.
- [x] Copy selected generic core crates with neutral hash-bound reuse provenance: dramatic script, monotonic alignment, timed captions, and atomic journal.
- [x] Publish `pt-contracts-v1.0.0` / `pt-services-v1.0.0`, including `StageExecutor` / `RunServices` and explicit TypeScript mirrors.
- [x] Build the portable `fixtures/shared/two-scene-v1` diagnostic package with saved media, a reused figure, authored transitions, and 176 measured words / 170 highlight groups.
- [x] Publish the fixture digest and provenance in `contracts-ready.json`; preserve synthetic/imported evidence labels and audience rejection.
- [x] Exchange 14 actual Rust-serialized contract cases with TypeScript: two valid diagnostic cases and 12 invalid cases with matching failure codes.
- [x] Add sample/span, hash, reveal/attestation, registration, read-along, supported-cue, and exact 96,000-sample dark-hold checks. Repair the misplaced fixture nod before the first contract freeze.
- [x] Install root dependencies/aliases and exact Three.js / declaration pins at `0.186.0`; run the reported workspace tests, builds, and identity scans.
- [x] Integrate the offline foundation in commit `cb122e7`, following the two earlier player commits.

### Agent 2 — Runtime, server, and studio

- [x] Implement SQLite run/stage/unit/event/budget/library storage, content-addressed artifacts, one active run, and the shared execution-service boundary.
- [x] Implement fixture checkpoints/restart reuse, corrupt-artifact rejection, cancellation epochs, idempotency, snapshot/journal/SSE cursors, and read-only recorded replay.
- [x] Add counted fake-transport cases for reservation recovery, uncertain attempts, late charges after cancellation, and independent money/request quota handling. Live transport through `RunServices` remains pending.
- [x] Implement loopback HTTP endpoints, mutation origin checks, and artifact allowlisting, with server checks.
- [x] Build the thin studio's start/cancel/history, chronological journal, artifact inspector, and recorded replay views.
- [x] Record a Chrome journal check covering start/cancel/reload/reconnect/replay: 39 unique contiguous events and zero replay POSTs. This check did not mount the shared player.

### Agent 3 — Generation

- [x] Freeze cached Oz and Wells chapter bodies with raw/clean hashes, exact chapter selection, and Unicode scalar provenance; add source-boundary regressions.
- [x] Extract offline response/evidence, source, registration, and visual-preflight helpers with saved fixtures and regression cases.
- [x] Preserve the 176-word / 170-group acoustic replay and `point-following-same-boundary-v1`; diagnostic replay grants no reveal permissions.
- [x] Add conservative fact/turn matching and failures for missing coverage, repeated/ambiguous terminals, mismatched evidence, and endpoint disagreement beyond 0.75 seconds.
- [x] Add preflight cases for missing subjects, unready entrances, future/unrelated references, duplicate ownership, impossible/omitted actions, and incomplete reviews; allow purposeful stillness and multiple pictures.
- [x] Implement shared source-stage validation and saved-payload diagnostic `PackageCandidate` assembly. Explicit missing findings remain attached; unimplemented production stages return `NeedsRepair`.

### Agent 4 — Player and audience

- [x] Record the original player baseline and selected `0.186.0` dependency provenance without relabeling historical receipts.
- [x] Implement bounded hash-verified diagnostic/production loaders and media checks; the saved diagnostic fixture is rejected by the production loading path.
- [x] Implement deterministic evaluation, reveal filtering, transport/seek epochs, terminal state, pause-sign state, registration adapters, and measured read-along component logic.
- [x] Implement schematic shell/camera/profile checks, plaque layout/homography helpers, and stage/house blackout composition primitives.
- [x] Pass the reported 43 player tests; probe the shared fixture's hashes, media, timeline, transition, and unchanged read-along groups.
- [x] Supply the [shell geometry brief](docs/implementation/player/shell-brief.md) and deterministic state-capture metadata. Actual rendered pixel capture remains pending.
- [x] Build the audience booth/routes, unavailable/retry state, source credits, and control shell; report five catalog-rule tests, typecheck, and build passing. Accepted-package playback controls are not connected yet.

## Remaining work

### Agent 1 — Shared boundaries and integration

- [ ] **I1 — Verify the committed baseline:** integrate Agent 3's candidate-path rejection fix (**G12**), reproduce appropriate workspace checks in a clean checkout, and record exact commit/revisions/results. The current clean-checkout report fails that regression; earlier working-directory results do not close it.
- [ ] **I2 — Finish service metadata:** resolve R2-002 / R2-005 quota scope, operation/repair/retry identity, structured retry timing, provider-job recovery, and book/source/acceptance bindings across Rust and TypeScript.
- [ ] **I3 — Support new PNG candidates:** add optional dimensions to `CandidateWrite` in both languages and propagate through runtime writes; preserve existing media bytes.
- [ ] **I4 — Complete independent production verification:** bind exact inventories, source/fact/media evidence, full required review coverage, recipes, profile, and player/checker versions. Replace `production-verifier-pending` only when its checks actually execute.
- [ ] **I5 — Close the diagnostic integration gate:** verify browser-started generation, durable candidate/journal storage, shared-player mount/capture, restart/cancel/replay, zero provider dispatch, and continued audience rejection together.
- [ ] **I6 — Reconcile handoff status:** retire resolved requests for module/dependency setup, contract publication, dependency locks, and the fixture nod. Confirm newer source-seeding/candidate-storage work before marking those requests closed; keep profile browser acceptance pending.
- [x] **I7 — Public repository confirmed:** on October 26, 2026, the operator required `jeffcato/paper-theater` to remain public so hackathon judges can review it. Repository instructions and handoffs now reflect this decision; visibility is no longer a blocker.

### Agent 2 — Runtime, server, and studio

- [ ] **R1 — Finish real diagnostic candidate storage:** verify the in-progress source/saved-artifact seeding and `NeedsRepair` candidate persistence. A new runtime integration test is present; publish its result and ensure failed/cancelled runs cannot publish.
- [ ] **R2 — Wire the server execution path:** the reviewed executable still starts `FixtureExecutor`. Select and supply the real generation executor/inputs for the saved-media slice without claiming that synthetic stage JSON is generated media.
- [ ] **R3 — Register the offline helper:** connect the bounded `tools/generation/helper.py` recipe to `RunServices::helper`; verify allowlisting, input/output/media limits, deadline, and cancellation.
- [ ] **R4 — Integrate provider dispatch:** route every model/research/transcription/review/helper operation through the single durable broker. Reconcile inherited accounting, quotas, reservations, uncertain jobs, and repair limits before enabling live mode.
- [ ] **R5 — Finish shared publication/recovery:** consume the frozen `PackageCandidate` and acceptance verifier, validate frozen bytes, recover interrupted exports/activation, and update catalog last. Existing publication scaffolding still needs shared-schema integration and boundary evidence.
- [ ] **R6 — Mount diagnostic playback in studio:** consume Agent 4's typed loader/mount API, replace provisional policy placeholders, and show the actual candidate with missing-evidence status.
- [ ] **R7 — Complete the real journal walkthrough:** expose source → actual decision → image/mask/audio/package output, using the same views for live and saved events. Verify full candidate replay changes no accounting/catalog state and dispatches nothing.

### Agent 3 — Autonomous generation and reviews

- [ ] **G1 — Direction/reference research:** connect source-backed cast/beat coverage and director-selected scenes/pictures; persist initial bounded research, exactly one canonical image per applicable subject type, and a separate book art profile. Resume reuses compatible bindings.
- [ ] **G2 — Voice pilot:** persist narrator/character identities and scene acting; audition the selected Flash TTS model first for new final recordings. Review adjacent takes, source-supported speaker handoff, safe joins, and scored passages before bulk synthesis; preserve existing accepted recording identities.
- [ ] **G3 — Speech/acoustic production:** wire synthesis, primary transcription and independent whole-interval checks through the broker; retain original prose, measured points, complete per-fact coverage, hashes, and conservative later endpoints. Resolve ambiguous saved terminals or leave them ineligible.
- [ ] **G4 — Enforce preflight in the stage pipeline:** make the offline source/readiness/reference/ownership/action checks block dependent artwork. Use measured draft layout, placeholders, and one consolidated bounded planning revision.
- [ ] **G5 — Visual production/registration:** connect scoped Image/SAM adapters, actual reference bytes, reviewed masks, semantic body scale, pivots/supports, and canonical sprite reuse. Reconcile actual geometry/readability against the validated player profile.
- [ ] **G6 — Presentation/master mapping:** compile eligible first/incoming pictures, introductory narration behind closed curtains, narration-free transitions, supported cues, and closure within one master. Preserve exact resampling/trim/gap mappings and all measured word groups.
- [ ] **G7 — Music and mix:** persist/compile `MusicBible`, integrate Clip recovery/decoding, arrange measured phrase-aware reuse, and review joins/ducking/intelligibility. Deliver one 48 kHz stereo PCM16 WAV, target -18 LUFS integrated and ≤-1 dBTP; preserve exact speech placement.
- [ ] **G8 — Choreography:** compile supported source-bound actions from accepted anchors; account for each required intention as executed or explicitly revised. Impossible required actions must fail rather than disappear.
- [ ] **G9 — Automated acceptance:** complete all five image review criteria, independent acoustic checks, full audio-window coverage, voice/mix continuity, and actual-render narrative readability. Require opening/entry, duration-labeled hold, and paired action captures; failed/uncertain findings block acceptance.
- [ ] **G10 — Final shell:** use Agent 4's registered briefs to generate/review shell parts through the runtime broker; deliver measured assets and local lettering/font artifacts for the shared player.
- [ ] **G11 — Accepted excerpt:** integrate the full pipeline for a source-supported Oz excerpt with narrator/character handoff, two sets, later reveal, reused figure, score transition, immutable acceptance, and saved journal. Regressions must reject missing subjects/actions, empty entrances, duplicate depiction, poor scale, and missing opening/action review evidence.
- [ ] **G12 — Close the candidate-path regression:** verify rejection of a candidate whose returned path differs from the requested path, alongside stale bytes, wrong pins, and late cancellation. Current additional checks appear to fix the clean-checkout failure; report the exact test result and send owned changes to Agent 1 for integration.

### Agent 4 — Shared player, audience, and browser evidence

- [ ] **P1 — Consume frozen public types:** migrate provisional player records and policy identities to `@paper-theater/contracts`; make the shared-fixture probe an executable check of the exact frozen fixture.
- [ ] **P2 — Complete the public player mount:** export the typed mount/control/lifecycle API and connect the renderer, media element, resource ownership, authored shell/curtains/lighting, sign, and read-along. Pure helpers and state-only capture are not a mounted theater.
- [ ] **P3 — Connect both applications:** hand the same diagnostic mount to studio; implement accepted-release loading, play/pause/restart/seek/volume/read-along, and teardown in audience. Keep diagnostic bypasses out of audience routes.
- [ ] **P4 — Finish rendered shell/read-along:** bundle the local font, implement projected two-to-three-line text/reflow and plaque growth, preserve group atomicity, and integrate the reviewed final shell artwork.
- [ ] **P5 — Produce actual render captures:** implement browser pixel/frame output bound to candidate/manifest/profile/player/checker/sample/contributor identities for Agent 3's automated reviews.
- [ ] **P6 — Verify Chrome behavior:** play/pause/seek/rewind/end, sample−1/sample/sample+1 boundaries, complete lighting reset, exact blackout hold, backward-seek spoiler removal, delayed loads, rapid controls, failed resume, context loss, and reduced motion.
- [ ] **P7 — Verify geometry/layout numerically:** support error ≤1e−6 world units, projected DOM/Three.js corner error ≤1 CSS px, aperture blackout channels ≤1/255, and plaque clearance ≥8 CSS px. Exercise 960×540 / 1280×720 / 1920×1080, fullscreen, 100%/200% text, and camera/pointer extremes.
- [ ] **P8 — Measure the delivery computer:** record Chrome/OS/hardware, ten entry/exit cycles without growing owned resources, and 60 seconds of warmed playback at 1280×720 / DPR ≤1.5. Require p95 frame interval ≤33.3 ms, graphics ≤256 MiB, master ≤256 MiB, images ≤4096 px; report CPU/decoded-audio memory separately.
- [ ] **P9 — Validate/freeze the production profile:** coordinate browser-proven geometry/defaults with Agent 1 and registration/captures with Agent 3. The diagnostic constants are frozen; `paper-player-r186-v1` is still `selected_unverified` for production.

### Full-chapter delivery and project coordination

- [ ] **D1 — Establish live operating limits:** operator supplies the production spend ceiling and scoped request/repair limits; Agent 2 persists authorization and reconciles inherited accounting. Offline work can continue while this input is pending.
- [ ] **D2 — Confirm the presentation computer:** use the current Mac provisionally; record the actual target before final performance acceptance.
- [ ] **D3 — Produce both complete chapters:** after M5, Agent 3 generates all required prose through the same accepted pipeline; Agent 2 records journals/calls/time/repairs and publishes; Agent 1 verifies full coverage; Agent 4 enables the two accepted tickets. Check a small Wells unit before bulk work.
- [ ] **D4 — Verify identity reuse:** exercise later-chapter book-local sprite/voice lookup offline; preserve canonical identities without producing an extra chapter.
- [ ] **D5 — Rehearse actual delivery:** verify static routes, MIME/cache behavior, hashes, bounded load/retry, complete playback, resize/fullscreen, reduced motion, graphics recovery, and repeated visits for both packages. Keep a local rehearsal copy and saved studio journals; prove playback/walkthroughs need no provider availability.
- [ ] **D6 — Prepare submission materials:** confirm current event rules/reuse requirements and submission timing, practice a short recording/export, freeze the demonstrated release, and prepare the finished theater plus source → decisions → artifacts → playback walkthrough. Public exports use an explicit allowlist; external deployment is a separate action.

## Next integration order

1. Agents 1 and 3 close **G12/I1** and confirm the current contracts; Agent 4 finishes **P1/P2** while Agents 2–3 verify **R1/R2**.
2. Agents 2 and 4 connect studio mounting (**R6**); Agent 1 closes the complete zero-dispatch diagnostic slice (**I5**).
3. Continue independent adapter/helper/broker work (**R3/R4**, **G1–G10**) and numerical player acceptance (**P5–P9**). Production dispatch depends on **D1**.
4. Finish the shared verifier/publication path (**I4/R5**) and accepted excerpt (**G11**) before scaling to both chapters (**D3**).
5. Complete delivery/rehearsal and recording (**D5/D6**).

The build plan's planning targets are the integrated fixture on October 26–27, autonomous excerpt on October 28, both chapters on October 29–31, release checks/freeze on November 1, recording/submission preparation on November 2–3, and recovery buffer on November 4. These are planning targets, not verified event deadlines or completion claims.

## Verification record

These results are reported in the linked integration/player handoffs; this TODO review did not rerun application suites. Later uncommitted changes require their own results.

| Check | Last recorded result | Evidence / limit |
|---|---|---|
| `cargo test --workspace --offline` | Earlier working snapshot passed; later clean-checkout regression failed | [Integration verification](docs/implementation/integration/verification.md) and [newer worker update](docs/implementation/integration/worker-updates.md); **G12/I1** remain open. |
| `npm run check:contracts` | Pass, 14 records | Same report; two valid and 12 invalid serialized cases. |
| `npm run check:player` | Pass, 43 tests | [Player checks](docs/implementation/player/checks.md); component/state evidence, browser/device gates pending. |
| `npm run build` | Pass | Integration report; both apps and both packages build. |
| `python3 tools/check_product_identity.py .` plus app/server artifact scans | Pass | Integration report; preserve the gate on subsequent changes/exports. |
| `node apps/studio/scripts/browser-check.mjs` | Pass | Integration report; journal lifecycle/replay only, 39 events, no mounted stage. |
| Audience catalog tests / typecheck / build | Pass; five catalog tests | Player checks; unavailable-state smoke, accepted playback pending. |

Generation regression sources are under `tools/generation/test_*.py` and `crates/pt-production/src/generation/tests.rs`; runtime/server cases are under their owned `src/tests.rs`. Record exact command/results when completing remaining items, including the new diagnostic runtime integration test.

## Work log

| Date | Completed / observed | Follow-up |
|---|---|---|
| 2026-10-26 | `d2b73ab`: repository bootstrap and four implementation handoffs. | Grow bounded runnable slices under the ownership rules. |
| 2026-10-26 | `43edaa7`: shared player components, audience booth, fixtures, and documentation. | Actual player mount/rendering and browser/device acceptance remain. |
| 2026-10-26 | `6fc6285`: recorded shared two-scene fixture probe. | Convert the probe to a maintained executable shared-contract check. |
| 2026-10-26 | `cb122e7`: shared contracts and integrated offline diagnostic foundation. | Clean-checkout verification and the complete mounted diagnostic slice. |
| 2026-10-26 | Later clean-checkout report exposed accepted substituted candidate paths; Agent 3's working changes appear to address the defect. Agent 1 reported the remote visibility discrepancy. | Candidate validation tracked under **G12/I1**; the operator resolved visibility under **I7**. |
| 2026-10-26 | PM compared build requirements, four lane records, tests/source, and current Git state; created this log. Additional source/acoustic and runtime candidate-storage changes were in progress. | Keep those changes pending until their owners publish verification and Agent 1 integrates them. |
| 2026-10-26 | Operator confirmed the repository must remain public for hackathon judge review. Updated repository documentation, agent instructions, handoffs, and **I7**. | Preserve public visibility; the visibility blocker is closed. |

The TODO document itself passed the product identity check and a local-link/whitespace check when created. No application tests or provider calls were run for this documentation task.

When updating this log, retain the work history, mark only the demonstrated increment complete, link its evidence/commit, and state what remains. Schematics, diagnostic media, cached capability probes, and successful builds do not establish autonomous production acceptance.
