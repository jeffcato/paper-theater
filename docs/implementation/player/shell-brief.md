# Shell geometry brief for Agent 3 (Agent 4)

Validated schematic shell under profile `paper-player-r186-v1`
(`validateSchematicShell()` green in `check:player -- shell`). Use these
exact numbers when registering artwork for the **bounded** shell (the
studio/bounded sink); imported flattened shell art remains diagnostic
reference only, not dimensionally compatible assets.

**Scope split with the immersive house.** The full-viewport audience
house is governed by [`immersive-art-brief.md`](immersive-art-brief.md)
(35 Muse Image surfaces generated from `src/multiplane.ts` +
`src/house.ts`), not by the numbers below. Where the two shells name the
same surface — lectern/plaque, title board, INTERMISSION sign, apron and
the facade/stage opening — the immersive brief's geometry supersedes
this brief for the audience, and this brief's geometry continues to
govern the bounded sink. Do not register immersive deliver sizes
against the bounded constants, or bounded artwork against the immersive
world rects. Both remain diagnostic until the Agent 1 profile revision
in [`integration-request.md`](integration-request.md) is frozen.

- Opening X=[−8,8], Y=[0,9] at Z=1.3; outer shell X=[−11,11], Y=[−3,11].
- Layer depths (fixed world Z): inner 1.3, columns 1.8, apron/deck front 2.4,
  plaque 2.8, signage 3.0.
- Deck X=[−8,8], Z=[−5.6,2.4], Y=0; apron X=[−8,8], Y=[−3,0], Z=2.4. The deck
  front Z equals the apron join Z; column bases must meet declared deck or
  outer plinth anchors with connecting returns closing exposed gaps.
- Plaque X=[−7,7], Y=[−2.8,−0.4], Z=2.8; safe text X=[−6.5,6.5],
  Y=[−2.55,−0.65]. Entire content area sits below the projected stage floor
  edge with ≥8 CSS px clearance at every camera/pointer extreme.
- Camera: perspective, vertical FOV 40°, up=[0,1,0], near=0.1, far=100; base
  target is the shell bounds' XY center at Z=0; base distance reserves 10% of
  frame dimensions (`baseDistance()`). Authored target offset X±0.5/Y±0.25,
  dolly [d,d+2]; pointer adds X±0.8/Y±0.4, resets on leave, zero under
  reduced motion. No history-dependent smoothing.
- INTERMISSION sign board visible bounds X=[−4,4], Y=[5,7], Z=3.0; ropes at
  X=±3, Y=11; lowering 1.2 s / raising 0.8 s with bounded tilt (≤3°) and
  bounce (≤0.15 units); hidden pose clears the maximum visible frustum top by
  ≥1 world unit across the envelope. The opening title card is a separate
  board: X=[−5,5], Y=[2.94,6.06] at Z=2.5 in the bounded code, replaced in
  the immersive house by `house.title-board` in the immersive brief. In
  the target direction both boards are blank bitmaps with all lettering
  typeset by the player in Noto Serif; the lettering currently baked into
  the code-painted boards (in a provisional serif) is interim only.
- Text: Noto Serif Regular (bundled locally at integration), 22 CSS px, 1.3
  line height, 100–200% viewer scaling, two-to-three-line chunks. If two
  lines cannot fit, extend the plaque downward in 0.25-unit steps and refit —
  never shrink text or split a highlight group.
- Rendering: sRGB output, linear working target, no tone mapping; cutout
  alpha 0.35, shell 0.10, curtain 0.05 with matching shadow-silhouette
  thresholds; `PCFShadowMap`, shadow maps ≤2048², linear filtering, mipmaps
  off; images ≤4096 px; graphics ≤256 MiB, master ≤256 MiB.

Any demonstrated defect in these constants requires one recorded profile
revision plus affected acceptance reruns — never silent per-module tweaks.
