# Registered chapter and excerpt scope

INT-03 partial implementation. Proposed services `pt-services-v1.11.0`, report/checker v6 and matrix `pt-acceptance-required-v6`. Publication and exact-head CI are recorded separately in the ready receipt when complete. Other INT-03 provenance/acoustic/delivery work remains pending.

The independent verifier now executes `complete-chapter` and `complete-excerpt` against the compiled `pt-registered-source-scopes-v1` catalog. It first reuses actual source-slice, ordered read-along and scene-coverage verification, including bounded hash-checked full source, selected source, relationship, timeline and read-along reads. A candidate's title, scope label or submitted Pass cannot establish the intended chapter extent.

| Source | Canonical chapter scalars | Chapter span in frozen clean book |
|---|---|---|
| `pg-55-chapter-1` | 5958 | `[2319,8277)` |
| `pg-36-book-1-chapter-1` | 12654 | `[1087,13741)` |

Both book coordinates and canonical chapter-local coordinates are registered. All offsets are half-open Unicode scalar offsets. No trimming or Unicode normalization is performed. Playback and fact coordinates remain local, using the existing versioned relationship to the full source.

`complete-chapter` requires the entire registered chapter span and its canonical source digest/count. `complete-excerpt` requires a nonempty contiguous selected span wholly inside that chapter, with actual substring equality and complete selected-source representation. Unknown scopes or missing mapping/bytes stay Missing; mismatched registered hashes/counts/origins, partial chapters and invalid coverage fail. Existing Fail, Uncertain and Missing observations remain adverse.

## Exact owner adoption

Runtime selects `AcceptanceVerificationPolicyV2` independently from its trusted source registration. Pin the full-source FileRef, registered scope ID, scalar count/origin, selected full-source span, selected Source role and exact relationship FileRef. Preserve the existing `verify_acceptance_v2` and consumed-file recheck path. Callers cannot register arbitrary text by naming it after a supported chapter. Runtime/generation implementations are unchanged by this shared handoff.

Rust exports `registered_source_catalog_v1()` and `registered_source_catalog_sha256_v1()`; TypeScript exports `registeredSourceCatalogV1()` and the catalog version/digest/byte constants. The returned catalog is an owned copy. Catalog SHA-256: `865410192be03386b3ac16543aea02a24abd5ff0fb936c91c12d92b8e9fe6231` (2128 bytes). Original imported source receipts/raw hashes are retained as provenance references, not a claim that provider provenance has been verified.

For a serialized scope Pass, both languages require registered mapping metadata and consumption of exact catalog/full/slice/relationship and coverage hashes. JSON consistency validation does not authenticate execution or grant publication authority. The checker identity includes the catalog and verifier source. Historical reports v1–v5 and matrices v1–v3 retain their original meanings; unpublished draft matrix identities v4/v5 are not reused.

## Verification and remaining gates

Public-boundary TDD first demonstrated a mislabeled excerpt escaping an executed chapter failure, then demonstrated serialized Pass validation accepting an omitted source mapping. Regression cases cover both sources and coordinate frames, partial/excerpt scopes, missing/stale files, forged mapping/catalog consumption and adverse findings; Rust-generated TypeScript cases include historical v4/v5 receipts.

Current matrix: seven implementations across the registry; six implemented / 31 pending in each excerpt or chapter scope. Diagnostic scope retains its three defaults. Speech fidelity, independent per-fact acoustic support, provider provenance, creative/device checks and publication authority remain separate. Synthetic caption positions exercise source reconstruction only. No accepted chapter, bulk approval or public synthetic release is created. `production-verifier-pending`, private receipts, archived evidence and zero-provider-call behavior remain intact.
