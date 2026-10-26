# Agent 4 prompt: shared player and audience experience

You are the Paper Theater player agent. Read [COMMON.md](COMMON.md), [INTERFACES.md](INTERFACES.md), and your ownership row in [README.md](README.md). Extract and complete one framework-independent player used by both the studio and audience app.

## Own

- `packages/player/**`
- `apps/audience/**`
- `fixtures/player/**`
- `docs/implementation/player/**`

Agent 1 owns shared contracts/fixtures, root configuration and lockfiles. Agent 2 owns `apps/studio`; supply an importable player rather than editing its UI. Agent 3 authors presentation/cues and requests actual render evidence through your capture interface; you never invent missing permissions or retime its authored narrative.

## Read-only inputs

Use transient `SOURCE_ROOT` to inspect:

- `docs/plans/2026-10-25-paper-theater-player-handoff.md`
- `docs/architecture/playback/01-experience.md` through `04-performance-timeline.md`
- `docs/architecture/no-spoilers.md`
- `docs/examples/oz_scene/theater/` — evaluator/compiler, engine, read-along and release inspector
- The adjacent theater/release/player regression tests, package build configuration and saved registration fixtures

Preserve the working evaluator/transport/registration patterns and fail-closed release checks. Historical diagnostic browser results are evidence for their actual version/profile only. Do not use the old frontend package name, cached bundles, source maps or absolute experiment paths in the product.

## Begin now

1. Inspect the current 0.170.0 player and establish a reproducible source/test/render baseline in your owned product paths using diagnostic inputs. Do not mutate the read-only experiment workspace. Capture actual runtime/types versions and keep original receipts distinct.
2. Verify availability of selected Three.js 0.186.0 and compatible TypeScript declarations against primary package/documentation sources. Propose exact dependency/lock updates to Agent 1. Migrate through the documented compatibility steps, auditing APIs, alpha/shadows/color, rendering and disposal. Replace `PCFSoftShadowMap` with the selected `PCFShadowMap`; retain WebGLRenderer and one render scheduler. Report a concrete unavailable dependency rather than silently switching versions. If Tutor is available, verify/index the exact matching source corpus; do not accept a corpus name as revision proof or make unrelated consumers wait on optional tooling.
3. While contracts are pending, isolate pure evaluation, mounting conversion and lifecycle cases in your own fixtures. Consume Agent 1's exact shared revision before exposing public package interfaces. Never fork the public schema.
4. Implement hash-bound package/media loading, bounded fetch/decode, immutable accepted handles, scoped asset ownership and explicit diagnostic loading. A diagnostic fixture must report missing acceptance IDs and be rejected by the audience production path. Replace unsupported production-capability blockers only alongside their real checks.
5. Expose the shared API from `INTERFACES.md`, including a deterministic capture interface for Agent 3. Each capture names exact manifest/candidate, profile, player/checker, sample and contributor identities. Supply neutral fixtures until new art has valid authorization and acceptance; no provider calls from the player.

## Complete the runtime

- One media element drives the delivered master. Read `currentTime` once per rendered frame; derive a clamped integer sample and use it for imagery/text. On `ended`, explicitly evaluate `sample_count`. `evaluate(compiled, sample)` reconstructs the full state without replaying history. Seek/load epochs reject stale completions; rewind removes future contributors synchronously.
- Implement all registration adapters once: scalar-to-UTF-16 source offsets, top-origin pivots to the existing bottom-origin convention, native aspect and semantic body scale, complete V3 basis/camera/support conversion. Prove orientation with asymmetric art, projection parity and support contacts. Never negate just actor Z or reuse incompatible Euler angles.
- Freeze the selected shell/camera/profile with Agent 1 after schematic validation. Preserve layered proscenium/deck/apron joins and a below-stage plaque. Use the exact geometry, camera envelope, local font, alpha/color/shadow defaults and projected DOM homography specified in the player handoff. Submit measured shell geometry/briefs to Agent 3 for eventual artwork; imported shell art remains diagnostic.
- Render narrative stage/floor/actors/shadows into an aperture-clipped target, multiply by blackout gain, then render independently lit house layers/signage and foreground text. Full blackout must cover unlit/emissive contributions too. Ineligible content never enters any pass, even at zero gain.
- Consume authored opening/scene/closing intervals from Agent 3. Required first/incoming pictures must already be eligible. Preserve exact two-second dark holds and narration-free formal transitions. Seek directly into a transition's destination phase without restarting its wait. The terminal snapshot has closed curtains within the master endpoint.
- Implement user-pause INTERMISSION on UI time with interruptible sign movement and latest-intent ownership. Distinguish user pause, buffering, readiness waits, seeking and natural ending. A failed resume preserves the sign/retry state. Reduced motion keeps semantic/timing requirements while simplifying decoration.
- Implement toggleable two-to-three-line exact read-along in the projected plaque. Load fonts before layout; keep measured groups atomic and the entire active chunk visible. No future chunks enter the DOM/accessibility tree. Reflow around the active group at resize/text-size changes without changing samples. Preserve the selected readable font and plaque growth/camera-refit rule instead of shrinking text.
- Release listeners, RAF callbacks, observers, audio/object URLs and GPU resources on navigation/disposal. Preload within resource limits; never display incomplete incoming assets or advance story time while awaiting readiness.

## Audience app

Build a thin two-ticket booth, loading/retry states, accessible controls, optional read-along, fullscreen/reduced motion, credits and return/replay navigation. Neutral title/ticket artwork reveals no later story facts. The audience loads only accepted immutable packages and contains no credentials, generation calls, studio journal or experimental preview bypass.

Until accepted packages exist, show an honest unavailable state for audience tickets. Keep runnable diagnostic demonstrations in the explicit developer/studio fixture path. Do not pass synthetic acceptance just to populate two working tickets.

## Required acceptance

Add the handoff's player checks; request Agent 1's root `check:player` alias. Groups are `contracts`, `loader`, `transport`, `shell`, `timeline`, `readalong`, `integration`, then the aggregate. These are commands to implement, not pre-existing passing suites.

Use real browser capture plus state assertions. Required cases include:

- Sample−1/sample/sample+1 reveal/transition boundaries, exact 96,000-sample dark hold, deterministic sequential versus arbitrary seek and exact terminal state.
- Missing/stale evidence, later baked-in imagery, forged flags, delayed textures after seek, rapid controls, rejected play and context loss.
- Support error ≤1e−6 world units; Rust/Three.js projected corners and DOM corners within 1 CSS pixel; full-blackout aperture channels ≤1/255 outside the defined antialias/house-overlay exceptions.
- 960×540, 1280×720 and 1920×1080 content rectangles, actual fullscreen, 100%/200% text and combined camera/pointer extremes; ≥8 CSS pixels between the panel inset and projected floor edge.
- Ten entry/exit cycles with no growing owned resource/listener/loop count. On the actual demo computer after preload/warmup, measure 60 seconds at 1×, 1280×720, DPR ≤1.5: p95 frame interval ≤33.3 ms, graphics ≤256 MiB and master ≤256 MiB. Report CPU/decoded-audio memory separately. Historical software-renderer measurements do not establish this target-device result.
- Actual opening/entries, meaningful holds and paired action captures for narrative review. Mechanical browser success cannot override missing subjects/actions or failed/uncertain creative review.

Run the identity checker on your source, built web output/source maps and any exported fixture artifacts. Return exact commands/results, contract/profile/fixture revisions, version/corpus provenance, actual browser/device details and unresolved production-acceptance IDs. Keep advancing independent player work while final art or accepted chapters are pending.
