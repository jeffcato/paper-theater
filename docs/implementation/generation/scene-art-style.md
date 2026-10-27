# Scene art style: illustrated die-cut paper (generation lane)

All narrative image assets — canonical character sprites, scene pictures,
scenery — are **illustrations that read as die-cut paper**, in the same
family as the theater house. The look: hand-drawn black ink outlines of
even weight, fine engraved hatching for shade, flat transparent colour
washes on a warm paper ground, slightly naive toy-theatre proportions,
matte, never glossy. This document is the scene-side companion to the
house's [immersive art brief](../player/immersive-art-brief.md); where
both apply, the construction rules below are identical.

## Die-cut construction (every scene asset)

- Each asset is one flat card: frontal, orthographic, no perspective, no
  camera angle, no depth of field. Depth in the scene comes from card
  layering, never from painted perspective.
- Clean cut contour edges on every isolated piece. Cutouts and figures
  are segmented from a flat mid-grey ground with a small empty margin,
  reviewed as real masks, never as diagnostic mattes.
- Paper reads as paper: visible fibre and faint age toning are welcome;
  photographic texture, airbrushed gradients, lens blur, vignette, cast
  shadows on the ground, and glow are not (lighting effects stay
  procedural in the player).
- Story-bearing assets keep their depicted facts exact — recognizable
  subjects, locations and actions from the source. Style never simplifies
  away a required fact: no symbolic substitutes, no dropped figures, no
  invented scenes.
- No lettering, words, numbers or signatures are ever baked into a
  bitmap. All text is typeset by the player over blank fields.
- Flat warm gaslight from the upper left on non-tile pieces, matching
  the house. Seamless scenery tiles carry no directional light.
- Figures keep native aspect, declared pivots, support contacts and
  canonical dimensions per the registration checklist; the illustrated
  redraw must preserve the measured geometry it replaces.

## Prompt composition (future Visuals/registration work)

Each request sends, in order: the common illustrated die-cut profile,
the asset's evidence-scoped subject prompt (source spans and required
facts named, never a whole-chapter bible leaked into one asset), the
registration format rule for its type, and the negative. Attach only
researched subject references that the subject requires; generic
scenery uses style alone. One reviewed and accepted sprite anchors the
rest of its book's set, mirroring how the accepted `house.facade`
anchors the house.

Negative (every request): no lettering of any kind; no photographic
texture, photorealism, 3D or glossy CG; no perspective or camera
effects; no characters, scenes or motifs outside the asset's own
evidence scope.

## Registration and review

Scene assets register through the same deterministic gates as house
art: exact pixel size, sRGB 8-bit, OCR finds no characters, seam
continuity on tiles, aperture regions flat mid-grey, mask review for
cutouts, texel aspect within 1% of the card aspect, luminance and
contrast where text is typeset over the piece. Registered art stays
diagnostic until accepted; acceptance binds the exact bytes, prompt and
recipe, and the player consumes only accepted art.
