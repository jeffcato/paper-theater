# Offline acoustic StageExecutor

`GenerationExecutor` now routes `Stage::Acoustic` to `generation/acoustic.rs` through `pt-services-v1.2.0`. The implementation calls only RunServices: verified artifact reads, the allowlisted helper, execution-scoped candidate reads, bounded candidate writes and proposed journal entries. It opens no files/database at runtime, dispatches no provider, changes no library binding and creates no package/publication candidate.

Fixture execution:

1. Verify StageInput and its artifact bytes; check the acoustic manifest, source/audio bindings and complete required response/receipt inventory before any helper attempt. Input filenames follow the existing flat acoustic envelope (`manifest.json`, `source.txt`, `excerpt.wav`, saved report, units, transcribe and meta files).
2. Issue one `AlignAudio` operation. Its recipe is SHA-256 of the compiled product-local `helper.py` plus `adapters.py` bytes, matching runtime registration. The operation ID binds run/epoch/unit/revision and sorted selected artifact references. Timeout is bounded by the run wall-time allowance and 30 seconds; output is bounded to 16 MiB. The runtime owns authorization, scratch files and the durable zero-money/zero-provider-request attempt.
3. Require the matching operation ID and exactly one diagnostic `acoustic-replay.json` candidate. Read it through `RunServices::read_candidate`; independently verify its exact bytes, size, path, media type, provenance and recipe. Unknown candidates, stale epochs and forged metadata fail closed; there is no filesystem or invented-artifact-ID fallback.
4. Match every consumed input digest, preserve the cached word/group observations exactly, and validate full Unicode source reconstruction plus atomic point grouping using the shared read-along validator. Its transient source-domain validation record is never exported as a delivery read-along; delivery mapping remains separate.
5. Return the original helper candidate plus `acoustic-validation.json`, with complete input/pin/report/implementation identities, typed findings and a reuse decision. Return **NeedsRepair**: grouped read-along integrity does not establish independent per-fact verification, speech fidelity, narrator continuity or rendered review. No reveal permission or new production attestation is issued.

Recorded replay requires a verified committed `acoustic-replay.json` and its original source/response/receipt inputs. It does not call the helper, read an uncommitted candidate or emit a journal proposal. If an earlier `acoustic-validation.json` is supplied, its report/recipe/source/audio bindings are checked and its original failed/uncertain outcomes are retained. Replay proposes only a new validation summary; runtime retains ownership of any persistence. Missing replay data fails without dispatch. Live mode fails explicitly; it cannot fall through to provider spending.

The actual runtime integration test seeds the saved acoustic bundle using `Store::seed_acoustic_fixture`, composes `Runtime::new(..., Arc::new(GenerationExecutor)).with_offline_acoustic(...)` and executes Stage::Acoustic. Runtime persists both verified artifacts, records one zero-cost/zero-provider-request helper authorization and returns `stage_needs_repair`. It preserves all **176 source words, 170 highlight groups and six measured points** and leaves the production catalog empty.

Verification:

```sh
cargo test -p pt-production generation:: --offline
cargo test -p pt-production --test candidate_read_contract --offline
python3 -B -m unittest discover -s tools/generation -p 'test_*.py'
python3 tools/check_product_identity.py .
```

Results: **12 generation tests, 4 candidate API tests and 17 Python tests pass**. Seven acoustic tests cover the real subprocess/runtime flow, fixture preflight, replay without dispatch, corrupt/stale/unowned/multiple candidates, changed words/groups or source hashes, forged acceptance/reveal claims, preserved uncertainty and cancellation after helper return, candidate read and journal proposal. New tests first failed for the absent stage output, helper dispatch before missing-input rejection, and loss of Uncertain during recorded replay; each was corrected before the final green run. Existing Python helper bytes, media and receipts are unchanged.

`acoustic-stage-ready.json` records the exact consumed runtime/code/fixture hashes and commands. The registered helper/runtime implementation currently has owner-held working-tree changes; Agent 1 must include that completed runtime work when creating a clean integration commit. No worker Git mutation is performed for this task.

Next bounded integration belongs to Agent 2: route Stage::Acoustic through GenerationExecutor in the saved-fixture composition and provide its acoustic artifact bundle separately from package inputs. Current FixturePipeline delegates only Source/Package to generation, and both seeders use flat `manifest.json` / `source.txt` paths. A private stage-input bundle selector must keep these distinct; generation deliberately rejects conflicting paths. The completed direct acoustic stage already works without that UI/whole-run composition change.

Subsequent generation work needs an explicit fact inventory and conservative independent source-interval matching. The existing ambiguous/partial independent transcript is retained and cannot become accepted reveal evidence by replaying it.
