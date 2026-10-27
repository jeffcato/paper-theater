# G20: scoped requirements and private proposals

Generation package execution now consumes `required_acceptance_checks(AcceptanceScope)` from `pt-contracts`; the retained `REQUIRED_CHECKS` literal is only the integration-owned test's compatibility hook. Playback remains v1.0.0; the checked services revision is v1.5.0 (acceptance records are v1.4-compatible). All candidates remain diagnostic and NeedsRepair. No semantic review execution, release approval or provider authorization is inferred.

Requirements are the union of the selected shared scope, authored requirements, context additions, proposal check IDs and every adverse finding. Original authored acceptance is retained privately. Public findings collapse duplicate IDs conservatively (Fail, Uncertain, Missing, Pass), retain outcomes and remove private messages/receipt references. Earlier failures survive new passing proposals, duplicate records and invalid replacement scope bindings.

The executable [scope fixture](../../../fixtures/generation/acceptance-proposals-v1/scopes.json) records actual StageExecutor behavior:

| Requested purpose | Shared requirements | Coverage requirements |
| --- | --- | --- |
| Diagnostic | 3 | Production remains blocked |
| Autonomous excerpt | 37 | complete-excerpt, complete-source-coverage |
| Production chapter | 37 | complete-chapter, complete-source-coverage |
| Excerpt inheriting chapter requirement | 37 plus inheritance | complete-chapter remains required |
| Diagnostic below supplied chapter minimum | Chapter selected; context rejected | No fully bound proposal; custom/adverse requirements survive |

Missing/malformed contexts conservatively select chapter requirements. The maximum of requested scope, minimum and source purpose selects requirements; a mismatch cannot produce a complete binding. These are untrusted candidate declarations. Runtime must independently freeze its scope and expected identities, regardless of candidate labels.

Private input `acceptance-context.json` has schema version 1 and rejects unknown fields. See the [diagnostic example](../../../fixtures/generation/acceptance-proposals-v1/acceptance-context.json). Fields are `scope`, `minimum_scope`, `purpose`, `source_scope_id`, Unicode-scalar `source_span`, narration rate/count, `player_version`, `candidate_checker_version`, optional `device_id`, `inputs`, optional `checker_identity`, additional required IDs, optional inherited evidence inventory, `observations`, and original `reports`. Context metadata is limited to 1 MiB, 7 input roles, 1,024 observations, 64 reports and 128 additional requirements.

The seven input roles are source, narration, alignment, presentation map, recipe, profile and player. Each is an explicit FileRef verified against bounded bytes read through RunServices. A separate pinned JSON `AcceptanceCheckerIdentity` must equal the compiled shared checker identity. Generation does not read a moving UI tree or invent an implementation identity. Missing identities produce unresolved findings and an empty new proposal list. Synthetic player/recipe bytes in the positive test are explicitly diagnostic, never a real player build or production recipe.

Fact inventory records derive only from authored reveal facts with matching source hash/span, image-scope attestation, reveal contributor, actor and actual asset bytes. Fact IDs are `<reveal-id>-fact-<ordinal>`; every actual timeline cue ID is included. Missing associations generate `unbound-authored-fact:<id>` and cannot acquire dummy IDs/hashes or acoustic-derived image applicability. Partial inventories remain private and unresolved.

Each proposed `AcceptanceEvidenceProposal` binds exact payload inventory, source, native narration, alignment, presentation map, delivered master, recipe, profile, player, checker, fact inventory and all public asset hashes. Source spans, source scope, versions, device identity and source/narration/delivery sample domains/counts are explicit. Targets must match the shared check's target class, authored fact/asset association or bounded audio interval. Raw evidence must be a verified private input of at most 16 MiB; each proposal is at most 64 KiB. Context/output metadata is at most 1 MiB. Existing selected-master (256 MiB), other input (32 MiB/file, 64 MiB combined), 64-reference and helper limits remain unchanged.

Candidate writes through RunServices, in dependency order:

1. Public payload allowlist, then `inventory.json`.
2. `private/authored-acceptance.json` and, when source bindings permit, `private/acceptance-facts.json`.
3. `private/acceptance-proposals.json` (new observations) and `private/acceptance-evidence-inventory.json` (unchanged inherited records plus additions).
4. `private/acceptance-proposal-status.json`, binding the context, payload, fact/proposal/evidence inventories, original report references and unresolved observations.
5. Sanitized public `acceptance.json`, then `manifest.json`. `diagnostic-document.json` remains private inspection data.

The graph is acyclic: proposals bind payload/fact identities, never their own evidence inventory. Status binds the resulting inventories and is never an authority. Output artifact FileRefs and code hashes are recorded in [readiness](acceptance-proposals-ready.json). The [example directory](../../../fixtures/generation/acceptance-proposals-v1/README.md) contains private metadata only; its borrowed media are reconstructed by the tests.

Audience export uses exactly `PackageCandidate.payload_inventory` plus inventory/acceptance/manifest metadata. Raw source, alignment, native narration, recipes, context, player/checker identity inputs, receipts and private reports are excluded. The remaining source words in audience read-along/captions are intentional. Known private hashes cannot escape by changing MIME type/path; private aliases of required public entries fail closed. Actor images must remain in the allowlist. Package verification checks original payload bytes and strict path ordering; public metadata contains no raw-evidence FileRefs or private finding messages. StageOutput/PackageCandidate required-artifact metadata stays private.

The real export test writes only that allowlist, verifies it with `verify_package_files`, then invokes the independent shared acceptance checker from a separate private root and a test-harness policy. Even a fully bound claimed pass cannot execute image review or remove `production-verifier-pending`. Production code in generation never constructs a trusted policy/capability or invokes activation.

The original G16 acoustic, G18 presentation and G19 chapter reports are supplied as verified private FileRefs and retained verbatim, including nested original receipts/offsets/bounds and all seven outcomes. Four matched observations remain Pass; strange-event and balloon-ascent remain Uncertain; unprovided-fact remains Missing. None acquires an image association. Existing tests retain 176 words, 170 groups, six measured points, G17's temporary 180-second master, exact G18 PCM/mapping and G19's temporary 600-second bounded reconstruction. G18's frozen report remains `a53a04f1a589afc30b4f001fea726cc474965a2a2728a204f888c39a8aa747db`; both helper recipes remain unchanged.

Replay reconstructs identical private artifact bytes through candidate writes without helper/provider redispatch, candidate reads, journal emission or library updates. Supplied recorded private candidate FileRefs must match the reconstruction; a stale input/version rejects the result. Scratch candidate writes can precede rejection; no persistence/accounting/catalog authority is granted. Fixture mode can still propose its diagnostic journal event.

TDD evidence: scope selection, private record aliases, unknown scope authority fields and replay journal emission failed before implementation. `invalid_scope_retains_inherited_requirements_and_adverse_records` subsequently failed on lost `retain-custom`; preserving typed inherited records separately from binding eligibility made it green. `unbound_adverse_observations_keep_their_original_outcome` failed when an absent checker reduced Fail to Missing; Fail/Uncertain/Missing outcomes now survive unresolved bindings. The existing 180-second export test exposed copying all candidate writes; its exporter now uses the public metadata allowlist and asserts private source/directories are absent. All 62 generation, 25 Python, four candidate API, nine shared acceptance and one runtime package tests pass. Exact commands/counts are in readiness. Generation used zero provider calls, no UI edits and no Git operations.

Integration: apply the [exact compatibility replacement request](acceptance-compatibility-request.md) before asking generation to remove the literal. No live integration agent was available in this session; acknowledgement and cleanup remain pending. The shared compatibility suite currently passes with the literal retained.

Runtime: supply independently pinned context purpose/minimum/source scope, seven exact input roles, player bytes/version and checker-identity bytes. Freeze the public payload identity; consume the generated fact/evidence FileRefs as the remaining two roles of a trusted nine-role `AcceptanceVerificationPolicy`. Construct that policy from runtime's own expected records, preserve all inherited/adverse inputs, execute `verify_acceptance` against separate public/private roots, and recheck inputs before later use. Persist reports privately. Candidate JSON is never the verifier capability or activation decision. General chapter evidence import, the [G19 registration/fanout](chapter-acoustic-registration.json), semantic/device reviews and full production acceptance remain pending.
