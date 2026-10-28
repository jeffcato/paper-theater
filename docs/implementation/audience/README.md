# Audience app — full-viewport 2.5D theater (Agent 4)

The audience page is one continuous Three.js canvas filling the viewport,
drawn by the shared `@paper-theater/player` world (`createTheaterWorld`).
Every visible element is a flat, code-painted card at a fixed depth.
The code-painted cards are the interim diagnostic rendering: the target
house art is the 35 registered blank bitmaps specified in
[`../player/immersive-art-brief.md`](../player/immersive-art-brief.md)
(same layers and geometry, lettering typeset by the player, glow and
motion kept procedural), consumed through a house-art manifest that is
not yet implemented. No registered house art is consumed yet.

| World | Layers (depth relative to the group's reference plane) |
| --- | --- |
| Box office (z≈60) | brass grille and BOX OFFICE crossbar +1.5 · mahogany counter, ticket stubs, paper shadows, UNDER REHEARSAL plates, inspection card +0.8…+1.2 · foyer doors, velvet drapes, credit playbills −1 · damask wall, STALLS archway, gas sconces −3 |
| Aisle | staggered seat rows with paper patrons (z 54→30), carpet and runner, damask side walls |
| House (facade z=2.5) | foreground patrons +2.5 · brass rail +1.8 · pit railing and read-along lectern +0.8 · footlights +0.1 · facade with pediment cartouche and side boxes 0 · frozen curtains −0.3 · authored stage −0.8 and deeper |

The camera sits slightly house-left of centre and looks back at the frame
centre, so every flat card is seen at a slight angle, like standing paper.
Pointer movement translates the camera slightly. An off-axis window holds the
frame centre while nearer and farther cards slide past it. The camera is
fixed when reduced motion is set. Picking a ticket opens its card:
title, author/chapter, a book-level synopsis, the chapter's verbatim opening
words from the frozen source, and readiness. **Take Your Seat** punches the
ticket. The camera then dollies through the ticket window and the opening
foyer doors, down the centre aisle, and steps aside into an orchestra seat. The brass rail's
**Box Office** lever reverses the walk.

## Honest state

No accepted package exists, so both tickets carry an UNDER REHEARSAL plate.
Taking a seat shows the closed curtain with the title card reading "Under
rehearsal — not yet published". Play opens that curtain across the 1.5 s
curtain travel and reveals the empty stage: the profile deck, wing cards
on the groove planes, and the backcloth. It does not start a performance
or load a fixture. Restart and seek stay dimmed. The audience path never
requests a diagnostic fixture; the harness records every request. Mounting
a performance in the audience is pending a catalog-entry → release-reference
contract from the runtime lane.

## Accessibility

All visible UI is in the canvas. A visually hidden DOM mirror (`<main
class="a11y">`) carries a live status region, ticket and card actions, the
seat toolbar and the credits. Focusing a mirror button lights its counterpart
in the world. Focus moves into the inspection card and the seat toolbar when
the view changes. If WebGL is unavailable, a plain HTML program list replaces
the world.

## Evidence (`node apps/audience/scripts/chrome-audience.mjs`)

Headless Chrome (SwiftShader), DPR 1, empty accepted catalog, at 960×540,
1280×720 and 1920×1080: `audience-{booth,inspect,aisle,seat,seat-deeplink}-{W}x{H}.png`
and `audience.json`. The harness checks:

- exactly one canvas covering the viewport, with a matching drawing buffer
- no visible DOM element over it
- real mouse picks for ticket → card → seat → aisle → seat → Box Office lever → lobby
- keyboard Tab/Enter through the same flow
- a deep link to a seat starting seated
- no page errors and no diagnostic or fixture requests

Software-renderer timings are not target-device measurements. Player-level
results (frame intervals, graphics memory, teardown) are in
[`../player/checks.md`](../player/checks.md).
