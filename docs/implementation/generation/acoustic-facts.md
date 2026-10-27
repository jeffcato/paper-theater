# Explicit source-fact acoustic matching

`Stage::Acoustic` accepts the private `source-facts.json` artifact through verified `StageInput` / `RunServices` reads. [The diagnostic inventory](../../../fixtures/generation/acoustic-facts-v1/source-facts.json) extends the saved acoustic bundle without changing its media, observations, receipts or helper recipe. The output is private `fact-evidence.json`, referenced and hash-bound by `acoustic-validation.json`. This is narration-domain evidence; it grants no reveal permission or production acceptance.

The inventory uses schema version 1 and `excerpt_unicode_scalars`: half-open Unicode scalar offsets into the exact `source.txt` bytes. It pins source, narration, manifest and primary provider recipe hashes, sample rate, and independent receipt paths/digests. `required_fact_ids` is independent of the definitions so missing facts remain visible. Each fact has explicit `source_spans` and supporting `interval_ids`; each interval names its complete source span and ordered `primary_unit_ids`. Unknown fields, duplicate identities and invalid ranges fail closed. No semantic fact extraction is inferred.

Matching verifies the saved primary request/response/audio/recipe bindings and independently pinned receipt/response/model identities. Unit PCM must equal the declared slice of the full narration waveform. Both sources must cover the complete supporting interval with matching text; a suffix near an expected endpoint cannot substitute for missing prose. Full interval and terminal occurrences must be unique in source, primary and independent observations. Independent turns must be ordered, nonoverlapping and inside the actual narration frames. Case, punctuation and internal apostrophes are normalized; names, spelling and repetitions are preserved. The existing read-along `Em`/`M` substitution is insufficient for fact verification.

Raw decimal timestamps are converted with exact integer ceiling in the original 24 kHz narration domain. Agreement requires `4 * abs(primary_end - independent_end) <= 3 * sample_rate`, retaining the existing 0.75-second rule. The conservative bound is the later measured end. A fact can span several fully verified intervals; every spoken source character must be covered and its bound is the maximum of all supporting bounds. Punctuation between adjacent intervals does not create missing speech. Partial intervals, repetitions, missing definitions or bindings, and excessive disagreement yield no bound.

The diagnostic inventory produces:

| Fact | Outcome | Primary / independent end | Conservative narration sample |
|---|---|---|---|
| `cellar-instruction` | Pass | 545280 / 546720 | 546720 |
| `Dorothy-urged` | Pass | 506640 / 498720 | 506640 |
| `screaming` | Pass | 506640 / 498720 | 506640 |
| `urged-cellar` | Pass, two supporting intervals | maxima 545280 / 546720 | 546720 |
| `strange-event` | Uncertain | repeated “thing” in independent interval | null |
| `balloon-ascent` | Uncertain | repeated words in independent interval | null |
| `unprovided-fact` | Missing | required definition absent | null |

Each result includes explicit source spans, interval identities, outcomes/reasons and a nullable bound. Passing intervals include both measured starts/ends and exact model/request/response/receipt/audio/recipe references. The report pins the inventory, normalization report, manifest and matcher implementation hashes. Typed `acoustic-fact:<id>` findings are returned through `StageOutput`. Imported observations and synthetic negative-test mutations remain diagnostic. The independent imported receipt has no original harness recipe; its complete bytes are pinned, and broader production verification remains missing.

All **176 source words, 170 highlight groups and six measured points** remain byte-equivalent to the normalized cached observations. No final presentation-domain bound is emitted: resampling, trims and gaps need a separately pinned mapping. Image applicability, independent production acceptance, speech fidelity, narrator continuity and rendered reviews remain unresolved, so the stage always returns **NeedsRepair** and an empty reveal-permission list.

Recorded replay verifies the committed normalization report and original inputs, recomputes byte-identical fact evidence, and checks any previous inventory/result/recipe bindings. Changed facts invalidate the old result; they never rewrite recordings or receipts. Existing Fail/Uncertain findings survive recomputation. Replay performs zero helper calls, candidate reads or journal proposals. The real runtime test persists the fixture result through its registered helper, reads the committed artifacts through public Store APIs, drops the helper-enabled runtime and reconstructs them through the replay executor. Provider dispatch panics in the replay services; helper/candidate-read/journal counters remain zero and runtime snapshot/journal are unchanged.

The public runtime seeder freezes a bundle on its first call. Compose the acoustic directory and the separate fact inventory into one temporary input directory **before** calling `seed_acoustic_fixture`; a second call verifies existing inputs instead of appending files. The helper still receives exactly its original 42 consumed files. No helper allowlist change or provider access is needed.

Verification: **53 production unit tests (21 generation / 32 runtime), 4 candidate API tests and 17 Python tests pass**, plus formatting and product-identity checks. New tests cover shared enclosing turns, multi-interval facts, complete/partial intervals, ambiguous full/terminal occurrences, exact 0.75-second acceptance and one-sample rejection, hash/recipe/request/audio/model mismatches, wrong PCM placement, name-normalization limits, changed replay inventories and preservation of adverse findings. See [acoustic-stage.md](acoustic-stage.md) for commands and [acoustic-stage-ready.json](acoustic-stage-ready.json) for exact identities. Only Agent 1 stages/commits the generation-owned changes.
