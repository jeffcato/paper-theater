# P1: finish shared playback type adoption

Prepared October 26, 2026. **P1 local implementation complete:** [mapping](../player/shared-types.md), compile-red evidence and 121/121 checks reported. Root lockfile/locked-install follow-up remains integration I14; P16 repair is now locally green and next player task is [P18 geometry proof](player-geometry-task.md). Owner: `player`. Work in `packages/player/**`, audience consumer types/tests if needed, and player-owned documentation. The package import/export defect was resolved by `cf3fe27`; the shared playback revision remains `pt-contracts-v1.0.0`. Provider service revisions are independent of this playback migration.

## Observed gap

`packages/player/src/types.ts` still declares provisional copies of public wire records. `shared.ts` documents a deferred type-level migration, and player does not declare `@paper-theater/contracts` as a dependency. Frozen-fixture runtime compatibility already works; preserve its adapters, validation and behavior rather than building a second schema.

## Required increment

1. Consume the public package exports for wire identities/records: manifests/FileRefs, playback timeline and tagged cues, discriminated lights, registered actors, read-along, reveal/evidence records and applicable policy/revision identities. Use aliases or explicit typed adapter boundaries where the player normalizes a public record into a private evaluation view.
2. Retain genuinely private render/evaluation state, flat views, transport handles, loaded-handle branding and mount/capture types locally. Do not force private structures to masquerade as public records. Public inputs should accept the frozen shared shapes without unchecked casts hiding incompatibility.
3. Preserve runtime validation of untrusted JSON and unsupported action rejection. A TypeScript annotation does not validate bytes or authorize a package. Keep accepted-handle provenance gates, diagnostic rejection, all media bounds and fail-closed capture unchanged. Do not weaken types to `any`, broad index signatures or unsafe double casts merely to compile.
4. Declare the actual workspace contracts dependency in the player-owned package manifest. Root lockfile/config changes belong to integration: provide the exact request and proceed with the existing workspace package link for local work; never regenerate/stage the root lockfile from this lane. Record lockfile integration as pending until the owner applies it.
5. Remove obsolete comments about waiting for the contracts freeze/package exports. Publish a clear mapping of public records to retained private normalized views and any real wire mismatch requiring integration action. Do not change frozen playback contracts, fixture bytes or provider-service schemas.

## Test-first proof and completion

Add meaningful strict TypeScript consumer cases that pass actual shared tagged-union timelines/lights/read-along into the public player boundary and reject incompatible actions/shapes. Record the observed failing test/compile diagnostic before the minimal migration and the passing command afterward; do not manufacture a failure for a case already supported. Test unsafe JSON separately through existing runtime validators.

Run the affected package typecheck/build, strict consumers and player suite, then relevant title/recovery/lifecycle/chapter/rendering/audience regressions for touched boundaries. Preserve the current verified fixture hashes and both original v1 readiness failure/v2 eligibility. If executable build identity changes, refresh only genuinely affected receipts and explain identity-only updates; retain immutable original evidence.

Return owned paths, exact red/green evidence and commands, the type mapping, compatibility findings and precise integration dependency/lockfile request. Preserve the uncommitted P17 title fix/evidence; make no shared Git operations. Integration alone stages/commits/pushes this checkout. Zero provider calls.

P17's player component is complete; server still owns rerunning the strict R12 studio check. This task does not claim actual demo-device performance, full browser/profile acceptance, an accepted audience chapter or production publication.
