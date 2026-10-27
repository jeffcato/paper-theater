# Paper Theater

An autonomous illustrated paper theater for classic books.

Generation prepares dramatic speech, illustrated scenery and figures, music, measured read-along and choreography. Playback presents immutable prepared performances through one shared Three.js player.

The first release targets two complete first chapters: *The Wonderful Wizard of Oz* (Gutenberg #55) and *The War of the Worlds*, Book One Chapter I (Gutenberg #36). Generation uses a local Rust server and a web studio; audience playback uses a static ticket booth.

## Prior work

Paper Theater’s public history starts on October 26, 2026. Before that date the author had a private experiment: architecture notes from October 24–25, an Oz staging player, acoustic fixtures, and four local crates (`dramatic-script`, `monotonic-align`, `timed-captions`, `atomic-journal`). Those crates were brought into this repository from that experiment. Their file hashes are in [integration reuse-provenance.json](docs/implementation/integration/reuse-provenance.json). Generation and player lineage are recorded in [generation reuse-provenance.json](docs/implementation/generation/reuse-provenance.json) and [player baseline](docs/implementation/player/baseline.md). Saved audio and transcripts under `fixtures/generation/acoustic/` are historical diagnostic evidence, including Google `gemini-3.5-transcribe` receipts and one `muse-voice-transcribe-1.0` receipt. They are not hackathon-window provider calls, and they are not an accepted performance. Work written in this public repository is the shared contract layer, the local studio and server, the audience booth, and the offline diagnostic integration. A Meta Model API path for Muse Spark, Muse Wonder, Muse Image, and SAM 3.1 is not in this tree yet. The Muse Image house-art direction is documented in [immersive-art-brief.md](docs/implementation/player/immersive-art-brief.md) (35 blank, story-neutral surfaces for the immersive audience house, lettering typeset by the player); it makes and authorizes no provider calls, and the player does not consume registered house art yet. Pre-event components stay in the tree with the notices in those provenance files, and they are not the submission by themselves.

## Implementation

A self-contained Rust/TypeScript workspace, shared schema-v1 records, measured saved-media fixture and contract checks are implemented. The diagnostic fixture preserves 176 words and 170 highlight groups. Production acceptance and complete chapter delivery remain pending.

Run `cargo test --workspace --offline`, `npm run check:contracts`, `npm run check:player`, `npm run build`, and `python3 tools/check_product_identity.py .`. Shared revisions/digests and outstanding integration work are in [contracts-ready.json](docs/implementation/integration/contracts-ready.json). Product-local specifications are in [contracts](docs/spec/contracts.md), [playback](docs/spec/playback.md), and [server](docs/spec/server.md).
Start with the [parallel implementation handoffs](docs/agent-handoffs/README.md), [shared rules](docs/agent-handoffs/COMMON.md), and [interface boundaries](docs/agent-handoffs/INTERFACES.md). Agent 1 owns bootstrap/contracts/integration; the other agents own runtime/studio, generation, and player/audience.

Local project root: `~/Projects/paper-theater`. Supply any external read-only source location through transient session context or `SOURCE_ROOT`; do not persist that path or depend on it at build/runtime.

## Repository

GitHub: [jeffcato/paper-theater](https://github.com/jeffcato/paper-theater). The repository must remain public so hackathon judges can review it, with `main` as its default branch. Follow [repository collaboration rules](docs/implementation/integration/repository.md) and the root [agent instructions](AGENTS.md).

## Identity check

```sh
python3 tools/check_product_identity.py .
```

This check enforces the product identity rule on names and file bytes. Scan built first-party binaries and exported packages explicitly as described in the handoffs. Application checks report their actual implementation and acceptance status in the integration handoff.
