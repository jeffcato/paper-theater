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

## 2026-10-26 — chapter-sized WAV loading (P14, matches server R11)

The loader is media-aware: only the delivery master claim
(`entries["audio"]` typed `audio/wav`) may approach the new
`MAX_AUDIO_BYTES` (256 MiB) under its own audio budget, with post-read
hash identity and `inspectWav` confirmation so the label alone never
accepts. Every other artifact — including non-master audio — stays under
the tight `MAX_FILE_BYTES` (32 MiB) per-file cap, and the shared
non-audio total stays under `MAX_TOTAL_BYTES` (256 MiB); both fire on
declared bytes before the offending ref is read. Handles stay byte-free
(`files.size === 0`) so a 34 MiB master never pins to the handle.

`npm run check:player`: 98/98 pass (9 new chapter tests red→green:
valid >32 MiB master with exact bytes/hash, exact 256 MiB ceiling,
over-ceiling/meta/asset/total rejection, garbage/truncated/wrong-hash/
wrong-size/read-failure cases). `npm run check:chapter` proves a real
180 s PCM16 stereo 48 kHz master (34,560,044 bytes, synthesized into a
temp dir — never into fixtures) in headless Chrome with the real build:
loader delivers exact bytes + matching sha with no bounds/invalid_wav
issues, WebAudio decodes 2 ch / 48 kHz / 8,640,000 frames / 180 s with
the ramp head intact, the `<audio>` element reports 180 s, seeks to
90/179 exactly and advances under playback, tampered/truncated bytes
fail identity, garbage labeled as WAV reports `invalid_wav` and blocks
`loadRelease`, and the AudioContext closes. Evidence: `chapter.json`
(31 local requests, 0 remote, 0 provider calls). Generation keeps its
own 32 MiB production cap — no full-chapter production claim. P4/P12/P13
retained: staging, theater and plaque checks green. Zero provider calls;
full device acceptance still pending.

## 2026-10-26 — browser control races and ten-cycle teardown (P15)

`npm run check:player`: 106/106 pass (7 race/stale transport tests plus a
disposed-mount silence test, red→green; one red run corrected the test,
not the code — a seek intentionally does not cancel a pending play, so the
completion still reports the live playing state with a benign hidden-repeat
raise). `npm run check:lifecycle` proves latest-intent control in headless
Chrome with the real build, real `<audio>` clock, three.js sink and
read-along plaque: pause/play/seek/restart bursts land exact samples at
opening/blackout/closing with the pause pose, play-wins reports playing
with the sign settling hidden, restart resets to 0/readiness/hidden, a
synchronously-paused play never flips reason to playing, a 404 resume
rejects (`no supported source`) with reason and sign preserved plus one
error, and pause→play→pause retargets mid-travel (lowering@0.68, never
settled, finally visible). Ten load/mount/play/seek/dispose cycles then
prove teardown: every cycle emits 14 events, freezes after dispose (null
tick, capture throws), creates GL resources (13 geometries, 7 textures)
and returns RAF/listeners/observers/DOM/object-URLs plus geometries,
textures and both render targets to baseline. Evidence: `lifecycle.json`
(237 local requests, 0 remote, 0 provider calls).

Fixed two reproduced player defects: the diagnostic fixture's two canvas
textures were never owned (`material.dispose()` frees no map) — every
cycle leaked 2 textures until the sink dispose loop claimed maps
explicitly; and `sign.cancel()` cancelled frame id 0 with no loop running
— the sign now only cancels a genuinely pending frame. Two harness-side
findings, not player defects: `play()` on a sourceless element pends
instead of rejecting (the proof uses a fast-failing 404 plus timeout
races; bare media awaits are banned), and a tick during an in-flight seek
truthfully reports `seeking` over a standing user pause (unit-pinned
reason model; the sign still follows the pause). Caller-owned cleanup is
recorded in the evidence: media element state, caller-appended DOM, no
player object URLs, and context slots via canvas removal/GC. Retained:
chapter, staging, theater, plaque, audience 5/5 and inventory v2 144/144
(executed build digest refreshed). Zero provider calls; headless evidence
only, not demo-device performance.

## 2026-10-26 — WebGL context loss and recovery (P16)

`mountPlayer` now owns an explicit graphics-loss protocol with no builder
refactor: it watches the sink canvas for context loss (preventing default
so restoration stays possible), pauses media through the non-user path —
the INTERMISSION sign never lowers merely because graphics failed, and an
already-paused reason is kept — retains the sample, blocks ticks and
captures with an actionable `graphics_lost` error plus retry state, and
rebuilds through the caller-owned `recreateSink` factory on the same
canvas (`ThreeSinkOptions.canvas`, additive). Recovery re-renders at the
retained sample and never autoplays: `play()` while lost rejects, so the
first successful post-recovery play is necessarily explicit. A graphics
sequence rejects completions overtaken by a newer seek, restart, loss or
disposal, and the aborted fresh surface is disposed.

`npm run check:player`: 115/115 pass (10 fake-canvas loss/recovery tests,
red→green). `npm run check:recovery` forces real loss/restoration with
`WEBGL_lose_context` in headless Chrome: loss while playing at blackout
pauses promptly with sample/sign/plaque frozen and restores to
pixel/state/contributor/plaque-exact equality with a fresh control
(aperture 0, house lit, acceptance still false); loss while paused with a
backward rewind mid-rebuild aborts stale and retries clean at the new
sample (no future contributors); restart stays usable during the outage;
rejected post-recovery play stays retryable; disposal before restoration
stays silent and a replacement mount verifies. Three loss cycles plus
final teardown: 7 sinks all return geometries/textures to 0 with 14
targets disposed and RAF/listeners/observers/DOM/URLs at baseline.
Evidence: `recovery.json` plus two posed screenshots (83 local requests,
0 remote, 0 provider calls).

Two harness-side findings, not player defects: `play()` on a sourceless
element pends instead of rejecting (fast-failing 404 plus timeout races;
bare media awaits stay banned), and pose assertions inside a
microtask-only race window need one UI-clock settle wait or they read the
pre-window emission. Retained: P15 lifecycle, chapter, staging, theater,
plaque, audience 5/5 and inventory v2 144/144 (executed digest refreshed).
Zero provider calls; headless evidence only, not demo-device p95/memory.

## 2026-10-26 — authored title in front of closed curtains (R12 player fix)

The sink drew the title card in the narrative scene while closed curtains
composite over it in the later house pass, so the card was covered even
with `title_visible` true (staging only ever asserted the flag, never the
pixels — the studio strict check caught it). The card now lives in the
house pass at its authored depth (in front of the 2.2 curtains), and the
render loop tints its material by the same blackout gain the composite
quad uses, so dark holds stay dark; house lighting, sign and calibration
are untouched. No public API shape changed.

`npm run check:title` (new) proves it in headless Chrome with the real
build at 960x540, 1280x720 and 1920x1080: sample 0 shows title_visible
with closed curtains and thousands of bright title-text pixels inside the
projected card rect on both the canvas readback and the capture pixels
(1207/1208, 2249/2251, 5201/5204 — identical bar to the studio check:
R,G,B > 150, more than 20), plus a blackout guard (aperture 0, house lit)
per viewport. Evidence: `title.json` plus three posed title frames (118
local requests, 0 remote, 0 provider calls). Red first: the proof harness
reported zero title pixels on both paths at all viewports pre-fix.

Two harness-side findings, not player defects: the card-rect Y range must
run top-to-bottom in CSS coords (an inverted range counts nothing), and
capture targets hold linear light — the capture side sRGB-decodes before
applying the identical presented-image bar (text reads 234,223,194 on
screen vs 210,188,138 linear in the target). Inventory v2 still passes
144/144: exactly the 8 title-window samples x 3 viewports changed hashes,
all other 120 captures byte-identical. Retained: player 115/115,
lifecycle, recovery, chapter, staging, theater, plaque, audience 5/5.
Server strict assertion and studio files untouched; no commit — staging
and commits stay with integration.

## 2026-10-26 — full-viewport 2.5D multiplane world (immersive audience)

Opt-in immersion on the one shared player. With `immersive` unset the sink
behaves as before, and the studio still uses that bounded path.

- **`src/multiplane.ts`** (pure)
  - Depth tables. Stage side, relative to the facade at z = 2.5: +2.5
    patrons, +1.8 rail, +0.8 lectern, 0 facade, −0.3 frozen curtains (2.2),
    −0.8 … −4 stage. Lobby, around z = 60: +1.5 grille, +0.8 counter, −1
    doors, −3 backdrop.
  - Contain-fit 16:9 frames. Seat frame on the facade plane, y −5.65…12.3.
    Lobby frame on the counter plane, y 0.3…7.7.
  - Side viewpoint (`SIDE_VIEW_DEGREES`, 7° house-left). At rest the
    camera stands off the centre line facing the frame centre, so cards are
    seen at a slight angle. The fit keeps the nearer, larger half inside the
    viewport. The plaque uses the inner rect of the angled lectern, and the
    rail uses its outer bounds.
  - Smoothstep aisle dolly. From the ticket window to the front row (z 30)
    the camera stays within ±1.4 of the aisle centre. It steps aside only
    after that.
  - Off-axis pointer parallax. The camera moves along its own axes, up to
    ±0.6/±0.3 in the lobby and inside the frozen ±0.8/±0.4 envelope at the
    seat. The window shift holds the frame centre, and the angled
    proscenium stays within 1 px (was exact before the side view).
    Parallax is zero under reduced motion.
  - Lectern and rail rects use the 0.25 growth step. Also: door swing and
    the DPR cap of 1.5.
- **`src/house.ts`**
  - Code-painted toy-theatre cards, house pass only: grille, counter, ticket
    stubs with paper shadows, UNDER REHEARSAL plates, inspection card, foyer
    doors, drapes, credit playbills, damask, sconces, aisle rows with paper
    patrons, carpet, facade with cartouche and side boxes, footlights, pit
    railing, lectern, foreground patrons, and the brass rail (levers plus
    seek gauge).
  - A velvet map for the frozen curtain panels. Its folds gather as the
    curtains open; the slide geometry and coverage are unchanged.
- **`src/shell.ts`**
  - `createImmersiveSink`. It hides the bounded columns, header, apron and
    plaque backing, and adds `setView` (world camera plus `setViewOffset`),
    `resize`, `pick` (raycast, with u along the gauge track), `renderIdle`,
    `lecternRect`/`railRect` and `memory()`.
  - Shared fix: title-card and INTERMISSION lettering now shrink to fit.
    Long titles were clipped before.
- **`src/immersive.ts`**: `createTheaterWorld`.
  - One `requestAnimationFrame` scheduler. Each frame ticks the attached
    mount, or renders the idle house when no mount is attached.
  - Journey on UI time, pointer hover/click picking, and a ResizeObserver
    refit.
  - Each frame the PlaqueView container is placed on the projected lectern
    inset (reflow on size change). `fitPlaque()` applies the growth and
    refit rule.
  - Teardown of listeners, the frame loop, the observer, GPU resources and
    the WebGL context.

Commands and results, on this shared machine (load average about 24 during
the later runs):

- `npm run check:player`: 163/163 pass. The new `multiplane` cases run in
  the shell group (layers, framing at 5 aspects, lectern/rail clearance
  under pointer extremes and growth 0/4/12, parallax ordering, journey,
  doors).
- `node scripts/chrome-immersive.mjs` passes at 960×540, 1280×720 and
  1920×1080 in headless Chrome/156 with SwiftShader. Evidence:
  `immersive.json` and `immersive-{lobby,hover,inspect,aisle,stage,intermission}-{W}x{H}.png`,
  plus `immersive-stage-resized-1600x1200.png`.
  - Real mouse input: hover lifts ticket B, clicking it opens the card, and
    Take Your Seat starts the dolly. The picks recorded are exactly ticket
    and seat.
  - At the seat, the readiness-v2 diagnostic performance is mounted
    through `loadDiagnostic` and is reported as diagnostic, not accepted.
    Noto Serif is certified.
  - The rail Play lever starts audio.
  - The plaque shows 3 lines, all inside the lectern inset.

    | Viewport | Floor clearance | Rail clearance | Plaque growth |
    | --- | --- | --- | --- |
    | 960×540 | 23 px | 28 px | 4 steps |
    | 1280×720 | 30 px | 36 px | 0 |
    | 1920×1080 | 45 px | 54 px | 0 |

  - A gauge click at the track centre gives u = 0.494–0.497, and the
    playhead lands at 0.495–0.498 of the master.
  - With the pointer at (1, 1), the facade plane moves 0 px. Layer shifts
    run in order from −3.6 px (patrons) to +4.5 px (backcloth) at 1280×720.
    Raycast picks at the pure projections of all 6 rail controls hit the
    right control.
  - Reduced motion gives zero shift.
  - The Pause lever lowers INTERMISSION.
  - A live resize to 1600×1200 refits: the drawing buffer is 1600×1200 and
    all 6 rail picks still hit.
  - Ten create/dispose cycles each end with 0 listener delta, 0 frames
    after dispose, 0 geometries and textures (`renderer.info`), and 0
    canvases.
- Frame intervals at 1280×720 over 10 s while playing (software GL only;
  not the target device):

  | Run | Machine load | p50 | p95 | Frames in 10 s |
  | --- | --- | --- | --- | --- |
  | First | lighter | 33.4 ms | 50.0 ms | 263 |
  | Later | load average about 24 | 116.6 ms | 149.9 ms | 88 |

  Neither run meets or disproves the 33.3 ms p95 target.
- Accounted graphics memory at 1280×720, DPR 1: 97.1 MB, under the 256 MiB
  budget. This is an allocation account from `sink.memory()`, not a driver
  readout.
  - Textures (all canvas textures and figures, RGBA8): 45.5 MB.
  - Two render targets (RGBA8 plus 32-bit depth): 14.7 MB.
  - Drawing buffer, assuming front/back plus 4× MSAA color and depth: 36.9 MB.
  - Live `renderer.info`: 72 geometries and 40 textures.
- Regression runs on the bounded sink after these changes: check:theater,
  staging, title, geometry, lifecycle, recovery, inventory, plaque and
  chapter pass. Their evidence files were refreshed by these runs.
  - `check:chrome` (`chrome-capture.mjs`) fails, and the failure predates
    this change. It builds `createThreeSink(320, 180)` without figures,
    then mounts a timeline whose actors use `card-set-a`. The sink's
    fail-closed unknown-asset check rejects that, and the check is
    unchanged here.
- Audience:
  - `npm test`: 8/8 pass. New cases: verbatim opening lines from the frozen
    source, spoiler-free synopses, honest readiness wording.
  - `npm run typecheck`: passes.
  - `node scripts/chrome-audience.mjs`: passes at all 3 viewports. Details
    are in [`../audience/README.md`](../audience/README.md).
- `npm run build` at the root (contracts, player, audience, studio) and
  `python3 tools/check_product_identity.py .` (884 files): both pass.

Pending, not claimed here:

- The immersive seat framing is a new view of the frozen stage. The camera
  is farther back, framed on the facade plane. It needs an Agent 1 profile
  revision and Agent 3 review captures in this framing before production
  acceptance.
- Graphics recovery is not wired for the immersive world (no `recreateSink`),
  so the mount reports its reload path.
- The audience mounts nothing until a catalog-entry → release-reference
  contract exists.
- Still owed: target-device 60 s p95 and memory, real assistive-technology
  testing, and DPR above 1 in the immersive harness.

## 2026-10-26 — immersive house art direction (documentation alignment)

Documentation-only alignment to [`immersive-art-brief.md`](immersive-art-brief.md);
no renderer behavior changed and no new browser result is claimed here.

- The code-painted house in `src/house.ts` above is the interim diagnostic
  rendering and the explicit fallback. The target direction is the
  brief's 35 registered blank, story-neutral bitmaps: lettering typeset
  by the player in bundled Noto Serif (never baked, OCR-rejected at
  registration), flat cards at the brief's layers, and paper shadows,
  glow halos, ticket punch, curtain gather, lever/needle motion and all
  lettering kept procedural. Story titles and credits currently painted
  into tickets/playbills in code are interim; in the target they are
  typeset over blank `lobby.ticket` / `lobby.playbill-*` fields.
- The rail target is decomposed as in the brief (`rail.plate`,
  `shared.brass-plate` control plates, a player-rotated `rail.lever`,
  `rail.gauge` with track u 0.06–0.94, sliding `rail.needle`). The
  lever drawn inside each code-painted control plate and the canvas
  seek inset are interim equivalents, not the registered-art geometry.
- Consuming registered art still needs the house-art manifest loader
  described in the brief's follow-up; it is not implemented, so the
  pending item above stands: the brief's geometry and deliver sizes
  join the requested Agent 1 profile revision (see
  [`integration-request.md`](integration-request.md) §7), and art
  registered against the brief stays diagnostic until that revision is
  frozen and the art is accepted. The brief authorizes no provider
  calls.
