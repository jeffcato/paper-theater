# Player and audience goal

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

**Objective:** complete the Required implementation checklist for the single shared player and accepted-package audience workflow. Follow [RUNBOOK](RUNBOOK.md). Assign this list only to the current exclusive `packages/player/**` / `apps/audience/**` owner. Suggested model: Sol medium; high for media/render/recovery races.

Own player/audience code/manifests/tests, player fixtures/docs and this file. Do not edit Studio, runtime, shared schemas or root lockfiles. Adopt the active redesign and P1/P4/P12–P18 proofs without overwriting them. Consult [the role requirements](../04-player-and-audience.md) and current exact profile/capture interfaces. No provider calls or second player.

## Required implementation

- [ ] **PLY-01 — Freeze and expose a stable functional player/UI handoff.** Reconcile the current redesign, shared-type migration, title/recovery fixes and registered rendering. Identify exact build/profile/dependency/ownership boundaries; send integration the narrow lockfile/profile requests. Provide Studio a stable load/mount/resize/dispose example immediately. Preserve current work; an overlapping redesign writer requires an explicit handoff before edits.
- [ ] **PLY-02 — Complete real media and shell binding.** Consume registered figures/scenery/required house art with exact bytes, dimensions, native aspect, pivots, mirroring/support and profile compatibility. Reject unavailable/invalid assets before playback, enforce bounded preloading/resident resources and dispose owned assets. Keep narrative/house domains and blackout over every narrative pass; interim diagnostic geometry does not become accepted final artwork.
- [ ] **PLY-03 — Join production acceptance and package loading.** Adopt integration's genuine shared acceptance/export/catalog boundary. Verify complete immutable bytes, required target evidence/profile/player identity, explicit source/sample maps and actual WAV/PNG bounds before minting a usable accepted handle. Reject forged flags, stale/missing reports, synthetic production media and unknown actions; retain the separate explicit diagnostic path for Studio only.
- [ ] **PLY-04 — Finalize the shared browser/capture seam.** Fix actual player-owned lifecycle/projection/render bugs that block Studio or generation, and return exact app-owned repros to Studio. Mount only positive content dimensions or expose actionable readiness; maintain one media playhead/scheduler and exact contributor/state/pixel/profile identities in captures. Supply the source-bound opening/entry/hold/action captures generation requests. Do not edit Studio's app/assertions or retime authored content.
- [ ] **PLY-05 — Close remaining behavioral regressions.** Prove sample−1/sample/sample+1 reveal/transition boundaries, arbitrary backward spoiler removal, exact dark/terminal phases and full lighting reset. Preserve measured group/read-along font/reflow/100–200% text, geometry/support/clearance, title/curtains/sign and reduced motion. Retain rapid control/rejected play/context recovery and ten-cycle teardown. New bugs get meaningful regression coverage; unchanged matrices need not rerun every edit.
- [ ] **PLY-06 — Complete accepted-only audience navigation.** Load catalog and the real selected package from a stable static route with truthful loading/error/retry states, fullscreen/volume/read-along/reduced-motion controls, credits and clean return/replay. Oz/Wells tickets open only for actual accepted releases; fallback leaves Wells unavailable. No generation credentials/journal/diagnostic bypass in audience; refresh/deep links and provider-offline playback work.
- [ ] **PLY-07 — Provide and run profile/device measurement tooling.** Produce a reproducible harness for three content rectangles, fullscreen, 100/200% text, pointer/camera extremes, ten entry/exit cycles and 60-second warmed actual-Mac timing/resource capture. Bind hardware/OS/Chrome/player/profile/media and distinguish graphics accounting from CPU/decoded-audio use. Enforce selected p95 ≤33.3 ms, graphics/master ≤256 MiB and texture ≤4096 bounds. Run available real-device proof; unavailable physical access stays an explicit operational gate, not a fabricated result.
- [ ] **PLY-08 — Deliver the frozen player and rehearsal evidence.** Run relevant unit/browser/build/identity checks, record exact source/profile/assets and known limits, and return integration the stable explicit-path handoff. Coordinate final acceptance/loading with INT-06 and viewport/R12 with Studio. Freeze feature/media work for the delivery plan; report whether actual selected chapter playback/device gates have really been exercised.

## Checks and dependencies

Use `npm run check:player` and relevant current player scripts (`check:geometry`, `check:theater`, `check:inventory`, `check:lifecycle`, `check:recovery`, `check:title`, `check:chapter`, `check:plaque`) via the player workspace as applicable, audience checks/build and identity scans on actual outputs. Read current package scripts before invoking; record actual results. INT-02/06 supplies contracts/acceptance; generation supplies approved media/capture inventory; Studio owns its mount layout. Use diagnostic mechanics cases while actual accepted assets are absent; never turn those into a public release.

## Operational validation gates

Actual demo-Mac access, reviewed registered final media and a genuinely accepted published chapter may be external. Finish their loader/harness/integration code and run every available proof; report absent media/device gates separately. A headless SwiftShader run is not the actual-Mac p95 result. Operator owns external release/submission decisions.

## Evidence and status

| Item | Owned change / handoff | Exact check and outcome | Source/contract identity / limitation |
|---|---|---|---|
| — | Not started under this goal | — | Preserve active UI/player work and old receipts |

## Dependency requests

| To | Exact missing input/action and reproducer | State |
|---|---|---|
| — | None recorded | — |
