# Offline acoustic StageExecutor

`GenerationExecutor` routes `Stage::Acoustic` through RunServices: verified artifact reads, the allowlisted offline helper, execution-scoped candidate reads, bounded candidate writes and proposed journal entries. Generation opens no runtime files/database and dispatches no provider. The current consumed boundary is `pt-contracts-v1.0.0` / `pt-services-v1.3.0`; the helper/candidate signatures introduced by v1.2.0 remain compatible.

Fixture execution verifies source/audio/manifest bindings and the complete response/receipt inventory before one bounded `AlignAudio` helper operation. The helper recipe remains SHA-256 of compiled product-local `helper.py` plus `adapters.py`. Runtime owns authorization, scratch files and the durable zero-money/zero-provider-request attempt. Generation reads the single diagnostic `acoustic-replay.json` candidate through `RunServices::read_candidate`, verifies all consumed hashes, preserves cached word/group observations exactly and validates complete Unicode source reconstruction plus atomic point grouping. Unknown/stale/forged candidates fail closed.

An explicit private `source-facts.json` now produces [complete per-fact matching](acoustic-facts.md) and `fact-evidence.json`. Matching verifies raw observations, independent model identity, receipt/response/recipe/source/audio hashes, exact PCM placement, complete unique source intervals, complete fact coverage and the original 0.75-second rule. It uses the later measured endpoint in the narration sample domain. The diagnostic fixture matches four facts and leaves three unresolved; the original **176 words, 170 groups and six points** remain unchanged. Missing inventories produce a typed Missing finding. Helper bytes, original media and receipts are unchanged.

Fixture outputs are `acoustic-replay.json`, optional `fact-evidence.json` and `acoustic-validation.json`. The validation summary binds input/pin/report/implementation identities, typed findings and a reuse decision. All outputs remain diagnostic and **NeedsRepair**. Per-fact acoustic arithmetic does not establish final presentation mapping, image applicability, speech fidelity, narrator continuity or rendered acceptance. No reveal permission, library binding or publication candidate is issued.

Recorded replay requires the committed normalization report and original observations. It recomputes identical fact evidence, checks previous inventory/result/matcher bindings, and preserves prior Fail/Uncertain findings. It calls no helper/provider, reads no uncommitted candidate and emits no journal proposal. Changed inventory, implementation or saved results invalidate their matching result. Missing replay data fails without dispatch. Live mode fails explicitly without a spending fallback.

The actual runtime test seeds one composed acoustic/fact bundle, executes the registered helper through Runtime and persists all three candidates with one zero-cost/zero-provider-request helper attempt. Replay reconstructs byte-identical fact evidence from those committed artifacts after the helper-enabled runtime is dropped. Runtime snapshot/journal stay unchanged and the production catalog stays empty. Separate protocol tests verify stale bindings, uncertainty preservation and cancellation boundaries.

Verification:

```sh
cargo test -p pt-production --locked --offline
cargo test -p pt-production generation:: --offline
cargo test -p pt-production --test candidate_read_contract --offline
python3 -B -m unittest discover -s tools/generation -p 'test_*.py'
python3 tools/check_product_identity.py .
rustfmt --edition 2024 --check crates/pt-production/src/generation/{acoustic.rs,acoustic_facts.rs,acoustic_tests.rs,mod.rs}
```

Results: **53 production unit tests (21 generation / 32 runtime), 4 candidate API tests and 17 Python tests pass**, plus formatting and identity checks. Contract-first tests exposed missing matching output, missing-receipt classification, union coverage across complete intervals, stale replay inventories and overwritten adverse findings; each passed after its implementation fix. Exact consumed hashes/results are in [acoustic-stage-ready.json](acoustic-stage-ready.json).

Agent 2's [stage-specific bundle routing](../runtime/acoustic-integration.md) is complete. Its first seed must include the explicit fact inventory to exercise this extension; seeding twice does not append to a frozen bundle. The next bounded work is to supply actual authored fact scopes and a pinned narration-to-presentation mapping, retaining unresolved cached intervals. Broader production evidence remains required. Agent 1 owns the shared Git index, integration commit and push; this task changes only generation-owned paths.
