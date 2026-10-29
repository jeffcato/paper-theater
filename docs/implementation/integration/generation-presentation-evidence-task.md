# G18: verified delivery-domain fact bounds

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

Prepared October 26, 2026. **G18 direct stage complete:** [handoff](../generation/presentation-evidence.md), 61 reported checks, exact PCM/recipe/mapped bounds and zero-redispatch replay. Runtime routing is R14; generation next takes [G19](backend-next-tasks.md). Owner: `generation`. Work in generation Rust/helpers, owned tests/fixtures and generation documentation. G16 fact matching and G17 package master capacity are complete at their stated diagnostic scopes. Preserve their source/media identities, unresolved findings and replay guarantees. Zero provider calls; no credentials or shared Git operations.

## Existing inputs and scope

G16 [fact evidence](../generation/acoustic-facts.md) has four matched facts with 24 kHz narration bounds; two uncertain and one missing fact remain unresolved. It deliberately emits no final presentation bounds. The separate immutable `fixtures/generation/two-scene-readiness-v2/presentation-map.json` names the same narration hash, places that narration into 48 kHz delivery segments and records opening/transition/closing gaps. A matching hash is a starting point, not proof that the map describes the delivered audio.

Implement a verified private narration-to-delivery evidence path through the existing StageInput/RunServices boundary. Use a derived diagnostic fixture/output rather than changing the frozen v1 or published readiness-v2 bytes. Do not add a second player, public schema or production approval mechanism.

## Required increment

1. Read/hash-check the original narration, selected delivered WAV, exact presentation map, timeline/read-along bindings and G16 fact report/inventory through runtime services. Pin their actual hashes and the mapping implementation/recipe. Reject mismatched source/rates/master/map identities, malformed/out-of-range/overlapping segments, undeclared gaps and stale observations before deriving bounds.
2. Verify the declared placements against actual delivered PCM and the documented resampling/edit recipe, including native frame counts, trims, gaps and processing-delay compensation. Do not assume a rate ratio or map filename proves waveform placement. Where exact mapping/recipe/PCM verification is unavailable, emit a typed unresolved finding and no delivery bound.
3. Convert verified measured narration ends into final 48 kHz samples with integer/rational arithmetic and conservative upward rounding. Define interval-end ownership at segment cuts explicitly so a word ending at a cut is not confused with the following inserted gap. Handle facts with multiple fully covered supporting intervals, and take the maximum mapped supporting bound. A trimmed, uncovered or ambiguous interval cannot receive a fabricated bound.
4. Emit a private hash-bound presentation fact report containing the original narration-domain result, mapped delivery result or null, supporting segment identities and rejection reasons. Retain all raw 176 words/170 groups/six points. Keep the three unresolved facts unresolved and preserve adverse findings. Diagnose resulting entry/fade eligibility against the existing timeline without rewriting scene timing, speech or attestations to make it pass.
5. Recorded replay must reconstruct byte-identical mapping evidence without helper/provider redispatch, new journal proposals or accounting/catalog mutation. A changed narration, master, map, fact inventory or implementation invalidates the dependent report. Missing independent production recipes/reviews still block acceptance: diagnostic mapped arithmetic never grants a reveal permission, accepted handle or publication.

## Acceptance verification

Implement the delivery-bound behavior, then record actual verification results and commands. Cover the real saved v2 placement plus synthetic boundary cases: inserted gap before/after a bound, exact cut endpoints, cross-segment supporting intervals, nonintegral rate conversion/ceiling, trims/delay, unmapped or reordered speech, changed PCM/hash/recipe, invalid domain limits and replay after changed inputs. Preserve exact 0.75-second agreement semantics and unresolved evidence.

Use actual saved PCM for positive verification and explicitly diagnostic temporary media for negative cases; do not forge raw provider receipts or a missing resampler recipe. Keep tests under the unchanged acoustic helper envelope. Do not change helper/adapters recipe or dispatch chapter-sized inputs in this task.

Return the generation handoff/ready record, exact commands/results, raw-to-mapped bound examples, source/recipe identities, replay counters and remaining gaps. Run affected generation/Python/candidate checks, formatting and product identity checks; retain G16/G17 regressions. Integration alone applies root/shared changes and stages/commits/pushes.

Next chapter-scale follow-up is G19: bounded acoustic segmentation with pinned chapter/chunk/source/receipt identities and safe cross-segment evidence. That needs its own recipe/registration handoff if helper behavior changes. Full chapters, creative evidence, production acceptance and publication remain pending.
