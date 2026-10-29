# Reopened P16: fix the two I13 recovery races

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

Prepared October 26, 2026. **Local repair complete:** [receipt](../player/recovery-repair.json), six red-first regressions, 127/127 checks and unchanged integration probe green. Independent integration review/CI remains I14; next player task is [P18](player-geometry-task.md). Owner: `player`. P1's local type adoption and P17 title fix are implemented; preserve their uncommitted changes and evidence. Integration owns the pending P1 root lockfile update. Work only in player-owned code/tests/docs, make zero provider calls and leave all shared Git operations to integration.

Use the exact [I13 findings](i13-player-recovery-findings.md) and public-API reproducer `tools/integration/probe_i13_recovery.mjs`. The successful existing recovery matrix does not cover either reported defect.

1. **Pending Play during graphics loss:** invalidate outstanding playback intent on loss even when the clock still reports paused. Its eventual completion must not leave media playing while graphics are lost or after a later pause/restart/disposal. Preserve a standing user pause and sign pose; graphics failure must not lower the sign as a new user pause. Recovery stays paused until an explicit new Play.
2. **Unhandled automatic restore failure:** handle event-driven recovery rejection when the factory is absent, rejects or becomes stale, including when no caller joins the promise. Retain actionable lost/error/retry state and zero unhandled rejections. Explicit `retryGraphics()` must still reject to its caller when recovery fails; do not swallow all failures or claim graphics are ready.

Repair both defects and verify them with focused public-boundary regressions. Add controlled pending-clock tests and real Chrome coverage for the missing/no-factory and rejecting-factory branches, with bounded waits. Include newer intent and disposal while recovery is pending. Preserve one media clock/render loop, retained sample/plaque, no autoplay, stale texture/sink disposal and no future contributors after rewind.

Build the current player and run the integration probe with output under `/tmp`, for example `node tools/integration/probe_i13_recovery.mjs --output /tmp/pt-i13-probes.json`. It must exit zero with both defect observations resolved; report the exact red and green output. Do not edit the integration-owned probe to weaken or suppress either assertion. If its assumptions need correction, send a concrete proposal and independent reproducer to integration.

Retain 121 player checks plus new regressions, strict shared-type consumers, P17 title, lifecycle/recovery/chapter/staging/theater/plaque, audience rejection and v2 capture identity behavior. Save owned repair evidence, expected-error counts, resource/disposal counters and remaining limitations. Return exact files/commands and the probe result to integration for independent rerun and CI wiring. Root dependency/lockfile changes and studio recovery UI remain their owners' work.

Both findings must be verified fixed before P16 closes again. Broader browser/profile/demo-device acceptance and production publication remain pending.
