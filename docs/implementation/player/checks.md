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
