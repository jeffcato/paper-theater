# G21: general chapter evidence importer

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

Prepared October 26, 2026. **G21 complete:** [handoff](../generation/chapter-import.md), 130 tests, generic bounded evidence/replay and separately registered new helper pending runtime. Next generation task is [G22](generation-direction-task.md). Owner: `generation`. G19 bounded chapter bundles and G20 scoped private proposals are complete at their stated offline scopes. Next remove the saved-excerpt-only limitation: the current chapter producer/verifier preserves the original G16/G18 evidence context and does not import arbitrary complete chapter turns/transcripts. Runtime registration/fanout and live calls remain outside this task.

Work in generation Rust/helpers, owned tests/fixtures/docs. Preserve Claude's UI, source/shared contracts, original G16–G20 fixtures/receipts and published helper recipe identities. Zero provider calls; no credentials or shared Git/lockfile operations.

## Required increment

1. Define a versioned, bounded importer for full source/narration plus ordered source turns and actual saved primary/independent observations in the existing supported raw schemas. Pin source/WAV/PCM, provider/model/request/response/receipt/recipe identities and exact global source/sample domains. Do not depend on hard-coded saved turn names, counts or the 176-word excerpt scope.
2. Reconstruct the verbatim source with explicit half-open Unicode scalar coverage; preserve punctuation, names, speaker boundaries and measured timestamps. Validate raw turn offsets, original PCM slices and complete ordered source coverage. Missing transcripts/recipes/independence remain unknown rather than fabricated from another observation or inferred from synthetic audio.
3. Produce the immutable context/index/segment dependencies required by the bounded G19 unit path. Derive local/global offsets with integer arithmetic; avoid copying an unbounded global evidence bundle into every unit. Recheck the tightest actual per-file, metadata, aggregate/reference-count and helper-output limits. Reject an unsupported evidence size explicitly rather than silently dropping context.
4. Preserve global source/terminal uniqueness and complete supporting intervals across segment boundaries. Window-local uniqueness, partial turns or sliced observations cannot manufacture independence or resolve ambiguity. Missing cross-unit dependencies retain null bounds/NeedsRepair. Use G20's private proposal formats/public payload separation; general import never grants acceptance or a final presentation map by itself.
5. Implement deterministic saved-record/StageInput replay without helper/provider redispatch, journal/accounting/catalog changes or mutation of original receipts. Use fully labeled synthetic test observations for new positive chapter fixtures; they are test data, not genuine provider/production evidence. Large temporary media stays out of fixtures.

## Compatibility and runtime handoff

Keep the existing G19 recipe and example byte identities stable. If new runtime helper behavior/imports are needed, introduce a separately versioned recipe with its full source closure, exact consumed FileRefs/envelope and registration request; do not silently update the old recipe registration. Server owns authoritative whole-bundle import, helper registration, bounded fanout/commits/restart/cancellation. No unregistered dispatch or new scheduler is authorized.

Integration I17 is replacing G20's source-literal compatibility test. Remove the retained dead REQUIRED_CHECKS hook only after its owner publishes the acknowledgement/passing test; otherwise keep it inert and continue this independent importer task. Do not edit shared compatibility tests.

## Verification and delivery

First demonstrate rejection of a correctly bound different complete source/turn inventory by the old saved-only path, then implement a minimum valid generic import. Test multiple complete turns and Unicode/speaker boundaries, missing/duplicate/reordered turns, changed request/model/recipe/audio/source, ambiguous repeated text, chunk cuts through facts/turns, absent independent observations, limit-before-read behavior and byte-identical replay. Keep source completeness distinct from synthetic/unobserved full-chapter acoustic correctness.

Retain G16 four matched/three unresolved facts, original words/groups/points, G17 media bounds, G18 PCM mapping, G19 600-second capacity proof and G20 scoped/adverse/private export behavior. Return exact schemas/examples, verification commands/results, allocation/size limits, importer/recipe identities and owner requests. Formatting/identity checks and relevant generation/Python/candidate tests must pass. Integration alone stages/commits/pushes; full autonomous generation, listening/creative/device review and publication remain pending.
