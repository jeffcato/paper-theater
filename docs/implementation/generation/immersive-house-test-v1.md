# Immersive house art — Wave 1 test (diagnostic)

Status: **diagnostic only. Not registered, not accepted, not consumed by
the player.** Wave 2 has not run; it awaits operator review of this wave.

## Dispatch

The runtime broker has no live image dispatch (its transport records no
product dispatch authorization). The operator approved a one-batch
diagnostic exception on 2026-10-26: direct Muse Image calls from
[`tools/generation/house-art-wave1.mjs`](../../../tools/generation/house-art-wave1.mjs)
using the project `.env` key (never printed or serialized), search/web/
shell tools disabled, no reference images attached. The brief's
hand-coloured lithograph profile was used unchanged; the operator's
earlier painterly reference images were deliberately ignored.

Model `muse-image-1.0` via `POST https://api.meta.ai/v1/images/generations`
(model list verified with the project key before generation). Cap was
2 calls; 2 were used. Pricing was not relied on.

## Candidates

Both are `house.facade`, returned at 2000×1216 and stretch-resized by
`sips` to the exact deliver size 2048×1245. Raw returns and hashes are in
`fixtures/generation/immersive-house-test-v1/` (`manifest.json`,
`manifest-b.json`, `checks-a.json`, `checks-b.json`).

- **A — brief prompt verbatim** (`house.facade.png`). Flat frontal
  hand-coloured lithograph: black ink, engraved hatching, crimson/gold/
  bottle-green washes on aged cream paper. Blank green cartouche, two
  profile spectators per box, grey stage opening under a crimson
  scalloped valance. No lettering (OCR empty).
- **B — brief prompt + GEOMETRY LOCK suffix** (`house.facade-b.png`).
  Same lithograph family, fuller canvas coverage, three spectators per
  box, cream cartouche on a green pediment. Grey is lighter and warmer
  than `#808080`; OCR reports only ornament noise (`| i`), no lettering.

## Deterministic checks (`tools/generation/check_house_art.py`)

| Check | A | B |
| --- | --- | --- |
| Exact size 2048×1245 | pass | pass |
| OCR finds characters | pass (none) | pass (ornament noise only) |
| Aperture grey near `#808080` (sampled centre) | 78% of samples, mean RGB 122/117/112 | 0% (grey too light/warm) |
| Non-grey in aperture top band v 0.225–0.37 | 89% (valance) | 98% (valance) |

## Visual review (this session, both images eyeballed)

Style: both hit the brief — flat die-cut card, ink outline, hatching,
transparent washes, restrained palette, naive toy-theatre proportions,
story-neutral profile spectators. Neither copied perspective, scenes or
lettering from anywhere. This is the correct lithographic direction.

Geometry: **neither candidate is registrable as-is.** Both inset the
facade inside a paper margin, place the grey opening narrower than the
brief's aperture (u 0.1846–0.8154, v 0.2247–bottom), and hang the valance
inside that aperture's top band. Cutting either image along the brief's
exact coordinates would slice towers and valance and leave paper margin
in the wrong place. Prompt wording alone (including the GEOMETRY LOCK
suffix) did not make the model honour exact u/v bounds.

## Recommendation for the Wave 2 decision

1. Prefer **A** as the style anchor: its grey value, cartouche and muted
   engraving are closest to the brief. B is the better spectator count
   but the worse registration base.
2. Do not rely on prompted aperture coordinates for any asset. Before
   Wave 2, choose one registration strategy and record it in the brief:
   measure each return's actual grey bounds and warp/crop to the card
   geometry during registration, or generate apertures oversized and
   composite the exact opening programmatically. The current brief text
   assumes the first is unnecessary; Wave 1 proves otherwise.
3. Keep the 2-call repair discipline per asset. Facade-class geometric
   cuts are the highest-risk type; tiles and opaque cards should be
   lower-risk when Wave 2 is approved.

Outputs: `fixtures/generation/immersive-house-test-v1/` (2 candidates,
2 raw returns, 2 manifests, 2 check files). No other lane's files were
changed except the new tools in `tools/generation/`.
