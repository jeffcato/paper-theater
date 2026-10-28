# I21: reject plan-only Pilot approval

Services `pt-services-v1.9.0` separate unfinished plan structure from recorded evidence inspection. Empty reviews, digest-shaped strings and non-approval booleans cannot produce a Pilot Pass. Report/checker advance to v5. Published matrix v3 stays byte-identical: **five implemented checks and 32 pending production verifiers**, including `pilot-recording-review`. Complete synthetic recorded evidence can pass byte/target binding while its review remains Uncertain. No listening, bulk Speech, production or catalog authority is created.

## Source selection and actual regressions

The isolated candidate starts at published `ec9656f9118e2ccef3ca51c4427c1fffa62d08a9`. At inspection the two drafts were already local commits: Pilot `a1848f6cf63d4b966c9d271e49d8b4d2c6944b8d` and delivery mapping `4dabe3d1a630f36a01f8f8d53eed5fe984c619f7`, both ahead of published main. Their original commits, patches, source/test pins and working edits are preserved; neither is an ancestor of this correction. The unrelated delivery-mapping/v5 implementation, draft matrix identities, generation fixture edits, TODO/PM work and current root checkout remain outside it. No history is reset or rewritten.

The original public-verifier case was retained as `pilot_recording_review_rejects_plan_only_without_recordings_or_reviews`. Its desired Missing outcome initially returned Pass. A second public case showed nonempty target strings still passed without recordings. Further red cases exposed lost original plan Fail findings, a Fail hidden by missing targets, a serialized Fail downgraded to Missing, a private evidence alias and paired provider records selecting the wrong recipe or omitting actual take inputs. Lossless original failing outputs and exact red/green commands/hashes are recorded in the ready/source receipts. Compilation reds for new interfaces are distinguished from these behavioral failures.

## Private contracts and callable APIs

```rust,ignore
validate_pilot_plan_structure(&serde_json::Value)
    -> ContractResult<PilotPlanStructureV1>

verify_acceptance_with_pilot(doc, package_root, base_policy,
    &PilotReviewPolicyV1, proposals, private_root)
    -> ContractResult<PrivateAcceptanceVerification>

verify_acceptance_v2_with_pilot(doc, package_root, source_slice_policy,
    &PilotReviewPolicyV1, proposals, private_root)
    -> ContractResult<PrivateAcceptanceVerification>
```

The existing v1/v2 APIs remain available. An independently supplied Pilot policy always requires its own report row; a candidate that omits the declaration retains an explicit MissingDeclaration blocker. A legacy proposal may be structurally checked, but without independently selected recorded inputs its Pilot review stays Missing. `validate_pilot_plan_structure` returns only a digest, take/review counts and preserved false approval flags. Zero reviews are valid unfinished plan structure. This result is not a recording or listening receipt.

`PilotReviewPolicyV1` is private runtime-owned expected input, not a candidate proposal. It pins the plan, plaintext Source role, original Source-stage receipt (`source_record`), Direction, canonical registry, voice catalog and recipe. Each expected performer specifies speaker/canonical/model/voice identity and two to four ordered take records. Every take has exact committed request, raw response, observation, native audio ArtifactRefs/FileRefs and a Unicode scalar source span. The plaintext source hash must match the independent acceptance binding; the original Source record stays separately pinned.

An optional recorded review pins its committed request/raw response/observation, original reviewer observer/provider/model, reference recording, assembled recording, join recipe and music context. Absent review/reference/assembly/join/music inputs remain unresolved. These fields must be supplied from independently verified runtime state. Deriving expected pins from a candidate's own plan or copying a Pass field establishes no authority.

Inspection reads the actual hash-verified private bytes through the existing safe path, file/aggregate limits and immutable recheck pins. It compares exact request, response, observation and audio identities, actual source substrings, expected canonical/voice/model/recipe fields, complete take targets and request/response links. Native WAV inspection uses the shared stateless PCM16 inspector, verifying physical frame/rate/channel/bit declarations without decoding floats or modifying media. Metadata is bounded to 1 MiB, source to 32 MiB, each native WAV to 8 MiB, policy references to 96 MiB/256 entries and policy JSON to 64 KiB; the shared 512 MiB cumulative read budget remains.

If original v2 issuance/result records are present, shared validators check their pairing, current/completed disposition, expected recipe/model/reviewer, exact payload/raw response, required context and take inputs, and original audio output identity. No retry/recovery or obsolete result becomes current evidence. Absent provider metadata and saved/imported/synthetic provenance never authenticate a reviewer. These are reads only; no reservation, unknown charge, quota, repair, original receipt or operation identity is changed.

A review must target the entire expected adjacent take set plus its actual reference/context. Both request and response require exactly one occurrence of all eight criteria: listening, identity, continuity, intelligibility, speech fidelity, room tone, safe joins and music context. Partial/duplicate/stale targets or criteria fail closed. Original plan/observation findings and recorded Missing/Uncertain/Fail criteria remain adverse. Hash-bound adverse metadata is retained before inspecting recording/context dependencies, so deleting a dependency cannot hide an already recorded Fail. Metadata values and failed reads are cached within the inspection; missing metadata is never retried or reinterpreted, and audio is not duplicated. A claimed Pass is retained as a claim and becomes effective Uncertain.

## Reports, compatibility and remaining authority

Private `PilotReviewObservationV1` records policy/plan/source hashes, consumed hashes, actual inspected counts, byte/target binding outcome, individual claimed/effective criterion outcomes and `production_review_executed: false`. It adds no raw prose, audio, response bodies or private filesystem paths. Known Pilot artifacts cannot alias public package payloads. This check does not clear the separately pending complete public-export review.

Rust and strict TypeScript validators agree on policy, structural, recorded and report records. They reject unknown fields, unsafe/oversized references, promoted criteria, invented production review execution, unconsumed policy/source hashes, forged Pilot Pass rows and adverse-result downgrades. The opaque verification handle and `recheck_inputs()` remain required; serialized metadata is not an authenticated execution capability.

Published reports/checkers v1–v4 and matrices v1–v3 retain their original identities. A genuine synthetic v4 report is archived from the published baseline generator; it is not relabeled as a v5 Pilot inspection. Archived reports cannot acquire the new field. Unpublished v4/v5 matrix drafts remain preserved in their original local commits and are not advertised as supported published boundaries. Playback and provider-service/envelope records remain unchanged. The executed checker fingerprint includes the Pilot inspector **and** provider metadata/service validators it calls.

The missing production boundary is authenticated independent reviewer execution/provenance and genuinely executed listening, continuity, joins and score-context judgments. This increment deliberately cannot return Pilot approval, even if all copied criterion labels say Pass. `pilot-recording-review` remains `executed: false`; the private observation describes executed recorded-input mechanics separately. `production-verifier-pending`, NeedsRepair, all other missing reviews, actual accounting/live authorization, UI/profile/device and catalog gates remain.

## Exact runtime and generation adoption

Runtime must build `PilotReviewPolicyV1` from its verified Source/Direction/registry/catalog and durable, immutable committed take/review records. Supply the actual Source-stage FileRef as `source_record`, the independent plaintext Source-role FileRef as `source`, original observer identities and complete expected ArtifactRefs. Keep request registration, original result/raw/audio persistence, accounting, epoch checks and private report commit/recheck ownership. Until those exact inputs exist, keep the legacy Missing path. The v2-with-Pilot API retains independently pinned source-slice mappings; no global/local coordinate conversion is inferred.

Generation keeps proposing G23/G24 v1 private requests/observations/raw envelopes; the shared inspector consumes those existing identity shapes. Retain the exact recorded phase and original request/raw/audio bytes. The fixture/compiler's `format_verified`, declared passage coverage or false approval flags do not satisfy listening. Reference, assembled audio, join recipe and actual score context must be committed and correctly targeted before their bindings can be complete. No runtime or generation implementation is edited here; authenticated reviewer integration needs a separately versioned owner handoff.

Exact source pins, actual final scoped tests and exact-head CI are in `acceptance-pilot-review-source.json`, `acceptance-pilot-review-ready.json` and `pr-ci-results.json`. I20's earlier green result is not reused. UI/player/Studio source and dependency files are unchanged from the explicit published baseline. Their existing gates are not cleared by this backend correction. Zero provider calls or credential loads.
