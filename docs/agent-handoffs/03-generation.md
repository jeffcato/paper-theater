# Agent 3 prompt: autonomous generation and acceptance

You are the Paper Theater generation agent. Read [COMMON.md](COMMON.md), [INTERFACES.md](INTERFACES.md), and your ownership row in [README.md](README.md). Implement the source-to-candidate pipeline through the shared runtime services. Prioritize reusable offline adapters and conservative evidence matching, then a small integrated excerpt.

## Own

- `crates/pt-production/src/generation/**`
- `tools/generation/**`
- `fixtures/generation/**`
- `docs/implementation/generation/**`

Agent 1 owns shared contracts, copied generic core crates and the production crate manifest/module root. Agent 2 owns database writes, all reservations/dispatch and publication. Agent 4 owns the player/capture interface and rendering profile. Submit exact dependency/shared-schema requests instead of editing those paths.

## Read-only inputs

Resolve through transient `SOURCE_ROOT`, reading the owning documents as each stage needs them:

- `docs/architecture/generation/01-source-and-direction.md` through `07-audio-direction-and-timeline.md`
- `docs/examples/oz_demo/{director,audio,http,quota,rate_limit}.rs` and the associated TTS wire adapter
- `docs/examples/oz_scene/{meta,stage,layout,stage_art,stage_run}.rs` and SAM decoding
- `docs/examples/acoustic_proof/{transcribe,readalong,endpoint_probe,music_probe,reference_probe}.py`
- Selected `prototype/tools/` source, audio, acoustic, visual, presentation, review and export helpers
- `docs/reviews/2026-10-25-audio-coherence-investigation.md`
- `docs/reviews/2026-10-25-thunder-child-anemic-generation-investigation.md`

Reuse parsers/transports, semantic registration, source validation and negative cases. Do not import a monolithic experimental orchestrator or its relaxed trial policies. Preserve raw observations and existing media; code migration alone does not authorize new calls or regeneration.

## Begin now: bounded first handoff

1. Extract pure source, response parsing, normalization, read-along and geometry functions into your owned product paths. Keep Python acoustic/media helpers behind bounded JSON/file envelopes. Replay saved responses with zero dispatch; remove all embedded source paths and retired identities from adapted code.
2. Freeze exact Oz #55 Chapter 1 and Wells #36 Book One Chapter I spans from compatible cached source. Preserve raw/clean source hashes and Unicode scalar mappings. Validate actual opening/ending prose and nested Book One selection; contents headings and repeated Book Two chapter labels must not be mistaken for the target body.
3. Adapt the known 176 source words/170 highlight groups without changing measured points or grouping intervals. Provide conservative source-turn matching and fact-to-anchor mappings. Test unique versus repeated/ambiguous terminals, incomplete independent interval coverage, missing facts, mismatched hashes and disagreement outside 0.75 seconds. A nearby final timestamp alone is insufficient.
4. Implement the published `StageExecutor` against `RunServices`. Read/write candidates through those services; generation never opens the authoritative SQLite database, updates catalog pointers or dispatches a provider independently. Return typed artifacts/decisions/findings with complete input and recipe identities.
5. Produce a structurally valid diagnostic `PackageCandidate` from saved inputs for the first joint slice. Explicitly enumerate missing production evidence. Do not turn incomplete imported evidence into passing attestations.

## Production stages

Implement incrementally using the fixed order in `INTERFACES.md`:

- **Source/direction/research:** source-backed cast and beat inventory, director-selected scenes/pictures, required initial reference research, one applicable canonical image per subject type and a separate common art profile. Research supports coverage and may add missing types. Preserve concrete, recognizable scenes; no symbolic substitutes or invented speech roles.
- **Pilot/speech:** persistent speaker/model/voice IDs and concise acting direction, with multi-take continuity and neighbor comparisons before bulk synthesis. Audition the selected Flash TTS model first for new final recordings; preserve accepted Lite identities. Sentence-aware units retain exact prose. Correct unintended level/silence discontinuities using safe measured edits; do not overlap voiced words or disguise identity drift.
- **Acoustic evidence:** primary measured transcription without expected-source hints; independent verification of complete fact-supporting intervals; conservative later endpoints, raw response/recipe hashes and exact presentation mapping. Highlighting and reveal permissions remain separate. Larger 60–90-second transcription batches require a real bounded pilot before bulk use; do not assert the proposal is already validated.
- **Visual preflight:** before story artwork, validate source coverage, first/incoming-picture readiness, applicable reference roles, one owner per visible instance, actual eligibility/exposure and feasible required actions. Compile a cheap draft against measured audio and placeholder geometry. Return one consolidated Muse revision packet; unresolved required defects stop dependent spending.
- **Visuals/registration:** scoped Image requests with unreviewed internal search/shell disabled; actual reference bytes; separate clean environments and subjects; SAM masks/crops, semantic body dimensions, pivots, native aspect and support contacts. Reuse one canonical character without another Image/SAM call. Diagnostic white mattes cannot become approved masks. Reconcile audience-camera size/contrast with Agent 4's validated profile.
- **Presentation/music/mix:** director chooses scenes before new master placement freezes. Introduce needed context behind closed curtains and make incoming pictures eligible before fade-in, preserving prose order. Reserve narration-free formal transition slots and final closure. Compile a real persistent `MusicBible` into cue requests; use reviewed phrase-aware edits, fades or deliberate silence, not automatic full-clip restart cycles. Deliver one 48 kHz stereo PCM16 WAV with exact sample mappings, target -18 LUFS integrated and ≤-1 dBTP, plus actual continuity/intelligibility/mix review.
- **Choreography:** compile only the selected supported actions against accepted anchors and registered assets. Account for every required intention as executed or explicitly revised; impossible required actions fail. Agent 4 indexes/evaluates this authored timeline and does not repair it at playback time.
- **Review/package:** use Agent 4's actual capture interface, exact candidate/profile/player hashes, opening/entry frames, duration-labeled holds and paired motion evidence. Compare visible subjects/actions with current source and compiled cues. Keep all five scoped image checks, full bounded audio-window coverage, narrative readability and objective integrity separate. Fail/uncertain findings block acceptance. Assemble the immutable inventory and acceptance dependencies; Agent 2 publishes it.

Shell art is a shared reusable asset task: consume Agent 4's registered geometry/briefs, generate and review parts through Agent 2's broker, then return measured assets. Do not choose incompatible dimensions or implement a second renderer.

## Negative and positive cases

Your offline regressions must reject missing narrative subjects; empty/unready entrances; later facts in references; unrelated references on empty scenery; the same logical subject baked into scenery and staged as a card; impossible required actions or silent cue omissions; canvas-padding-based body scale; missing review opening/action coverage; stale hashes; uncertain semantic findings; and ambiguous independent acoustic matches.

Also pass purposeful source-supported stillness, valid multi-picture scenes, source-compatible reference/character reuse, unchanged raw point groups and unchanged media reuse after local arithmetic fixes. Avoid per-book production branches and arbitrary scene/asset quotas.

## Acceptance and reporting

First expose and run `cargo test -p pt-production generation::` and offline helper tests under your lane, using actual cached parser/geometry cases plus meaningful negative fixtures. Fixture/replay must prove zero provider calls. Run the identity checker on source, candidate manifests, normalized evidence and exports.

Live validation waits for explicit product spending/quota authorization through the runtime broker. No old allowance, successful offline test or synthetic evidence grants that authorization. Do not relabel unresolved acoustic/voice/narrative checks to finish the slice.

Return exact commands/results, consumed contract/profile revisions, imported/transformed provenance, unresolved fact/action/review IDs, and the next bounded integration step. A complete production result requires the actual autonomous excerpt, then both full required chapters; the first offline handoff is an explicitly diagnostic increment.
