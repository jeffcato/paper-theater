# Generation diagnostic handoff

The bounded first handoff is implemented. Source freezing, offline acoustic validation and diagnostic package assembly implement the shared StageExecutor through RunServices. The other nine production stages return explicit NeedsRepair findings. No autonomous production excerpt or accepted full chapter is claimed.

The two frozen chapters come from cached Gutenberg editions. Oz preserves the existing 5,958-scalar body and source hash exactly. Wells selects Book One Chapter I, “The Eve of the War,” with 12,654 scalars after whitespace/formatting cleanup. Selection checks actual opening/ending prose and parent-book boundaries. Raw editions, clean derivatives, hashes and per-scalar raw mappings are retained locally. Contents headings and repeated Book Two numbering cannot satisfy the selector.

The pure acoustic adapters replay 12 saved source turns. All 176 source words, their measured points and all 170 highlight groups match the imported report exactly. The scoped `Em`/`M` normalization remains explicit; it does not certify speech fidelity. Independent raw responses and all consumed media/response/receipt identities remain bound to the normalized derivative. The derivative has its own recipe/hash. It grants no reveal permission.

Conservative fact matching requires complete unique source-turn text from both models, ordered independent intervals, unique terminals, complete fact containment, matching source/audio/response identities and endpoint agreement within 0.75 seconds. It uses the later endpoint. A matching final timestamp alone cannot pass. These functions produce diagnostic observations, not production attestations.

Measured geometry tests use the cached registration and preserve native aspect/contact pivots. Body scale is a separate semantic measurement; diagnostic white mattes, canvas dimensions, nonfinite values, missing/unmeasured contacts and invalid crops fail. The local preflight validator consolidates missing subjects, empty/unready entrances, reference leaks, duplicate owners, impossible/omitted/unsupported actions, incomplete opening/action/hold/image/audio review, stale source and uncertain findings. Purposeful stillness, multiple pictures and compatible reference reuse pass structural preflight.

The candidate in `fixtures/generation/diagnostic-candidate/` reuses shared saved media unchanged. It writes sorted inventory, acceptance, manifest and load document in dependency order. Existing diagnostic fixture attestations retain their exact synthetic/imported provenance; none is promoted to autonomous evidence. All required production checks remain blocking. Existing Fail and Uncertain findings survive assembly. The executor verifies service-returned candidate identities and rechecks cancellation after writes and journal emission.

Verification:

```sh
python3 -B -m unittest discover -s tools/generation -p 'test_*.py'
PT_GENERATION_EXPORT=fixtures/generation/diagnostic-candidate cargo test -p pt-production generation:: --offline
cargo test -p pt-production runtime::tests::real_generation_candidate_keeps_saved_media_and_missing_acceptance_private --offline
python3 tools/check_product_identity.py .
```

Results: 11 Python tests, 4 generation tests and 1 runtime integration test pass. The package export also verifies its exact written bytes with the shared package checker. Offline services panic on provider dispatch; replay additionally runs with network sockets forbidden. No provider call, credential access, accounting reset or publication occurred. Identity scans include source, frozen sources, original imported responses, normalized evidence and the candidate export.

TDD evidence: new source/replay/matching/geometry/preflight tests first failed for absent implementations; negative matching/preflight/geometry cases then failed before their validators were added. Rust package execution first failed for absent GenerationExecutor; source execution returned NeedsRepair before implementation. Preservation of failed findings first returned Missing, and substituted-candidate rejection first accepted the wrong path; both were corrected before the final passing runs. The shared pre-freeze cue failure was repaired by its owner. Pure adapter extraction began before the later TDD instruction was added.

`handoff.json` records consumed contract/profile/fixture identities, source freezes, commands/results, exact unresolved reveal scopes/action/review IDs and next integration work. Neutral component/file/content provenance is in `reuse-provenance.json`. Remaining boundary changes are specified in `integration-requests.md`.

The [offline acoustic stage](acoustic-stage.md) now uses the registered helper and verified candidate-reader boundary. Next bounded integration: route its stage-specific input bundle through the saved-fixture runtime composition, then replay an explicit fact inventory through conservative matching. Keep ambiguous independent matches unresolved. Later live work still requires product spending/quota authorization through the runtime broker, inherited accounting reconciliation, real voice/acoustic/visual/music/mix reviews, browser capture and a verified rendering profile. After an accepted autonomous excerpt, both complete chapters remain required.

A separate [frozen capture sampling handoff](capture-inventory.md) now supplies 48 sample positions / 144 player requests for openings, reveal/transition boundaries, duration-labeled holds, paired action motion and terminal state. Its offline state probe reports the late incoming picture and missing `nod-em` gesture state; it contains no rendered images or production approval.

The separate [readiness v2 proposal/publication](readiness-v2.md) relocates the formal transition into a measured sentence pause, preserving every retained PCM frame and all source/highlight identities. Scene 2 becomes eligible before fade-in; v1 stays frozen. V2 has an immutable file index, new master/map/manifest identities and its own capture requests. Production reviews remain pending.
