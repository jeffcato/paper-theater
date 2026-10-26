# Repository and collaboration

Product root: `~/Projects/paper-theater`.

GitHub remote: `https://github.com/jeffcato/paper-theater.git`. Initial visibility is private; the default branch is `main`.

The initial commit contains project instructions, agent handoffs, identity enforcement and repository hygiene. It does not implement the product or certify production acceptance.

## Agent coordination

Follow the ownership table in [the handoffs](../../agent-handoffs/README.md). In a shared workspace, Agent 1 owns the Git index, branch operations, explicit staging, commits and pushes. Other agents edit only their owned files and return a change report. Use implementation branches and pull requests for subsequent work. If the operator supplies isolated worktrees, workers may commit assigned files on their assigned branch and Agent 1 integrates them.

Preserve the root instructions, installed handoffs and repository history when bootstrapping the Rust/TypeScript workspace. Do not initialize another repository inside a package or import experimental history.

## Initial checks

`python3 tools/check_product_identity.py .` is the available repository check and runs in GitHub Actions for pushes and pull requests. Agent 1 extends CI with real shared-contract and application checks as those commands are implemented.

Credentials, dependency caches, local databases, production state, private evidence and generated media are ignored. Check ignore rules and stage explicit reviewed paths before a commit. Runtime release packaging remains an independent allowlisted/hash-verified process.

Keep source/receipt digests truthful. Git attributes disable newline conversion for fixture, evidence and immutable performance paths. Any additional hash-bound path must receive equivalent byte-preserving treatment before it is committed.
