# Immersive house-art manifest and fallback rule

## Proposed manifest record shape

The house-art manifest enables the full-viewport multiplane theater world to consume registered-style illustrated paper house art, referencing `docs/implementation/player/immersive-art-brief.json` as the authoritative source of truth for all 35 surfaces.

```json
{
  "schema_version": 1,
  "brief": "immersive-house-art-v1",
  "diagnostic": true,
  "accepted": false,
  "assets": {
    "house.facade": {
      "url": "house.facade-v2.png",
      "sha256": "247fba44cfd088e1b6d8272ca272e335e38bbbea85566ba1554365147490b40c",
      "deliver_px": [2048, 1245],
      "alpha": "aperture"
    },
    "lobby.grille": {
      "url": "lobby-grille.png",
      "sha256": "a5d5bb94987279281844be9346e0292681ab2593e5a78d879f34d4c8c8b1a578",
      "deliver_px": [2560, 1792],
      "alpha": "aperture"
    },
    "lobby.wallpaper": {
      "url": "lobby-wallpaper.png",
      "sha256": "061060b43415387ce57c2f47b9389ee197258964649f1695fbec4b2f82dee85a",
      "deliver_px": [512, 512],
      "alpha": "tile"
    },
    "lobby.drape-left": {
      "url": "lobby-drape-left.png",
      "sha256": "da61a3aa154785ab5799ccc2ddf1e5cf4dbcba0189a81f3377fa10a7f9607c71",
      "deliver_px": [352, 2112],
      "alpha": "cutout"
    }
  }
}
```

### TypeScript definitions

Defined in `@paper-theater/player` (`src/house-art.ts` and `src/house-brief.ts`):

- `HouseArtAssetRecord`: per-asset entry containing `url`, `sha256`, deliver size `deliver_px`, registration `alpha`, `nine_slice_px`, and `regions`.
- `HouseArtManifest`: manifest record specifying `schema_version`, `brief`, and dictionary or array of `assets`.
- `LoadedHouseAsset`: immutable verified asset record binding validated bytes to authoritative brief geometry.
- `LoadedHouseArt`: immutable handle holding the map of verified assets.

## Fail-closed loader binding

Bound in `packages/player/src/loader.ts` via `loadHouseArtManifest(manifestBytes, expectedDigest?, read)`:

1. **Hash-bound verification**: every asset's file bytes are checked against its declared SHA-256 digest (`house_art_digest_mismatch`).
2. **Authoritative geometry validation**: every asset ID must match one of the 35 brief surfaces (`unknown_house_art_asset`). Declared deliver dimensions and PNG IHDR headers must match the brief's `deliver_px` (`house_art_geometry_mismatch`).
3. **Availability**: read failures on declared assets throw `house_art_unavailable`.
4. **All-or-nothing**: any failure throws before returning any handle; a mount never partially applies a set.

## Per-asset fallback rule

- **Missing assets fall back**: if an asset from the brief's 35 surfaces is not included in the manifest, the immersive house gracefully falls back to the existing procedural code-painted card for that surface.
- **No manifest supplied**: with no manifest (or `null`/`undefined`), the house rig rendering is 100% pixel-identical to baseline code-painted cards.

## Geometry, alpha keying, and procedural preservation

- **Apertures**: cut away the opening along exact brief geometry (e.g., proscenium stage opening `u 0.1846-0.8154, v 0.2247-1.0`; box-office window `u 0.1625-0.8375, v 0.2813-0.8036`).
- **Segmented cutouts & keyed tiles**: mid-grey keying (`isMidGrey`, R/G/B within 18 levels of 128, neutral delta <= 12) keys out the flat background to full transparency (`alpha = 0`), creating clean die-cut paper edges.
- **Kept procedural**: typeset lettering (cartouche "PAPER THEATER", crossbar "BOX OFFICE", arch "STALLS", playbill text), ticket punch holes and paper shadows, sconce and footlight gaslight glows, curtain gather repeat, and brass rail lever/needle motion remain purely procedural.
