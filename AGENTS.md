# Paper Theater agent instructions

Read `docs/agent-handoffs/COMMON.md`, `INTERFACES.md`, and your assigned role prompt before implementation. The product root is `~/Projects/paper-theater`.

## Product identity

Use Paper Theater, `paper-theater`, `pt-*` and `@paper-theater/*` for new identities. Generic reused crate names may remain. The retired experiment identity and every name beginning with its prefix must be absent from product-owned names and content, including comments, documentation, metadata, fixtures, source maps, generated records and exports. The identity checker encodes the exact prohibited prefix numerically; do not reproduce it in instructions or negative test literals.

External source workspaces are read-only. Supply their location transiently; never serialize an absolute source path, link to it at runtime, or symlink product code to it. Copy selected code into the product, adapt paths and preserve truthful neutral hash-bound provenance. Immutable evidence and spending accounting must not be falsified or reset to satisfy naming rules.

## Ownership

The GitHub repository `jeffcato/paper-theater` must remain public so hackathon judges can review it. The operator confirmed this requirement on October 26, 2026; it supersedes earlier bootstrap visibility guidance. Keep `main` as the default branch.

- `integration` (Agent 1): root configuration/lockfiles/Git integration; shared Rust and TypeScript contracts; copied generic core crates; production crate manifest/module root; shared fixtures and integration tooling.
- `server` (Agent 2): `crates/pt-production/src/runtime/**`, `crates/pt-server/**`, `apps/studio/**`, runtime fixtures/documentation.
- `generation` (Agent 3): `crates/pt-production/src/generation/**`, `tools/generation/**`, generation fixtures/documentation.
- `player` (Agent 4): `packages/player/**`, `apps/audience/**`, player fixtures/documentation.

The complete path table is in `docs/agent-handoffs/README.md`. Only Agent 1 changes shared schemas, fixtures, root lockfiles and the production crate manifest/module root. Send concrete proposals instead of editing another lane's files.

Temporary operator assignment (October 28, 2026): a separate redesign agent owns `apps/studio/**` while that redesign is active, including Studio tests and configuration. Server R19 stays in backend/runtime/server-owned paths and must not edit the Studio tree. Integration I21 and generation G25 also exclude it. Coordinate shared HTTP/SSE/player/contract changes with their owners and dependency/root-lockfile changes with integration. Preserve the separate active player/audience redesign. Run combined UI checks against an explicitly frozen handoff; moving redesign files are not a verified backend checkpoint. This temporary assignment does not transfer shared Git ownership.

Continuous goal queues are in [docs/agent-handoffs/goals/README.md](docs/agent-handoffs/goals/README.md). Each assigned worker may update checkboxes, evidence and dependency requests in only its corresponding `integration.md`, `server.md`, `generation.md`, `player.md` or `studio.md` there; this is a narrow exception to integration's documentation ownership. Integration alone edits the shared goal index/runbook, root TODO and shared interfaces. Do not remove requirements or mark blocked work complete. Assign the player list to the current exclusive player/audience owner, not a second overlapping redesign writer.

In a shared working directory, Agent 1 alone stages/commits, switches branches and pushes. Workers preserve other agents' edits and report their owned changes. In operator-provided isolated worktrees, commit only assigned paths on the assigned branch. Do not reset others' work, force-push, or initialize nested repositories.

## Execution

Delivery priority (October 28, 2026): follow the [seven-day delivery plan](docs/plans/2026-10-28-hackathon-delivery.md). Finish the smallest useful active I21/R19/G25 handoff, then prioritize ordinary production transports, creative-stage wiring and genuine acceptance/publication over further standalone proof expansion. Two chapters remain the target; the operator approved one complete accepted chapter plus the recorded walkthrough as the Day 3 fallback. This changes backlog order, not ownership, evidence requirements or spending authority. Preserve active UI work through its bounded functional handoff.

When assigned a continuous implementation goal, follow its [runbook](docs/agent-handoffs/goals/RUNBOOK.md): implement/check off the next unblocked required item and continue without another task-selection turn. Record exact blockers and advance independent work. Finish only when the required implementation checklist is evidenced complete; report remaining live/device/chapter gates separately and obey the goal runtime's blocking/completion rules. Do not start additional agents or paid operations merely because a task appears in a goal list.

Implement bounded runnable increments. All implementation requires clean git commits and contract-first TDD (first write a failing test that defines the spec, then write the minimum code required to make that test pass). Start with zero-provider-call fixture/replay work and a small shared-contract handoff. Do not import a generic DAG engine or build a second player.

Runtime owns durable reservations, requests, database commits and publication. Generation proposes typed artifacts, decisions and reviewed package candidates. Player validates and renders the authored package. Studio and audience use that same player.

Live product calls require established spending/request/repair authorization. Historical experiment allowances do not automatically authorize production. Preserve reservations, uncertain attempts and exhausted repairs across migration.

Retain verbatim prose, measured timing, independent per-fact reveal evidence, canonical sprites/voices, recognizable visual storytelling and automatic required reviews. Missing, stale, ambiguous or uncertain evidence fails acceptance. Diagnostic/synthetic/imported fixtures do not become accepted autonomous releases.

## Verification

Run `python3 tools/check_product_identity.py .` and appropriate checks for changed behavior. Add executable application checks as their implementations arrive; do not claim unimplemented commands pass. State actual results and pending evidence in handoffs.
