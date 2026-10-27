# I4: independent acceptance evidence, first increment

Prepared October 26, 2026. Owner: `integration`. I13 reconciliation is complete in publication `f585fb5`, CI receipt `6698b20`; source/browser checks are pinned separately to `23f8f7a`. Shared services are `pt-services-v1.3.1`, playback remains `pt-contracts-v1.0.0`. Preserve all historical heads and immutable evidence. Zero provider calls; no credentials or live authorization.

## Bounded objective

Implement the first executable independent acceptance check: complete required-check coverage and exact evidence/input/version bindings. Current structural/package verification exists, but `production_blockers` deliberately retains `production-verifier-pending`. Reuse existing validators and exact-byte inventory verification rather than reimplementing them. This increment must distinguish an executed verification result from an artifact merely declaring `pass`.

Work in integration-owned contracts, tests/tools and documentation. Send runtime/generation/player consumer changes to their owners as exact handoffs; do not absorb their active working files. Follow contract-first TDD and sole-writer explicit-path Git rules.

## Required increment

1. Define the authoritative required-check inventory for each stated candidate scope, including complete source/chapter coverage, independent per-fact evidence and required audio/visual/rendered/profile/device reviews. Resolve current duplicate lists with an explicit versioned shared description, without silently dropping a missing requirement. Keep production chapter requirements distinct from diagnostic test coverage.
2. Verify evidence references against actual available bytes: exact payload inventory, source/narration/master/map/asset identities, recipes, target spans/domains and profile/player/checker versions. Bind evidence to the candidate actually checked; unrelated, stale, duplicate/conflicting or out-of-scope records cannot cover a requirement. Preserve the acyclic payload → acceptance → manifest → catalog hash order.
3. Run the checks that are genuinely implemented from an independent entry point and construct a deterministic private verification report from their results. A candidate-supplied `pass`, autonomous provenance or verified-profile label is not proof that a checker ran. For creative, provider, device or other unavailable verifiers, return typed missing/unimplemented findings with the exact required binding rather than inventing success.
4. Emit complete required-check coverage with pass/fail/uncertain/missing status, target scope, consumed hashes, executed checker identity and reasons. Preserve all failed/uncertain findings; omitting a requirement or adverse observation cannot improve acceptance. Specify how generation may propose evidence and how runtime will later consume the independently verified result, without granting either a new dispatch/publication path now.
5. Publish matching Rust/TypeScript report/validation contracts only if this increment needs a new shared boundary, with explicit version and compatibility rules. Do not repurpose provider metadata or broaden legacy authorization. Keep public payloads free of private raw source/receipt/journal data; provide an explicit export-boundary handoff if new private evidence is introduced.

## Test-first proof

First write a failing behavior test for a candidate that removes a required check or supplies forged/stale passing evidence; record the failing command/assertion, minimal fix and green rerun. Add cases for missing checks, duplicate/conflicting coverage, wrong source/media/map/asset hash, wrong target/span/sample domain, wrong recipe/profile/player/checker identity, uncertain/adverse evidence, modified bytes after a receipt and diagnostic/imported/synthetic provenance. Cross-language records must agree where mirrored.

Include a positive test for a narrowly implemented binding/coverage check, but keep the overall diagnostic candidate rejected. No synthetic test fixture becomes an autonomous release. Retain both canonical v1 readiness failure and corrected v2 diagnostic eligibility with their exact hashes.

Run affected contracts/strict/native/packed checks, appropriate offline workspace/app builds and identity scans; record exact test/commit/CI identities. Do not repeatedly rerun unrelated suites without a new change or unresolved concern. Return the executable verifier/report API, required-check matrix, red/green evidence and concrete worker handoffs.

## Gates that remain closed

Do **not** remove `production-verifier-pending` until all required independent verifiers genuinely execute with complete evidence. This first increment does not close all of I4, enable live dispatch, reconcile actual accounting, certify target-device performance or activate the audience catalog.

I13's [two P16 findings](i13-player-recovery-findings.md) now have a [local repair](../player/recovery-repair.json): pending Play and unjoined restore branches pass six new regressions and the unchanged probe. Integration I14 must independently verify and wire that coverage; preserve original red evidence and never transfer prior CI to this new repair. P17's title fix is now supplied; server must rerun strict R12 before that separate gate closes. These fixes can proceed alongside this acceptance increment.
