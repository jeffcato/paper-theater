# I17: behavioral compatibility and independent source coverage

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

Prepared October 26, 2026. **I17 complete:** [handoff](acceptance-source-coverage.md), G20 acknowledgement 9b2e5b6, source checker 4fa698c/services v1.6.0 and exact-head CI. Four executed/33 missing verifiers; next [I18](source-slice-mapping-task.md) supplies explicit source relationships. Owner: `integration`. I15 is published as services v1.5.0, code `7d50703`, receipt `d0edbe1`; public provider methods still need runtime wiring and live mode remains disabled. G20's scoped requirement/proposal implementation is complete with its compatibility hook retained. Preserve Claude's UI and all active worker files. Zero provider calls.

## First: close the G20 shared-test seam

Apply generation's exact [compatibility replacement request](../generation/acceptance-compatibility-request.md). Replace the integration-owned source-text search for `const REQUIRED_CHECKS:` with assertions over `fixtures/generation/acceptance-proposals-v1/scopes.json` and the shared scoped accessor. Verify actual shared/effective IDs, retained custom/inherited/adverse requirements, excerpt/chapter distinctions and rejected downgraded scope. Keep the existing legacy requirement-preservation proof behavior-based; do not weaken it or delete generation's hook yourself.

Publish passing shared-test evidence and an explicit owner acknowledgement that generation may remove the dead literal in its own cleanup. That cleanup must not be absorbed from a moving worker tree by a broad commit. Review the G20 boundary against I4's expected types/export rules while changing the shared test; do not take ownership of its implementation.

## Then: execute complete-source-coverage under I4

1. Implement a genuine independent `complete-source-coverage` check using the runtime-pinned source bytes/scope/span and candidate's actual source/read-along/scene coverage. Define Unicode scalar coordinates explicitly and compare actual ordered content, not merely a supplied coverage flag or receipt. Reuse existing structural/read-along validators; add the missing independent scoped coverage proof rather than duplicating their code.
2. Reject omitted, repeated, overlapping, reordered or out-of-scope source coverage and stale source identities. If current records do not establish a needed mapping, emit a typed missing result and a concrete schema/owner request; do not infer complete chapter coverage from an excerpt or manufacture missing facts/scenes.
3. Keep `complete-chapter`, provenance, speech fidelity, semantic fact/image/audio/device reviews and all other unavailable verifiers distinct. Passing source coverage proves only that scoped source is completely represented; it does not prove accurate speech or an accepted autonomous chapter. Retain `production-verifier-pending` and all adverse findings.
4. Bind the executed result to exact inputs, coverage scope and checker identity. Explicitly version any matrix/checker/record change and coordinate fixture/consumer adoption; preserve old matrix/receipt identities. Candidate JSON/report declarations still cannot authenticate execution. Provide runtime R15 and generation a precise updated handoff without editing their active modules.

## Verification and delivery

Observe a red test showing that missing/reordered source is not independently diagnosed, then implement the minimum passing check. Cover diagnostic/excerpt/chapter scope, Unicode/punctuation, valid complete coverage, one omitted source interval, overlap/duplicate/reorder, wrong hash/span and attempts to use a complete excerpt as chapter proof. Confirm private source bytes/paths do not enter the public report/payload.

Run affected Rust/TypeScript contract/export/strict consumers and offline acceptance tests, with exact source/commit/CI pins. A narrow positive source check may pass while overall production remains blocked. Preserve all G20 custom/adverse coverage and legacy requirements. Return the compatibility acknowledgement, executable verifier scope, exact verification results and remaining production-check count. Shared/root-owned changes only, explicit-path commits, no UI or live dispatch.
