# R12: integrate the current shared player into studio

Prepared October 26, 2026. **Consumer implementation and player P17 title fix are complete; strict R12 awaits the server rerun.** See the [actual handoff](../runtime/studio-player.md). Server can proceed with [R13](server-provider-adapter-task.md) while preserving the red title assertion. Owner: `server`. Work in `apps/studio/**`, server-owned browser checks and runtime documentation; change runtime/server code only if a reproduced integration defect requires it. Preserve the completed R10/R11 work, existing contracts (`pt-contracts-v1.0.0` / `pt-services-v1.2.0`), other lanes' changes and standing Git permissions. Use zero provider calls.

## Existing boundary and observed gap

The studio preview already verifies diagnostic package bytes, binds registered figures, attaches the shared renderer and uses one audio-element clock. R11's storage/HTTP capacity is complete. Do not repeat those implementations.

`apps/studio/src/main.tsx::Preview` currently omits `MountOptions.readAlong` and `ThreeSinkOptions.title`. It does not include the bundled font, plaque controls, measured projection or camera refit that player has completed. These are consumer integration tasks, independent of generation G16/G17. Player P15 is complete in local `fa361fe`; preserve its leak/cancellation fixes and lifecycle guarantees.

Use the current code and [player mounting handoff](../player/mount.md), especially the read-along studio usage section. The staging prose includes historical render-domain descriptions; current house/aperture code and [staging receipt](../player/staging.json) govern the independent-house behavior.

## Required increment

1. Consume the already-verified package `entries.read_along` words/groups and `timeline.opening.title` / `chapter_label`. Pass the track to `mountPlayer` and the authored strings to `createThreeSink`. Keep existing registered figure bindings and one shared media clock. Do not synthesize timings, titles or missing track data; show a clear unavailable/error state when required records are invalid.
2. Load the player's bundled Noto Serif stylesheet/font through its public package export. Wait for `mount.plaque.fontReady` before certifying line fits. Add read-along toggle and 100/150/200% controls using the mount/plaque API.
3. Follow measure → plaque growth → shell refit → sink camera update → projected overlay placement on scale changes and resize. Use player exports rather than copying its evaluator, renderer, font measurement or camera mathematics. Position against the actual preview surface and CSS dimensions; keep the grown plaque in frame.
4. Route preview playback through the shared transport so the mounted story, plaque and pause sign agree. Preserve seek/restart, readable house/sign/plaque during narrative blackout and diagnostic rejection. Keep diagnostic/provenance labels truthful.
5. On run/package replacement, preview reload, navigation and unmount, dispose the mount/sink and consumer-owned observers, font/layout subscriptions, audio/image URLs and media state. Prevent a superseded asynchronous load from attaching a stale preview. This verifies consumer cleanup; player P15 has completed the separate core race/ten-cycle matrix.

## Evidence required

Drive the real studio page and its mounted preview in the server-owned Chrome harness. A separate off-page player mount is not proof of studio integration.

- Use the readiness-v2 diagnostic package and bind receipts to its verified package/manifest, player bundle, profile and browser identities.
- At 960×540, 1280×720 and 1920×1080 browser viewports, exercise 100/150/200% text, resize, toggle, playback/pause, seek/rewind and restart. Verify loaded font, actual active-group/media-sample agreement, single visible chunk/current highlight where applicable, measured line fits and no plaque clipping.
- Capture authored title/closed curtains, visible figure/nod, and paused narrative blackout with a black aperture, lit house, readable sign and plaque. Retain exact diagnostic/accepted-false status.
- Replace/reopen the preview and navigate saved runs; verify no duplicate plaque/canvas, stale callbacks or surviving consumer URLs/subscriptions. Record real console errors separately from known browser notices.
- Retain server WAV integrity/range/recovery checks, studio check/build and existing acoustic/browser checks. Run source and built-product identity checks. Keep large temporary media out of fixtures.

Return owned file list, exact commands/results, browser receipt/screenshots and remaining limitations in runtime documentation. Shared API changes go to the owning lane as concrete requests. This completes the studio consumer increment only; target-device performance, production evidence, accepted chapter generation and catalog activation remain separate gates. Agent 1 retains shared-directory staging/commit/push ownership.
