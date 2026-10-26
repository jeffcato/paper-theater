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
7 tests: 3 lifecycle + 4 capture). The three.js API usage typechecks
against the pinned 0.186.0 declarations.
