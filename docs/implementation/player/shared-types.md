# Player shared-types mapping (P1)

Adopted October 26, 2026. Shared revision `pt-contracts-v1.0.0`
(`@paper-theater/contracts@0.1.0`, `CONTRACT_REVISION` in
`packages/contracts/src/index.ts`). The player consumes the shared contract
types directly; every legacy player record name below is now an alias of the
identical shared type, not a fork.

## Public records: player name -> shared type

Identical shape, kept under the stable player name:

- `Digest`, `Id`, `Sample`, `Interval`, `SourceSpan` (wire identities)
- `FileRef`, `ReleaseRef`, `Provenance`, `Manifest` (manifests/FileRefs)
- `RegisteredCard`, `SceneInstance` (registered actors)
- `RevealFact`, `RevealGroup` (reveal/evidence)
- `SceneTransition`, `Easing`, `StoryCue` (timeline; cues are tagged per
  action — see shape changes)
- `ReadAlongWord`, `ReadAlongGroup` (read-along words/groups)

Pure renames, aliased (`export type { Shared as Legacy }` in
`packages/player/src/types.ts`):

- `CameraSpec` -> `Camera`
- `LightSpec` -> `Light` (shape change — see below)
- `LightingSpec` -> `Lighting`
- `StageSnapshot` -> `Snapshot`
- `SceneRecord` -> `Scene`
- `OpeningRecord` -> `Opening`
- `ClosingRecord` -> `Closing`
- `CompiledTimeline` -> `Timeline`
- `ReadAlongTrack` -> `ReadAlong` (already a field-for-field match,
  including `policy: 'point-following-same-boundary-v1'` and all digest
  identities)

Derived, never forked:

- `SupportedAction` = `CueAction["action"]` (the seven production actions)

Newly available shared vocabulary (re-exported for consumers):

- `Facing`, `CueAction`

## Intentional shape changes (shared schema wins)

1. **Tagged cue params.** `StoryCue.params` was `Record<string, unknown>`;
   it is now typed per action (`enter` carries `entrance_id/target/facing`,
   `gesture` carries `gesture/intensity`, and so on). `evaluate.ts` reads
   the narrowed params directly; `validateTimeline` keeps its runtime
   action allowlist for hostile bytes.
2. **Discriminated lights.** `LightSpec` carried optional spot extras on a
   flat record; `Light` requires `angle_radians/penumbra/distance/decay`
   exactly for `type: 'spot'` and forbids relying on them for
   `type: 'directional'`. `toLightView` enforces the same rule at runtime
   (`lighting: incomplete spot`).

## Retained private views (not public records)

- `SharedCueView` / `FlatLight` (`src/shared.ts`): flat normalization
  outputs. `toCueView(cue: StoryCue)` and `toLightView(light: Light)`
  annotate shared inputs and re-validate every field at runtime, rejecting
  hostile shapes with `unsupported action ...` / `lighting: ...`
  diagnostics. `normalizeCues` runs the cue adapter once at mount time and
  lets the validated shared cues flow on unchanged.
- `EvaluatedState`, `EvaluatedActor`, `PhaseKind` (`src/evaluate.ts`):
  pure-evaluation outputs.
- `MountHandle`, `MountOptions`, `MountSource`, `MountReadAlong`,
  `FrameCapture`, `FrameSink`, `GraphicsStatus`, transport/clock/sign
  types (`src/mount.ts`, `src/transport.ts`): handles and boundaries.
- `PlayerFailure`, `PlayerFailureCode`, `fail()` (`src/types.ts`): the
  player-owned failure taxonomy (distinct from `ContractFailure`, whose
  `code` is an open string). Runtime validators `isDigest/isId/isSample/
  isInterval/isFiniteVec/assertSampleRate` and media bounds
  (`MAX_AUDIO_BYTES`, `MAX_FILE_BYTES`, `MAX_TOTAL_BYTES`) are unchanged,
  as are accepted-handle provenance gates and fail-closed capture.

## Known wire mismatch (no integration action needed)

- Pre-contract `move` cues could carry a non-contract `origin` override;
  the evaluator honored it as the motion start. No producer emits `origin`
  (the shared validator's move case reads only `target`; no fixture, test,
  or lane references it), so the evaluator now always starts from the
  actor's current position. A cue carrying `origin` still validates and
  plays; the extra property is ignored.

## Proof

- Red (before): `tsc --noEmit -p tsconfig.json` on
  `tests/contract-adoption.test.ts` reported 4 errors — player timelines
  not assignable to shared `Timeline` (flat `LightSpec` vs discriminated
  `Light`), player cues not assignable to shared `StoryCue` (untyped
  params), plus two unused `@ts-expect-error` directives (spot extras,
  wrong-branch param access).
- Green (after): `npm test --workspace @paper-theater/player` — 121/121
  pass, including 6 new `contract adoption` cases covering shared
  timeline/lights/read-along at the public boundary, compile-time
  rejection of incompatible shapes, and runtime diagnostic rejection of
  hostile wire actions.

## Lockfile request (pending, integration-owned)

`packages/player/package.json` now declares
`"@paper-theater/contracts": "0.1.0"` (same convention as
`apps/studio`). Local work resolves through the existing root workspace
link (`node_modules/@paper-theater/contracts` -> `packages/contracts`,
built `dist` with types). No install or lockfile edit was made from this
lane. To record the dependency, integration should run at the repo root:

```
npm install --package-lock-only
```

and stage the resulting root lockfile change. Until then, lockfile
integration for this dependency is pending.
