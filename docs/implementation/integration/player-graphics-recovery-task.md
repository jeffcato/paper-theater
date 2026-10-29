# P16: recover safely from WebGL context loss

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

Prepared October 26, 2026. **Reopened after [I13 review](i13-player-recovery-findings.md): pending Play can resume after loss, and automatic restore rejection can be unhandled.** Original matrix at `c62d5d5` passes its covered cases; both edge defects need red-first fixes. I13 reconciliation/CI is complete with those defects open. Owner: `player`. Work in `packages/player/**`, player-owned fixtures/checks/docs and audience code only where a concrete consumer connection is needed. Preserve local P15 commit `fa361fe` and completed media/rendering/read-along behavior. Use zero provider calls and current pinned contracts/profile. No demo-computer dependency for this headless functional increment.

## Observed gap

P15 proves transport races and ten clean teardown cycles. The current `createThreeSink`/`mountPlayer` exposes no explicit graphics-lost/restored readiness boundary. Construction failure reports `webgl_unavailable`, but an already-running context can fail later. P6 therefore remains open for context loss/reload recovery.

## Required increment

1. Handle graphics context loss on the actual shared sink/mount path. Pause media promptly, retain the observed media sample and user pause intent, stop invalid frame/capture delivery and expose an actionable graphics error/retry state. Distinguish graphics suspension from a user pause; do not lower the INTERMISSION sign merely because graphics failed.
2. Rebuild or restore the owned graphics resources from the same verified package/figures, then reconstruct imagery and plaque from the retained sample through the existing pure evaluator. Keep rendering/capture unavailable until the replacement surface is actually ready. Recovery must neither grant acceptance nor substitute unverified/missing artwork. Require an explicit Play action after recovery; handle rejected playback without bypassing transport intent.
3. Prove rewind during recovery cannot restore later scene contributors. Reject stale texture/load/recovery completions after a newer seek, package replacement or disposal. Restart and leaving the theater must remain usable while recovery is pending. If automatic restoration is unavailable, retain a clear bounded reload/retry path rather than an apparently playing blank surface.
4. Retain one media clock, one active render delivery loop and explicit sink/consumer cleanup. Dispose replaced geometries/materials/textures/render targets and context listeners; preserve P15's owned-resource baseline. Keep any new shared-player API additive and document integration instructions for server R12 without editing studio-owned code or frozen public wire records.

## Browser proof

Use the real mounted readiness-v2 package, figure textures, plaque and audio clock in headless Chrome. Trigger actual loss/restoration with `WEBGL_lose_context`; if the extension is unavailable, record that branch as unverified rather than claiming recovery from a synthetic DOM event alone.

- Exercise loss while playing and while user-paused, including an opening sample, narrative blackout and a later revealed scene. Assert media stops, no successful pixel capture while lost, no stale state delivery and an actionable retry state.
- Restore/reload and compare sample, scene, contributor identities, plaque and narrative/house pixel regions with a fresh mount at the same sample. During blackout require the dark aperture/lit house split; acceptance stays false for this diagnostic package.
- Seek backward while recovery is pending, restart, replace the mount and dispose before restoration. Verify latest intent and no late contributors/callbacks or accidental autoplay. Prove rejected post-recovery Play remains retryable.
- Run at least three loss/recovery cycles plus final teardown with listener/RAF/DOM/URL/GL resource accounting. Preserve normal chapter, staging, theater, plaque, inventory-v2 and P15 lifecycle regressions; rerun affected checks and record exact commands/results.

Save graphics-recovery receipts and representative screenshots with exact fixture/manifest/profile/player/browser identities, expected errors and unsupported branches. Return owned paths, results, consumer API/handoff and remaining limitations. Use implementation-first delivery with focused verification and standing Git permissions; root/shared changes go to integration as concrete requests.

This completes the stated P6 graphics-recovery increment only. Public-type adoption P1, remaining browser/reduced-motion/fullscreen checks, strict geometry/profile acceptance and actual demo-device p95/memory gates remain separate. Production release loading/publication stays gated.
