# Paper Theater agent instructions

Read `docs/agent-handoffs/COMMON.md`, `INTERFACES.md`, and your assigned role prompt before implementation. The product root is `~/Projects/paper-theater`.

## Product identity

Use Paper Theater, `paper-theater`, `pt-*` and `@paper-theater/*` for new identities. Generic reused crate names may remain. The retired experiment identity and every name beginning with its prefix must be absent from product-owned names and content, including comments, documentation, metadata, fixtures, source maps, generated records and exports. The identity checker encodes the exact prohibited prefix numerically; do not reproduce it in instructions or negative test literals.

External source workspaces are read-only. Supply their location transiently; never serialize an absolute source path, link to it at runtime, or symlink product code to it. Copy selected code into the product, adapt paths and preserve truthful neutral hash-bound provenance. Immutable evidence and spending accounting must not be falsified or reset to satisfy naming rules.

## Ownership

The GitHub repository `jeffcato/paper-theater` must remain public so hackathon judges can review it. The operator confirmed this requirement on October 26, 2026; it supersedes earlier bootstrap visibility guidance. Keep `main` as the default branch.

- Agent 1: root configuration/lockfiles/Git integration; shared Rust and TypeScript contracts; copied generic core crates; production crate manifest/module root; shared fixtures and integration tooling.
- Agent 2: `crates/pt-production/src/runtime/**`, `crates/pt-server/**`, `apps/studio/**`, runtime fixtures/documentation.
- Agent 3: `crates/pt-production/src/generation/**`, `tools/generation/**`, generation fixtures/documentation.
- Agent 4: `packages/player/**`, `apps/audience/**`, player fixtures/documentation.

The complete path table is in `docs/agent-handoffs/README.md`. Only Agent 1 changes shared schemas, fixtures, root lockfiles and the production crate manifest/module root. Send concrete proposals instead of editing another lane's files.

In a shared working directory, Agent 1 alone stages/commits, switches branches and pushes. Workers preserve other agents' edits and report their owned changes. In operator-provided isolated worktrees, commit only assigned paths on the assigned branch. Do not reset others' work, force-push, or initialize nested repositories.

## Execution

Implement bounded runnable increments. All implementation requires clean git commits and contract-first TDD (first write a failing test that defines the spec, then write the minimum code required to make that test pass). Start with zero-provider-call fixture/replay work and a small shared-contract handoff. Do not import a generic DAG engine or build a second player.

Runtime owns durable reservations, requests, database commits and publication. Generation proposes typed artifacts, decisions and reviewed package candidates. Player validates and renders the authored package. Studio and audience use that same player.

Live product calls require established spending/request/repair authorization. Historical experiment allowances do not automatically authorize production. Preserve reservations, uncertain attempts and exhausted repairs across migration.

Retain verbatim prose, measured timing, independent per-fact reveal evidence, canonical sprites/voices, recognizable visual storytelling and automatic required reviews. Missing, stale, ambiguous or uncertain evidence fails acceptance. Diagnostic/synthetic/imported fixtures do not become accepted autonomous releases.

## Verification

Run `python3 tools/check_product_identity.py .` and appropriate checks for changed behavior. Add executable application checks as their implementations arrive; do not claim unimplemented commands pass. State actual results and pending evidence in handoffs.
