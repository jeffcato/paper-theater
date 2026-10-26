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
