# G25: verified unhinted primary transcription

Stage::Acoustic recognizes `private/primary-transcription-context.json` before its unchanged legacy G16/G19/G21 routes. It revalidates G24, independently verifies actual copied PCM, compiles an audio-only request, ingests saved original responses and maps measured observations to original native recordings and global source positions. Every result stays private, diagnostic and NeedsRepair. No provider/helper issuance, execution, recovery, materialization, commit, UI or Git operation is performed by the compiler.

## R19 boundary and frozen predecessor

[G24 source and fixture pins](speech-g24-freeze.json) were frozen before overlapping router/test edits. G24 implementation/helper files and all prior fixtures, receipts, measured points and unresolved facts remain unchanged.

R19 owns materialization, registration, genuine ArtifactRefs, immutable original raw/audio/usage capture, durable commits and checkpoint/replay adoption. The [private handoff](primary-transcription-handoff.json) was published before implementation. Its schema adopts R19's exact schema-1 `native-pcm16-frame-copy-v1` receipt rather than introducing a second materializer.

The integration test invokes R19's public backend APIs in an isolated store, obtains its committed audio/receipt and feeds them to the real generation executor. The adopted copy recipe is checked against [R19 readiness](../runtime/speech-runtime-ready.json). R19's actual audio payload equals generation's compiled request. That proves current interface/byte compatibility; it does not extend an earlier R19/I21 exact-source or production-acceptance claim. Fixture-side adapter metadata remains synthetic and unaccepted.

## Strict private schema 1

| Record | Required fields |
| --- | --- |
| Context | `speech_input`, `speech_context`, `speech_plan`, `take_manifest`, `copy_plan` FileRefs; `batch_id`; actual `batch_audio` ArtifactRef; `copy_receipt` FileRef; runtime-pinned `copy_recipe_sha256`; request/parser/mapping `recipe` FileRef; `provider`, `model`, `estimate` Usage, `timeout_ms`; nullable `request_payload` and `observation` FileRefs; bounded matching `limits`. |
| Matching limits | `max_words` 1..2048; `max_source_tokens` 1..8192; `max_match_cells` 1..2,000,000; `max_candidate_edges` 1..16,384. Excess rejects before matrix work; no prose/observations are truncated. |
| Observation | `schema_version`, `provenance`, `provider`, `model`, original `request` ArtifactRef, exact `audio` ArtifactRef, `copy_recipe_sha256`, `recipe_sha256`, nullable original `response`, optional original `issued` ProviderOperationV2 / `result` ProviderResultV2, original `findings`. |
| Model request | Exactly `schema_version: 1`, `mode: VERBATIM`, `word_timestamps: true`, `audio: <verified committed ArtifactRef>`. No source prose, expected transcript, book/character/speaker text, lexicon or matching hints. |
| Proposal | Published ProviderProposalV2, kind Transcription. Its payload is the genuine explicitly registered request ArtifactRef; provider inputs contain only the verified batch audio, never source/copy/voice metadata. |

Typed context/observation records reject unknown fields, including caller execution/approval authority. Estimate integers and timeout follow shared bounds; zero money does not waive requests. Recipe bytes/hash and the StageInput recipe pin must match. The context is content, not authorization.

The exact input shapes and examples are in [handoff](primary-transcription-handoff.json) and [readiness](primary-transcription-ready.json). Implementations are [entrypoint](../../../crates/pt-production/src/generation/primary_transcription.rs), [verification](../../../crates/pt-production/src/generation/primary_verify.rs), [raw parser](../../../crates/pt-production/src/generation/primary_words.rs), [global matching](../../../crates/pt-production/src/generation/primary_match.rs) and [native mapping](../../../crates/pt-production/src/generation/primary_mapping.rs).

## Independent source and PCM verification

The compiler reconstructs the original Speech execution in Replay through an in-memory verification sink. Its exact committed source/unit plan, take manifest and copy plan must match reconstructed bytes. Speech in turn revalidates Pilot, Direction and Source, including original source/turn/request/raw/recipe/model/voice/catalog identities. A serialized successful plan or caller's approval flag is insufficient. Original observer scope is retained; no committed ID is created from a CandidateFile.

The selected batch must exist in that exact G24 plan. R19's complete receipt must bind the genuine committed manifest/plan/audio refs, source digest, batch ID, copy recipe, format, mappings, unplanned remainder and closed authority fields. Unknown or changed receipt claims reject. The native recordings and actual batch are inspected with the shared PCM16 WAV inspector. Rate/channels/frames, complete segment coverage, duration/byte limits and native identities must match. Every selected native PCM frame is compared against the actual batch data extent. Reordered, repeated, skipped, inserted or converted frames reject, including containers that remain valid WAVs. Noncanonical source data offsets come from the inspector, not a presumed 44-byte source header.

The compiler compares; it does not copy/assemble audio. Mixed native formats cannot share a batch or acquire a common narration clock. The input envelope stays at 64 refs/64 MiB, native recordings 8 MiB, metadata/request/raw/candidate JSON 1 MiB. Only the exact selected committed batch role receives the smaller original configured limit and 16 MiB/90-second hard bounds. Original Speech bounds are read from verified bounded metadata before payload reads; forged MIME labels cannot enlarge other roles. Original plans retain 64 segments/256 batches and explicit unplanned ranges. A temporary >8 MiB distinguishable batch proves the selected-role allowance without changing native limits.

## Request commits and original response ingestion

Without `request_payload`, generation returns exact request bytes and `primary-request-commit-pending`. Runtime verifies/commits those bytes, supplies a genuine input ArtifactRef and explicitly registers its FileRef. A recorded candidate alone is not registration. The registered payload must equal exact recompilation before a v2 proposal exists. No mode, including Live, invokes provider/helper services; unavailable execution remains Missing without fallback.

Saved observations bind exact request/audio/copy recipe/request recipe/provider/model. Where present, issuance must match the proposal and current runtime-bound StageInput observer, and the result must match that issuance and original raw ref. Foreign epochs, wrong descriptors, stale IDs or hashes reject. Uncertain, obsolete, incomplete and unresolved attempts retain original records and charges, with no usable measured result or repurchase. Cross-run/epoch reuse requires runtime's separately audited mapping, never relabeling an issued record.

The supported saved wire shape is one Gemini candidate with `finishReason: STOP` and `content.parts[].audioTranscription.words[]`, each with `word`, `startOffset`, `endOffset`. A present `modelVersion` must match the pinned model. Free text and `audioTranscription.text` are not timing evidence. Multiple candidates, refusal, truncation, malformed JSON, missing timings, empty recognition, invalid offsets or multi-token word annotations remain unresolved with their original raw FileRef and exact original bytes intact. A valid prefix is not promoted into complete evidence after a malformed word. The normalized receipt reports parser status and the invalid index where applicable.

## Exact measured coordinates and global ambiguity

Offsets accept nonnegative decimal-second strings with up to nine fractional places. Integer nanoseconds preserve the original decimals; conversion is `ceil(nanoseconds × sample_rate / 1,000,000,000)` using integer arithmetic. Ordering and physical duration are checked exactly. There is no float tolerance, timestamp clamp, guessed endpoint or expected-word interpolation.

Measured equal offsets remain points with zero native duration. A positive interval collapsed by sample ceiling stays a positive measured interval with explicit uncertainty; it does not acquire a fabricated duration or become a claimed measured point. Original offset strings, integer nanoseconds and batch samples remain in each normalized observation.

Matching uses the full pinned source locally, never a turn-local scope or a model hint. Existing `monotonic-align` tokenization/Unicode NFKD/alphanumeric normalization is reused and hash/version-bound; original Unicode words and scalar spans remain unchanged. There are no spelling/synonym replacements. Bounded prefix/suffix LCS tables enumerate all optimal monotonic occurrence edges. A source mapping is forced only when both source and heard occurrences are unique and cannot be skipped in any optimal alignment. Repeated `the` remains globally ambiguous even when one occurrence lies in the known native take. Missing/substituted/invented recognition and unmatched source remain explicit. A native take's source span is a subsequent consistency check, never a way to prune global ambiguity.

Batch samples map through verified piecewise offsets to original native sample domains. An interval crossing an artificial join or a point exactly on a shared join remains one original observation with no assigned native domain. It is not split, duplicated or spliced. Native interior-cut flags retain uncertainty even for a physically locatable interval. Source/native conflicts and sub-sample collapse remain unresolved. All global candidates and raw measurements remain available privately; no shared narration/delivery clock exists.

`point-following-same-boundary-v1` groups a measured point only with a following positive interval in the same unambiguous native/source domain at the same measured boundary. If distinct original instants collapse to the same integer sample, the point remains unresolved rather than extending it by rounding coincidence. A group's duration comes solely from the positive interval; the point's own samples remain unchanged. Unattached/trailing/ambiguous points remain explicit.

Primary measured/source mappings are not independent acoustic verification or listening/fidelity approval. The later 0.75-second independent-endpoint rule is unchanged and is never used as a word/highlight tolerance. Fact bounds and independent evidence remain null; reveal permissions remain empty. Source coverage includes unmatched source plus all unplanned/unobserved batch/native ranges; no batch result claims complete chapter acceptance.

## Replay and evidence

Normalized outputs bind exact source/take/plan/audio/copy/request/observation/raw/recipe identities and compiler/parser/matcher/WAV-inspector source hashes. Replay with the same registration phase reproduces request/proposal/handoff/normalized bytes, findings and cache keys, without rewriting original receipts/audio or creating provider/helper/journal/library/accounting/catalog effects. Original usage remains observation data. Every record stays private; a public-image alias cannot export its protected hash.

Reference cases use distinguishable deterministic PCM so byte-copy defects are detectable. Their transcripts are synthetic measurements and prove mechanics only. The R19 case consumes actual server-committed materialization/receipt; it makes no real transcription call. [Fixture index](../../../fixtures/generation/primary-transcription-v1/README.md) and [ready receipt](primary-transcription-ready.json) record exact commands/counts/hash pins and red/green history. G16's 176 words/170 groups/six points and unresolved facts, G19/G21 recipes, G24 source/native evidence and presentation regressions are retained.

Runtime's next work is private registration, request commits and durable Acoustic checkpoint/replay adoption. Actual provider authorization/transport, metered transcription pilot, independent full-interval fact evidence, listening/continuity/fidelity, final assembly/presentation and production publication remain separate gated work. No UI changes are required.
