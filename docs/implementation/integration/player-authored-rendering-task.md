# Player task: render the authored diagnostic performance

Prepared October 26, 2026. Owner: `player`. Tracks **P12** in [TODO.md](../../../TODO.md). This task can proceed without the server agent, new artwork, credentials, or provider calls.

## Outcome

Make the existing shared player render the saved registered figure according to the evaluator's actual instance visibility, transforms, and gestures. Produce browser evidence that `nod-em` visibly changes the targeted figure and that backward seeking hides unrevealed instances. The static orientation fixture remains useful calibration evidence; this increment connects authored state to the rendered performance.

## Inputs

- Code baseline: `32fca28` provides visible final-composite capture; `10d512e` fixes instance-targeted cue dispatch. Preserve both increments.
- Published diagnostic fixture: [two-scene-readiness-v2](../../../fixtures/generation/two-scene-readiness-v2/revision.json), reported published by `generation` as `8969fb3` on `generation/readiness-v2`. Its generation-owned content matches local `0ca5d42`; both identify the same immutable v2 package.
- Expected manifest SHA-256: `2070bb5f88d5d68fe6156aa4df17673634fc110584ec5f7c03d82bf5bddc502b`.
- Published sampling inventory: [capture-readiness-v2](../../../fixtures/generation/capture-readiness-v2/inventory.json), 48 positions / 144 requests over three viewports. Both diagnostic entry-readiness checks pass. See [authored correction](../generation/readiness-v2.md).
- Current shared boundary: `pt-contracts-v1.0.0` / `pt-services-v1.2.0`. Exact registered geometry, reveal bounds and timeline samples remain authoritative.
- V2 figure eligibility is 30.51 s, before incoming fade-in at 38.410375 s. The original v1/spoken PCM are preserved; 17 Python plus five Rust generation checks are reported passing. These are diagnostic readiness results; production reviews remain pending.

## Implementation

1. Reuse the existing loader, evaluator, mount and Three.js sink. Bind the fixture's verified saved figure to scene instances, preserving native aspect, pivots, canonical scale and support placement. A book-local asset may be reused by multiple instances; each instance must retain its own state.
2. On every evaluated frame, apply instance presence, position, rotation/facing and supported rigid gestures to the corresponding scene objects. Filter ineligible contributors before every narrative render/shadow pass, including blackouts and backward seeks. Render the nod as actual motion around the registered pivot; metadata alone is insufficient.
3. Keep the canvas/mount integration compatible with studio. Add package-owned asset-binding API only as needed; rendering stays in `packages/player`. Release the textures, materials, meshes and listeners this increment owns.
4. Extend the existing player browser runner to accept the versioned fixture/inventory. Verify manifest/media and exact executed player/checker identities. The supplied inventory may name an earlier player build: preserve it, report the old binding, and create a fresh derived request inventory in player-owned fixtures using the existing version-aware generator.

For example, after building player, fresh requests can be generated with:

```sh
python3 -B tools/generation/capture_inventory.py --fixture fixtures/generation/two-scene-readiness-v2 --output fixtures/player/capture-readiness-v2/inventory.json
```

Do not overwrite generation-owned published inventories/probes or either fixture revision. Work only in player-owned packages, audience paths if needed, player fixtures, and player documentation. Shared schemas, root lockfiles and server files retain their existing owners. Follow the current shared Git ownership rule; report owned changes to `integration` for review/commit.

## Completion evidence

- Saved figure rendering follows the authored instance state at all 144 requested captures, with diagnostic/unaccepted status retained.
- Before/during/after nod frames show the expected targeted visual change and return; another instance sharing the asset is unaffected. Record actual pose and pixel/image evidence, not only state digests or `gesture: nod`.
- Sample boundaries, backward seeks, entry readiness, exact blackout hold and terminal state behave consistently. Final-composite blackout remains black on visibly rendered content.
- Each receipt binds request ID, sample, manifest/profile, current player/checker identities, contributor IDs, viewport and actual image hash. Explain any failed/uncertain case explicitly.
- Existing player tests and relevant Chrome/inventory checks pass; identity checks include owned source and capture outputs. Report exact commands/results and zero provider calls.

Return the owned file list, API usage for studio, capture index/representative frames, verification commands/results and remaining gaps. This task does not certify final shell artwork, full narrative review, target-device performance, or production acceptance.
