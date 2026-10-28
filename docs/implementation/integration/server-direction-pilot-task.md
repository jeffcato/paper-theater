# R18: durable saved Source/Direction/Pilot composition

Prepared October 27, 2026. Owner: `server`. R17 public provider services are implemented with injected offline proof only; ordinary construction/live credentials/transports remain unavailable. G22 Direction and G23 Pilot are ready for runtime composition. Work in backend runtime/server/tests/fixtures/docs only; no UI, real provider calls or shared Git operations.

Before changing overlapping runtime files, provide integration I20 an immutable snapshot of the completed R17 handoff. Preserve its aggregate-failure history; new R18 work must not erase or be mislabeled as that checkpoint.

## Small R11 adoption first

Apply only the stateless `runtime/media.rs::validate_wav` delegation in the [I19 server handoff](acceptance-audio-format.md). Use **inspect_pcm16_wav**, keeping existing size/hash/open-file/range/error handling. Do not apply delivery-only constraints to native helper/narration audio. Run `wav_conformance`/`wav_inspector` before and after; record mechanical refactor parity honestly rather than manufacturing a red test. Changes to shared tests/expected pins go to integration.

## Runtime vertical slice

1. Consume the exact [G22 Direction request](../generation/direction-runtime-request.json) and [G23 Pilot request](../generation/pilot-runtime-request.json), including their original source/registry/catalog/recipe/selection/provenance bindings. Freeze separately namespaced immutable input scopes before seeding. Caller plan/catalog/receipt fields cannot set expected runtime identities or approve casting.
2. Route Source → real Direction → real Pilot in an explicitly pinned saved diagnostic recipe. Commit proposed request/proposal bytes before supplying real run-owned ArtifactRefs on the next phase. Persist exact raw observations and original issued/result/audio identities; never fabricate an ArtifactRef, relabel a receipt or authorize requests from candidate JSON.
3. Exercise the ordinary runtime path using pinned saved inputs only. Test R17 integration with explicitly injected test-only transports/grants where necessary. Keep ordinary production construction unavailable; no new live enable flag, credential lookup, HTTP provider route or legacy/helper fallback. Pilot-only grants cannot authorize bulk Speech.
4. Persist private direction/cast/pilot/request/observation/status artifacts and every unresolved/adverse finding atomically with source/revision/epoch/checkpoint. Diagnostic continuation through NeedsRepair is recipe-specific; never turn saved/synthetic plans or silent test WAVs into completed voice/research/semantic approval. No accepted library voice/image binding or public payload export.
5. Prove cancel before/during request/result commit, crash/restart reuse, changed source/registry/catalog/recipe/raw/audio rejection, exact two-phase mapping and byte-identical saved replay without redispatch, added accounting/catalog state or duplicate observations. Keep unknown/obsolete results attached to their original work and preserve private/public allowlists.

Test-first for the missing actual Direction/Pilot runtime route and commit seam, then implement minimally. Retain R11–R17 regression requirements, source/checker/matrix identities and fail-closed missing frozen-player registration. Return exact input/phase mappings, source/test/binary pins, request/attempt counters, red/green evidence and owner requests. No browser/UI rerun is required; R12 Nonpositive viewport remains separate.

Integration alone reviews/stages/commits/pushes. G24 Speech work remains a later runtime phase, and real auditions, bulk synthesis, semantic/listening/device acceptance and publication stay gated.
