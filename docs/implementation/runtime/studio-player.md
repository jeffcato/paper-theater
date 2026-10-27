Latest P17 rerun: the strict harness is unchanged, but the complete check now fails with `Nonpositive viewport`. R12 remains open. [Rerun record](p17-studio-rerun.json). The title-blocker observations and capture hashes below describe the earlier run; replaced screenshots are identified in the rerun record.

# R12 studio consumer handoff

The studio consumer is implemented. **R12 remains incomplete:** the shared player's house pass draws closed curtains over its narrative title card. The strict Chrome check deliberately fails that title-pixel assertion. Fixing player-owned render ordering is outside this task's server-only scope; the concrete request is in [integration-requests.md](integration-requests.md).

`Preview.tsx` consumes verified package timeline, read-along, shell and registered figure records through public player exports. It passes the package's exact opening title/chapter label to `createThreeSink` and words/groups to `mountPlayer`. Missing/invalid records or unavailable bundled Noto Serif produce an unavailable state. Font readiness precedes measured layout. The 100/150/200% controls and read-along toggle use the mount/plaque APIs.

The layout reserves the profile's full three-line height, calls `plaqueGrowthSteps`, `refitShell`, `envelopeCameraPose`, `setCameraPose` and `projectPlaqueSafe`, and positions against the actual CSS surface. Reserving the height keeps framing/chunk boundaries stable on rewind. Canvas inline dimensions are explicitly responsive. Player owns font measurement, chunk packing, rendering and camera projection; studio owns overlay placement and observes surface/chunk changes.

The hidden audio element supplies the one shared clock. Play, pause, seek and restart use the mount transport. Restart also seeks the media to zero and clears future plaque context through the mount's read-along API. Load sessions own their abort controller, URLs, animation frames and observer. Replacement, close and unmount dispose the mount/sink and retain the audio reference long enough to pause/reset even a detached element. Late fetch/font/resize callbacks cannot attach or update a superseded preview. Run navigation also aborts stale package-reference lookups.

## Actual checks

- `npm run check --workspace @paper-theater/studio` — TypeScript and the event-cursor test pass.
- `npm run build --workspace @paper-theater/studio` — passes; retains the shared Three.js chunk-size advisory.
- `npm run check:preview --workspace @paper-theater/studio` — **exits 1 on the shared-player title defect**, after the other required consumer checks pass. The equivalent executed entry point was `node apps/studio/scripts/preview-browser-check.mjs` with the complete existing run/acoustic flow enabled. `--preview-only` is an optional development shortcut and is not the final receipt's command.
- `cargo test -p pt-production --locked --offline runtime::` — 32 pass, including acoustic cancellation/replay and R11 WAV boundary, integrity, ranges/recovery-related runtime cases.
- `cargo test -p pt-production --test candidate_read_contract --locked --offline` — four pass.
- `cargo test -p pt-server --locked --offline` — seven pass, including WAV full/range delivery and tight request/JSON limits.
- `cargo build -p pt-server --locked --offline` — passes.
- Source, built studio and built server identity checks pass; both staged and unstaged whitespace checks pass.

A broader production run encountered a concurrent generation test failure (`fact_matching_rejects_partial_coverage_hash_changes_and_missing_evidence`). Its subsequent targeted rerun passes after the owning lane's changes. No generation code was edited here; no final full-generation-suite result is claimed.

## Chrome evidence

[Receipt](studio-player/receipt.json) and [hash verification](studio-player/verification.json) bind the actual studio-mounted preview to the verified private package manifest, read-along digest, selected profile, browser and four bundle/font files. All 15 screenshot hashes were checked. The harness does not substitute an off-page player mount.

| Browser viewport | Text scales | Measured lines | Playback frames | Highlight mismatches |
| --- | --- | --- | --- | --- |
| 960×540 | 100/150/200% | 3 | 73 | 0 |
| 1280×720 | 100/150/200% | 3 | 73 | 0 |
| 1920×1080 | 100/150/200% | 3 | 73 | 0 |

All nine cases verify loaded font, media/sample agreement, one chunk/current highlight, line fits, projected placement, floor clearance and no surface/browser-viewport clipping. Toggle, resize, seek/rewind and restart pass. Three eligible nod poses per viewport change actual canvas pixels with a settled sign and unchanged camera. Paused dark-hold captures have a black aperture, lit house, visible INTERMISSION sign and readable plaque; the media sample stays frozen. Representative [nod](studio-player/1920x1080-nod-0.5.png) and [paused blackout](studio-player/1920x1080-blackout-paused.png).

Eight cleanup cases pass: close; queued resize after close; superseded fetch; missing track; missing font; late font completion; navigation while playing; and reopening a different saved run. They verify zero consumer URLs, observers, pending frames, canvases/chunks or surviving media source, and observed GL resource deletion. Repeated open also leaves exactly one canvas/plaque. This is consumer teardown evidence, not player P15's separate core ten-cycle/resource matrix.

The complete harness retains start/cancel/reload/reconnect, read-only replay, real acoustic helper cancellation/replay, server restart reuse, source inspection and verified audio playback. Real console errors: **zero**. Six browser notices, including intentional disconnect/restart transport failures, are recorded separately.

## Remaining title blocker

At sample zero the verified title is `Paper Theater diagnostic`, chapter label `Saved excerpt`, `title_visible=true`, curtains closed, and no future plaque text. However, all three title captures contain **zero bright title-text pixels** in the projected card region: [960×540 frame](studio-player/960x540-title.png). The current sink adds `titleCard` to `stage`, then composites the house scene containing the opaque curtains over it. The requested player fix must make the authored title visible in front of closed curtains on screen and capture. The strict check stays red until that proof exists; `receipt.complete` is false.

Zero provider calls and no credential access. Production loading rejects the diagnostic package; acceptance/publication stay false. Actual demo-device performance, final typography/art, accepted generation and catalog activation remain separate gates.

Owned files: `apps/studio/src/{Preview.tsx,main.tsx,style.css}`, `apps/studio/package.json`, `apps/studio/scripts/{browser-check.mjs,preview-browser-check.mjs,studio-player-check.mjs}`, runtime documentation and screenshots. No runtime/server implementation or player/shared-contract files were changed for R12. Agent 1 retains staging/commit/push ownership.
