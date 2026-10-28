#!/usr/bin/env node
// Immersive house art brief (Agent 4 -> Agent 3). Single source of truth
// for every bitmap surface of the full-viewport world: world geometry and
// depth, on-screen size at the acceptance viewport, delivered pixel size,
// alpha/tiling/9-slice registration, typeset-overlay regions and the
// Muse Image prompt (shared art profile + subject + format rules).
// Writes docs/implementation/player/immersive-art-brief.{json,md}.
// Makes no provider calls: generation and review run in the generation lane
// through the runtime broker, and registered art stays diagnostic until
// accepted.
import { writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(pkgDir, "..", "..");
const plan = await import(pathToFileURL(join(pkgDir, "dist", "src", "multiplane.js")).href);
const { LECTERN, BANNER_SEEK_TRACK, bannerLayout, INSPECT, INSPECT_BACK, TICKET_SLOTS, LOBBY_Z, PROSCENIUM_Z } = plan;

// Acceptance viewport: 1280x720 CSS at DPR 1.5 = 1920x1080 device pixels.
const VIEW = { width: 1920, height: 1080 };
const onScreen = (journey, x, y, z) => {
  try {
    const cam = plan.worldCamera({ journey, pointer: [0, 0], reducedMotion: true, ...VIEW, growthSteps: 0 });
    const r = plan.projectRect(cam, { x, y, z }, VIEW);
    return [Math.round(r.width), Math.round(r.height)];
  } catch {
    return null;
  }
};

const PROFILE = [
  "Illustration for a Victorian juvenile-drama toy theatre, drawn as a hand-coloured lithographed sheet of the 1840s-1880s (\"penny plain, twopence coloured\").",
  "Crisp black ink outlines of even weight; shading built from fine engraved hatching and cross-hatching, never airbrushed gradients;",
  "flat transparent watercolour washes laid slightly off-register from the line, as in hand stencilling;",
  "restrained palette of deep crimson and carmine, ochre and burnished gold, mahogany and walnut browns, bottle green, Prussian-blue accents and warm cream-buff paper, with soft black for the deepest shadow.",
  "Visible laid-paper fibre and faint age toning; matte, never glossy.",
  "Drawn as a flat, frontal, orthographic elevation of one die-cut card piece: no perspective, no vanishing point, no camera angle, no depth of field.",
  "Soft warm gaslight from the upper left, hatched shade on the right and lower sides (seamless tiles excepted: their format rule below requires even, directionless lighting and takes precedence over this sentence).",
  "Charming, theatrical, slightly naive proportions; one piece of a matching set for the same miniature theatre.",
  "Printed as illustration on thick cream card stock with a paper tooth: every isolated piece carries a clean, crisp die-cut contour edge.",
].join(" ");
const NEGATIVE = "No lettering, words, numbers, monograms, signatures or watermarks anywhere (all lettering is typeset later). No characters, scenes or objects from any particular story. No modern elements, no photographic texture, no 3D-rendered or glossy CG look.";
const FORMAT = {
  opaque: "Fill the whole canvas edge to edge with the piece; no border or margin around the image.",
  aperture: "Leave the opening described above as perfectly flat, even, untextured mid-grey (#808080) with nothing drawn inside it; it will be cut away along exact geometry.",
  cutout: "One isolated piece centred on a perfectly flat, even, untextured mid-grey (#808080) background with an empty margin of about 6% on every side; no cast shadow, glow or vignette on the background; a clean continuous die-cut paper edge around the piece with no white halo or fringe.",
  tile: "Seamless repeating tile: the left edge continues the right edge and the top edge continues the bottom edge exactly; even lighting with no directional falloff, no vignette and no focal point.",
  tileX: "Seamless horizontally: the left edge continues the right edge exactly; even lighting across the width.",
  tileY: "Seamless vertically: the top edge continues the bottom edge exactly; even lighting along the length.",
  nine: "Plain, evenly lit centre that can be stretched; all ornament kept within the corner and edge bands.",
};

// Surfaces. `fit` derives the pixel height from the world aspect so the
// registered texel aspect equals the card aspect exactly.
const lectern = { x: LECTERN.x, y: LECTERN.y, z: LECTERN.z };
const banner = Object.fromEntries(bannerLayout().map(({ id, rect }) => [id, rect]));
const ticket = TICKET_SLOTS[1];
const A = [
  // ---- Box office (lobby) ------------------------------------------------
  {
    id: "lobby.grille", group: "Box office", surface: "Brass ticket-window grille and name crossbar", layer: "+1.5 grille",
    rect: { x: [-8, 8], y: [-1, 10.2], z: LOBBY_Z + 1.5 }, view: 0, fit: 2560, alpha: "aperture",
    regions: { aperture: "window u 0.1625-0.8375, v 0.2813-0.8036 (rectangle)", text: "crossbar name-plate u 0.1625-0.8375, v 0.2500-0.2813: BOX OFFICE · ADMISSION BY TICKET" },
    prompt: "The brass ticket-window grille of a Victorian theatre box office, seen straight on and symmetrical. A broad frame of chased brass and gilded mahogany surrounds a tall rectangular window. Directly above the window runs a flat brass crossbar carrying a long, plain, blank engraved name-plate. Above the crossbar, a round arch head is filled with a radiating fan of slender brass bars around a small sunburst boss, set against a dark painted backing. Fluted brass pilasters with acanthus capitals flank the window, a moulded sill runs beneath it, and small rosettes and riveted straps decorate the frame. The large rectangle between the sill and the crossbar is the opening.",
  },
  {
    id: "lobby.booth-surround", group: "Box office", surface: "Box-office front panelling around the grille (wide and tall screens)", layer: "+1.5 grille",
    tile: [4, 4], rect: { x: [8, 12], y: [3, 7], z: LOBBY_Z + 1.5 }, view: 0, px: [512, 512], alpha: "tile",
    prompt: "Box-office front panelling: gilded mahogany stiles and rails framing recessed walnut panels edged with a thin brass bead, as one square repeating unit.",
  },
  {
    id: "lobby.counter", group: "Box office", surface: "Mahogany counter front (repeats along the counter)", layer: "+0.8 counter",
    tile: [8, 4.2], rect: { x: [-4, 4], y: [-2, 2.2], z: LOBBY_Z + 0.8 }, view: 0, fit: 1280, alpha: "tileX",
    prompt: "The front of a mahogany box-office counter seen straight on: a polished brass rail along the very top edge over a carved cornice, then two recessed fielded panels with ogee mouldings separated by a reeded pilaster, ending in a dark plinth at the bottom edge; one horizontal repeating unit.",
  },
  {
    id: "lobby.ticket", group: "Box office", surface: "Admission ticket blank (both tickets; hover lifts and tilts it)", layer: "+0.8 counter",
    rect: { x: ticket.x, y: ticket.y, z: ticket.z }, view: 0, fit: 1024, alpha: "opaque",
    regions: { mask: "rounded rectangle, corner radius 0.055 of height", punch: "player cuts a hole at u 0.144, v 0.18, radius 0.055 of height", text: "stub band u 0.03-0.19 (ADMIT ONE, vertical); header v 0.10-0.20; title v 0.22-0.52; subtitle v 0.56-0.70; status v 0.74-0.86 (all u 0.24-0.96)" },
    prompt: "A blank Victorian theatre admission ticket printed on stiff cream card, filling the frame: a double-ruled carmine border with small corner fleurons, a narrow stub at the left separated by a printed line of perforation dots, a small engraved scroll ornament at top centre, and empty horizontal panels for printed lines, all left blank. A slight letterpress impression and soft edge wear.",
  },
  {
    id: "shared.brass-plate", group: "Box office", surface: "Brass plate, 9-slice: UNDER REHEARSAL plates, retry notice, Return Ticket and Take Your Seat buttons", layer: "+0.8 counter",
    rect: { x: [0, 2.7], y: [0, 0.55], z: LOBBY_Z + 0.91 }, view: 0, px: [1024, 209], alpha: "opaque", nine: [96, 64, 96, 64],
    regions: { text: "centre field, typeset per use; hover/focus glow and pressed shade are drawn by the player" },
    prompt: "A plain engraved brass name-plate, wide and shallow: bevelled edges, a fine ruled inner border, and a small round rivet near each corner; the face is blank, softly burnished and slightly worn.",
  },
  {
    id: "lobby.inspect-card", group: "Box office", surface: "Inspection programme card (synopsis, opening lines, readiness)", layer: "+1.2 card",
    rect: INSPECT, view: 0, fit: 1664, alpha: "opaque",
    regions: { text: "title v 0.06-0.18; author/chapter v 0.19-0.26; synopsis v 0.27-0.40; opening lines v 0.43-0.64; readiness v 0.66-0.74", wells: `buttons u ${((INSPECT_BACK.x[0] - INSPECT.x[0]) / 7.8).toFixed(3)}-0.468 and 0.532-0.955, v 0.798-0.954` },
    prompt: "A blank Victorian theatre programme card of thick cream board, wider than tall. An engraved ornamental border of interlaced ribbon and laurel in carmine and gold runs around it, with a small pair of comedy and tragedy masks at top centre. Inside lies a wide, quiet blank field with the faintest ruled guide lines, and near the bottom two shallow empty rectangular recesses side by side.",
  },
  {
    id: "lobby.doors", group: "Box office", surface: "Foyer double doors, split into two leaves at the centre line; each swings open on its outer hinge", layer: "-1 doors",
    rect: { x: [-2.5, 2.5], y: [-0.2, 7.2], z: LOBBY_Z - 1 }, view: 0, fit: 1024, alpha: "opaque",
    regions: { split: "left leaf u 0-0.5, right leaf u 0.5-1" },
    prompt: "A closed pair of tall Victorian theatre auditorium doors seen straight on, filling the frame: mahogany leaves with three fielded panels each, the upper panels glazed with etched glass showing a simple star-and-garland pattern, brass push-plates and kick-plates, and a slender brass handle on each leaf beside the meeting stiles.",
  },
  ...["left", "right"].map((side) => ({
    id: `lobby.drape-${side}`, group: "Box office", surface: `Velvet portière drape, ${side} of the doors`, layer: "-1 doors",
    rect: { x: [side === "left" ? -4 : 2.6, side === "left" ? -2.6 : 4], y: [-0.4, 8], z: LOBBY_Z - 0.95 }, view: 0, fit: 352, alpha: "cutout",
    prompt: `A single heavy crimson velvet portière drape hanging from a hidden pole, deep vertical folds, gathered and tied back at about two-thirds of its height by a thick gold cord with a tassel, the tie-back sweeping the drape to the ${side}, a gold bullion fringe at the hem.`,
  })),
  ...[["a", "a lyre within a laurel wreath", [-6.5, -4.3]], ["b", "crossed comedy and tragedy masks bound with ribbons", [4.3, 6.5]]].map(([v, vignette, px]) => ({
    id: `lobby.playbill-${v}`, group: "Box office", surface: `Framed playbill ${v === "a" ? "left" : "right"} of the doors (credits typeset over it)`, layer: "-1 doors",
    rect: { x: px, y: [2.3, 5.6], z: LOBBY_Z - 0.96 }, view: 0, fit: 640, alpha: "opaque",
    regions: { text: "headline v 0.07-0.15; title v 0.18-0.45; author v 0.62-0.68; source v 0.69-0.74; tagline v 0.84-0.90 (all u 0.12-0.88)", vignette: "v 0.47-0.60" },
    prompt: `A framed Victorian theatre playbill in a narrow gilt frame, filling the frame: cream poster paper with an ornate printed border, a blank headline band at the top, broad blank bands for printed lines, and a small engraved vignette of ${vignette} in the lower middle; slightly aged, flat and pasted.`,
  })),
  {
    id: "lobby.wallpaper", group: "Box office", surface: "Lobby damask wallpaper", layer: "-3 backdrop",
    tile: [3, 3], rect: { x: [5, 8], y: [3, 6], z: LOBBY_Z - 3 }, view: 0, px: [512, 512], alpha: "tile",
    prompt: "Victorian flocked damask wallpaper: a symmetrical pattern of stylised acanthus and pomegranate medallions in raised deep claret flock on a darker oxblood ground, with a fine gold hairline trellis.",
  },
  {
    id: "lobby.archway", group: "Box office", surface: "Gilded STALLS archway behind the doors", layer: "-3 backdrop",
    rect: { x: [-3.4, 3.4], y: [-3.2, 7.6], z: LOBBY_Z - 2.98 }, view: 0, fit: 768, alpha: "aperture",
    regions: { aperture: "jambs u 0.1618-0.8382 from v 0.3009 to the bottom edge, round head centred u 0.5, v 0.3009 rising to v 0.0880", text: "keystone tablet u 0.40-0.60, v 0.014-0.069: STALLS" },
    prompt: "A gilded plaster round-headed doorway surround set in a wall, seen straight on and symmetrical: fluted pilasters with gilt capitals, an egg-and-dart archivolt, a keystone shaped as a small blank tablet at the crown, and plain plinth blocks at the base. The tall arched space inside the surround, down to the floor, is the opening.",
  },
  {
    id: "lobby.sconce", group: "Box office", surface: "Gas wall sconce (glow halo stays procedural)", layer: "-3 backdrop",
    rect: { x: [4.6, 6.6], y: [4.4, 6.4], z: LOBBY_Z - 2.95 }, view: 0, px: [384, 384], alpha: "cutout",
    prompt: "A Victorian brass gas wall sconce: an ornate scrolled bracket on an oval backplate holding a frosted tulip-shaped glass globe, lit softly from within so the pale warm glow stays inside the glass.",
  },
  {
    id: "lobby.palm", group: "Box office", surface: "Indoor palm in a vase, flanking the ticket grille (the right card is mirrored)", layer: "+1.5 grille, in front of the frame",
    px: [864, 1152], alpha: "cutout", optional: true,
    prompt: "An indoor parlor palm in an urn, seen straight on as one die-cut card: many long arching fronds of bottle green, each leaflet outlined in black ink with fine engraved hatching, rising from a cream ceramic urn with a gilt rim, foot and a band of gilt acanthus.",
  },
  // ---- Aisle and auditorium ---------------------------------------------
  ...[["a", "the first, second, fourth, sixth and seventh seats"], ["b", "the second, third, fifth and eighth seats"]].map(([v, seats]) => ({
    id: `aisle.row-${v}`, group: "Aisle and auditorium", surface: `Stalls seat row ${v.toUpperCase()} (alternates down the aisle)`, layer: "aisle z 54 to 30",
    rect: { x: [1.6, 15.5], y: [-3.2, 2.2], z: 46 }, view: 0.3, fit: 1536, alpha: "cutout",
    prompt: `A row of eight upholstered theatre stalls seats seen from directly behind as a flat frieze: crimson velvet seat backs with gilt piping and small brass caps. Seated theatre-goers are shown from behind in ${seats}, as dark paper silhouettes with a thin warm rim light: gentlemen in top hats and bowlers, ladies in bonnets with ostrich feathers or with upswept hair.`,
  })),
  {
    id: "floor.carpet", group: "Aisle and auditorium", surface: "Facing carpet band along the bottom of each stall row", layer: "row frame, 0.12 behind the seats",
    tile: [3.2, 3.2], px: [512, 512], alpha: "tile",
    prompt: "Theatre carpet seen straight on as a flat paper band: a small repeating diaper of gold quatrefoils and lozenges with fine dark outlines on a deep crimson ground.",
  },
  {
    id: "floor.runner", group: "Aisle and auditorium", surface: "Facing centre-aisle runner just in front of each row's carpet", layer: "row frame, just in front of the carpet band",
    tile: [2.6, 5.2], px: [256, 512], alpha: "tileY",
    prompt: "A Prussian-blue aisle runner seen straight on as a flat paper band, with a narrow gold Greek-key border along both long edges, repeating along its length.",
  },
  {
    id: "house.wall", group: "Aisle and auditorium", surface: "Facing wall covering for the stall-row jambs and the panels around the proscenium", layer: "row frame jambs and 0 surround",
    tile: [3, 3], rect: { x: [13, 16], y: [8, 11], z: PROSCENIUM_Z - 0.05 }, view: 1, px: [512, 512], alpha: "tile",
    prompt: "Auditorium wall covering: deep crimson silk damask with a gilt hairline lattice and small gold fleurons at the crossings, darker and richer than a lobby paper.",
  },
  {
    id: "house.ceiling", group: "Aisle and auditorium", surface: "Facing ceiling band across the top of each stall row", layer: "row frame ceiling band",
    tile: [4, 4], px: [512, 512], alpha: "tile", optional: true,
    prompt: "A painted theatre ceiling border seen straight on as a flat paper band: a gilded rosette in a pale cream and soft blue coffered square framed by small garland mouldings.",
  },
  // ---- Proscenium and pit ----------------------------------------------
  {
    id: "house.facade", group: "Proscenium and pit", surface: "Proscenium facade: pediment cartouche, side boxes with spectators (style anchor)", layer: "0 proscenium",
    rect: { x: [-13, 13], y: [-3.2, 12.6], z: PROSCENIUM_Z }, view: 1, fit: 2048, alpha: "aperture",
    regions: { aperture: "stage opening u 0.1846-0.8154 from v 0.2247 to the bottom edge", text: "cartouche u 0.377-0.623, v 0.035-0.199: PAPER THEATER" },
    prompt: "The proscenium front of a Victorian toy theatre, perfectly frontal and symmetrical. A wide pediment band runs across the top, hung with swags of gilt garland, with a large blank oval cartouche in a gold frame on a bottle-green ground at its centre. Beneath it is the tall rectangular stage opening, edged by a gilt bead moulding with a short crimson scalloped valance along its top edge. On each side stands a tall cream-and-gold tower holding two tiers of private boxes. Each box is a round-headed opening with a crimson swagged pelmet and a gilt balustrade front, holding two or three tiny paper spectators in profile turned toward the stage: ladies with fans and feathers, gentlemen in evening dress. Fluted gilt pilasters edge each tower. The stage opening, from below the valance down to the bottom edge, is the opening.",
  },
  {
    id: "house.curtain", group: "Proscenium and pit", surface: "Velvet curtain panel (both panels; folds gather as it opens)", layer: "-0.3 stage front",
    rect: { x: [-8, 0], y: [0, 9], z: PROSCENIUM_Z - 0.3 }, view: 1, fit: 768, alpha: "tileX",
    prompt: "A crimson velvet stage curtain panel hanging straight down in deep, regular vertical folds, with engraved shading in the troughs and highlights on the crowns, a gilt braid band near the top, and a heavy gold bullion fringe along the bottom hem.",
  },
  {
    id: "house.title-board", group: "Proscenium and pit", surface: "Title board in front of the closed curtains", layer: "0 proscenium",
    rect: { x: [-5, 5], y: [2.94, 6.06], z: PROSCENIUM_Z }, view: 1, fit: 1024, alpha: "opaque",
    regions: { text: "heading v 0.25-0.58, subtitle v 0.62-0.82 (u 0.06-0.94)" },
    prompt: "A narrow title board as used in front of a toy-theatre curtain: a dark ebonised panel with a fine double gilt rule inset near the edges and small gilt corner fleurons; the face is plain and blank.",
  },
  {
    id: "house.sign-board", group: "Proscenium and pit", surface: "INTERMISSION board (lettering typeset; pause-driven drop)", layer: "+0.5 sign",
    rect: { x: [-4, 4], y: [5, 7], z: 3.0 }, view: 1, fit: 1024, alpha: "opaque",
    regions: { text: "lettering u 0.10-0.90, v 0.25-0.75", ropes: "eye-plates at u 0.125 and 0.875 on the top edge" },
    prompt: "A hanging painted signboard for a theatre interval: a long rectangular board of dark brown wood in a raised gilt moulded frame, with two small brass eye-plates on the top edge for ropes; the face is plain and blank.",
  },
  {
    id: "house.rope", group: "Proscenium and pit", surface: "Sign rope", layer: "+0.5 sign",
    tile: [0.09, 0.72], px: [64, 512], alpha: "tileY",
    prompt: "A short vertical length of twisted hemp rope with a tight, regular lay, filling the frame edge to edge across its width, repeating along its length.",
  },
  {
    id: "house.footlights", group: "Proscenium and pit", surface: "Hooded footlights along the stage edge (glow stays procedural)", layer: "+0.1 footlights",
    rect: { x: [-8.2, 8.2], y: [-0.62, 0], z: PROSCENIUM_Z + 0.1 }, view: 1, fit: 2048, alpha: "cutout",
    prompt: "A row of nine hooded brass footlight shells along the front edge of a toy-theatre stage, seen straight on: each a small half-round brass hood with a scalloped rim, evenly spaced on a narrow dark wooden batten.",
  },
  {
    id: "house.apron", group: "Proscenium and pit", surface: "Stage front below the footlights", layer: "+0.05 apron",
    rect: { x: [-8.2, 8.2], y: [-3.2, -0.62], z: PROSCENIUM_Z + 0.05 }, view: 1, fit: 1536, alpha: "opaque",
    prompt: "The front face of a toy-theatre stage below the footlights, as a wide horizontal strip: warm walnut boarding with a gilt cyma moulding along the top edge, three recessed panels with thin gold lining and small painted rosettes, and a dark skirting along the bottom edge.",
  },
  {
    id: "house.pit-rail", group: "Proscenium and pit", surface: "Orchestra-pit balustrade either side of the lectern", layer: "+0.8 orchestra pit",
    tile: [2.27, 0.85], rect: { x: [7, 9.27], y: [-3.6, -2.75], z: LECTERN.z }, view: 1, fit: 512, alpha: "tileX",
    regions: { key: "mid-grey between and around the balusters is keyed out by the player before tiling; the left edge must continue the right edge after keying" },
    prompt: "A low brass orchestra-pit balustrade seen straight on: a polished top rail and bottom rail joined by slender turned brass balusters at even spacing, on a perfectly flat, even, untextured mid-grey (#808080) ground showing between the balusters; that grey will be keyed out before the tile repeats.",
  },
  {
    id: "house.lectern", group: "Proscenium and pit", surface: "Read-along lectern face (the DOM plaque is projected onto its inner field)", layer: "+0.8 orchestra pit",
    rect: lectern, view: 1, fit: 1536, alpha: "opaque", nine: [96, 48, 96, 48],
    regions: { text: "plaque inset u 0.035-0.965, v 0.085-0.915: keep calm and dark (relative luminance <= 0.03) so cream #f1e6c8 text reaches >= 7:1 contrast; stretches vertically for plaque growth" },
    prompt: "The face of a reading-lectern plaque standing in a theatre orchestra pit: a wide panel in a brass frame with a gilt border of fine beading, small rosettes at the four corners and a modest scrolled crest at top centre. The frame surrounds a large plain inner field of very dark bottle-green slate with only the faintest texture, kept calm and dark.",
  },
  ...[["left", "a lady in an ostrich-feather bonnet, a gentleman in a tall silk top hat, a child, and a lady with an upswept chignon and comb"], ["right", "a gentleman in a bowler, a lady in a wide feathered hat, a gentleman in a top hat, and an elderly lady in a lace cap"]].map(([side, people]) => ({
    id: `house.patrons-${side}`, group: "Proscenium and pit", surface: `Front-row patrons, lower ${side} corner`, layer: "+2.5 house foreground",
    rect: { x: side === "left" ? [-18.5, -9.7] : [9.7, 18.5], y: [-5.8, -2.6], z: PROSCENIUM_Z + 2.5 }, view: 1, fit: 1024, alpha: "cutout",
    prompt: `The heads and shoulders of a front row of Victorian theatre-goers seen from directly behind, as dark ink silhouettes with a thin warm rim light from the stage, from left to right: ${people}.`,
  })),
  // ---- Banner controls: lithographed medallions hung on the proscenium banner
  ...[
    ["play", "banner.play", "Play medallion", "the standard play symbol: one large equilateral triangle pointing right, cream with a gilt edge, and a small empty gilt bezel on the upper right of the border", "bottle-green"],
    ["restart", "banner.restart", "Restart medallion", "the standard restart symbol: one gilt circular arrow open at the top, its arrowhead pointing clockwise", "deep crimson"],
    ["readalong", "banner.readalong", "Read-along medallion", "the standard open-book symbol: a book lying open with two blank cream pages and a gilt centre fold, and a small empty gilt bezel on the upper right of the border", "Prussian blue"],
    ["return", "banner.return", "Box office medallion", "the standard admission-ticket symbol: one cream theatre ticket with scalloped ends, a perforated stub, and blank panels", "ochre-gold"],
  ].map(([key, id, surface, emblem, ground]) => ({
    id, group: "Banner controls", surface: `${surface} on the proscenium banner (the player typesets the label on the ribbon and lights the card)`, layer: "+0.07 banner",
    rect: banner[key], view: 1, px: [512, 512], alpha: "cutout", optional: true,
    regions: { text: "ribbon u 0.30-0.70, v 0.745-0.845 (label typeset by the player)" },
    prompt: `A round die-cut medallion in the manner of a Victorian toy-theatre sticker, seen straight on: a cream paper rim, a ring of gilt beading and a thin gilt line around a ${ground} ground, showing ${emblem}. Below the emblem a blank cream scroll ribbon with swallowtail ends spans the lower third for a typeset label.`,
  })),
  {
    id: "banner.seek", group: "Banner controls", surface: "Seek gauge on the proscenium banner (the player draws the pointer and typesets the label)", layer: "+0.07 banner",
    rect: banner.seek, view: 1, px: [1280, 240], alpha: "cutout", optional: true,
    regions: {
      track: `seek track u ${BANNER_SEEK_TRACK[0]}-${BANNER_SEEK_TRACK[1]}, v 0.33-0.57 (picks map linearly across it; the pointer rides it)`,
      text: "ribbon u 0.36-0.64, v 0.62-0.86 (label typeset by the player)",
    },
    prompt: "A long low die-cut gauge plaque, about five times as wide as it is tall, in the manner of a Victorian toy-theatre sticker, seen straight on: a cream paper rim, a ring of gilt beading around a bottle-green ground, a long recessed cream ruler strip engraved with fine black graduation ticks and longer ticks at the ends and middle, a gilt acanthus scroll at each end of the strip, and below the strip a blank cream scroll ribbon with swallowtail ends for a typeset label. No pointer.",
  },
];

const failures = [];
const assets = A.map((a) => {
  const worldW = a.rect && !a.tile ? a.rect.x[1] - a.rect.x[0] : a.tile?.[0];
  const worldH = a.rect && !a.tile ? a.rect.y[1] - a.rect.y[0] : a.tile?.[1];
  const px = a.px ?? [a.fit, Math.round(a.fit / (worldW / worldH))];
  // Every asset with a world size (card or tile, fit- or px-sized) must keep
  // texel aspect within 1% of world aspect. Sprites without a world size
  // are exempt; today no asset lacks a world card.
  if (worldW && worldH && Math.abs(px[0] / px[1] - worldW / worldH) / (worldW / worldH) > 0.01) failures.push(`${a.id}: aspect`);
  if (Math.max(...px) > 4096) failures.push(`${a.id}: over 4096 px`);
  const screen = a.rect && a.view !== undefined ? onScreen(a.view, a.rect.x, a.rect.y, a.rect.z) : null;
  const format = a.alpha === "opaque" && a.nine ? `${FORMAT.opaque} ${FORMAT.nine}` : a.nine ? FORMAT.nine : FORMAT[a.alpha];
  return {
    id: a.id,
    group: a.group,
    surface: a.surface,
    layer: a.layer,
    world_size: worldW ? [Number(worldW.toFixed(3)), Number(worldH.toFixed(3))] : null,
    tile_world: a.tile ?? null,
    onscreen_px_1920x1080: screen,
    deliver_px: px,
    alpha: a.alpha,
    nine_slice_px: a.nine ?? null,
    regions: a.regions ?? null,
    optional: a.optional ?? false,
    bytes_rgba8: px[0] * px[1] * 4,
    subject_prompt: a.prompt,
    prompt: `${PROFILE} ${a.prompt} ${format} ${NEGATIVE}`,
  };
});
if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}
const total = assets.reduce((s, a) => s + a.bytes_rgba8, 0);
const required = assets.filter((a) => !a.optional).reduce((s, a) => s + a.bytes_rgba8, 0);
const mib = (b) => (b / 1048576).toFixed(1);
const brief = {
  brief: "immersive-house-art-v1",
  for: "generation lane (Muse Image through the runtime broker); consumed by the player as registered house art",
  geometry: "packages/player/src/multiplane.ts + src/house.ts (world units, +Z toward the audience)",
  acceptance_viewport: "1280x720 CSS at DPR 1.5 (1920x1080 device px); onscreen sizes at the box office (lobby) or seat view",
  art_profile: PROFILE,
  negative: NEGATIVE,
  formats: FORMAT,
  budget: { decoded_rgba8_mib: Number(mib(total)), required_mib: Number(mib(required)), image_max_px: 4096 },
  assets,
};
const outDir = join(root, "docs", "implementation", "player");
await writeFile(join(outDir, "immersive-art-brief.json"), JSON.stringify(brief, null, 2) + "\n");

const ALPHA = { opaque: "opaque", aperture: "geometric cut", cutout: "segmented cutout", tile: "seamless tile", tileX: "seamless across", tileY: "seamless down" };
const lines = [
  "# Immersive house art brief: Muse Image surfaces (Agent 4 → Agent 3)",
  "",
  "Generated by `node packages/player/scripts/immersive-art-brief.mjs` from the",
  "world plan; the JSON twin (`immersive-art-brief.json`) carries the full composed",
  "prompt for every asset. This brief makes no provider calls and authorizes none.",
  "Generation, review and registration belong to the generation lane through the",
  "runtime broker once live authorization is established for those calls (it is",
  "not established by this brief; provider dispatch remains gated as recorded in",
  "`docs/agent-handoffs/INTERFACES.md`). Registered art stays diagnostic until",
  "it is accepted.",
  "",
  "## How each prompt is composed",
  "",
  "Each Muse Image request sends, in order: the **common art profile**, the asset's",
  "**subject prompt**, the **format rule** for its registration type, and the",
  "**negative**. Attach no external researched references; this is generic",
  "scenery under the style-only rule. One internal exception: generate",
  "`house.facade` first, and once it has passed review and acceptance, attach",
  "that accepted facade — a product-generated member of the same set, not an",
  "external reference — as the house style reference for the rest of the set,",
  "so the whole theatre reads as one illustrated sheet.",
  "",
  "**Common art profile.** " + PROFILE,
  "",
  "**Negative (every request).** " + NEGATIVE,
  "",
  "| Registration | Format rule |",
  "| --- | --- |",
  ...Object.entries(FORMAT).map(([k, v]) => `| ${ALPHA[k] ?? "9-slice"} | ${v} |`),
  "",
  "## Rules the player depends on",
  "",
  "- **Scope.** This brief governs the immersive audience house only. The",
  "  bounded studio shell keeps the geometry in `shell-brief.md`; where the",
  "  two shells name the same surface (lectern/plaque, title board, sign,",
  "  apron, facade opening) this brief's immersive geometry supersedes for",
  "  the audience. Deliver sizes are final for generation, but the immersive",
  "  framing still needs the Agent 1 profile revision recorded in",
  "  `integration-request.md` before production acceptance; until then all",
  "  art registered against this brief is diagnostic.",
  "- **No baked lettering.** Every word is typeset by the player in the bundled",
  "  Noto Serif over blank fields, so titles, verbatim opening lines, readiness and",
  "  labels stay exact and swappable. That covers BOX OFFICE, STALLS, ADMIT ONE,",
  "  UNDER REHEARSAL, PAPER THEATER, INTERMISSION and the banner control labels (the ribbons are blank). Registration",
  "  should reject any image where OCR finds characters.",
  "- **Story-neutral.** The house is shared by every book: no characters,",
  "  props or motifs from either chapter. Patrons are generic period theatre-goers",
  "  seen from behind or in tiny profile.",
  "- **Flat cards.** Each image is a frontal elevation; depth comes only from",
  "  the card's Z layer. Light falls from the upper left on every non-tile card; tiles carry no",
  "  directional light.",
  "- **Alpha.** Generated images are opaque. Apertures are cut by the player",
  "  along the exact geometry below; the opening area is painted flat mid-grey.",
  "  Cutouts are segmented from a flat mid-grey ground and reviewed as real masks,",
  "  never as diagnostic mattes. One keyed tile: `house.pit-rail` paints the",
  "  gaps between its balusters flat mid-grey, which the player keys out before",
  "  the tile repeats (its left edge must still continue its right). Sizes keep",
  "  the texel aspect equal to the card aspect (≤1% error, ≤4096 px). The five",
  "  banner controls hang in world space on the proscenium's green banner; they",
  "  keep the cream paper rim of their die-cut edge, so the player keys only the",
  "  flat mid-grey ground (not the rim) and the card reads as a printed sticker.",
  "- **Kept procedural, not bitmaps.**",
  "  - Paper shadows under tickets, gas and footlight glow halos (so the house",
  "    lights can dim), and hover/focus rim glow.",
  "  - The ticket punch hole, the curtain gather (texture repeat), the seek pointer,",
  "    the pilot jewels, the dim wash on inactive controls and all lettering.",
  "  - The narrative stage itself, which is authored package art and not house",
  "    dressing.",
  "",
  `## Surfaces (${assets.length} images, ${mib(total)} MiB decoded RGBA8; ${mib(required)} MiB without optional)`,
  "",
  "On-screen size is in device pixels at the acceptance viewport, measured from",
  "the box office (lobby) for lobby assets and from the seat for house assets. Aisle",
  "rows are measured at journey 0.3, with the camera about 15 units from the row at z 46.",
  "",
  "| # | Asset | Surface | Layer | World size | On-screen px | Deliver px | Registration |",
  "| --- | --- | --- | --- | --- | --- | --- | --- |",
  ...assets.map((a, i) => `| ${i + 1} | \`${a.id}\` | ${a.surface}${a.optional ? " *(optional)*" : ""} | ${a.layer} | ${a.world_size ? a.world_size.join(" × ") : "—"}${a.tile_world ? " (tile)" : ""} | ${a.onscreen_px_1920x1080 ? a.onscreen_px_1920x1080.join(" × ") : "—"} | ${a.deliver_px.join(" × ")} | ${ALPHA[a.alpha]}${a.nine_slice_px ? `, 9-slice ${a.nine_slice_px.join("/")}` : ""} |`),
  "",
  "## Prompts",
  "",
];
let group = "";
for (const [i, a] of assets.entries()) {
  if (a.group !== group) {
    group = a.group;
    lines.push(`### ${group}`, "");
  }
  lines.push(`**${i + 1}. \`${a.id}\`** · ${a.deliver_px.join("×")} · ${ALPHA[a.alpha]}`, "");
  lines.push(`> ${a.subject_prompt}`, "");
  if (a.regions) lines.push(...Object.entries(a.regions).map(([k, v]) => `- ${k}: ${v}`), "");
}
lines.push(
  "## Budget and review",
  "",
  `The full set decodes to ${mib(total)} MiB of RGBA8 without mipmaps. The current`,
  "code-painted set is about 43 MiB. At the acceptance viewport the drawing buffer",
  "and targets account for about 93 MiB, which leaves the 256 MiB graphics budget",
  "with headroom.",
  "",
  "Larger viewports at DPR 1.5 already strain the budget because of the drawing",
  "buffer, independent of art. That case is tracked in `checks.md`.",
  "",
  "Suggested deterministic registration checks, before automated creative review:",
  "",
  "- exact pixel size, and sRGB with 8 bits per channel",
  "- OCR finds no characters",
  "- seam continuity on tile edges",
  "- aperture regions flat mid-grey within tolerance",
  "- lectern inset luminance and contrast",
  "- mean luminance within the set's range so the cards read as one lighting",
  "",
  "## Player follow-up (not yet implemented)",
  "",
  "The renderer still paints these surfaces in code. Consuming the registered art",
  "needs a house-art manifest: id → URL, SHA-256, size, mask, 9-slice and region",
  "geometry. The loader binds it with the same fail-closed rule as figures, and the",
  "code-painted set remains the explicit diagnostic fallback. The record shape",
  "will be proposed to Agent 1 if it crosses lanes.",
  "",
);
await writeFile(join(outDir, "immersive-art-brief.md"), lines.join("\n"));
console.log(`immersive art brief: ${assets.length} assets, ${mib(total)} MiB (${mib(required)} MiB required)`);
