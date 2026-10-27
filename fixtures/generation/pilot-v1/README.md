# Private synthetic voice-pilot examples

Six cases: `request-phase`, `saved-audio`, `dialogue`, `established-reuse`, `provider-double`, `review`. Each has an exact StageInput/StageOutput, input index and private candidates. Source/Direction files are borrowed by hash from frozen G22 cases where identical; all other test bytes are local. No source-workspace path is serialized.

The small 24 kHz mono PCM16 clips contain synthetic silence. Raw take envelopes declare the requested passage; actual spoken coverage, stable identity and listening are unproved. Catalog voices and reviews are fake, even where a configured real model name is pinned. The established Lite binding is synthetic reuse metadata, not a production-approved voice. The dialogue case has one genuine source utterance and explicitly lacks a second adjacent performance. Every case remains diagnostic/NeedsRepair.

```sh
PT_PILOT_EXPORT=fixtures/generation/pilot-v1 cargo test -p pt-production generation::tests::pilot:: --locked --offline
```

See [protocol and schemas](../../../docs/implementation/generation/pilot.md) and [readiness](../../../docs/implementation/generation/pilot-ready.json). This separate fixture version leaves G16–G22 evidence and all helper recipes unchanged.
