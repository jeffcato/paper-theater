# Current backend worker assignments

Updated October 27, 2026. Detailed [backend assignments](backend-next-tasks.md) preserve Claude's UI and sole integration Git ownership. All work is test-first, backend-only and zero-real-provider-call.

- **Integration I20:** [coherent backend checkpoint](backend-checkpoint-task.md). Capture exact completed R17/G23 source before overlap, resolve aggregate Pilot failures on one isolated complete snapshot and publish full scoped backend/CI evidence. Exclude UI/unfinished work; focused passes do not establish aggregate green.
- **Server R18:** [durable saved Direction/Pilot](server-direction-pilot-task.md). Hand off R17 pins, adopt the native shared WAV wrapper with parity checks, then persist Source→Direction→Pilot two-phase requests/results/audio/checkpoints/replay. Preserve authoritative source/registry/catalog/epoch and missing approval; no live mode or UI edits.
- **Generation G24:** [bounded Speech compiler/saved takes](generation-speech-task.md). Hand off G23 pins, preserve verbatim complete source/voice units and compile request/native-take/transcription manifests without expected-prose hints or fabricated audio/timing/approval. Retain closed bulk pilot gate, private exports and replay.

Completed: I19 services v1.8.0 (179 Rust/41 report cases, CI 24f74e5, five checks/32 missing); R17 offline Services adapter (165 scoped/restart checks, recorded aggregate Pilot failures); G23 offline Pilot (103 generation/19 Pilot cases, NeedsRepair and bulk approval false). No coherent full aggregate, real auditions, production approval or UI redesign readiness follows from these separate results. R12 viewport and I14 stable UI/lockfile handoff remain separate.
