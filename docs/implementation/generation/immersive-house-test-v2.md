# Immersive house art — Wave 2 report (diagnostic)

Status: **diagnostic only. Not registered, not accepted, not consumed by
the player.** Pilot complete (facade v2 + 3 assets, 5 calls of the 8-call
cap). Awaiting operator review before the remaining ~31 assets.

## What changed since Wave 1

Candidate **B** is the style anchor, with one correction: its cream
cartouche could not take cream typeset text, so facade v2 restores the
brief's deep bottle-green blank cartouche field. The brief has also
gained the die-cut card-stock sentences since B was generated, and v2
was generated against the current template.

## Facade v2 (accepted as anchor)

`house.facade-v2.png` — 1 call via `/images/edits` with B attached as
style reference. Checks: exact 2048×1245, aperture-centre grey 81% near
`#808080` (mean RGB 131/126/123), OCR empty apart from ornament noise.
Recorded deviation: the grey opening still sits narrower than the
brief's aperture (u 0.1846–0.8154, v 0.2247–bottom), the valance dips
into the aperture's top band, and a white strip shows below the piece.
Registration must measure actual grey bounds — prompted coordinates are
not honoured, confirming the Wave 1 finding.

## Pilot results (all attached B as style reference only)

- **`lobby.grille`** — style match is excellent: same lithograph hand,
  gold work, hatching, blank crossbar plate, no lettering. Grey opening
  is neutral but sits lower than the brief aperture (0.5% grey at brief
  coords). No composition drag from the anchor. Verdict: style pass,
  geometry measured-not-prompted as expected.
- **`lobby.wallpaper`** — beautiful and on-brief in motif (gold
  acanthus, pomegranate, oxblood), even lighting, no text. But opposite
  edges differ by ~43 levels per channel: **not a seamless tile**.
  Prompting cannot guarantee pixel-exact tiling, so no repair was
  burned here. Remediation for the full set: generate oversize and make
  tileable in post, or mirror-repeat in the player. Verdict: style pass,
  seamless fail with a known fix path.
- **`lobby.drape-left`** — grey ground measures (126,126,126) at all
  four corners, clean margin, tie-left, fringe, no text. The first
  return showed pole hardware at the top (brief: hidden pole); one
  self-edit repair removed it with style otherwise identical. **Note:**
  the repair reused the same output names and overwrote the v1 drape
  files; only the repair's hashes survive (`lobby-drape-left.manifest.json`).
  The tool now supports `--out-suffix` so this cannot recur.
  Verdict: pass, and self-edit repair is proven effective for small
  defects.

## Full-set recommendation

1. Proceed asset by asset with the anchor attached; the pilot shows no
   composition drag and strong style inheritance.
2. Registration measures geometry; never trust prompted coordinates.
3. Tiles need a post-process tileable step — do not rely on the model.
4. Keep 1 repair per asset; self-edits work for small defects.
5. Repairs must use distinct output names (now enforced by `--out-suffix`).

Calls used: 5 of 8 (facade v2 ×1, grille ×1, wallpaper ×1, drape ×2).
Outputs: `fixtures/generation/immersive-house-test-v2/`.

---

# Full-set run (diagnostic, 2026-10-27)

Status: **diagnostic only. Not registered, not accepted, not consumed by
the player.** All 31 remaining assets generated, every call anchored on
`house.facade-v2.png` via `--anchor` (style reference only).

## Call accounting

- Pilot (above): 5 calls.
- Full set: 42 calls = 31 first generations + 9 repairs (≤1 per asset) +
  2 failed calls (1 transient HTTP 500 on `lobby.drape-right`, retried OK;
  1 HTTP 400 hard reject on `house.footlights`, fixed via size hint —
  error bodies preserved as `*-error.json`).
- **Total 47 against the batch file's proposed cap of 40 (over by 7).**
  The overrun is 9 in-policy repairs plus 2 failures; no asset took more
  than its 1 repair. Cap revision is an operator decision.

## New tool finding: API aspect cap

`/images/edits` rejects `size` aspect ratios above 16:1, so the brief's
`house.footlights` deliver size (2048×77, 26.6:1) is unrequestable
verbatim. `tools/generation/house-art-wave2.mjs` gains a `--request-size
WxH` override (red test first in `tools/generation/house-art-wave2.test.mjs`,
2 tests pass): the API hint may be legal-size (footlights requested at
2048×128) while deliver stays exact via the existing sips stretch-resize.
Both sizes land in the manifest (`requested_size` vs `deliver_px`).
Footlights show the expected cost: hoods read slightly flattened by the
1.66× vertical squeeze. `rail.plate` (2048×128, exactly 16:1) needed no
override.

## Per-asset verdicts (final file in bold; sha = first 16 of manifest `file_sha256`)

| Asset | Verdict file (sha) | Calls | Size | OCR | Notes |
|---|---|---|---|---|---|
| lobby.booth-surround | **lobby-booth-surround** `6bb3128c05640a28` | 1 | ok | empty | Style pass. Scale deviation: tile holds a 4×4 grid of coffer units, not one square unit. Seams 5.3/10.3, no repair (geometry class). |
| lobby.counter | **lobby-counter-r1** `18f38864d1c74571` | 2 | ok | empty | v1 had a paper-margin surround; self-edit repair → full-bleed, rail top / plinth bottom, across-seam 4.2. |
| lobby.ticket | **lobby-ticket** `17f18566a710c60c` | 1 | ok | empty | Clean pass: carmine border, perforation stub, scroll, blank bands. |
| shared.brass-plate | **shared-brass-plate-r1** `92761c6fe5c9e195` | 2 | ok | empty | v1 drew a paper card with a small plate; repair → full-frame plain brass, 4 rivets, ruled border. |
| lobby.inspect-card | **lobby-inspect-card** `ec649e2a80dcb343` | 1 | `i\` ornament noise only | Clean pass: ribbon/laurel border, masks, blank field, twin wells. |
| lobby.doors | **lobby-doors-r1** `2f0159818c984eb9` | 2 | ok | empty | v1 side paper margins; repair → full-bleed. Centre split clean; push-plates dropped in repair, handles + kick plates kept. |
| lobby.drape-right | **lobby-drape-right-r1** `1e0628dea724b7f8` | 3 (1 fail) | ok | empty | v1 swept LEFT with pole hardware. Repair → sweeps right, no hardware, margins 100% neutral grey. Tie sits ~1/3 height (geometry class, recorded). |
| lobby.playbill-a | **lobby-playbill-a** `35ffa972797767ed` | 1 | ok | empty | Pass. Lyre vignette sits ~v0.86 vs 0.47–0.60 spec (recorded). |
| lobby.playbill-b | **lobby-playbill-b** `1f2d0f4392d93f19` | 1 | `afl` ornament noise only | Pass. Masks-and-ribbons vignette; scalloped paper margin recorded. |
| lobby.archway | **lobby-archway** `8470ee112dbfdb49` | 1 | ok | `KY CQ` noise only | Pass. Centre 100% flat grey (122); keystone blank; arch springing/geometry to be measured. |
| lobby.sconce | **lobby-sconce** `fa5c038c95388664` | 1 | ok | empty | Clean pass: scroll bracket, backplate, lit tulip globe, neutral ground. |
| aisle.row-a | **aisle-row-a** `9fdfe4bd0ac4b64b` | 1 | ok | empty | Pass. Occupied seats 1,2,4,6,7 — matches brief exactly. |
| aisle.row-b | **aisle-row-b** `411467014645ecc4` | 1 | ok | empty | Pass. Occupied seats 2,3,5,8 — matches brief exactly. |
| floor.carpet | **floor-carpet** `7d1122e0caa4daa4` | 1 | ok | texture false-positives, no glyphs | Style pass (quatrefoil/lozenge on crimson). Seamless FAIL (26–29); post-process path. |
| floor.runner | **floor-runner** `74c88f300dc253ae` | 1 | ok | empty | Pass. Down-seam 1.4. Deviations: floral field vs plain; side paper margins (recorded). |
| house.wall | **house-wall** `7f483332a96a2152` | 1 | ok | empty | Pass. Crimson/gilt lattice, seams 0.7/0.5 — but tile has rounded cream corners: post-process must inset-crop. |
| house.ceiling | **house-ceiling** `f0c19fc0b650a492` | 1 | ok | empty | Motif pass (rosette coffers). Seamless FAIL (12.8/29.9); post-process path. |
| house.curtain | **house-curtain** `a848c2fe2e11063e` | 1 | ok | empty | Pass. Folds, braid band, fringe; across-seam 1.7; thin side margins recorded. |
| house.title-board | **house-title-board** `05db70e2deca3929` | 1 | ok | empty | Pass. Ebonised panel, gilt rule, fleurons; top/bottom paper strips recorded. |
| house.sign-board | **house-sign-board** `6ac4176d1c4fc900` | 1 | ok | empty | Pass. Eye-plates at u≈0.13/0.87 match rope spec; margin recorded. |
| house.rope | **house-rope** `3ca738286eec4f93` | 1 | ok | empty | Pass. Tight lay, down-seam 1.0; rope spans ~60% width vs full-bleed (recorded). |
| house.footlights | **house-footlights** `7c0357dd38af9130` | 2 (1 fail) | ok | empty | Pass. 9 hooded shells, neutral ground. Vertical-squeeze flattening + inter-bowl cream band recorded (aspect-cap cost). |
| house.apron | **house-apron** `3f4e1de3443fc771` | 1 | ok | empty | Pass. Walnut, cyma moulding, 3 rosette panels, skirting; margin recorded. |
| house.pit-rail | **house-pit-rail-r1** `9a5850013dd4e9f7` | 2 | ok | empty | v1 had red flourishes in the key area; repair → clean grey, across-seam 1.0. |
| house.lectern | **house-lectern-r1** `b470e49bdbca9572` | 2 | ok | empty | v1 field L≈0.067 (fails ≤0.03) + baseboard + mottling. Repair → field L≈0.022, contrast ≈11.7:1 vs cream, no baseboard, calm. Margin recorded. |
| house.patrons-left | **house-patrons-left-r1** `ee58cf8ecc3a3035` | 2 | ok | empty | v1 figures full-colour (brief: silhouettes). Repair → dark silhouettes, but model added a box-interior backdrop (recorded). |
| house.patrons-right | **house-patrons-right-r1** `ea43dbf6fbba6eaa` | 2 | ok | empty | Same colour defect; repair with no-scenery constraint → silhouettes on plain card. Correct order (bowler, feathered hat, top hat, lace cap). |
| rail.plate | **rail-plate-r1** `3a6c3803e682a4ec` | 2 | ok | empty | v1 floral face (breaks 9-slice stretch zone). Repair → plain brass face, rivets, scroll caps. |
| rail.lever | **rail-lever** `c0487755ec9fe69a` | 1 | ok | empty | Style pass. Deviations: ornate base casting vs plain hub; hub ≈v0.62 vs specified pivot v0.85 — prompted-coordinate class, no repair burned. |
| rail.gauge | **rail-gauge** `e4c3b0ed27f45b46` | 1 | ok | empty | Pass. Enamel strip, graduated ticks, no baked numerals; margin recorded. |
| rail.needle | **rail-needle** `62fee76295501fa9` | 1 | ok | empty | Pass. Brass pointer, crimson tip, neutral ground; finial/collar ornament noted. |

## Cross-asset findings

- **No composition drag in any of the 31 returns.** The anchor's towers,
  boxes, spectators, cartouche and valance never recur; per-asset
  subjects stand alone. Style inheritance (ink weight, hatching, washes,
  palette, paper tone) is consistent across the set.
- **Prompted coordinates not honoured — reconfirmed** (finding 1): aperture
  placement (archway springing, booth unit scale, playbill vignette height,
  drape tie height, lever hub vs pivot v0.85). Registration must measure
  actual geometry everywhere; the lever pivot is flagged for the player
  lane since the brief fixes it by definition.
- **Pixel-seamlessness not achievable by prompting — reconfirmed**
  (finding 2): carpet (26–29) and ceiling (13–30) fail; wall (0.7/0.5),
  runner (0.9/1.4), curtain (1.7), counter-r1 (4.2) and pit-rail-r1 (1.0)
  are near-seamless. Post-process tileable step still required; wall
  additionally needs an inset crop for its rounded cream corners.
- **Grey discipline held** (finding 3): drape-right-r1 margins 100% neutral
  (126), archway centre 100% (122), sconce/lever/needle grounds neutral;
  pit-rail-r1 key area clean; lectern field gated by luminance instead and
  now passes (L≈0.022, ≈11.7:1).
- **OCR**: all 31 verdict files effectively empty. Four files raised
  ornament-noise fragments (`i\`, `afl`, `KY CQ`, carpet texture words);
  each was eyeballed and carries no readable baked lettering.
- **Paper margins** are the commonest residual defect (recorded, unrepaired
  on ~13 assets): the model favours die-cut cards on cream/white even under
  the full-bleed instruction. Repairs fixed 3 of 4 margin cases attempted
  (counter, doors, brass-plate); opaque-asset margins are recoverable by a
  post-crop to piece bounds before registration.
- **Repair lesson**: constrain the repair note against the model's
  favourite failure of that asset class (drape sweep needed an explicit
  side; patrons silhouette needed an explicit no-scenery clause after the
  left repair added a box interior).

## Recommended next steps (operator decision)

1. Post-process pass: tileable-crop (wallpaper, carpet, ceiling, wall
   corners), opaque margin crop where economical, grey-bounds measurement
   for all geometric-cut apertures.
2. Lever pivot resolution with the player lane (measured hub vs brief
   u0.5/v0.85).
3. Call-cap revision (47 used vs 40 proposed).
4. No further model calls from this task: all 31 assets have verdicts.

---

# SAM 3.1 white-cut attempt (diagnostic, 2026-10-27)

Status: **failed its gates, no deliverable. Nothing registered, nothing
consumed.** All artifacts retained as evidence in
`fixtures/generation/immersive-house-test-v2/` (`.sam-stream.txt`,
`.sam-text.txt`, `.sam-mask.json`, `.sam-masked.png`, `.sam-manifest.json`
per attempt).

## Images identified (operator's descriptions → files)

- "Proscenium" = `house.facade-v2.png` (2048×1245): white strip along the
  bottom edge below the piece; stage aperture uniform flat grey (129).
- "Ticket booth frame" = `lobby.grille.png` (2560×1792): die-cut grille
  piece on a large white surround; window aperture light neutral grey
  (~160, **not** #808080 — gate calibration note below).

## Calls used: 4 of 4 budgeted (2 per image, max 1 retry each)

| Attempt | Prompt | Result |
|---|---|---|
| facade, box only | full-piece box + 8 white negatives | speckle mask, coverage 5.5% — degenerate |
| facade-r1, box + positives | + positives on pediment, towers, aperture | coherent piece mask but flat fills excluded: grey kept 44.6%, white removed 82.5% — gates fail |
| grille, box + positives | + positives incl. fan, crossbar, pilasters, sill, aperture | piece kept, aperture Swiss-cheesed: grey kept 34.3%, white removed 93.3% — gates fail |
| grille-r1, corrected points | positive moved onto true grey (1300,1100) after orientation fix | near-identical mask — positive-in-grey does **not** force inclusion |

## Findings

1. **SAM 3.1 excludes large flat fills from "the object" even with interior
   positive points and a bounding box.** Figured regions (towers, fan,
   pilasters, hatching) segment well; flat card, flat grey apertures, and
   flat cream grounds are treated as background. White-cut-by-SAM therefore
   cannot meet the grey-preservation gate on these pieces.
2. **No composition or prompt-format fault found.** Key entitled on first
   try (existing `META_API_KEY` works for `sam-3.1`); wire format, box
   grammar, and mask decode all conform to the official docs (verified
   against the vendored parser, which pre-validates every record).
3. **Probe bug (mine, contained):** my throwaway BMP reader assumed
   bottom-up rows; sips writes top-down (`h<0`). Prompt points for the
   first attempts were vertically flipped. Re-measured correctly
   (`/tmp/sam_bounds2.py`): all negatives were still truly white and all
   positives truly interior, and the corrected grille retry reproduced the
   same mask — so the flip did not cause the failures. The committed
   checker (`check_house_art.py`) and committed PNG codec were already
   orientation-correct; only throwaway code was affected.
4. **Gate calibration note:** the grille aperture is tone ~160, outside the
   #808080±14 grey gate borrowed from the facade. Any future attempt needs
   per-image aperture-tone calibration.
5. Untried within this task: concept-phrase mode and any further call would
   exceed the authorized 4-call budget — both are operator decisions.

## Tooling delivered (tested, kept)

- `tools/generation/house-art-mask.mjs` (+ `.test.mjs`, 6 tests pass):
  box/concept requests, SSE handling, manifest + gates.
- `tools/generation/sam_mask_apply.py`: stdlib PNG codec, mask apply,
  grey/white/agreement stats (`--self-test` green).
- `tools/generation/vendor-meta-sam/` (official parser dist + LICENSE +
  `PROVENANCE.md`, hashes verified): prompt building, stream parsing,
  mask decode. No repo lockfile touched.
- Recommended follow-up (not implemented, needs operator go-ahead):
  deterministic border flood-cut on near-white, which the tone measurements
  above show is separable from card and aperture fills.

---

# Deterministic flood-cut (diagnostic, 2026-10-27) — GATES PASS

Status: **diagnostic only. Not registered, not accepted, not consumed by
the player.** Zero provider calls. After the SAM route failed its gates,
the white cuts were done deterministically: 8-connected border flood-fill
through pixels with min channel ≥ 250
(`tools/generation/sam_mask_apply.py --flood-cut`, red-first test in
`house-art-mask.test.mjs`, self-test green). Only canvas-connected white
goes transparent; enclosed fills, card, apertures, and all kept RGB are
untouched by construction.

## Verdicts

| File (sha16) | Transparent | Grey kept | White removed | RGB mismatches | Residual white |
|---|---|---|---|---|---|
| `house.facade-v2.flood-cut.png` `247fba44cfd088e1` | 301,931 (11.8%) | 1.0 (768,124 px) | 1.0 | 0 | 0 |
| `lobby-grille.flood-cut.png` `a5d5bb9498727928` | 1,493,648 (32.6%) | 1.0 | 1.0 | 0 | 0 |

Eyeball on magenta composites (`/tmp/*-magenta.png`, throwaway): facade
bottom strip gone, cut follows the card contour with no figured damage;
grille surround gone, grey window complete with no holes, fan/crossbar/
pilasters/sill/card intact. Manifests
(`house.facade-v2.flood-manifest.json`, `lobby-grille.flood-manifest.json`)
bind input/output hashes, method, threshold, and stats.

Caveat (recorded, not repaired): near-white card slivers at canvas edges
that touch the border are indistinguishable from background white and cut
with it; aged card and all figured content stop the flood and survive.
Anti-aliased fringe leaves no measurable residual (0 px ≥ 250 kept).

---

# Promotion to canonical names (operator-approved 2026-10-27)

The two flood-cut outputs replaced the canonical fixture files:

- `house.facade-v2.png` is now `247fba44cfd088e1` (was `4c4e5599…`);
  original preserved as `house.facade-v2.pre-flood.png` (hash matches the
  superseded generation manifest).
- `lobby-grille.png` is now `a5d5bb9498727928` (was `32f399b9…`);
  original preserved as `lobby-grille.pre-flood.png`.
- Dimensions unchanged (2048×1245, 2560×1792); `.flood-cut.png` working
  copies retained (flood manifests reference them); replacement recorded
  in both `.flood-manifest.json` files under `promotion`.

## Cross-lane proposal for Agent 4 (concrete, per INTERFACES.md)

The player lane pins the OLD bytes. Agent 3 does not edit those files;
recommended updates:

- `docs/implementation/player/house-art-manifest.md`: `house.facade`
  sha256 → `247fba44cfd088e1b6d8272ca272e335e38bbbea85566ba1554365147490b40c`;
  `lobby.grille` sha256 →
  `a5d5bb94987279281844be9346e0292681ab2593e5a78d879f34d4c8b1a578`.
- `apps/audience/src/dev-art.ts`: same two sha256 replacements.
- No action needed in `packages/player/tests/house-art.test.ts`: it
  hashes the files at runtime (self-consistent). Note: that suite is not
  runnable from this lane as-is (imports unbuilt `./src/index.js`;
  pre-existing, unrelated to this swap).
