# Exact-report acceptance coverage ledger v1

INT-05 increment; services remain compatible. The matrix is unchanged at v3: **five implemented checks and 32 pending production verifiers**. This ledger does not implement any missing verifier or authorize publication.

Rust: `PrivateAcceptanceVerification::coverage_ledger()` rechecks all consumed files, validates the execution report and returns `AcceptanceCoverageLedgerV1`. Every selected requirement has its exact target, evidence owner(s), executed-checker identity, consumed hashes, outcome and reasons. The ledger retains the full source/media/map/assets/recipe/profile/player/checker binding and exact compact report digest. Custom requirements retain `unassigned` ownership; missing verifiers remain unexecuted and blocking. No row is inferred from a submitted Pass label.

Rust `ledger.validate_for_report_bytes(bytes, &expected_checker)` and async TypeScript `validateAcceptanceCoverageLedgerV1(ledger, reportBytes, expectedChecker)` check the same strict wire contract. Consumers independently select the report bytes and expected checker. Missing/duplicated targets, changed ownership, forged execution/outcomes, changed report bytes, unknown fields and duplicate JSON keys (including escaped aliases) fail. TypeScript rejects the duplicate-key ambiguity that ordinary `JSON.parse` otherwise hides. Validators describe metadata; they cannot reconstruct the Rust execution capability.

Adverse findings retain their original report observations. For diagnostic provenance the aggregate may be Fail while a recorded review observation remains Uncertain; the ledger preserves the actual aggregate and exact report binding. Neither outcome is replaced with Pass. `production-verifier-pending`, NeedsRepair and publication/live/accounting/device gates remain intact.

## Exact runtime adoption

After executing the shared verifier with runtime-selected trusted policy and before persisting its report, call `coverage_ledger()`. Persist the exact `serde_json::to_vec(verification.report())` bytes privately; their digest must equal `ledger.report_sha256`. Persist the ledger only under the same active-epoch/cancellation/transaction boundaries and recheck consumed bytes before commit. Use it to expose the implementation/missing-evidence status privately; do not export raw reports, private evidence or the ledger through an audience payload allowlist. Keep the existing publication rejection until the complete trusted verifier path exists.

Generation keeps proposing evidence against the frozen policy; it cannot create an execution receipt from this wire type. Player continues rejecting serialized/diagnostic authority. Source-owned runtime/player adoption remains with those lanes.

Four public Rust tests and 15 Rust-produced cross-language cases cover complete selected scope, stale consumed source, adverse outcomes, missing/duplicated rows, substituted targets/owners/checker/report bytes, invented execution, duplicate JSON keys and forged grants. Native packed exports and strict consumer declarations include the new APIs. Raw red/green evidence is retained in `goal-checks`; full combined-source tests/CI are tracked separately in the integration checkpoint receipt.
