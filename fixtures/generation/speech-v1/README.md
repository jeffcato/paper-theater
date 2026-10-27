# Private Speech compiler examples

Cases cover planning, saved native takes, incompatible formats, source-supported dialogue, recorded v2 metadata, bounded/cross-cut transcription, explicit unplanned remainders and source-scoped acting. Every case has StageInput/StageOutput, an exact input index and private candidates. Unchanged Source/Direction/Pilot files and audio can be borrowed by hash from older product fixtures.

All new audio is small synthetic silence. Model names are configured identities; voices, receipt usage and selections are test data. Declared source coverage and native WAV structure do not prove spoken words, continuity or approval. Pilot/bulk approval remains false, all outputs remain NeedsRepair, and no synthesis/transcription occurs.

```sh
PT_SPEECH_EXPORT=fixtures/generation/speech-v1 cargo test -p pt-production generation::tests::speech:: --locked --offline
```

The optional [owned test runner](../../../tools/generation/run_speech_tests.py) links the real compiled library during unrelated native cfg(test) edits. It substitutes no product/shared module or runtime implementation. See [schemas and handoff](../../../docs/implementation/generation/speech.md) and [readiness](../../../docs/implementation/generation/speech-ready.json). All older evidence and helper recipes remain unchanged.

If unrelated runtime production code cannot compile, explicit `--cached-runtime` compiles current generation source into a temporary library against the last real compiled runtime API and prints that binary's SHA-256. This checks current source behavior; it does not certify the unfinished native runtime build. The normal runner never silently selects that mode.
