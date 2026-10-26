# Agent 3 integration requests

Consumed `pt-contracts-v1.0.0`, `pt-services-v1.2.0` and shared fixture digest `5a662fb9d1687fd61a692acdd4265bc12aac9d5bfacb8e5f75e58b472dbf776c`. Rendering profile `paper-player-r186-v1` remains selected/unverified.

Resolved:

- Frozen fixture cue-slot repair, verified source/package seeding and private diagnostic candidate storage.
- CandidateWrite optional PNG dimensions (v1.2.0); existing media is reused without rewriting it.
- Execution-scoped verified `RunServices::read_candidate` (v1.1.0).
- Registered bounded offline helper and the real generation Stage::Acoustic consumer. Its runtime integration test preserves 176 words/170 groups, records zero provider requests/money, persists the normalization/validation candidates and retains NeedsRepair. See [acoustic-stage.md](acoustic-stage.md).

Next runtime integration, owned by Agent 2:

1. Provide private stage-specific input bundle selection, for example `seed_inputs_for_stage(run, Stage) -> Vec<ArtifactRef>`, retaining original verified references. `seed_fixture` and `seed_acoustic_fixture` both use flat `manifest.json` / `source.txt` paths. Supplying both bundles unchanged to one StageInput conflicts; generation fails closed rather than choosing one by accident. The helper must continue receiving its exact acoustic relative paths.
2. After selecting the acoustic bundle, route Stage::Acoustic through GenerationExecutor in FixturePipeline when the registered offline recipe is available. The current composition sends only Source and Package to generation. Do not relabel NeedsRepair or bypass the missing fact/fidelity/voice/rendered gates.
3. Include the completed runtime helper/fixture/runner changes when committing this stage; the real integration test consumes those working-tree APIs. Shared candidate-read/dimensions contracts are already published.

Generation-owned follow-up paths: `crates/pt-production/src/generation/{mod.rs,acoustic.rs,acoustic_tests.rs}` and `docs/implementation/generation/**`. Agent 1 owns the shared index, integration commit and push. Preserve all other workers' and operator edits; this task performs no Git mutations.
