# I4 first increment: independent bindings and coverage

Services `pt-services-v1.4.0` add private acceptance policy/proposal/report records, report version 1 and matrix `pt-acceptance-required-v1`. Playback and legacy acceptance/provider/helper/candidate wire records remain unchanged. **Production acceptance stays blocked.**

## Executable boundary

```rust,ignore
verify_acceptance(document, package_root, runtime_policy, additional_proposals, private_inputs_root)
    -> ContractResult<PrivateAcceptanceVerification>
```

`PrivateAcceptanceVerification` has no public constructor or deserializer. `report()` exposes immutable private metadata; `recheck_inputs()` verifies consumed bytes again. The serializable `AcceptanceVerificationReport` is not a capability. Rust `report.validate(expected_checker)` and TypeScript `validateAcceptanceVerificationReport(report, expectedChecker)` check wire consistency, complete coverage and checker identity; they cannot authenticate execution from JSON.

Independent CLI:

```sh
cargo run -p pt-contracts --example verify-acceptance --offline -- \
  PACKAGE_ROOT PRIVATE_DOCUMENT_JSON PRIVATE_POLICY_JSON \
  PRIVATE_ADDITIONAL_PROPOSALS_JSON PRIVATE_INPUTS_ROOT
```

The CLI emits deterministic private JSON. Exit **2** means checks ran but release is blocked; exit 1 means input/verification failure. No first-increment report grants production approval. It performs no provider calls or credential access.

The executed checker is identified by version plus a SHA-256 fingerprint of its compiled source closure: acceptance/structural/common/playback/service definitions, crate exports, dependency lockfile, workspace manifest and authoritative matrix bytes. This is a source/dependency identity, not a signature or an assertion that a separately named semantic verifier ran. Runtime must compare it with its trusted expected identity and invoke the actual implementation.

## Authoritative required matrix

The single versioned source is [acceptance-required-v1.json](../../../packages/contracts/src/acceptance-required-v1.json), compiled into Rust and copied into the packed TypeScript distribution. Both languages expose `acceptance_check_matrix` / `acceptanceCheckMatrix` and scoped required-check accessors.

| Scope | Base requirements | Meaning |
| --- | ---: | --- |
| diagnostic | 3 | Package bytes, evidence bindings and declaration coverage only; never an accepted release. |
| autonomous_excerpt | 37 | Full declared excerpt/source coverage and all applicable production evidence. |
| production_chapter | 37 | Complete chapter/source coverage and all applicable production evidence. |

Only `package-byte-integrity`, `evidence-binding-integrity` and `required-check-coverage` execute here. All other verifiers return typed missing/unimplemented outcomes, or retained failure/uncertainty. Required rows expand to the source span, each pinned fact in its source/narration/delivery domain, each image asset, or the entire narration/master sample interval. Facts and required cue IDs come from the runtime-pinned private fact inventory; the candidate cannot remove them to shrink coverage.

All 24 existing generation production requirements are retained. Excerpts explicitly replace `complete-chapter` with `complete-excerpt`; both require `complete-source-coverage`. Added explicit requirements cover primary acoustic evidence, delivery fact mapping, speech fidelity, pilot review, audio format, graphics recovery, control lifecycle, title visibility and the public export boundary. Candidate-declared extra/unknown checks remain required and unsupported/missing. Failed/uncertain observations also add requirements. Duplicated declarations/coverage fail. The generation-owned duplicate list is not edited during its active work; its exact adoption handoff is below.

The legacy `production_blockers` gate now diagnoses omitted required coverage and still unconditionally retains `production-verifier-pending`. Candidate `pass`, autonomous provenance or a verified-profile label cannot supply independent execution.

## Exact inputs and evidence

Runtime policy pins nine private roles: source, narration, alignment, presentation map, recipe, profile, player implementation artifact, fact inventory and evidence inventory. It also pins source scope/span, sample domains/counts, payload inventory, asset hashes, legacy version labels, expected independent checker and any additional requirements/device identity. These expected identities must come from runtime's trusted compilation/registry state, never be copied from candidate claims as authorization.

The verifier reuses `validate_playback`, `verify_package_files` and `FileRef::verify`. It additionally compares the actual manifest bytes with the supplied private document and reads the private role files through bounded, path/symlink/hash checks. Source text, narration/alignment/map identities and parsed map/profile/fact/cue metadata must match. Actor references must be inventoried. Recipe/profile/player/checker versions and all asset identities are bound explicitly. Source span means half-open Unicode scalars; sample targets explicitly select source/narration/delivery domains. Map arithmetic, actual audio decoding, fact completeness and all creative/device judgments remain unimplemented here.

Each private proposal binds the exact expected context and target and references actual raw bytes. Wrong hashes, scope/span/domain, recipe/profile/player/checker/version, changed/missing bytes, duplicate/conflicting coverage and unrelated targets cannot cover a requirement. Validating a receipt's byte bindings does not execute its declared semantic checker. A receipt saying `pass` remains an untrusted observation; unimplemented semantic rows cannot pass.

The separately pinned evidence inventory contains required unresolved proposals, including adverse receipts. The verifier loads those itself and merges additional caller proposals. Omitting a receipt from the caller list cannot omit it from verification. Preserve unresolved ledger entries; resolving/superseding them requires a later explicit runtime/verifier policy, not candidate omission. The inventory does not reference its own hash through proposal bindings, preserving acyclic hashing. Proposed metadata itself and all successfully read artifacts have consumed hashes. Failures/uncertainty remain in observations, checks and blockers; no adverse record is overwritten by a later pass.

Successful reads are retained as private read pins. Mutation after verification makes `recheck_inputs()` fail. Missing identities stay missing; no dummy successful manifest/acceptance digest is manufactured. Callers must use immutable CAS snapshots and recheck immediately before any later consumer decision.

Report metadata contains scoped hashes, outcomes, executed checker identity and typed reasons, not source text, raw receipt contents, journal entries, filesystem roots or private file paths. Original receipt/detail bytes remain in private storage under their consumed hashes. Known private-input/proposal hashes appearing in a production payload are rejected; full export sanitization remains an unimplemented required review.

## Worker handoffs and export boundary

**Generation:** replace its private required-check constant with the shared scoped accessor in a separately owned change. Keep inherited/custom requirements and every adverse finding. Propose source/fact/cue inventories and private evidence with complete byte/recipe/domain/version bindings. These are proposals, not approval. Do not append raw source, narration, alignment, prompts/recipes, fact/evidence inventories, receipts, journal data or the private report to an audience payload. The integration compatibility test currently reads the generation `REQUIRED_CHECKS` literal to prove all 24 inherited IDs remain present; coordinate its replacement with integration when adopting the accessor.

**Runtime:** construct the trusted policy and evidence inventory from immutable compilation/registry and unresolved receipt state. Freeze payload inventory → legacy acceptance → manifest first. Then invoke the independent verifier and persist its private report/hash separately. The report binds the actual manifest/acceptance and consumed input hashes; it is not part of the payload inventory and creates no self-hash cycle. Retain the in-process capability, recheck bytes, and keep activation closed: all reports still include `production-verifier-pending` and `production_approved: false`. Never redeem a deserialized report or a caller-supplied pass as a publication capability. This increment adds no dispatch, ledger reconciliation, catalog or HTTP path.

Exact adapter records are exported at `pt_contracts::*`. Set `AcceptanceVerificationPolicy.schema_version = 1`; select `scope`; populate all nine unique `inputs: Vec<AcceptanceInput>` with private-root-relative verified `FileRef`s. Pin `source_scope_id`, scalar `source_span`, narration sample rate/count, `payload_inventory_sha256`, `asset_sha256s`, optional `device_id`, trusted `player_version`, legacy `candidate_checker_version`, `checker_sha256 = acceptance_binding_checker().sha256`, and `additional_required_check_ids`. The manifest supplies delivery sample rate/count and master identity. Serialize `AcceptanceFactInventory` and `AcceptanceEvidenceInventory` with schema version 1 into their pinned private roles. A separate `Vec<AcceptanceEvidenceProposal>` contains additions only; it cannot replace that inventory.

Call `verify_acceptance(&document, package_root, &policy, &additional_proposals, private_root)?`, then `verification.recheck_inputs()?` and `verification.report().validate(&acceptance_binding_checker())?`. Treat any error as rejection. Persist `report()` only in private runtime storage; preserve findings/blockers and use its actual `manifest_sha256`, `acceptance_sha256`, `binding`, `checker` and `consumed_hashes` for audit. No public `RunServices` signature or runtime transition changes in this handoff. The CLI exercises the same sequence; its current successful execution still exits 2 because production is blocked.

**Player/Studio:** no loader or accepted-handle change is requested. Keep private verification metadata in the private inspector/CAS; a future summary boundary may expose check IDs/outcomes without raw records. The audience package remains an explicit payload allowlist, not a directory archive. The I13 recovery probes still fail at this increment's candidate `c965855`, outside successful aggregate CI; the locally supplied repair awaits independent I14 integration. Graphics/control verification is unimplemented here. P17's supplied title fix still needs the server-owned strict R12 rerun.

## Verification limits

The first red test removed the required list while forging autonomous/profile labels; the old blocker list reported only `production-verifier-pending`. The new coverage diagnosis passes. Further test-first cases cover independent execution vs claimed pass, retained ledger failures, stale source/media/map/asset/recipe/profile/player/checker bindings, wrong spans/domains, missing/changed bytes, duplicate/conflicting coverage, report forgery and private unknown fields. A narrow package/coverage check genuinely passes; every diagnostic candidate stays rejected.

Full independent creative/provider/fact/device verifiers, accepted chapters, actual inherited accounting, live authorization, production verification and catalog activation remain pending. No synthetic fixture or historical CI result becomes acceptance evidence for another candidate.

Current additive source-coverage increment: services v1.6.0 independently executes one further check with versioned matrix/checker/schema-2 reports. See [I17 scope and exact R15/G20 adoption](acceptance-source-coverage.md) and [its source/CI receipt](acceptance-source-ready.json). The original first-increment/v1 matrix and receipts above retain their original identities. There are now four implemented checks and 33 still-unimplemented excerpt/chapter verifiers; production-verifier-pending remains.
