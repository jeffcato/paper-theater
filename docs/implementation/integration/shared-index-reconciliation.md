# I13 reconciliation and exact-head review

**Verdict: NOT READY for broader player/Studio recovery readiness.** The shared-index incident is reconciled and the integration-owned contract correction is verified. Two reproduced P16 defects remain assigned to player; active R12 and generation work are excluded.

## Findings

1. **HIGH, open — `packages/player/src/mount.ts:261`**: a controlled pending Play can complete after graphics loss and leave media playing while graphics remain lost. Public-API edge probes reproduce this. P16's existing matrix covers loss during established playback and during pause, leaving this race uncovered.
2. **MEDIUM, open — `packages/player/src/mount.ts:311`**: an automatic restored-event recovery rejection is unhandled when the consumer has no factory or no caller joins the rejected promise. R12 currently supplies no recovery factory. The edge probe records the unhandled rejection and retained lost state.
3. **MEDIUM, fixed in `23f8f7a` — original `packages/contracts/src/provider-metadata.ts:23`**: v2 validation accepts omitted nullable legacy result fields, but its assertion narrowed them to legacy declarations requiring presence. A strict consumer's null-only usage guard compiled and then threw on omitted usage. New wire aliases accurately include omitted observations and nullable file dimensions, preserving all legacy method types. The failing strict-consumer test now passes with the expected compiler diagnostic; a nullish guard compiles. Provider envelope version remains 2; the service patch is `pt-services-v1.3.1`.

Exact repros, scope, source pins and observed commands are in [the receipt](shared-index-reconciliation.json). The actionable player handoff is [here](i13-player-recovery-findings.md).

## Reconciled ownership and history

The original scope audit is preserved in `shared-index-incident.json`. All 32 mixed-commit blobs were checked against `c62d5d571915195e80ee5f2dc3615cd49c1849b7`: **19 integration-owned, 13 player-owned**. Its parent is `fa361fe7324e866a1f01c6ea8ec18d4a425ab1f2`; P15's ten paths are player-owned. Neither commit was rewritten.

All 19 integration paths were reviewed: Rust provider records/validators, fixture emitter/support/tests, exports, the original 76 cases, TypeScript validators/strict consumers/native packing, interface/readiness/specification/adapter/task records and worker handoff. The original fixture bytes remain unchanged. The later `14821da` handoff/packed-consumer changes and `a979c33` CI receipt were inspected as a separate six-path follow-up surface. Their successful CI remains bound to `14821da`, alongside the older `306059f` history; neither certifies the original mixed commit in isolation.

The initial working-tree inventory separately records 50 pending paths. Active server R12, generation G16/G17, PM instructions/tasks and subsequent lane edits are not part of the candidate. R12 was read to assess the new P16 consumer seam: it supplies no `recreateSink` or graphics retry UI and captures its original sink for camera fitting. A future server-owned recovery increment must manage the replacement sink/camera and explicit retry/reload. R12's independent title-pixel failure remains a separate inherited render-order gate. Its receipt is still incomplete. Original P16 evidence says 84 local requests while its narrative says 83; both observations are preserved. The fresh I13 P16 execution also observed 84.

Integration alone performs shared-checkout Git operations. Checks ran in detached worktrees; no changes remained staged during sustained verification. Corrections use new explicit-path commits and `git commit --only`. Workers return owned changes and receipts. This prevents the coordinator from committing unrelated staged paths; workers remain prohibited from staging or committing in the shared checkout.

## Exact candidate and verification

Local verification targets code commit **`23f8f7af3b93f79271153266c5e47bd3c0791c40`**, with `SOURCE_ROOT` unset. The receipt pins its full tree and the immutable copied server binary. Browser evidence was collected only after that binary's build completed.

- Rust workspace tests and server build pass; contracts cover 80 provider cases plus 14 playback cases. Native and packed Node imports and both strict TypeScript consumers pass. The original 76-case fixture is preserved; four new omitted/null cases are separately versioned in `provider-metadata-v2-i13.json`.
- Player 115 tests, Studio checks, all web builds, 17 Python tests, and source/delivery identity scans pass. Vite retains its chunk-size advisory.
- Both combined Chrome fixture flows pass with 39 ordered events, real offline acoustic cancellation/replay, verified audio, actual nod pixel changes and 11 captured frames each. Canonical v1 remains ineligible at incoming fade; readiness v2 is eligible. Both reject production loading. The fixture manifests retain their original hashes.
- Real `WEBGL_lose_context` recovery matrix passes: frozen media/plaque/sign, control-matching restored pixels/contributors, stale protection, seven disposed sinks, 14 disposed targets and zero remaining tracked resources. Fresh artifacts are stored separately; original player receipts/screenshots are unchanged.
- The new public-API edge probe **exits 1** for findings 1 and 2. This is a recorded failure, not a successful acceptance check or part of the current CI aggregate. The existing successful matrix does not cover those cases.

GitNexus PDG indexes were built at the initial head and corrected candidate. Exact compare surfaces use the P15 parent. Context/impact resolve the provider validator, mount and direct test consumers; source inspection covers Studio and the active R12 caller. Taint/dependence queries supplement the review. Entrypoint/callee truncation, cross-language fields, dynamic imports and callback/promise boundaries limit graph coverage; absent findings are not safety proof. Domain, architecture, language, completion and simplicity passes were performed inline under the workspace's agent ownership rules.

## Completed and pending boundaries

I2's contract portion is complete: versioned project/credential context, simultaneous quota descriptions, operation/repair/retry identity, safe usage/timestamps, preserved unknown charges, retry lower bounds and existing-job identity/status; compatibility remains explicit. The v1.3.1 correction changes declaration accuracy, with no provider method or transport enabled.

Server still owns credential-context resolution, persisted lineage, once-per-repair charging, atomic simultaneous-window enforcement, genuine receipt/timing/job adapters and a staged public issuance API. Actual accounting reconciliation, live authorization, book/source bindings, independent production evidence, target-device measurement, accepted chapters, production verification and catalog activation remain pending. Zero provider calls and no credentials were used.

I13 closes the reconciliation/review task with the findings above still open. CI is recorded separately for the published candidate head in `pr-ci-results.json`; a green CI result describes its existing checks and does not close the failing edge probes or R12's title gate.
