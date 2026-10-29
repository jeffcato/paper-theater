# Private primary-transcription reference cases

Seven cases cover the request commit boundary, measured points/intervals, global repeated-word ambiguity, exact decimal samples, join ambiguity, native cut uncertainty and actual R19 receipt consumption. Each supplies StageInput/StageOutput, an exact input index and private candidates. Identical older Source/Direction/Pilot/Speech bytes are borrowed by hash rather than rewritten.

Synthetic fixtures use distinguishable deterministic PCM and manually supplied synthetic recognition. They do not establish spoken fidelity, listening, independent fact evidence or production approval. `runtime-r19` consumes actual server-committed audio and its genuine copy receipt through public backend APIs; its source recordings/adapter metadata remain diagnostic/synthetic and no model call occurs. Runtime alone materializes and commits. All cases retain NeedsRepair and empty reveal permissions.

```sh
PT_PRIMARY_EXPORT=fixtures/generation/primary-transcription-v1 cargo test -p pt-production generation::tests::primary_transcription:: --locked --offline
```

The R19 example contains that isolated runtime's actual run-owned refs; an explicit re-export can create different refs. Recorded replay preserves exact refs within the frozen case. See [protocol](../../../docs/implementation/generation/primary-transcription.md), [R19 handoff](../../../docs/implementation/generation/primary-transcription-handoff.json) and [readiness](../../../docs/implementation/generation/primary-transcription-ready.json). This version leaves all G16–G24 fixtures and helper recipes unchanged.
