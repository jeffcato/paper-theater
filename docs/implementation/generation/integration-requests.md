# Agent 3 integration requests

Consumed frozen `pt-contracts-v1.0.0`, `pt-services-v1.0.0` and shared fixture digest `5a662fb9d1687fd61a692acdd4265bc12aac9d5bfacb8e5f75e58b472dbf776c`. Rendering profile `paper-player-r186-v1` remains selected/unverified.

Resolved during integration:

- Agent 1 fixed the pre-freeze `nod-em` cue-slot failure. Generation preserves that authored cue and its unchanged media/timing.
- Agent 2 supplies source and saved package inputs through StageInput and stores the proposed diagnostic PackageCandidate. The real-generation runtime test passes, retaining NeedsRepair and preventing catalog publication.

Pending exact boundary proposals:

1. Add `dimensions: Option<[u32;2]>` to `CandidateWrite`, mirror it in TypeScript, and propagate it through RunServices::write_candidate. A PNG candidate currently cannot satisfy FileRef::validate, which requires dimensions. Existing package assembly reuses input media without rewriting it.
2. Register the bounded offline helper `tools/generation/helper.py` through runtime's helper allowlist: JSON stdin/stdout, 64 KiB request, 16 MiB response/per-file cap, no credentials or provider transport. Replay currently runs directly in offline checks; generation does not claim a broker-integrated acoustic stage.
3. Add `RunServices::read_candidate(&CandidateFile) -> Result<Vec<u8>, ServiceError>` with exact file verification, active-epoch and scratch-path checks, mirrored where appropriate. Reproducer: HelperResult returns CandidateFiles, but generation cannot parse a newly produced normalization report through read_artifact without inventing a committed artifact ID. Reading candidate bytes through the runtime service lets the acoustic executor validate the report before returning its StageOutput.

Agent 1 integrated the implementation in clean commits. The operator explicitly authorized committing the remaining generation handoff documents. This follow-up stages only those documents and preserves other workers' and user edits.
