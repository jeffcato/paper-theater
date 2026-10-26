# Agent 3 integration requests

- Consuming `pt-contracts-v1.0.0`, `pt-services-v1.0.0`; no contracts-ready.json freeze published yet.
- Current shared fixture fails `validate_playback`: `cue_scene` at `nod-em`, "cue outside authored scene slot". Reproducer: `cargo test -p pt-production generation::tests::diagnostic_candidate_uses_saved_payloads_and_exposes_missing_acceptance --offline`. Agent 1 owns fixture repair; generation does not retime or omit the cue.
- Runtime `Store::commit_stage` currently rejects any `package_candidate`. Please support the diagnostic proposed package boundary, preserving NeedsRepair and preventing production publication. Generation exposes candidates only through RunServices.
- Runtime initial source stage currently receives no source artifacts. Please supply verified source/raw/frozen inputs and saved package inputs through StageInput; generation will not reach into runtime storage.
- `CandidateWrite` has no dimensions but `FileRef::validate` requires dimensions for image/png. For future new image candidates add `dimensions: Option<[u32;2]>` to CandidateWrite in Rust/TS and propagate through RunServices::write_candidate. Current package assembly reuses input media without rewriting it.
- Register bounded offline helper recipe for `tools/generation/helper.py` through runtime's helper allowlist. JSON stdin/stdout, 64 KiB request and 16 MiB response/file cap; zero credentials or provider transport. No helper dispatch is claimed integrated yet.

Only Agent 1 stages/commits in this shared repository. This lane's explicit paths: `crates/pt-production/src/generation`, `tools/generation`, `fixtures/generation`, `docs/implementation/generation`. Please integrate these as a clean reviewed commit after checks pass; no worker Git mutation has occurred.
