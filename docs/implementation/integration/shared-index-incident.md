# Shared-index incident: P16 and provider metadata

Recorded October 26, 2026. Mixed commit: `c62d5d571915195e80ee5f2dc3615cd49c1849b7`, parent `fa361fe7324e866a1f01c6ea8ec18d4a425ab1f2`.

## Verified scope

Player reported that a bare `git commit` included another lane's staged changes. Read-only Git inspection finds **32 changed files: 13 player-owned and 19 integration-owned**, rather than approximately 13 extra files. The integration portion includes provider-metadata Rust/TypeScript code, contract cases, shared fixtures and handoffs. The complete path partition and committed blob identities are in the [scope record](shared-index-incident.json).

All 32 paths exist in the current checkout. The parent P15 commit contains ten player-owned paths and no other lane's paths. The operator's decision is to preserve the combined commit. This audit made no index/history changes; the scope record is not a claim of content review or CI approval.

The working tree also has later provider-metadata/test edits and active server/generation work. Commit inclusion does not prove these later edits were included, reviewed or published. The local provider handoff names `pt-services-v1.3.0`; the previous reviewed CI remains bound to `306059f` and `pt-services-v1.2.0`. Do not transfer that CI result to the mixed commit or later changes.

## I13: coordinator recovery task

Owner: `integration`. Prioritize this before the next integration publication.

1. Compare the mixed commit with its parent and review all 19 integration-owned paths, including exact provider fixtures, API exports and readiness/compatibility claims. Compare later working edits separately. Confirm which I2 contract requirements are complete and which runtime/authorization/production requirements remain pending.
2. Review the additive P16 consumer API against server R12 without taking over active worker files. Retain original fixture identities, diagnostic rejection and the standing no-provider-call boundary.
3. Preserve `c62d5d5` and its parent. Do not amend, reset, rebase, cherry-pick away or otherwise rewrite this incident. Make any necessary corrections in new explicit-path commits; do not absorb active server/generation/PM edits by accident. Coordinate the index as its sole writer in this shared checkout.
4. Validate the exact candidate snapshot with affected contracts/player checks, strict/native/packed consumers, offline workspace/app builds and identity checks. Verify the relevant recovery and both fixture browser regressions. Record tests against that exact tree/commit, then record CI separately for the published code head. Historical successful CI at `306059f` remains historical.
5. Publish a reconciliation receipt with the mixed commit's ownership breakdown, follow-up commits, exact service revision, test/CI results and remaining gates. Restore a truthful worker handoff before marking I13 complete. Actual accounting, live dispatch, accepted generation and production publication remain gated.

## Standing ownership

[AGENTS.md](../../../AGENTS.md) says: “In a shared working directory, Agent 1 alone stages/commits, switches branches and pushes.” Workers report owned file changes and leave shared-index operations to integration. Explicit pathspecs are still necessary for the coordinator but do not authorize workers to commit in the shared checkout. Workers may commit assigned paths in operator-provided isolated worktrees under the same file's separate rule.

## P16 evidence status

The [recovery receipt](../player/recovery.json) has no failures and confirms actual `WEBGL_lose_context`, frozen media/plaque/sign during loss, matching restored control pixels/state/contributors, rewind/stale recovery protection and seven disposed sinks. The current committed receipt records **84 local requests**, zero remote and zero provider calls; the narrative handoff reports 83. Preserve both observations rather than silently editing player-owned evidence. Reported player tests are 115/115 with retained regressions. P16's functional milestone is complete; mixed-commit integration/CI and full browser/device/production gates are separate.

## I13 follow-up

The original scope-audit observations above are preserved. [The reconciliation](shared-index-reconciliation.md) reviews all 19 integration paths, P16 and the later I2 follow-ups separately. Candidate `23f8f7a` verifies services v1.3.1 and has its own successful CI; historical 306059f/14821da results remain scoped to their heads. Two reproduced P16 recovery findings and the active R12 title gate remain open. The initial scope JSON is unchanged and does not become a retroactive content/CI approval.
