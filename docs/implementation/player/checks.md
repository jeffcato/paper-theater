# Player checks (Agent 4)

`npm run check:player [-- <group>]` in `packages/player`. Each run rebuilds
with `tsc` and tests the freshly compiled `dist` output. Groups are commands
implemented in this lane (not pre-existing suites): `contracts`, `loader`,
`transport`, `shell`, `timeline`, `readalong`, `integration` (includes owned
fixture cases), then the aggregate with no group argument.

## Current results (October 26, 2026, this session)

Aggregate: **56 pass, 0 fail** (`contracts` 9 — 3 API surface + 6 frozen
shared revision, `loader` 3, `transport` 6, `timeline` 6, `readalong` 4,
`shell` 8, `integration` group 20 — 9 conversions/capture-identities/
lifecycle + 4 owned fixtures + 7 mount/capture).

Required-case coverage:

- Sample−1/sample/sample+1 at the swap boundary; exact 96,000-sample dark
  hold; negative 95,999-hold fixture fails and is not repaired; terminal
  snapshot has closed curtains at `sample_count`.
- Missing/stale evidence stays ineligible (`eligibleContributors`); the
  diagnostic fixture reports missing acceptance IDs and is rejected by
  `loadRelease`; digest mismatch rejects before further reads; unsupported
  actions/versions rejected with typed errors.
- Registration adapters: pivot `[0.25,0.9]` → engine `[0.25,0.1]` with plane
  offset `[0.25,0.8,0]`; native aspect enforced; `F*M*F` involution for the
  +Z flip; scalar-to-UTF-16 map keeps surrogate pairs intact.
- Schematic shell joins validate; camera envelope rejects out-of-range
  authored/pointer poses; reduced motion zeroes pointer parallax; plaque
  grows in 0.25-unit steps; inset homography solves exactly; full blackout
  drives every stage channel to ≤1/255.
- Transport: one clock sampled once per frame; `ended` evaluates
  `sample_count`; seeks publish synchronously with epoch ownership; user
  pause lowers the INTERMISSION sign; failed resume preserves sign/retry;
  out-of-bounds seeks throw.
- Read-along: point-event grouping preserved; groups atomic; only the active
  chunk is visible; silence has no highlight.
- Capture: `describeCapture` names manifest/candidate, profile,
  player/checker, sample and contributor identities with a state digest.

## Browser capture (claimed, headless Chrome + SwiftShader)

`npm run check:chrome` (`scripts/chrome-capture.mjs`) renders the neutral
diagnostic fixture through the real sink and asserts on final composited
pixels: visible content (nonblack 0.044), orientation preserved (left red
2.3× right red, bright top strip present, dim bottom), full blackout at zero
gain (max channel exactly 0, covering unlit materials), diagnostic capture
stamp, and a clean console. Evidence: `chrome-capture.png` (eyeballed this
session: red/blue orientation quad with white top strip, green actor card,
deck sliver) + `chrome-capture.json` receipt. Latest run: pass.

## Authored inventory execution (claimed, same browser)

`npm run check:inventory` (`scripts/chrome-inventory.mjs`) executes all 144
authored requests from `fixtures/generation/capture/inventory.json` — 48
sample positions × 960×540, 1280×720, 1920×1080 at DPR 1 — through one mount
per viewport against the shared diagnostic fixture. Hashes are refreshed
live (inventory bytes, manifest bytes) and recorded; the manifest still
matches the frozen `04f11a89…`, so no drift notes. Every capture is stamped
diagnostic/unaccepted with a state digest (48 unique digests: state is
viewport-independent, as specified). The `nod-em` instance-targeted gesture
reads `nod` at all 9 probe points (3 samples × 3 viewports), superseding the
stale `null` expectations in the generation state probe — that probe file is
generation-owned and left untouched for its owner to refresh. Evidence:
`inventory-capture.json` (144 rows) + `inventory-{960x540,1280x720,
1920x1080}.png` (1280×720 eyeballed: oriented fixture at the nod sample).
Latest run: pass, zero failures.

## Authored diagnostic performance (P12, claimed, same browser)

`src/assets.ts` binds registered figures (native aspect, pivot-at-origin
layout, mirror allowlist) with `nodAngle`/`facingAngle` rigid-gesture math;
`createThreeSink` accepts `figures` (loaded before the sink exists — a
failed figure rejects the mount, never a mid-play frame), applies per-frame
presence/position/rotation/mirror/gesture from the evaluated actors only,
converts local matrices as F·M·F, and releases every owned texture, material
and mesh on dispose. A genuine pivot-sign bug found by the new tests
(`planeOffsetDirect` general branch) is fixed.

`npm run check:inventory -- --fixture
fixtures/generation/two-scene-readiness-v2 --inventory
fixtures/player/capture-readiness-v2/inventory.json --tag v2` (fresh derived
inventory generated with the task's exact generator command, zero provider
calls, pins the current player build): 144/144 pass, manifest bytes match
the published `2070bb5f…`, player/checker identities verified against the
fresh bindings with no drift notes. Nod evidence: `em-2` gestures `nod` at
all 9 probe points; image hashes prove change-and-return (before/start/end/
after identical, mid-nod different). `inventory-v2-960x540.png` eyeballed:
the saved figure stands on the deck between the calibration quads.
Evidence: `inventory-capture-v2.json` (144 rows with image hashes and
contributor IDs) + 3 PNGs. Existing `check:player` 65/65 and the legacy v1
inventory evidence are untouched.

## Still pending (require the demo computer/device, not claimed here)

Rust/Three.js vs DOM corner parity (≤1 CSS px), support contacts (≤1e−6),
960×540 / 1280×720 / 1920×1080 + actual fullscreen + 100%/200% text +
camera/pointer extremes, ten entry/exit cycles with resource accounting,
60-second p95 frame interval ≤33.3 ms with graphics/master ≤256 MiB on the
demo computer, and opening/entry/action captures for narrative review. The
SwiftShader run above proves the pipeline, not target-device performance.

Audience: `npm test` (5 catalog-rule tests), `tsc --noEmit`, and `vite build`
pass; the built bundle smoke-serves `index.html` (200) with catalog 404, so
the booth shows the honest unavailable state with retry.

## 2026-10-26 — mounted read-along plaque (player + audience)

The model existed but `setReadAlong` only updated transport state. The
plaque is now mounted: `PlaqueView` (`packages/player/src/plaque.ts`)
renders the active two-to-three-line chunk (only that chunk in the DOM/a11y
tree), highlights the measured group for the current media sample with
`point-following-same-boundary-v1` atomicity, and supports toggle (hide +
clear), seek/rewind (same-chunk fast path, reflow otherwise), container
resize reflow (samples untouched), and 100%/150%/200% text (22/33/44 px
serif, clamped). Pure helpers `clampScale`, `charsPerLine`, `reflowChunks`
are unit-tested (4/4, now wired into `check:player` integration).

`npm run check:player`: 69/69 pass. `npm run check:plaque` verifies the
real `PlaqueView` in headless Chrome against the real 176-word
`fixtures/shared/two-scene-v1/read-along.json` track: one container child,
single `aria-current`, first-sample highlight, mid/late seeks, rewind
returns earlier chunk text, toggle clears and restores, 640→320 px reflow
preserves the active group, 200% computes to 44 px, silence clears the
highlight while retaining the chunk, zero console errors. The run caught
and fixed two harness-side reporting bugs (dropped `chunkText` field,
undefined expected-group lookup); the plaque implementation itself
rendered correctly. Evidence `plaque.png` / `plaque-200.png` eyeballed:
mid-track chunk with "trap" highlighted at 100% and reflowed at 200%.
Evidence: `plaque.json` (per-step DOM snapshots).

Audience Theater binds the plaque through a `usePlaque` hook (container +
read-along toggle + 100/150/200% selector + plaque styles). No accepted
package exists yet, so the hook stays dormant in production — no track or
samples flow — until a loaded performance supplies them. Audience
`npm test` 5/5, `tsc --noEmit`, and `vite build` pass with the binding.

## 2026-10-26 — P4 read-along integration (mount-owned plaque + theater)

`mountPlayer` takes a backward-compatible `MountOptions.readAlong` track
input (`packages/player/src/mount.ts:MountReadAlong`): the grouping is
verified up front (invalid tracks throw before any DOM exists), every
delivered media sample feeds the mount-owned `PlaqueView`,
`setReadAlong` controls it, and dispose releases its DOM/listeners/
observers. `mount.plaque` exposes the view (null without a track).

`packages/player/fonts/` bundles Noto Serif latin 400 woff2 + OFL 1.1 +
`noto-serif.css` (static registry/CDN fetches, zero provider calls, shas
in `fonts/README.md`). `PlaqueView.fontReady` resolves true only when the
face loads and covers the track text; the theater harness fails missing
readiness instead of certifying fallback metrics. Certified line fits use
loaded-font canvas measurement (`layoutMeasured`, pure + unit-tested) with
the rendered per-group CSS overhead folded into packing — the
character-count proxy remains only for provisional pre-font layout. Below-
stage placement comes from `projectPlaqueSafe` (pure pinhole projection,
verified against the three.js oracle to <1e-6 px).

`npm run check:player`: 79/79 pass (new: 4 measured-layout, 3 projection,
3 mount plaque-wiring). `npm run check:theater` passes at 960x540,
1280x720, 1920x1080 against readiness-v2 with the real `<audio>` clock
(28–84 playback frames per viewport, zero sample/group mismatches),
play/pause/seek/restart/rewind/silence/toggle, overlay tracking the
projection within 2 px, 3 measured lines at 100% and 200%, 28–52 px floor
clearance (≥8 required), full-track font coverage, per-viewport frame
capture + nod-em evidence, and clean disposal with zero console errors.
The run caught a real packing defect (unpadded canvas widths spilled a
4th CSS line); the probe-measured overhead fixed it. Growth fires
honestly: 960x540/1280x720 at 200% extend past the frame (76/40 px
overflow recorded in `theater.json`) — plaque extension per profile is
implemented, the companion camera refit stays pending. Evidence
eyeballed: `theater-*.png` show the stage with the 3-line "trap" chunk
below it; `-200.png` at 960 documents the overflow. P12 retained:
`check:inventory --tag v2` still 144/144 (binding drift note preserved).
Audience `npm test` 5/5 + `tsc` + `vite build` pass; the hook comment now
points loaded performances at the mount-owned plaque.

## 2026-10-26 — companion camera refit for plaque growth (P4)

The reported 76/40 px overflow at 200% text is eliminated. `refitShell`
(`packages/player/src/shell.ts`) frames the shell expanded downward by the
growth steps plus the full downward envelope margin under the same
10%-reserve rule; `envelopeCameraPose` takes an optional shell (default
unchanged); `FrameSink.setCameraPose` moves the three.js stage camera live.
TDD red→green: 5 new shell tests, including the pure proof that the grown
inset stays in-frame at the refit pose through all camera/pointer extremes
at all three viewports.

`npm run check:player`: 84/84 pass. `npm run check:theater` now refits per
scale, then runs a 4-pose extreme matrix (max target/pointer/dolly combos
plus reduced motion) per scale: zero overflow everywhere (worst case
960x540 at 200% holds a 19 px bottom margin, was +76 over), 3 measured
lines at 22/44 px on every pose, 22–50 px floor clearance, highlight
agreement on all 24 extreme poses, and the overlay tracking each
projection within 2 px. New retention: the evaluator scan finds the
zero-gain sample 1752000 and the composite pipeline renders it at max
channel 0 (dark hold black); capture/nod checks unchanged. Rapid
programmatic resizes trip Chrome's benign ResizeObserver-loop notice
(2–6 per viewport, counted in `theater.json`, layout proven settled by the
post-resize measurements); zero real console errors. Evidence: 9 eyeballed
PNGs (`theater-{WxH}[-200][-extreme].png`) — the 960-200 shot shows the
full 3-line "trap" chunk in-frame where the overflow ran off-screen
before. P12 retained: inventory v2 144/144. Camera refit is live
(`setCameraPose`); fullscreen, 10 entry/exit cycles, 60 s p95 and target-
device performance remain separate gates.

## 2026-10-26 — rendered staging shell/curtains/title/sign (P4)

The sink renders the neutral shell (back wall, columns, header, apron
facing, plaque backing), authored opening curtains, the package title card
and the pause-driven INTERMISSION sign. `curtainPanelX` / `signBoardY`
are unit-tested red→green (4 tests); mount forwards the transport sign
clock to `sink.setSign` (1 wiring test). The sign is the first
independently-lit house element: it renders after the composite quad on
capture and screen, surviving blackout gain per the house-stays-lit rule.

`npm run check:player`: 89/89 pass. `npm run check:staging` verifies
actual Chrome frames at 1280x720 with the real `<audio>` clock: opening
title over closed curtains, mid-travel curtains both ways, open scene,
blackout (gain 0, max channel 0), end state, plus pause → mid-lowering →
settled → resume → restart with screenshot-hash relations proving the
motion (lowering/travel visible, settled/restarted deterministic, resume
and restart remove the sign). The run caught three real defects: a back
wall occluding the players (V3-away depth negated), a stalled media clock
(harness servers now answer Range/206), and a vacuous resume that never
raised the sign (harnesses use transport-owned play/pause). Evidence:
14 eyeballed PNGs (`staging-*.png`) + `staging.json` with per-stop state
and audio-clock readings. P12 retained: inventory v2 144/144 (hashes
refreshed for the new architecture; blackout holds), theater + plaque
checks green. Zero provider calls; full device acceptance still pending.

## 2026-10-26 — independent house pass with black aperture (P4)

Permanent house geometry (columns, header, apron facing, plaque backing,
opening curtains — motion still media-driven) moved from the stage target
into the independent house pass with the INTERMISSION sign; only the
narrative aperture (back wall, title card, fixture, figures) composites
under the blackout gain. The inventory records stats rather than blackout
bounds, so P12 holds with refreshed hashes.

`npm run check:player`: 89/89 pass (no new pure surface; the move is
browser-only). `npm run check:staging` now asserts aperture-scoped ROIs
from WebGL readback (linear units): blackout opening exactly 0 with house
bright at every stop, curtains/title/curtain-travel unchanged.
`npm run check:theater` adds the paused blackout at all three viewports:
play/pause inside the dark hold (sample 1763264, gain 0), aperture 0,
house ≈0.17, sign text ≈0.84, plaque text visible in 3 lines with
clearance, overlay in frame — evidence `theater-*-blackout-paused.png`
shows the black aperture, lit house, lowered sign and readable chunk.
Resolved along the way: a 20-minute hang from an unguarded play promise
(harness media waits now race a timeout and recover via synchronous
restart), and an over-strict plaque assertion (in-gap samples show the
containing chunk unhighlighted; only past-end silence retains).
`check:inventory --tag v2` 144/144, audience 5/5, identity clean. Zero
provider calls; full device acceptance still pending.
