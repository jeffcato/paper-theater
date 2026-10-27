# Generation task: integrate per-fact acoustic evidence

Prepared October 26, 2026. Owner: `generation`. Tracks **G16** in [TODO.md](../../../TODO.md). Generation's usage window is available again. Existing Stage::Acoustic and server runtime composition are complete; preserve and extend them.

## Outcome

Turn an explicit source-scoped fact inventory and saved primary/independent acoustic observations into hash-bound per-fact verification results. Keep missing, stale, ambiguous or incomplete facts explicit. Grouped read-along integrity alone must not authorize a reveal or acceptance.

Inputs are the [acoustic stage/handoff](../generation/acoustic-stage.md), [ready record](../generation/acoustic-stage-ready.json), existing pure matcher `tools/generation/evidence.py`, cached acoustic media/responses/receipts, and [current playback evidence rules](../../spec/playback.md). Use the shared `pt-services-v1.2.0` boundary and completed runtime helper/candidate reads. Runtime integration is documented in [acoustic-integration.md](../runtime/acoustic-integration.md); its former missing-routing blocker is resolved.

## Bounded implementation

1. Accept a private generation-owned fact-inventory artifact through verified `StageInput`/`RunServices` reads. Each fact names exact Unicode source span/occurrence, source identity, and its supporting turn/interval. Use existing saved facts or explicit diagnostic test cases; no new model calls or shared-schema fork is needed.
2. Integrate the existing conservative matching rules into the acoustic stage. Require complete fact containment and complete supporting interval coverage from both acoustic sources, unambiguous terminal occurrence, correct model independence, original response/recipe/source/audio bindings, and the existing 0.75-second agreement rule in the correct sample domain. A nearby final timestamp alone is insufficient. Use the later measured bound.
3. Preserve original narration observations and every one of the 176 words / 170 groups / six measured points. Keep narration evidence, presentation mapping and final delivery identities distinct. If emitting a final-domain bound, require the exact pinned resampling/trim/gap mapping; an unavailable mapping leaves that bound unresolved.
4. Produce a bounded private `fact-evidence.json` candidate and typed findings/decisions through runtime services. Include fact IDs, source spans, evidence/recipe hashes, selected source occurrence and both anchor outcomes. Preserve diagnostic/imported/synthetic provenance and existing failed/uncertain results. Other missing production evidence continues to require `NeedsRepair`; passing arithmetic cannot promote the package to accepted.
5. Extend offline stage tests with a fully covered unique match and failures for repeated/ambiguous terminals, partial independent intervals, missing facts, source/audio/response/recipe mismatch, disagreement beyond 0.75 seconds, stale mappings and cancellation. Cover multiple facts sharing one valid enclosing turn while retaining independent coverage per fact.
6. Prove recorded replay reconstructs the same findings from verified saved observations without helper/provider redispatch. Changed fact inventory invalidates its matching result without rewriting or relabeling immutable recordings/receipts. Retain current stage/runtime regressions.

Work only in generation-owned code/tools/fixtures/docs. Helper code changes alter approved recipes: send exact new files/digests to `server` for allowlist registration before claiming runtime dispatch integrated. Preserve integration-owned contracts, root manifests/locks and other workers' changes. Keep any new records private to generation unless an actual shared contract change is approved through its owner.

## Completion evidence

Return exact owned files, consumed revisions/hashes, commands/results, representative per-fact output, unresolved fact IDs/reasons, and replay/zero-provider proof. Existing ambiguous cached cases may remain unresolved; never widen an anchor or fabricate an observation to close them. No credentials, new recordings, paid research/review, library activation or publication are required.

After this increment, generation's still-32 MiB artifact envelope needs alignment with server/player chapter-media work. That is separate from fact verification and does not block this saved-input task.
