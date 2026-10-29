# Audio-only transcription wire role

INT-02/INT-03 shared boundary; no service revision change. Runtime adoption, exact-head CI and the server/generation handoff return remain pending until separately recorded. This role grants no issuance, dispatch, recovery or publication authority.

`MAX_NATIVE_AUDIO_WIRE_BYTES` (22,373,720) covers exactly the base64 expansion of the 16 MiB verified PCM batch (22,369,624) plus 4 KiB bounded JSON metadata. Ordinary native wire stays capped at 1 MiB; the metadata record stays at 64 KiB. The role applies only when the committed operation is kind `transcription`, provider `gemini`, model `gemini-3.5-transcribe`; any other kind, provider or model keeps the 1 MiB wire ceiling.

Rust: `ProviderNativeExchangeV1::validate_bytes` selects the audio ceiling under that role and keeps the ordinary cap otherwise; wire bytes must still digest-match the bound request. TypeScript: `validateProviderNativeExchangeV1` with the same role rule and ceiling export. Both verify the committed issuance, the bound digests and the exact ceiling; neither truncates, widens generic limits, or performs an implicit second request.

Seven new exact-byte cases agree (30 s / 90 s representative bodies, exact ceiling, ceiling-plus-one, wrong role, wrong model, changed audio identity) plus an exact wire-replay check. 46 native cases pass in both languages. Server owns dispatch, cancellation, accounting and durable reads/commits; generation owns the transcription codec and payloads. Caps, selection rules and commands return to those lanes with this handoff.
