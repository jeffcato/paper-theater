# I20: coherent backend integration checkpoint

Prepared October 27, 2026. Owner: `integration`. I19/services v1.8.0 is published at `24f74e5`, CI receipt `4ffc5c1`: five independent checks and 32 missing verifiers. R17's service adapter and G23's Pilot compiler are locally implemented; R17 reports scoped passes and concurrent aggregate Pilot failures, while G23 now reports its final 103-test generation suite. Verify one exact combined backend snapshot before extending shared contracts again.

## Freeze and scope

1. At task start capture immutable source/patch pins for completed R13–R17 and G16–G23 handoffs and their dependencies. Server/generation should provide those exact owned snapshots before editing overlapping modules for R18/G24; new files/read-only planning may proceed meanwhile. This is a source handoff, not shared staging permission. Preserve mixed history and original red/green receipts.
2. Build a reviewable isolated candidate containing only complete backend code/tests/fixtures and published shared contracts. Exclude Claude's player/audience/studio-layout redesign and unfinished worker edits. Explicitly record the baseline player/UI version used; do not present its existing tests as verification of the current redesign or local player repairs.
3. Root I14 player/lockfile work remains gated on a stable UI-owner handoff. Do not run a broad root install/stage over moving UI dependencies or absorb those files into this checkpoint. If a real backend dependency needs a lock change, identify and isolate only that reviewed change.

## Review and verification

- Review actual execution-bound v2 methods, bounded helper lifecycle, private acceptance persistence, scoped private proposals and Source/Direction/Pilot request/result/replay boundaries. Validate ownership, source/recipe/version/epoch binding and privacy; no declaration grants authority.
- Run the **full scoped backend Rust aggregate** and complete generation/Python/contracts/strict/native/packed suites from that same frozen tree with source workspace access unset. Resolve the recorded aggregate Pilot failures or record a concrete unresolved blocker; a passing focused rerun is not a green aggregate. Preserve exact failed output and test any correction before green.
- Run WAV native/shared conformance and relevant real server restart/helper/replay checks against a fully built pinned binary. Retain production blocks, actual constructor/credential/live-unavailable behavior, unknown charges, all adverse findings and zero provider calls.
- Do not weaken or silently omit a failing check to achieve green. Keep the separate red R12/UI viewport gate and active UI work out of the backend-ready verdict; document every exclusion. No production/profile/voice/creative/device readiness follows.

Publish explicit-path commits and exact-head CI with backend source inventory, commands/results, excluded worktree inventory, relevant raw failure history and a clear readiness verdict. Do not relabel earlier CI or claim unpublished worker trees passed it. If a blocker belongs to another lane, return an exact owned repro and retain that failure; do not take over its implementation. Update the worker handoffs to the reviewed service/checker/matrix identities.

Completion means the stated backend snapshot is reviewed and its required aggregate passes, not that all 32 remaining production verifiers or UI/acceptance gates are complete. Real providers, spending/credentials and catalog activation remain disabled.
