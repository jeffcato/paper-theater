# Coherent integration candidate

Branch `integration/coherent-candidate` reconciles PRs #3–7 and frozen player v5 into one tree. Individual PR checks do not establish compatibility; only the combined gates below do.

## What was merged

- PR #3 (production-goal, matrix v6 source scope), PR #4 (production-authority), PR #5 (primary operation binding), PR #6 (audio wire role) arrive together through the stacked `integration/audio-wire-role` head.
- PR #7 (trusted player load + root lockfile) merges on top.
- Frozen player v5 sources were verified by digest before merging; genuine chapter media stays pending.

## Lineage reconciliation (acceptance verifier)

Main carried matrix v5 with pilot-recording-review and delivery-fact-mapping implemented; the stack carried matrix v6 with complete-chapter/complete-excerpt implemented. Neither side alone is the product. The candidate moves to matrix v7 with all nine checks implemented.

Semantic decisions, all enforced by tests:

- Executed checks report executed. The wire-era `!executed` expectations belonged to the unimplemented era and cannot hold under v7.
- Plan-only with digest strings or unresolvable review targets fails closed (InvalidBinding for malformed references, IncompletePilotEvidence for well-formed but unrecorded ones). No synthetic evidence becomes recorded evidence.
- Pilot structure validation stays in the shared `validate_pilot_plan_structure` (single implementation); delivery-fact-mapping keeps its dedicated verifier.
- Report validation requires executed checks to agree with recorded pilot mechanics instead of requiring unexecuted checks.

## Fixture chain

`acceptance-proposals` archives stay immutable per version. The v7 landing extends the loop to v1–v6 and pins the current `acceptance-proposals-v7/scopes.json`, generated from executed code via `PT_ACCEPTANCE_EXPORT`; v6 was backfilled the same way. Scope rows are byte-identical across all seven archives.

## Verification on this branch

- Full Rust workspace aggregate, `check:contracts`, player aggregate, audience checks, server build, identity scan.
- Exact-head push and PR CI on GitHub, recorded in the goals doc.
- PRs #3–7 stay open until owners review and merge; this candidate proves they combine, it does not replace their reviews.
