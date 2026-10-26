# Shared implementation boundaries

This is the minimum interface handoff for parallel work. Agent 1 owns the concrete Rust/TypeScript definitions and publishes their exact revision before cross-lane integration. These are implementation requirements, not claims that the APIs already exist.

## Dependency direction

`pt-contracts` has no dependency on the server, production implementation or player. `pt-production` contains sibling `runtime` and `generation` modules. The server composes their concrete implementations. Cross-lane calls use shared contract traits/records; neither sibling reaches into the other's database, transport or private data structures.

The TypeScript `contracts` package mirrors public JSON explicitly. The framework-independent `player` consumes it. Both web apps import the same player package. The studio belongs to Agent 2; the audience app belongs to Agent 4. Player DOM/rendering implementation remains inside Agent 4's package even when embedded by the studio.

## Runtime ↔ generation

Agent 1 freezes a small asynchronous `StageExecutor` / `RunServices` boundary using Rust structs/enums and a file/JSON helper envelope. Do not implement a generic plugin system. Use the selected fixed stage order:

`source → direction/coverage/research → pilot → speech → acoustic → visual_preflight → visuals/registration → presentation → music/mix → choreography → review → package`

The pilot validates adjacent voice takes, safe joins and a small scored passage before bulk speech. Asset-dependent review remains at its proper later stage; the pilot cannot fabricate missing rendered evidence. Shell preparation is a separately reusable artifact task. Stages can contain small fixed subtasks; no general DAG is needed.

| Record or service | Required meaning | Implementation owner |
|---|---|---|
| `StageInput` | Run/stage/unit IDs, epoch/revision, input artifact refs/digests, pinned recipe/profile, execution mode, authorized limits | Agent 1 defines; Agent 2 supplies |
| `StageOutput` | Proposed complete artifacts, concise decisions, automated findings, cache keys, candidate references and explicit outcome; no committed database IDs invented by a helper | Agent 1 defines; Agent 3 produces |
| `StageExecutor` | Execute the selected stage from validated inputs through supplied services; return output or typed failure | Agent 3 implements |
| `RunServices` | Read verified artifacts, create bounded candidate files, emit proposed journal entries, check cancellation, execute an allowlisted provider/helper operation, read relevant library bindings | Agent 2 implements |
| Provider operation | Attempt/unit ID, recipe and provider/model identity, request digest, bounded payload/artifact refs, request/token/money reservation estimate and timeout | Agent 3 prepares; Agent 2 authorizes and dispatches |
| Provider result | Actual completion/known rejection/ambiguous outcome, request/response hashes, raw response refs where available, usage, timing and sanitized diagnostics | Agent 2 records; Agent 3 parses |
| Library update | Stable entity/type/voice/reference binding with expected library revision and artifact hashes | Agent 3 proposes; Agent 2 atomically commits |
| `PackageCandidate` | Frozen payload inventory, acceptance inputs/results, manifest bytes/digest, provenance, and all required artifact refs; unresolved findings explicit | Agent 3 prepares; Agent 2 verifies and publishes |

Rust owns request authorization even when Python performs a provider transport. Each helper dispatch must carry an issued attempt identity and bounded operation descriptor. Python may not make a second request, retry, launch a different model or manage an independent ledger. Return bounded JSON on stdout; stderr is bounded/sanitized diagnostic output. Pass credentials only through the runtime's approved environment/secret channel, never serialized stage records.

Generation writes only candidate artifacts in assigned scratch space; runtime validates/hash-stores them and commits artifact references, state and journal events together under the active epoch. A canceled epoch cannot commit library bindings, stage success, acceptance or publication. Provider charges/late responses remain recorded truthfully.

**Publication split:** generation compiles/reviews the candidate and hashes its content; runtime owns filesystem activation, restart reconciliation and the final catalog update. Player performs independent load-time verification. Generation never updates SQLite or catalog pointers directly; player never grants creative/evidence acceptance.

## Generation → player

Use schema version 1, capability `paper-player-v1`, snake_case JSON, half-open intervals and integer samples ≤ `Number.MAX_SAFE_INTEGER`. Digests are lowercase SHA-256 hex. Source spans use Unicode scalar offsets; TypeScript converts to UTF-16 explicitly. Geometry is finite and uses +X right, +Y up, +Z toward the audience.

Agent 1 starts with the exact records in the source player handoff's “Interfaces and conversions”: `ReleaseRef`, `FileRef`, manifest/entries, registered card, scene instance, reveal facts and attestations, complete scene/camera/light snapshots, timeline/opening/transitions/closing, story cues, read-along words/groups and shell/profile. Preserve null/ineligible reveal semantics and all required evidence references.

| Boundary | Producer responsibility | Consumer responsibility |
|---|---|---|
| Narrative facts/reveals | Agent 3 supplies complete source scopes, image attestations, both acoustic anchors and conservative final-domain bounds | Agent 4 validates references/hashes and filters contributors; no inferred permissions |
| Authored timeline | Agent 3 resolves scenes, presentation placement, mandatory phases, required cue accounting, supported actions and terminal snapshot | Agent 4 validates/indexes it and evaluates deterministically; no retiming, missing-action repair or invented cues |
| Rendering profile | Agent 4 proves geometry/resource defaults with fixtures; Agent 1 freezes the shared profile revision | Agent 3 uses that exact profile for registration, preflight, final compilation and rendered review |
| Read-along | Agent 3 maps original measured words/groups to the master, preserving exact text/punctuation | Agent 4 lays out the active chunk without changing samples, splitting groups or previewing later text |
| Final shell | Agent 4 provides validated geometry/asset briefs and consumes registered art | Agent 3 generates/reviews shell artwork through the same broker and returns registered assets; Agent 1 coordinates acceptance |
| Render evidence | Agent 3 submits an explicit scene/action/reveal sampling inventory | Agent 4 exposes an offline/browser capture interface returning exact candidate/profile/player/sample identities and actual captures; Agent 3 runs automated creative review |

Supported actions: enter, exit, move, permitted facing/mirroring, nod/shrug/recoil/bow/wobble, bounded perspective camera cues and complete scene lighting. Reject pose swaps, hop, dynamic attach/detach, unsupported cameras and director-authored curtain/scene-transition controls. The pause sign is player-owned UI presentation.

Player API: `loadRelease(ref, policy)` returns a verified immutable handle or typed failure; `mount(container, handle)` provides `play`, `pause`, `seek(sample)`, `restart`, `setVolume`, `setReadAlong`, notifications and `dispose`; `evaluate(compiled, sample)` is pure. An explicit diagnostic entry point may render a valid fixture while reporting missing acceptance IDs. It is unavailable from the audience's production-loading path.

Hash order: freeze the sorted payload inventory excluding acceptance/manifest/catalog; bind acceptance to that inventory plus checker/profile versions; write the manifest referencing both; hash its exact written bytes; update the catalog last. Do not create self-hashes or hash cycles.

## Server → studio

Freeze `GET /api/v1/books`, `POST/GET /api/v1/runs`, `GET /api/v1/runs/{id}`, `POST /api/v1/runs/{id}/cancel`, and the run's `/events` and `/journal`, plus allowlisted `GET /api/v1/artifacts/{id}`. Requests are idempotent; conflicting reuse of an idempotency key fails.

Snapshot includes `last_event_seq`. Events have schema/run/sequence/epoch/time/type and causal task/decision/artifact references. Snapshot N followed by events after N must lose no committed event. Clients deduplicate; invalid cursors require a fresh snapshot. Replay reads journal/artifacts only. Large/raw payloads stay behind artifact refs, not in SSE messages. No fabricated global progress percentage.

## Contracts-ready handoff

Agent 1 publishes `docs/implementation/integration/contracts-ready.json` with contract revision, shared fixture path/digest, negative-case inventory, available check commands, service boundary revision and rendering-profile status. Publishing it freezes the shared boundary for that integration slice; it does not certify the rendering profile, fixture evidence or production acceptance.

Canonical fixture root: `fixtures/shared/two-scene-v1/`. Prefer compatible saved audio/art; preserve original measured timing and the known 176-word/170-group coverage where applicable. Otherwise use explicitly synthetic structural cases. Mark imported/synthetic/diagnostic status honestly; a schema-valid fixture with missing evidence must fail production loading. Each lane may add local negative cases under its own fixture directory, not edit the canonical fixture.

Cross-lane changes go through Agent 1 with an exact proposed API/field change, rationale, affected consumers and a reproducible case. Agent 1 updates both language representations and the shared serialized fixture together, then publishes the new revision.

## Implemented first boundary

The first frozen diagnostic slice is `pt-contracts-v1.0.0` / `pt-services-v1.0.0`; exact files, fixture inventory digest, commands and pending gates are in [contracts-ready.json](../implementation/integration/contracts-ready.json). Rust public records/traits are re-exported by `pt_contracts`; TypeScript public records and validators by `@paper-theater/contracts`. `PlaybackDocument` is a portable load/test envelope around individually hash-bound package files, not another manifest format. The canonical serialized wire records are in `fixtures/shared/two-scene-v1`.

`RunServices::write_candidate` accepts bounded byte proposals and returns candidate file metadata. `provider` and `helper` are separate allowlisted operations; fixture/replay provider calls must fail before credential access. `StageOutput` carries proposed library updates and an optional PackageCandidate; it contains no committed database IDs. `production_blockers` deliberately retains the independent acceptance-verifier gate. Profile/browser migration and provider quota/recovery refinements remain separate pending gates recorded in the handoff.
