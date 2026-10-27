# G24: bounded private Speech compiler

Stage::Speech revalidates Source/Direction/Pilot and compiles verbatim source units, content-only Speech proposals, saved native-take manifests and primary-transcription plans. It performs no synthesis, transcription, audio edit, provider/helper dispatch, library/accounting mutation or publication, including in Live mode. Diagnostic Pilot approval remains false and every output remains NeedsRepair.

## Frozen predecessor and source integrity

[G23 source/patch pins](pilot-g23-freeze.json) were published for I20 before changing the generation router/test registration. The patch reconstructs all seven exact completed G23 source files from pinned G22 baselines. G23 modules and all prior fixtures/receipts/helper recipes remain unchanged. G24 lives in new Speech modules plus the generation router/test registration.

Runtime supplies the original Pilot StageInput and committed plan. An in-memory verification service re-executes the actual Pilot compiler with its original inputs and observer identity, forcing Replay and refusing provider/helper/journal/library effects. Pilot in turn revalidates Direction and Source. The resulting Pilot-plan bytes must exactly equal the committed input: setting `bulk_speech_approved: true`, changing source/catalog/registry/voice data or supplying a claimed pass cannot bypass verification. The verifier produces only transient CandidateFiles, never committed ArtifactRefs.

The Speech context pins book/chapter, Pilot, binding snapshot and recipe. Canonical speaker/model/voice/catalog/language/accent/baseline/reference and established binding must match the verified Pilot and actual supplied ArtifactRefs. Missing/stale/mentioned-only identities remain unresolved with no replacement voice. Existing recording/model/voice identities are retained; no regeneration or upgrade is attempted. Runtime still owns authoritative registry and approval provenance.

## Ordered units and separate acting

Units preserve half-open Unicode scalar spans, original turn IDs, narration/dialogue attribution and every source character in order. Text concatenation is checked against complete source. Segmentation uses conservative sentence/paragraph/speaker boundaries, UTF-8 byte bounds and valid source-scoped Pilot acting boundaries. Known ambiguous abbreviations are not treated as a convenient byte cut. Unsupported sentence sizes stay intact in unresolved units; a unit-count limit retains the entire remaining source in an unresolved unit. Nothing is truncated to satisfy a request limit.

Stable identity and baseline remain separate from source-scoped acting. Valid scoped acting is retained as intent and can split a unit; it never enters `text`. Conflicting acting stays unresolved. A source citation verifies occurrence/scope, not artistic interpretation or audible performance. Every request is one-speaker, exact prose and unapproved. No vocal tags, invented dialogue, fixed inter-take silence, trims or gain changes are inserted.

## Private schema 1

Typed input, binding, observation and take-envelope records reject unknown fields. Definitions are in [Speech](../../../crates/pt-production/src/generation/speech.rs), [verification](../../../crates/pt-production/src/generation/speech_verify.rs), [units](../../../crates/pt-production/src/generation/speech_units.rs), [takes](../../../crates/pt-production/src/generation/speech_takes.rs) and [transcription](../../../crates/pt-production/src/generation/speech_transcription.rs).

| Record | Fields and meaning |
| --- | --- |
| `private/speech-context.json` | `book_id`, `chapter_id`; `pilot`, `pilot_input`, `bindings`, `recipe` FileRefs; `speech_provider`, `transcription_provider`, `transcription_model`; `estimate` Usage, `timeout_ms`, `limits`, `request_payloads`, `observations`. No caller run/epoch/attempt/credential/repair or approval authority. |
| Limits | `max_unit_text_bytes`, `max_units`, `transcription_max_wav_bytes`, `transcription_max_seconds`, `transcription_max_segments`; `transcription_max_batches` defaults to 128. All are checked within the hard envelope below. |
| Binding snapshot | Book/revision plus `entries[{speaker,canonical_id,provider,model,voice_id,catalog:ArtifactRef,language,accent,baseline,reference:ArtifactRef|null,established:ArtifactRef|null}]`. Matching pinned data does not establish approval. |
| Source unit | Stable ID, original turn/speaker/kind, scalar span, exact text and structural `request_ready`. An unresolved remainder has null speaker/turn; the prose remains present. Structural readiness never grants execution. |
| Speech request | Source hash, Direction/Pilot/bindings/recipe identities, exact unit/text, stable performer/model/voice/catalog/language/accent/baseline and separate acting/intents; `bulk_speech_approved: false`. |
| Saved observation | Existing G23 observation shape: schema/provenance, original request/response ArtifactRefs, optional original issued/result v2 records, original findings. Duplicate request observations are ambiguous and rejected. |
| Saved native-take response | Schema/provenance; request/recipe/source/bindings SHA-256s; `unit_id`, `turn_id`, scalar `span`, UTF-8 `text_sha256`, speaker/model/voice; original `audio` ArtifactRef and declared `format:{sample_rate,channels,bits_per_sample,frames}`. |
| Take manifest | Exact source/turn/voice/recipe/request/raw/observation/audio identities; actual native format and `[0,frames]`; deterministic reuse key. `global_narration_span`, assembled narration and delivery master are null. Audible fidelity stays Missing. Original results/charges/findings remain private observations. |
| Transcription plan | Compatible native-format groups, exact original audio hashes and native-to-proposed-batch sample intervals, rational duration, bounded planned WAV size, cross-cut uncertainty and explicit unplanned ranges. No actual batch audio, submission proposal, measured intervals or fact bounds exists. |

## Request commits and saved-take ingestion

Generation returns exact request CandidateFiles and a content-only handoff. Runtime must verify/commit desired request bytes, supply genuine ArtifactRefs and explicitly register their FileRefs in `request_payloads`. Only then does Speech construct shared ProviderProposalV2 of kind Speech. Merely recording a candidate cannot advance replay's phase. No CandidateFile is converted into an ArtifactRef.

Proposals preserve recipe/provider/model, exact payload and verified dependencies. They have no issuance/credential/repair authority. This increment has no execution path: unresolved Pilot/listening/continuity and runtime bulk grants remain blockers even when service methods are available. Defaults/unavailable support produces typed NeedsRepair, with no legacy/helper fallback.

Saved results must bind the exact proposal, original run/epoch/unit observer, issuance, request, result, raw response and native output. Foreign epochs, wrong model/voice/turn/source/recipe/request, stale artifacts and mismatched physical frames fail closed before any external candidate write. Incomplete/ambiguous/obsolete observations retain original metadata and usage and cannot become usable takes or trigger a replacement purchase.

Actual PCM16 WAV structure, rate/channels and frame count are validated through the existing public stateless decoder. Each original WAV remains unchanged. Compatible saved takes are reused; no resampling, trimming, joining or loudness normalization occurs. Format/frame success and a declared passage hash do not prove that any words were spoken. Independent acoustic/fidelity/listening checks remain required.

## Unhinted primary-transcription plans

Only verified saved native takes enter the plan. Consecutive source coverage and equal sample rate/channels/PCM16 can share a proposed batch; incompatible formats or missing source coverage start a new domain. Every segment retains its original ArtifactRef/hash, native sample interval and explicit batch-local interval. Durations use integer frame counts and rational sample-rate denominators. These offsets describe a future exact PCM-copy recipe, not an existing narration timeline.

PCM budgets include the 44-byte canonical PCM16 WAV header. The planner uses the smaller frame allowance from byte and duration limits, bounds segment/request fanout, and preserves explicit unplanned native ranges when limits cannot cover the remainder. Interior native cuts retain cross-cut uncertainty; no word boundary or fact eligibility is inferred.

The model payload is strictly constructed as `{"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":null}`. There is no expected source, transcript, hint list, speaker name, lexicon or book prose in it. Original source matching stays local in the private manifests. Runtime must materialize/verify bounded native PCM copies, pin their actual artifact and a separately reviewed copy/transcription recipe, then commit a real unhinted request before any later transcription proposal. This increment creates no PCM copy, transcription call, timestamp, common sample clock or reveal bound. Larger 60–90-second request sizing remains an unproved pilot proposal.

## Bounds, replay and handoff

Input bounds: 64 references/64 MiB aggregate; raw/clean source 16 MiB each; `.wav` with matching MIME 8 MiB; all other metadata and each request/response/candidate JSON 1 MiB. Changing metadata MIME cannot enlarge its limit. Runtime request/observation inventories are capped at 32 each. Unit text is configured within 1..1 MiB and unit count within 1..1024. Transcription planned WAVs are 45 bytes..16 MiB, duration 1..90 seconds, segments 1..64 and batches 1..256. A tiny incompatible budget stays unresolved instead of inventing a format. No existing helper recipe or chapter-master limit changes.

Replay reconstructs exact requests/proposals/unit/take/transcription/handoff bytes and cache keys, with the same explicit payload-registration phase and original observations/charges. Original receipt/audio files remain inputs and are not rewritten. Compiler source hashes bind the planning implementation separately from the runtime recipe. Changes to source/turns, Pilot, bindings/catalog/model/voice/baseline/acting, recipe, request/raw/audio or native format invalidate affected records; cache reuse cannot cross those changes silently. Runtime must audit any cross-epoch reuse mapping.

All context/catalog/raw/proposal/unit/take/transcription records are private, including public-image aliases of their hashes. StageOutput grants no library, account, catalog, reveal, acceptance or publication authority. Actual source provenance, Pilot approval, bulk grants, real listening/continuity, safe join/edit/assembly recipes, unhinted measured transcription, independent acoustic fact coverage and final presentation/master review remain pending.

The [readiness record](speech-ready.json) contains exact examples, red/green commands/results, owned paths and hash pins. The [runtime request](speech-runtime-request.json) describes commit/registration, immutable receipt/native-audio preservation and later assembly/transcription work. No real provider calls or UI/Git operations belong to G24.
