# Diagnostic mounting and frame capture (Agent 4)

`src/mount.ts` binds a loaded handle to one transport, the pure evaluator
and an injected `FrameSink`. `src/shell.ts` `createThreeSink` implements the
browser sink on the profile compositing pipeline.

## Mount contract

- `mountPlayer(container, {handle, timeline, reveals?}, options)` validates
  the timeline and normalizes cues through the shared-record adapters at
  mount time: unsupported cues throw here, never mid-playback.
- One scheduler step: `tick()` samples the clock once, evaluates the full
  state without replaying history, renders to the sink and emits `onState`.
  `dispose()` stops delivery and releases transport plus sink.
- Diagnostic handles mount and render, stamped `diagnostic: true` /
  `accepted: false` on every capture. `requireAccepted(handle)` is the
  production gate: anything but a loader-minted accepted handle throws
  `Release blocked: provenance:…`. The audience production path must call
  it before capture.

## Capture contract

- `mount.captureFrame(width, height)` returns manifest/candidate/profile
  identities, provenance + acceptance stamp, sample, phase, state digest
  and the pixel buffer. Without a pixel sink it fails closed (`not_ready`);
  after dispose it fails; nonpositive bounds fail.
- `createThreeSink(w, h)` renders the narrative stage (neutral diagnostic
  fixture drawn in code: deck plane, asymmetric red/blue orientation quad
  with bright top strip, green actor card — no story facts) into a linear
  target, multiplies by the blackout gain on the GPU, composites into a
  final target and presents the same bytes to the screen (`PCFShadowMap`,
  sRGB output, no tone mapping, linear filtering without mipmaps, DPR
  ≤1.5). `capturePixels` reads the final composite target via
  `readRenderTargetPixels` — never the pre-gain stage target. Per-actor
  registered artwork binds later; unlit basic materials deliberately prove
  the gain covers unlit pixels too.
- Headless harnesses have no WebGL context: construction rejects with
  `webgl_unavailable` instead of crashing. Browser pixel acceptance
  (aperture channels ≤1/255, corner parity, support contacts) stays pending
  for the demo computer; see [checks.md](checks.md).

Covered by `npm run check:player -- integration` (`tests/mount.test.ts`,
10 tests: 3 lifecycle + 4 capture + 3 plaque wiring). The three.js API
usage typechecks against the pinned 0.186.0 declarations.

## Read-along studio usage

- Host pages include `@paper-theater/player/fonts/noto-serif.css` (bundled
  Noto Serif latin woff2 + OFL, see `packages/player/fonts/README.md`)
  before mounting. Without the face, `mount.plaque.fontReady` resolves
  `false` and every line fit is provisional — studio must surface missing
  font readiness, never certify fallback metrics.
- Pass the package track: `mountPlayer(el, {handle, timeline}, {clock,
  sink, readAlong: {words, groups, container: plaqueEl, scale}})`. Mount
  validates the grouping up front (invalid tracks throw before any DOM is
  touched), feeds every delivered media sample to the view, routes
  `setReadAlong(enabled)` to it, and disposes it with the mount.
  `mount.plaque` exposes the view (`setScale` for 100%/150%/200%,
  `fontReady`, `ready`); it is `null` without a track.
- Position the plaque container from `projectPlaqueSafe(baseCameraPose(),
  viewport)`: left/top/width track the projection (≤2 px), height covers at
  least the safe inset and extends per `plaqueGrowthSteps` when measured
  lines need more room. Line fits come from loaded-font canvas measurement
  (`layoutMeasured`), never the character-count proxy; the view folds the
  rendered per-group CSS overhead into packing so CSS wraps exactly the
  packed lines.
- Refit the camera for growth: `refitShell(growthSteps)` frames the shell
  expanded downward by the steps plus the full downward envelope margin,
  `envelopeCameraPose(offset, dolly, pointer, reducedMotion, shell)` resolves
  any pose against it (default: the base shell, so existing callers are
  unchanged), and `sink.setCameraPose(pose)` moves the stage camera live.
  Studio flow per scale/resize: measure → grow → refit → setCameraPose →
  reposition the overlay from the refit projection → re-verify. The refit
  keeps the grown plaque and every camera/pointer extreme inside the frame
  (proven pure in `tests/shell.test.ts`, verified live in `check:theater`).

## Rendered staging (shell, curtains, title, INTERMISSION sign)

- `createThreeSink(w, h, {figures, title})` builds the neutral stage
  architecture in code (shell.json is style neutral-v1 with null images):
  inner back wall, columns, header, apron facing, plaque backing, two
  opening curtains, and a title card from the package opening strings.
  Curtain panels follow `state.curtain_open_ratio` (`curtainPanelX`);
  the card shows while `state.title_visible`. All opaque unlit materials
  composite through the stage target, so the blackout gain covers them;
  the profile shell/curtain alphas stay reserved for the future
  independently-lit house pass.
- The pause-driven INTERMISSION sign is the first house element: mount
  forwards the transport UI clock (`onSign`) to `sink.setSign`, and the
  rope-hung board renders in a separate house scene after the composite
  quad on every destination (capture + screen), unlit by the blackout
  gain per the house-stays-lit rule. Board height follows `signBoardY`
  inside the pinned SIGN_BOUNDS with the transport tilt bound; progress 0
  hides the rig. Renderer uses explicit clears (`autoClear = false`)
  since the house pass shares each destination.
- Depth note: the back wall stands at negated inner depth (V3 +Z-away
  maps to three.js distance, as F*M*F maps actor positions);
  front architecture keeps three.js-near depths matching the verified
  fixture space. An earlier revision occluded the players and was caught
  by the staging frames.
- Media servers must honor HTTP Range requests or the `<audio>` clock
  stalls with currentTime frozen at 0 (all check harnesses serve 206).
  Use transport-owned `play()`/`pause()` in harnesses: raw element
  playback bypasses sign intent and once certified a resume that never
  raised the sign. Guard every media wait with a timeout race: a wedged
  element must report `media stall` instead of hanging the run, and prefer
  synchronous recovery (`restart()` cancels the sign with no round-trip).
- Blackout verification is aperture-scoped, not full-frame: the opening
  ROI (projected world (0,2,0), below the sign band) must read exactly 0
  while the column ROI (projected (-9.5,4,1.8)) stays bright. Readback is
  linear (NoColorSpace target), so thresholds are linear units: column
  gray ≈0.013, curtain red ≈0.11, sign text ≈0.84.
- Plaque gap semantics: a sample inside a chunk span but outside every
  group interval shows that chunk with no highlight (current context);
  only past-chunk-end silence retains the previous chunk. Either way text
  stays visible; a fresh view with no chunk yet stays blank.
- Covered by `npm run check:staging` (evidence `staging.json` +
  `staging-*.png`), `npm run check:theater` (paused blackout + read-along,
  evidence `theater.json` + `theater-*-blackout-paused.png`) and the mount
  sign-forwarding test.
- Covered by `npm run check:theater` (combined theater+plaque harness,
  evidence `theater.json` + `theater-{WxH}[-200].png`) and the mount plaque
  wiring tests in `tests/mount.test.ts` (stub metrics; certify nothing).
