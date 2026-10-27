# Private Direction v1 examples

All model responses, provider metadata and creative choices here are labeled synthetic/diagnostic and remain NeedsRepair. Provider doubles exercise shared v2 consistency; no provider transport or credential access occurs. There is no accepted canonical voice/image, research result, final master, reveal permission or audience export.

Cases cover a complete dialogue source, a complete narration-only source, the request-before-commit phase, retained canonical identity/adverse findings, a provider-v2 double, and the selected real Oz/Wells sources. Each case provides StageInput/StageOutput, an input index and private candidate bytes. Materialize exactly the indexed FileRefs; selected-book raw/clean/frozen inputs refer to existing generation-owned source files instead of duplicating editions. Test-only `committed:test:*` IDs illustrate a runtime double, never real committed authority.

`response-schema.json` is the private model response format embedded in every request. Citation spans use file-local Unicode scalar indices. Unknown shapes and stale identities fail closed; unsupported source claims remain findings. A model may omit the optional request-hash echo; the observation/provider records bind the actual request.

Reproduce offline:

```sh
PT_DIRECTION_EXPORT=fixtures/generation/direction-v1 cargo test -p pt-production generation::tests::direction:: --locked --offline
PT_DIRECTION_EXPORT=fixtures/generation/direction-v1 python3 -B tools/generation/run_direction_tests.py
```

The second command compiles the real public production library and runs the same owned test source in a temporary harness, useful during unrelated runtime unit-test edits. It does not substitute a production implementation or shared contracts. Exact two-phase issuance/replay owner work is in `docs/implementation/generation/direction.md`.
