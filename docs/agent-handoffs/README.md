# Paper Theater: parallel implementation handoffs

Prepared October 26, 2026. These are implementation instructions; all new acceptance gates start pending. Product root: `~/Projects/paper-theater`.

Use four agents. Agent 1 owns shared contracts and integration; Agents 2–4 own disjoint implementation areas. All four can begin inspection and extraction immediately. Shared integration waits for Agent 1's small contracts handoff. The first joint milestone is a saved-media fixture traveling through browser → server → journal → package → shared player.

## Launch

Give each agent [COMMON.md](COMMON.md), [INTERFACES.md](INTERFACES.md), and its role prompt. Keep this whole directory available so relative links resolve.

| Agent | Prompt | Exclusive implementation area |
|---|---|---|
| 1 | [Contracts and integration](01-contracts-and-integration.md) | Workspace, shared Rust/TypeScript contracts, reusable core crates, canonical fixtures, integration checks |
| 2 | [Server and studio](02-server-and-studio.md) | Durable runner/storage/budget/publication, HTTP/SSE, studio |
| 3 | [Generation](03-generation.md) | Source/direction/research, speech/evidence, visuals, presentation/music/cues, automated review and candidate assembly |
| 4 | [Player and audience](04-player-and-audience.md) | Shared player, Three.js migration, shell/rendering/read-along, audience app, browser checks |

Agents need read access to the experiment workspace and write access to the product workspace. Supply the source location only as the transient `SOURCE_ROOT` environment variable or session context. If starting in that source workspace, capture its current directory before changing directories. Never persist the expanded source path in the product. Agent 1 copies this neutral handoff directory into the product at `docs/agent-handoffs/` and installs [the identity checker](check_product_identity.py) at `tools/check_product_identity.py`.

Run the agents in the same target workspace with the ownership rules below. If the operator instead supplies isolated worktrees, Agent 1 integrates their changes and publishes the shared contract revision to each; workers must consume that same revision.

## Git repository

Use the product's own Git repository at `~/Projects/paper-theater`, with GitHub remote `jeffcato/paper-theater` and default branch `main`. The repository must remain public so hackathon judges can review it, as confirmed by the operator on October 26, 2026. Preserve any repository bootstrap commit and its installed handoffs. Do not initialize a nested repository or import experimental Git history.

In a shared working directory there is one Git index and branch: only Agent 1 stages, commits, switches branches and pushes. Workers edit their assigned files and report changes; they do not run broad staging/reset commands. Agent 1 stages explicit reviewed paths and uses implementation branches/PRs. In isolated worktrees, workers may commit only their own files on their assigned branch; Agent 1 integrates and publishes the result. Never force-push or reset another worker's work.

## Exact file ownership

| Owner | Writable paths |
|---|---|
| Agent 1 | Root manifests, lockfiles, toolchain/build configuration, root documentation/`AGENTS.md`; `crates/pt-contracts/**`; `packages/contracts/**`; `crates/pt-production/Cargo.toml` and `src/lib.rs`; copied `crates/{gutenberg-catalog,dramatic-script,monotonic-align,timed-captions,atomic-journal}/**`; `fixtures/shared/**`; `docs/spec/**`; `docs/agent-handoffs/**`; `docs/implementation/integration/**`; `tools/integration/**`; `tools/check_product_identity.py`; shared CI configuration |
| Agent 2 | `crates/pt-server/**`; `crates/pt-production/src/runtime/**`; `apps/studio/**`; `fixtures/runtime/**`; `docs/implementation/runtime/**` |
| Agent 3 | `crates/pt-production/src/generation/**`; `tools/generation/**`; `fixtures/generation/**`; `docs/implementation/generation/**` |
| Agent 4 | `packages/player/**`; `apps/audience/**`; `fixtures/player/**`; `docs/implementation/player/**` |

Each agent owns the tests and package manifests inside its area. Only Agent 1 changes shared lockfiles or the shared production crate's dependencies/module root. Requests for those edits go to Agent 1. Agent 4 owns dependency compatibility research and player migration; Agent 1 applies resulting shared lockfile changes. This deliberately delegates the former integrator-owned version-preparation task to the player specialist while retaining one lockfile writer.

Agent 1 creates only the initial empty lane directories/module declarations; it does not write workers' implementations. After announcing bootstrap readiness, those paths belong exclusively to the listed worker. No other ownership transfer is implicit.

## Parallel start and integration order

| Phase | Agent 1 | Agent 2 | Agent 3 | Agent 4 |
|---|---|---|---|---|
| Begin together | Bootstrap and publish minimal interfaces | Inspect runner/store; prepare runtime cases in owned paths | Extract pure parsers/validators and replay cases in owned paths | Inspect player; establish version baseline and isolated rendering cases |
| Contracts ready | Publish shared fixture digest and callable service boundary | Connect runtime/HTTP/studio to real shared types | Connect generation executor to broker and artifact types | Connect loader/evaluator to fixture and shared types |
| First joint slice | Verify clean build and fixture browser flow | Start/cancel/restart/replay fixture run | Produce diagnostic candidate from saved inputs | Play fixture in studio; audience correctly rejects its diagnostic status |
| Production integration | Coordinate one accepted autonomous excerpt | Durable request authorization and publication recovery | Real adapters and complete evidence/quality gates | Final shell, production acceptance and target-device checks |
| Chapter delivery | Verify both accepted packages | Recorded journals and offline replay | Complete Oz Chapter 1 and Wells Book One Chapter I | Two-ticket audience flow and full playback checks |

Before contracts are ready, use local internal structures only; do not invent competing public schemas or copy provisional schemas into another lane. A blocked shared interface must be reported with a concrete proposal while other independent work continues.

## Shared integration gates

1. **Bootstrap:** clean workspace builds; shared Rust-produced fixtures are accepted/rejected consistently by TypeScript; product identity check passes.
2. **Fixture slice:** durable browser-started run, journal/replay, cancellation and diagnostic shared-player playback work without any provider call. A synthetic/imported fixture remains ineligible for audience publication.
3. **Production excerpt:** actual autonomous run reaches a complete accepted immutable package with the final shared shell, per-fact acoustic evidence, actual audio/rendered review and publication recovery. Missing/uncertain evidence prevents publication.
4. **Delivery:** two complete first chapters and recorded journals work through the audience app on the actual Chrome/demo computer, without provider availability.

No phase authorizes paid calls. Live dispatch requires separately established product authorization plus inherited reservation/accounting reconciliation. Historical experiment spending caps are scoped evidence, not a new production allowance.

## Identity enforcement

The retired experiment identity and every name beginning with its prefix are forbidden in product-owned file/directory names and content, including generated documentation, prompts, source, comments, crate/package metadata, URLs, cache keys, fixtures, receipts, source maps and exports. `COMMON.md` defines the migration rule without reproducing the forbidden name.

The checker encodes that exact prefix as character codes so it does not itself introduce a prohibited reference. It scans names, text and binary bytes, including ignored `var/` and normal `dist/` outputs. Git object history and dependency/build caches are excluded by default; repository Git configuration is checked. Scan first-party binaries and any separately located export roots explicitly. This is a naming gate, not a claim to recognize every compressed or encoded string.

```sh
python3 tools/check_product_identity.py .
python3 tools/check_product_identity.py apps/studio/dist apps/audience/dist
python3 tools/check_product_identity.py target/debug/pt-server
```

Run artifact commands after those artifacts exist. Missing roots fail. A clean source scan does not certify build caches excluded from that scan. Keep original immutable evidence outside the product when it cannot satisfy the naming rule; never rewrite an original receipt and retain its old hash or claim it remains raw evidence.

## Worker completion report

Return changed files, consumed contract/fixture revision, exact commands and results, remaining failed/pending cases, dependency requests, and the next bounded integration action. Separate implemented behavior from actual-media/browser evidence. Run the identity check on your files and outputs. Preserve other agents' edits and all spending reservations.
