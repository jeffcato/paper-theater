# G20: shared acceptance requirements and private proposals

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

Prepared October 26, 2026. **G20 implemented:** [handoff](../generation/acceptance-proposals.md), 101 reported tests and preserved scoped/adverse/private export/replay boundaries. Integration I17 must acknowledge the compatibility test replacement before dead-literal cleanup. Next generation task is [G21](generation-chapter-import-task.md). Owner: `generation`. G19 offline bundles/unit replay are complete; runtime registration/fanout remain server-owned under the exact [registration request](../generation/chapter-acoustic-registration.json). This task can proceed without that registration and without the UI redesign. Zero provider calls; generation-owned files only; no shared Git/lockfile operations.

Consume services v1.4-compatible acceptance records and the [I4 generation handoff](acceptance-evidence.md). Shared playback remains v1.0.0. Source definitions are `pt_contracts::required_acceptance_checks(AcceptanceScope)` and the shared fact/evidence inventory/proposal types. Proposals are untrusted observations, never acceptance or runtime authorization.

## Required increment

1. Replace the generation package path's authoritative private check list with the shared scoped accessor. Preserve the union of applicable shared requirements, inherited/custom requirements and every adverse finding. Scope must be explicit and consistent with the source/candidate purpose; generation cannot reduce the runtime's independently pinned scope by choosing diagnostic/excerpt labels. Preserve complete-chapter versus complete-excerpt and complete-source-coverage semantics.
2. Propose typed private fact/cue inventories from verified authored inputs. Bind exact Unicode source scope/spans, reveal IDs and asset hashes. Do not infer image applicability from an acoustic fact or fabricate a missing reveal/asset association. Missing authored bindings produce typed unresolved findings rather than dummy IDs/hashes or silently omitted facts.
3. Emit `AcceptanceEvidenceProposal` / `AcceptanceEvidenceInventory` as separately identified private candidate artifacts through RunServices. Pin exact payload/source/narration/alignment/map/master/recipe/profile/player/checker/fact-inventory/asset identities and explicit target sample domains/spans. References must verify against actual bounded bytes. A needed runtime/implementation identity must arrive as an explicit pinned input; do not read a moving UI workspace or manufacture an identity.
4. Retain all G16/G18/G19 pass/fail/uncertain/missing observations, original raw receipt identities and unresolved findings. Keep the evidence inventory acyclic and preserve conflicting/adverse records for independent checking. New passing proposals cannot erase earlier failures. Missing identities or semantic verifiers remain missing; no declaration becomes an execution receipt or capability.
5. Keep private source/alignment/recipes/fact/evidence inventories/raw receipts/proposal reports outside the audience payload inventory. Prove the package's explicit public allowlist and exact-byte hash order still hold. Generation proposes evidence; runtime owns trusted policy/frozen expected identities, independent verification, persistence and all activation decisions. Keep outcome NeedsRepair and reveal permissions closed.

## Shared compatibility seam

`crates/pt-contracts/tests/acceptance_verification.rs::the_authoritative_matrix_retains_all_legacy_production_requirements` currently reads `const REQUIRED_CHECKS:` directly from generation's module. That test is integration-owned.

Send integration the exact behavioral replacement/fixture request. Until its test is updated, preserve the existing literal as a documented non-authoritative compatibility hook while the actual package logic consumes the shared accessor; do not delete it and leave the shared suite failing. Do not edit the shared test/matrix yourself. Record this remaining cleanup precisely if the owner has not yet applied it.

## Verification and handoff

Implement the migration, then verify its changed behavior with focused checks. Cover all scopes, retained custom/adverse requirements, absent/forged scope/bindings, duplicate/conflicting proposals, stale raw bytes or media/recipe/version identities, missing asset/reveal association, source/sample domain mismatch and deterministic fixture/replay output without redispatch. Add a test showing private evidence is absent from exported payloads and forged `pass` cannot remove missing-verifier/production blockers.

Use diagnostic pinned fixtures for positive binding cases; synthetic bytes are explicit test data, not proof of an autonomous production. Retain G16's four matched/three unresolved facts, 176 words/170 groups/six points, G17 capacity, G18 exact PCM mapping and G19 bounded chapter behavior. Verify replay never authorizes a helper/provider or mutates accounting/catalog state.

Return owned files, exact verification commands/results, requirement-scope examples, emitted artifact/FileRef examples, private/public inventory proof and exact integration/runtime handoffs. Run affected generation/Python/candidate/shared compatibility checks, formatting and identity scans. Preserve Claude's UI and other workers' edits; integration alone stages/commits/pushes. Full acceptance, general chapter evidence import and runtime chapter fanout remain pending.
