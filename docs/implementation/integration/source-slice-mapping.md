# I18: independently pinned source-slice mappings

Services `pt-services-v1.7.0` publish private policy schema 2, relationship schema 1 and report/checker v3. Playback remains v1.0.0 and the immutable required matrix remains v2: the same four implemented checks and **33 missing excerpt/chapter verifiers**. This extends source coverage only. Provider calls/credentials/live activation: zero. Runtime/generation/UI implementations are not integrated here.

## Records and authority

All Rust records are exported at `pt_contracts::*`; TypeScript mirrors and validators are exported by `@paper-theater/contracts`.

```rust,ignore
pub struct FullSourcePinV1 {
    file: FileRef, scope_id: Id, scalar_count: u64, origin: u64
}
pub struct AcceptanceVerificationPolicyV2 {
    schema_version: 2,
    base: AcceptanceVerificationPolicy, // unchanged schema 1, nine local input roles
    full_source: FullSourcePinV1,
    full_source_span: [u64; 2],
    relationship: Option<FileRef>,     // exact private mapping bytes; never an implicit fallback
}
pub struct SourceSliceRelationshipV1 {
    schema_version: 1,
    full_source: FullSourcePinV1,
    slice: FileRef,
    slice_scope_id: Id, slice_scalar_count: u64,
    full_source_span: [u64; 2], local_span: [u64; 2],
    local_origin: u64, global_origin: u64,
}
```

The numeric literals above describe wire versions; actual Rust fields use `u32`. Unknown fields are rejected. The relationship is one complete contiguous slice, not a list, piecewise map or stitching engine. Empty, duplicate, overlapping, gapped, reordered or partial definitions cannot satisfy the expected single relationship. Unsupported mapping versions and absent mapping pins produce `missing_source_mapping`; malformed known-version metadata fails binding. Neither descriptor JSON nor a serialized report authenticates execution.

Runtime owns every expected pin: complete authoritative source file/scope/count/origin, selected full-source span, the base Source role's actual slice FileRef/local scope/span and the exact relationship FileRef. Generation may propose relationship bytes but cannot select trusted expectations or authorize work. Do not construct the runtime policy by trusting candidate fields. Full-source means the complete independently supplied file; its label is not an independent proof that it is a complete required chapter/book.

`base.inputs[Source]` still pins slice bytes. Base source hash/scope, fact inventory, reveal facts, read-along, scenes and evidence remain **slice-local**. Public manifest/read-along source hashes stay the slice hash. No playback-v1 span is reinterpreted or offset automatically.

## Explicit Unicode coordinates

Every count/span/origin is a nonnegative JSON-safe integer measured in Unicode scalars. A full file with `origin = O` and `scalar_count = F` occupies `[O,O+F)` in the trusted global domain. A selected span `[A,B)` must be contained in that interval and have length `N`, the actual slice scalar count.

The relationship must declare `local_origin = 0`, `local_span = [0,N]`, `global_origin = A`, and `full_source_span = [A,B]`. Checked private conversions are:

```text
global = global_origin + (local - local_origin)
local = local_origin + (global - global_origin)
full_file_index = global - full_source.origin
```

Use these only inside their half-open spans (the exclusive end is an endpoint, not a character). No UTF-8 byte, UTF-16 code-unit, trimming, normalization, punctuation or transcript substitution is permitted. The full file itself may have a nonzero origin, as G21's global source domain does. If full and slice scope IDs coincide, they must describe the same complete bytes/count and full interval; one ID cannot alias a proper subslice.

## Independent execution

```rust,ignore
verify_acceptance_v2(&document, package_root, &policy_v2, &additional_proposals, private_root)
    -> ContractResult<PrivateAcceptanceVerification>
```

This reuses the existing bounded safe reads/hash checks, opaque result and `recheck_inputs()`. Relationship metadata is capped at 1 MiB; full source at 32 MiB, within the existing cumulative read budget. Symlinks and unsafe paths fail closed. Both files are read through runtime-pinned FileRefs; descriptor file paths are never followed as independent authority.

The checker first verifies the actual candidate's local source/read-along/scene partition under I17 and matches local fact/scope metadata. It then reads the pinned mapping, checks every field against runtime expectations, reads actual full-source bytes, verifies their UTF-8/scalar count and compares the exact scalar substring `[A-O,B-O)` to the entire actual slice. Equal repeated text at a different position cannot replace the independently selected span. Missing/uncertain/adverse evidence still worsens results; all previous production blockers remain.

The new mode always requires `complete-source-coverage`, even in diagnostic scope. Generation must also declare it; omission still produces missing-declaration/coverage failures. A missing/unsupported relationship never falls back to an apparently successful local proof. The unchanged `verify_acceptance`/schema-1 policy still performs I17's file-local proof and keeps unsupported legacy nonzero subspans unresolved.

## Reports and compatibility

Report schema 3 uses checker `pt-acceptance-bindings-v3` and the **unchanged v2 matrix**. The checker source fingerprint includes the new relationship verifier. Optional private `source_slice: SourceSliceBindingV1` contains only expected hashes, source scope IDs, counts, origins, local/global spans and optional relationship hash. It has no FileRefs, paths or source text. A passing source row must consume the actual full, slice and mapping hashes. Missing mapping leaves that row Missing; a forged summary is not a capability.

Report validation retains paired versions 1/v1/checker-v1 and 2/v2/checker-v2; version 3 pairs with matrix-v2/checker-v3. Archived reports cannot claim mapped execution or silently acquire the new checker identity. Original v1/v2 matrix and fixture bytes, G20 acknowledgement and canonical diagnostics remain immutable. New v2 audit-fixture bytes preserve an actual prior diagnostic report and its original expected checker. Rust report construction now supplies `source_slice: None` for unmapped metadata; serde defaults preserve decoding of older JSON. Existing schema-1 policy structs and their callable API are unchanged.

TS `validateAcceptanceVerificationPolicyV2`, `validateSourceSliceRelationshipV1(value, expectedPolicy)` and `validateSourceSliceBindingV1(value, localBinding)` validate metadata/strict shapes only. Rust performs actual byte verification. `validateAcceptanceVerificationReport` checks all supported archive versions and consumption bindings, never execution authenticity.

Full source, slice relationship and raw evidence stay private. Known full-source/mapping hashes in a non-diagnostic public payload trigger the existing private-export binding failure. The verifier writes no payload files. Report summaries are private inspector/CAS metadata and must not be added to audience manifests/catalogs/HTTP exports. The full export-review verifier remains unimplemented. Historical diagnostic source assets are preserved; diagnostic rendering does not become production publication.

## Exact runtime R15 and generation adoption

Runtime builds policy-v2 from trusted compilation/registry state. For example, a full source at origin 600 with 1,200 scalars and a 400-scalar selected span `[700,1100)` requires:

- `full_source`: verified complete FileRef, its authoritative scope ID, `scalar_count: 1200`, `origin: 600`.
- `base`: unchanged schema-1 policy; Source is the exact 400-scalar slice file; `source_span: [0,400]`; source scope ID identifies the slice. All base fact/read-along/scene spans and hashes are local/slice identities. Refresh trusted `checker_sha256` and the separately pinned generation checker-identity artifact from the current compiled `acceptance_binding_checker()`.
- `relationship`: independently frozen private FileRef containing schema-1 relationship with those full/slice pins, `full_source_span: [700,1100]`, `local_span: [0,400]`, `local_origin: 0`, `global_origin: 700`.

Invoke `verify_acceptance_v2`, retain the in-process opaque result, call `recheck_inputs()` before later decisions, validate the private report against the current expected checker, and persist it privately under its actual hash. Read-only CLI `verify-acceptance` now selects schema-1 or schema-2 policy explicitly; unsupported or malformed policy versions never fall back. The `source-slice-fixtures --write-cli-fixture OUT` example creates synthetic private test input, not a production release. The verifier still exits 2 when checks execute but release is blocked.

Generation G21 keeps its original global source/turn/observation records and exact bytes unchanged. Propose the explicit relationship separately; create playback/fact metadata in the local slice domain using the stated checked conversion and retain global originals privately. Do not repoint old facts, shift playback scenes in place, relabel old checker files or narrow global uniqueness/semantic reviews into a new proof. Runtime independently chooses/fixes the expected pins and relationship hash; candidate proposals never become authority. No source acquisition, helper registration/fanout, new provider or publication path is granted.

Only complete scoped representation is improved. `complete-chapter`, `complete-excerpt`, speech fidelity, semantic fact/image/audio/creative checks, actual accounting/live authorization, profile/device evidence and publication remain separate. A fully represented slice does not certify the complete chapter. `production-verifier-pending` and all 33 missing-verifier gates remain.

## Test-first verification

The desired mapping test was first added against the absent policy/relationship/API, producing the recorded red compile. The existing missing-mapping case remains and still returns Missing. Positive cases verify zero/nonzero full and slice origins, astral/multibyte/combining characters, exact endpoints and complete-file identity. Negative cases cover byte/UTF-16 offsets, wrong hashes/counts/origins, truncation, gaps/overlap/empty/duplicate relationships, source/fact mismatch, repeated text substitution, punctuation changes, changed full/slice/map bytes, symlinks, bounded metadata, private export and post-receipt recheck mutation. Cross-language tests consume actual Rust reports and strict policy/relationship records, preserving archived validation. Exact code/checker/source/test/CI pins are in `source-slice-ready.json` and `pr-ci-results.json`.
