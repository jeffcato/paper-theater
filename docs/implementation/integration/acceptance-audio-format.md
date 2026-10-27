# I19: independently verified delivery WAV format

Services `pt-services-v1.8.0` execute `audio-format-review` over actual hash-verified package master bytes. Report/checker advance to v4 and the required matrix to v3. Five independent checks now exist; **32 excerpt/chapter verifiers remain unimplemented**. Format validity does not prove speech, listening, voice continuity, joins, loudness, mix quality, provenance or an accepted release. `production-verifier-pending` and all live/accounting/device/publication gates remain.

## Shared inspector and R11 reuse

Rust exports a contracts-owned stateless inspector with no production/runtime dependency:

```rust,ignore
inspect_pcm16_wav(&mut (impl Read + Seek), size: u64) -> ContractResult<Pcm16WavInfo>
inspect_delivery_wav(&mut (impl Read + Seek), size: u64) -> ContractResult<Pcm16WavInfo>
```

`inspect_pcm16_wav` preserves R11's native admission: classic RIFF/WAVE encoding 1, PCM16, one/two channels, 8..192 kHz, fmt length 16 or 18 (zero extension), and exact valid chunks. Native files at/below 32 MiB may be mono or another supported rate. Larger WAVs through 256 MiB must be stereo/48 kHz. `inspect_delivery_wav` additionally requires stereo/48 kHz at **every** size, including tiny files. The 32 MiB general artifact/native envelope, 256 MiB master envelope, helper bounds and 512 MiB cumulative acceptance-read budget remain distinct and unchanged.

Inspection checks the physical reader length, RIFF length/file extent, bounded complete chunk headers/extents/padding, a single fmt before data, one nonempty frame-aligned data chunk, PCM encoding/bits, channel/block alignment, byte rate and maximum 1,024 chunks. Unknown bounded ancillary chunks remain permitted, including odd padding and chunks after data, as R11 permits. Multiple data/fmt chunks, RF64/extensible/float formats, truncation, trailing bytes outside RIFF and invalid framing fail closed. Physical frames are **data bytes / block alignment**, not scalar interleaved samples or `(file_bytes - 44) / 4` when ancillary chunks exist.

The inspector uses fixed buffers and seek traversal, returns metadata, and rewinds on success. It does not read/decode PCM samples into floating-point buffers. An exact-256-MiB sparse-file test performs bounded header reads; over-limit admission is rejected without a master-sized allocation.

R11 remains server-owned and is not edited. A production-side, integration-owned conformance target compares the public R11 `validate_artifact_bytes` behavior against the new native inspector for 32 concrete native/delivery cases. Its source input is `crates/pt-production/src/runtime/media.rs`, SHA-256 `2bfe0b644c0559faaeebcbe9f72d303fe8a8438c579d87b879edad4e709b484f`. Contracts tests do not import runtime source or depend on production. The extra physical-length check agrees with R11's verified-file/slice callers, which already bind actual length.

## Exact server wrapper adoption

The server owner should replace only the stateless body of `runtime/media.rs::validate_wav` with this delegation:

```rust,ignore
pub(crate) fn validate_wav(reader: &mut (impl Read + Seek), size: u64) -> Result<()> {
    pt_contracts::inspect_pcm16_wav(reader, size)
        .map(|_| ())
        .map_err(|_| invalid())
}
```

Keep `artifact_limit`, `validate_artifact_size`, verified open-file/hash handling, rewind/range behavior and private runtime error boundary intact. Do **not** use `inspect_delivery_wav` for all runtime/helper WAVs; that would incorrectly reject existing small mono/native-rate narration. Use the stricter API only for explicit delivered masters. No new crate, parser dependency, reverse dependency or duplicated evolving policy is requested. Native conformance remains executable before/after wrapper adoption:

```sh
cargo test -p pt-production --test wav_conformance --locked --offline
cargo test -p pt-contracts --test wav_inspector --locked --offline
```

The production manifest only adds the integration-owned test target at `tools/integration/wav_conformance.rs`; no runtime/generation module is changed. Wrapper adoption remains a separate server-owned change. Until it lands, the conformance corpus guards equivalent native semantics; the shared inspector is authoritative for the new independent acceptance check.

## Independent acceptance execution

The existing `verify_acceptance` and `verify_acceptance_v2` APIs inspect `manifest.entries.audio` while its already hash/size/path-verified payload buffer is borrowed. No second master-sized copy or decoded buffer is created for format inspection. The actual written manifest must match the checked document, and the actual timeline FileRef is read/verified against the loaded timeline. Only the supported playback-v1 manifest/timeline schemas establish the delivery count domain. An inspected PCM16 master passes only when:

- The selected media label is `audio/wav`, actual channels are 2, actual rate is 48,000 and actual bits are 16.
- Manifest and timeline rates both equal 48,000.
- The physical frame count equals **both** manifest and timeline delivery sample counts.

The executed target is `{kind: audio, domain: delivery, span: [0, manifest.sample_count]}`. MIME, suffixes, claimed duration, verified-profile labels and candidate passing findings cannot supply execution or substitute another master. Missing media produces Missing; changed bytes, unsafe paths, malformed/ambiguous RIFF and wrong domains/counts fail. Immutable read pins include the actual master/timeline; later mutation makes `recheck_inputs()` fail. Retained Fail/Uncertain observations still worsen the row even when format itself is valid.

Report v4 has optional private `delivery_audio: DeliveryAudioObservationV1`. When native PCM inspection and candidate/timeline binding succeed, it contains only actual `Pcm16WavInfo` (encoding, channels, rate, bits, block/byte rates, file/data extents and physical frames), exact master/timeline hashes, media-label match and manifest/timeline rate/count metadata. Malformed/unavailable media does not acquire invented format values. A passing row must contain the observation, correct domain/counts and consumed master/timeline hashes. No source, raw audio, raw receipt contents or filesystem paths are added to report metadata or audience payloads.

## Version migration and owner handoffs

The immutable v1/v2 matrices and report/checker v1/v2/v3 receipts remain valid under their original expected identities. The actual prior v3 audit fixture is preserved separately. Matrix v3 changes only `audio-format-review`: implemented becomes true and target changes from candidate to delivery. Required IDs and scope membership remain 3 diagnostic defaults/37 excerpt/37 chapter requirements. Diagnostic code may explicitly declare/add the audio check; production scopes require it. Omitting declarations still fails coverage.

Report/checker v4 pairs with matrix v3. Archived report v3 still pairs with matrix v2 and cannot claim the new audio execution or observation. Source-slice metadata remains schema 1 and works in report v4; policy schemas 1 and 2 and playback coordinates are unchanged. Rust constructors of report records now supply `delivery_audio: None` when absent; older JSON decodes through serde defaults. Archive validation does not relabel execution. JSON validation is metadata consistency, never an authenticated verifier capability.

Runtime R15 refreshes its trusted expected `acceptance_binding_checker()`/identity artifact to v4, keeps independently pinned policy/source-slice inputs, invokes the actual verifier, rechecks immutable inputs and persists report v4 privately. Generation adopts matrix v3/checker v4 in its owned context/proposal fixtures and uses the delivery target `[0,N]` for any proposed audio evidence. Old candidate-target/pass-only records cannot become actual audio results. Candidate-supplied metadata, autonomous labels or a deserialized report never authorize activation.

Native narration/sample mappings and helper compatibility remain separate; do not retime/resample existing saved media to obtain a new receipt. Frozen v1/v2 delivery hashes and original diagnostic failures are preserved. Runtime/generation/publication/accounting/device adoption work remains owned by those lanes. No UI edits, provider calls, credentials or live dispatch occur here.

## Red/green evidence and actual checks

The initial saved-master test failed because `audio-format-review` was unimplemented and targeted the candidate. Porting R11's stateless behavior and performing actual borrowed-byte inspection made it execute. The next test deliberately changed declared delivery counts while preserving the physical WAV; it failed because format-only inspection still passed. Binding measured frames to both manifest and actual timeline made both mismatch cases fail with `audio_frame_count_mismatch`. A further red case showed an unsupported timeline schema could supply apparently matching numbers; rejecting unsupported count-domain schemas made that fail closed.

Regression cases cover real temporary PCM and saved audio, MIME-only garbage, mono/rate/encoding/bits/block/byte-rate errors, RIFF/chunk extents, fmt18, padding/ancillary chunks, duplicate/misaligned/empty data, chunk traversal limits, wrong/missing/unsafe/swapped masters, recheck mutation, provenance/pass-label forgery and retained adverse findings. R11/native/delivery conformance and exact/over-limit streaming tests run through public interfaces. Cross-language records include 41 actual-media/archive/forgery reports and all 32 native/delivery variants, plus the retained contract suites. Exact code/checker/source/test/CI pins are in `acceptance-audio-ready.json` and `pr-ci-results.json`.
