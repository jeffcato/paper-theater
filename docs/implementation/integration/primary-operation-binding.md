# Primary operation provenance binding

INT-03 shared boundary precursor; no service revision change. Runtime adoption, matrix wiring, complete verification and exact-head CI remain pending until separately recorded. This binding describes permitted provenance; deserialized JSON and successful validation grant no issuance, execution, recovery or publication authority.

`PrimaryProvenanceV1` binds one primary audio-only transcription record to its committed provider-v2 operation attempt and recorded result. It carries the operation attempt ID and request digest, the exact primary request artifact (whose bytes must match the supplied primary content), and the recorded response digest, which must equal the committed result's response (including null when nothing was recorded). Forged attempts, stale requests, tampered issuance, altered primary bytes or substituted responses fail.

Rust: `PrimaryProvenanceV1::validate_for_operation(op, result, primary_bytes)`. TypeScript: asynchronous `validatePrimaryProvenanceV1` with the same four arguments. Both recheck the committed operation, require the result itself to validate for that operation, and compare exact digests. These functions verify consistency, not whether a trusted runtime actually executed or a provider actually answered.

Six exact-byte Rust/TypeScript cases agree; forged and stale bindings stay rejected. The larger primary-transcription acceptance check, source-scope completion and acoustic/provenance gates remain open. `production-verifier-pending` remains mandatory.
