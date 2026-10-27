# G21: bounded generic chapter evidence import

The v2 importer accepts a complete source, canonical native narration, ordered speaker turns and saved observations without G16 turn names/counts or the 176-word excerpt assumption. `tools/generation/chapter_import.py` produces immutable private contexts/chunks; `generation/chapter_import.rs` independently reconstructs saved unit results through RunServices. Both paths retain diagnostic provenance, NeedsRepair, null unresolved bounds and closed reveal permissions. No provider/credential path, runtime registration, scheduler or publication behavior is added.

G19's helper/source/example bytes and `af53771eb6f8d5b807958416503fb525de6d408d605b7c86b705f084866fe647` recipe remain unchanged. V2 is a separate four-file recipe; exact source closure and runtime request are in [registration](chapter-import-registration.json). Its executable remains unregistered in product runtime.

## Input and exact identities

The complete [synthetic input](../../../fixtures/generation/chapter-import-v2/producer-inputs/chapter-evidence.json) is schema version 2:

```json
{
  "schema_version": 2,
  "provenance": "synthetic",
  "source_scope_id": "synthetic-chapter-v2",
  "source_origin": 700,
  "sample_rate": 8000,
  "source": "FileRef to exact UTF-8 source",
  "narration": "FileRef to complete native WAV",
  "turns": [{
    "id": "speech-0", "speaker": "Élodie", "text": "verbatim punctuation and whitespace",
    "source_span": [700, 737], "frames": [0, 8000],
    "audio": "FileRef to the original complete turn WAV",
    "primary": "Observation or null", "independent": "Observation or null"
  }],
  "facts": [{"id": "gate", "source_spans": [[719, 732]]}],
  "required_fact_ids": ["gate"],
  "inherited_findings": []
}
```

The illustration shows field shapes; use the linked complete JSON for executable FileRefs and complete coverage. Each Observation has explicit `provider`, `model`, and nullable `request`, `response`, `receipt`, `recipe` FileRefs. Actual bytes, rather than renamed summaries, bind each request/response/receipt/recipe. Receipts use the existing model/request_sha256/response_sha256/audio_sha256/harness_sha256 fields and a provider identity. Missing provider/recipe/request/response/receipt/transcript stays unresolved. A contradictory supplied identity fails import. Primary and independent provider, model, request and response identities must differ; another label on the same response cannot create independence.

Supported raw responses retain the existing shapes: primary `candidates[0].finishReason == STOP`, `content.parts[].audioTranscription.words[]` with word/startOffset/endOffset; independent `transcript` plus ordered complete `turns[]` with transcript/startMs/endMs. Requests remain opaque original bytes pinned by receipts; this importer does not certify prompt semantics or actual provider execution. New positive observations and requests are explicitly synthetic test data.

Source intervals partition every Unicode scalar of the complete supplied source, including punctuation and whitespace, in original order. The file's first scalar has the explicitly supplied `source_origin`; no UTF-8/UTF-16 offset guessing occurs. Every turn preserves its exact text and speaker boundary. Native PCM intervals also partition the complete narration. The producer verifies each original turn WAV is exactly its corresponding full-chapter PCM slice, then verifies streamed reconstruction from all chunks. Canonical mono PCM16 is required, at 8–192 kHz; this stage does not create a 48 kHz stereo delivery master or presentation map.

## Bounded context and units

One shared `private/chapter-context.json` contains the original inventory bytes and every complete saved observation/request/receipt/recipe as byte-preserving base64, with their original FileRefs. The decoded inventory must equal the parsed spec. No transcript is windowed, normalized into a replacement receipt, or silently omitted. Original bytes are also copied to a private audit directory. Complete source bytes are a separately pinned shared private file. Units reference these shared files; they do not copy chapter PCM or an unlimited raw evidence directory into each unit.

| Limit | Enforced bound |
| --- | --- |
| Full native chapter WAV | 256 MiB, streamed |
| Source, original turn WAV, chunk/helper file | 16 MiB each; WAV headers count |
| Original inventory, shared context, index, unit, record, each private output | 1 MiB each |
| Distinct saved request/response/receipt/recipe bytes | 512 KiB total before reads; encoded complete context must also fit 1 MiB |
| Unit/replay aggregate | 64 MiB; reserve five metadata MiB before dependency selection |
| Unit/replay references | 64; reserve unit + helper record + all three replay outputs |
| Turns / facts / chunks | 1,024 each |
| Global ambiguity work | source-token count × max(1, total fact-span count) ≤ 2,000,000 |
| Helper stdin / stdout | 64 KiB / 1 MiB |

Unsupported sizes fail explicitly, before output publication; declared payload/reference bounds fail before payload reads. Encoded context size and ambiguity work fail before PCM processing. No metadata/MIME alias bypasses the Rust pre-read limits. Runtime still owns subprocess resource/deadline/cancellation controls; no measured RSS claim is made.

Chunks use integer native sample ranges and retain global source/turn metadata in the context/index. Unit dependency lists include complete chunk identities. Dependency selection reserves every replay slot; a 60-segment fixture proves cold execution and all three private outputs can replay within exactly 64 references. If a complete supporting turn's dependencies cannot fit or are absent, its fact stays Missing with null bounds. All required chunks remain listed, including absent ones. Unknown inputs outside the exact unit allowlist fail closed.

## Facts and uncertainty

Primary words must cover each complete source turn exactly under the explicit apostrophe/codepoint-lowercase token comparison. Verbatim source and original recognized spelling remain separate. Independent full transcript and all ordered nonoverlapping intervals must agree with each other and the complete source turn. Word points preserve measured timestamps and the existing grouping policy; unresolved point IDs remain explicit.

Uniqueness is evaluated over the complete source and all complete saved primary/independent transcripts, never only a chunk. Fact phrases and their last up-to-four terminal tokens must occur uniquely. A duplicate observed in an incomplete independent transcript also remains Uncertain. Missing global observations cannot prove uniqueness. Facts spanning multiple turns require every complete supporting turn and every intersecting chunk; a matching final timestamp alone is insufficient.

Offsets use integer arithmetic and exact decimal-to-sample ceilings, including original JSON millisecond number spelling. The conservative bound is the latest complete supporting primary/independent endpoint. `abs(primary_end - independent_end) * 4 <= sample_rate * 3` retains the inclusive 0.75-second rule. The tests distinguish exact tolerance from one sample outside, and preserve the 2,001-sample ceiling of a timestamp just above 250 ms at 8 kHz. Bounds deliberately include complete supporting turns; they are conservative observations, not word-level or final-delivery permissions.

Original Fail/Uncertain/Missing findings remain private and worsen fact outcomes even after new matching observations. Missing recipes/independence, repeated global text, partial/cross-cut evidence and unsupported context never acquire invented bounds. Complete source/PCM representation and synthetic saved text alignment do not prove actual full-chapter acoustic correctness, voice fidelity or production acceptance.

## Replay, private outputs and owner boundary

Route `Stage::Acoustic` with `chapter-import-unit.json` and `chapter-import-helper-record.json`. Supply the exact unit.files allowlist (including `chapter-import-index.json`) plus those aliases; recorded replay may add the three private output candidates. [Executable StageInput](../../../fixtures/generation/chapter-import-v2/stage-input.json) and [StageOutput](../../../fixtures/generation/chapter-import-v2/stage-output.json) show the complete boundary. The v1 `chapter-unit.json` route remains unchanged.

Rust independently checks raw bytes/receipts, verbatim source coverage, integer timing, global uniqueness, complete original-turn PCM reconstruction and exact helper-record contents. It never invokes the new helper or a provider. Candidate outputs are:

- `private/chapter-import-validation.json`: exact input/recipe/implementation bindings, retained findings and the independently reconstructed diagnostic unit result.
- `private/chapter-import-proposals.json`: shared `Vec<AcceptanceEvidenceProposal>`, currently empty.
- `private/chapter-import-evidence-inventory.json`: shared schema-1 `AcceptanceEvidenceInventory`, currently empty.

There is no payload/master/presentation/player/checker/image-reveal binding in an acoustic-only import. Generation therefore retains `acceptance-input-bindings` and `production-verifier-pending`; it does not invent AcceptanceFacts, image applicability, cue IDs or dummy hashes. Runtime can later compose the verified private observations into G20 proposals with all actual bindings. No package candidate or audience payload is emitted. G20 export tests still exclude private source, contexts, receipts, recipes, facts/evidence and reports.

Saved output FileRefs must match byte-identical reconstruction before any new candidate write. Cache keys include the recipe and exact reconstructed validation content, so distinct source/evidence identities cannot share the recipe-only key. Missing unregistered records return NeedsRepair without redispatch in replay/fixture/live modes. There are no journal proposals, library updates, accounting calls, catalog writes or provider calls. Standalone helper subprocesses run only in explicit offline compatibility tests; StageExecutor replay uses Rust functions and saved bytes.

Runtime must independently import/verify the complete original chapter and bundle before authoritative storage, register the new source closure separately, freeze epoch/unit/revision/input pins, capture bounded helper output through verified candidate reads, and own fanout, commit/restart/cancellation and private persistence. Neither index nor serialized verification is an execution capability. New helper registration remains pending; G19 registration is not repointed. A sliced source with nonzero global origin still needs an independently pinned source-slice relationship before shared playback-v1 source acceptance; current `missing_source_mapping` must remain unresolved. No public coordinate conversion is added here.

## Verification and compatibility

The first Python test demonstrated old G19's `source identity mismatch` for the correctly bound different chapter, then failed because the generic importer was absent. The first Rust StageInput test failed with `missing acoustic input: manifest.json`. Later red tests exposed erased inherited failure, unknown provider being accepted, lost independent ambiguity, late ambiguity-work rejection, missing replay slots, and an unlisted input being accepted. Each received a bounded fix and passed before the next increment. Exact commands/results, recipe/code/fixture hashes and example FileRefs are in [readiness](chapter-import-ready.json).

Regressions retain G16's four matched/three unresolved facts, 176 words/170 groups/six points; G17's 180-second master; exact G18 PCM mapping; G19's 600-second proof; and G20 scoped/adverse/private-export behavior. The generic capacity test uses a separate temporary 1,200-second, 19,200,044-byte native WAV, split under the 16 MiB file cap. Committed v2 media are only small, labeled synthetic fixtures.

Integration's [published acknowledgement](../integration/g20-compatibility-ack.json) permits the separate generation-owned dead-literal cleanup; the inert hook is now removed. The original G20 v1 fixture/receipt bytes remain frozen and checked with the archived accessor. A separately versioned [v2 scope/proposal example](../../../fixtures/generation/acceptance-proposals-v2/README.md) adopts I17's current matrix/checker metadata with unchanged scope/custom/adverse requirements. The exact cleanup hunk and owner request are recorded [separately](g20-literal-cleanup.md). No UI/shared schema/Git operations were performed. Independent semantic/listening/creative/device acceptance and production publication remain pending.
