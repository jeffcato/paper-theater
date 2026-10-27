# Private G20 diagnostic examples

`scopes.json` is an executable StageExecutor behavior fixture for the integration-owned compatibility-test replacement. Tests compare all five rows to actual outputs; the shared literal stays until integration acknowledges its replacement.

The other JSON files are generated private declarations from two authored frozen reveal facts, the original cue, and one explicitly synthetic Uncertain observation. They are not verifier receipts. The synthetic player is `synthetic-player-v1`; missing semantic reviews and NeedsRepair remain. These files are not an audience export or a standalone media bundle.

Reconstruct the exact bounded inputs and metadata with:

```sh
PT_ACCEPTANCE_EXPORT=fixtures/generation/acceptance-proposals-v1 cargo test -p pt-production generation::tests::acceptance_proposals:: --offline
```

Tests borrow `fixtures/shared/two-scene-v1` bytes and `fixtures/generation/acoustic/excerpt.wav`; recipe/player/checker/raw observation inputs are explicit diagnostic bytes in `generation/tests/acceptance_proposals.rs::bound_fixture`. No new audio is committed. A changed compiled shared checker requires newly pinned identity bytes; a frozen example cannot authorize a later checker/player.

`public-allowlist.json` shows the exact ordered audience payload, excluding source/alignment/private evidence. Public inventory/acceptance/manifest metadata is added only after hashing its predecessors. `private/acceptance-proposal-status.json` binds the context, fact/evidence inventories and all identities. The status and inventories remain private and acyclic.
