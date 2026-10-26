# Integration requests to Agent 1 (Agent 4)

Ownership reminder: only Agent 1 edits root manifests, lockfiles, shared
schemas/fixtures and the production crate manifest/module root. The following
are exact proposals with failing cases, not edits.

## 1. Dependency pins (already applied in owned manifests; needs lockfile)

`packages/player/package.json` pins `three@0.186.0` and
`@types/three@0.186.0` exact, with provenance in
[version-provenance.md](version-provenance.md). Request: run the root
workspace install so the shared lockfile records these pins, and confirm the
resolved `@types/three` patch. Failing case without it: a fresh checkout has
no pinned three/types revisions for the player build.

Note: the player does not yet declare `@paper-theater/contracts` as a
dependency — it cannot be imported until the §3 boundary fix lands. The dep
will be added together with the type-level aliasing in that follow-up.

## 2. Root `check:player` alias

Already present at root (`check:player → npm run check:player --workspace
@paper-theater/player`) and verified working from this lane. No change
requested; record that the handoff's required aggregate command exists.

## 3. Shared contract revision: consumed at the adapter boundary

`contracts-ready.json` froze `pt-contracts-v1.0.0` / `pt-services-v1.0.0`
with the canonical fixture digest, and this lane has consumed it as far as
the package interface permits:

- `src/shared.ts` normalizes the frozen tagged-union cues and
  discriminated-union lights once, at the adapter boundary
  (`toCueView`/`toLightView`, exported from the package index).
- `check:player -- contracts` runs the frozen canonical fixture through the
  real loader/evaluator: frozen digest recomputed byte-for-byte, diagnostic
  rejected in production with missing IDs reported, swap/terminal
  evaluation, and the owned negative cases (short hold, unknown action).
- Field names already match the frozen mirror exactly, so no record was
  forked. Full type-level aliasing (annotating adapter inputs with the
  shared types) waits on the boundary fix below.

### Boundary issue: contracts package not directly importable (Agent 1)

Reproducer (no product files touched):

```sh
node -e "import('@paper-theater/contracts').then(m => console.log(m.CONTRACT_REVISION))"
# ERR_MODULE_NOT_FOUND: .../packages/contracts/src/types.js
```

Two defects, both outside this lane:

1. The package `exports` map points at TS source (`./src/index.ts`) whose
   `.js`-suffixed relative imports plain Node cannot remap. No runtime
   consumer (player, studio, audience) can import the package. Repointing
   `exports` at the built `./dist` (which exists) fixes it; the studio will
   hit the same wall when mounting.
2. `src/validate.ts` does not compile under consumer
   `noUncheckedIndexedAccess` flags, so even type-only source imports break
   strict consumers' builds. Built `.d.ts` (skipped via `skipLibCheck`)
   avoids the bleed.

Until both are fixed this lane reads the frozen JSON records as data and
adapts the shared shapes structurally. No provisional schema was invented:
identical records keep the frozen names, and the adapters reject anything
outside the frozen allowlists.

## 3b. Original shared-contract proposal (superseded by the freeze)

`packages/contracts/src` is currently empty — no `contracts-ready.json has
been published. Until then the player exposes only Agent 4-local provisional
types (`packages/player/src/types.ts`, clearly marked internal) and consumes
no shared revision. Request: when Agent 1 publishes the revision, include the
player handoff records (`ReleaseRef`, `FileRef`, manifest/entries, registered
card, scene instance, reveal facts, scene/camera/light snapshots,
timeline/opening/transitions/closing, story cues, read-along words/groups,
shell/profile) with the exact snake_case names from the handoff. This lane
will then migrate the provisional types mechanically and re-run
`check:player -- contracts`. Do not invent a competing public schema in the
meantime.

## 4. Rendering profile freeze

Profile `paper-player-r186-v1` is selected and schematic-validated
(`check:player -- shell`), but shared freeze awaits Agent 1 per INTERFACES.md
(Agent 4 proves defaults with fixtures; Agent 1 freezes the shared revision).
Failing case for proceeding without it: generation registration has no frozen
profile to target. The measured geometry brief is in
[shell-brief.md](shell-brief.md).

## 5. Shared fixture probe (October 26, read-only, uncommitted /tmp script)

The built player was run against `fixtures/shared/two-scene-v1` without
modifying it. Results for the integration record:

- Manifest (`diagnostic`, 3,262,560 samples, 2 reveals, 5 assets) verifies
  end to end: all artifact hashes, WAV decode and PNG dimensions pass.
- `loadRelease` rejects it (`provenance:release`,
  `missing_acceptance:reveal-1-image_scope`); `loadDiagnostic` reports 6
  missing acceptance IDs without accepting. This is the correct first-slice
  behavior: the audience must keep rejecting this fixture.
- Shared `timeline.json` (2 scenes, 1 cue, 1 transition) passes
  `validateTimeline` with zero errors; swap boundary resolves scene-1 →
  scene-2 with gain reaching exactly 0; terminal state closes the curtains.
- Shared `read-along.json` (176 words / 170 groups,
  `point-following-same-boundary-v1`) loads into `ReadAlongModel`
  unchanged, laying out 8 chunks.
- No shared-record fork was needed: the shared tagged-union `StoryCue` and
  discriminated `Light` shapes are runtime-readable through the player's
  `{action, params}` / optional-field views. The TS migration at
  contracts-ready time will narrow the shared unions explicitly.

## 6. Optional: pinned Tutor corpus

`tutor_status` exposes no source revision for the `three.js` corpus, so the
corpus name is not revision proof (see
[version-provenance.md](version-provenance.md)). If integration wants a
pinned 0.186.0 corpus, please build a separately named one from the pinned
source (`tutor_index`) — this lane did not block consumers on optional
tooling.
