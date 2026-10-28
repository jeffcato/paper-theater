// Full-viewport 2.5D multiplane world plan (immersive audience framing):
// discrete Z planes relative to the proscenium facade, contain-fit framing
// for any viewport aspect, a resting side viewpoint that shows the flat
// cards at a slight angle, the box-office-to-seat journey, off-axis pointer
// parallax that holds the frame centre, and the projected lectern/rail
// layout. Pure and DOM-free; the browser sink applies the same camera.
// The narrative stage interior keeps the frozen profile geometry.
import { CAMERA, CURTAIN_SAMPLES, DECK, OPENING, RENDER_DEFAULTS, SAMPLE_RATE } from "./profile.js";

export interface MultiplaneLayer {
  id: string;
  /** Depth relative to the layer group's reference plane (+Z toward camera). */
  z: number;
}

/** World Z of the proscenium facade; frozen curtains (2.2) are its stage front. */
export const PROSCENIUM_Z = 2.5;

export const MULTIPLANE_LAYERS: readonly MultiplaneLayer[] = [
  { id: "house-foreground", z: 2.5 },
  { id: "orchestra-pit", z: 1.2 },
  { id: "proscenium", z: 0 },
  { id: "stage-front", z: -0.3 },
  { id: "groove-1", z: -0.8 },
  { id: "groove-2", z: -1.5 },
  { id: "scenic-flats", z: -2.5 },
  { id: "backcloth", z: -4 },
];

/** Full curtain travel: 72,000 samples at 48 kHz. */
export const CURTAIN_TRAVEL_MS = (CURTAIN_SAMPLES / SAMPLE_RATE) * 1000;

export interface SceneDeck {
  id: "deck";
  kind: "deck";
  x: readonly [number, number];
  y: number;
  z: readonly [number, number];
}

export interface SceneFlat {
  id: "groove-1" | "groove-2" | "scenic-flats" | "backcloth";
  kind: "wing" | "cloth";
  x: readonly [number, number];
  y: readonly [number, number];
  z: number;
}

export type SceneCard = SceneDeck | SceneFlat;

const stagePlaneZ = (id: string): number => {
  const layer = MULTIPLANE_LAYERS.find((item) => item.id === id);
  if (!layer) throw new Error(`stage plane: ${id}`);
  return PROSCENIUM_Z + layer.z;
};

/**
 * Empty narrative stage seen through the open curtain. The deck is the
 * frozen profile deck. Each groove is a pair of wing cards at the opening
 * edges, so the centre stays clear for scenery. The backcloth fills the
 * opening, with a small overlap behind the proscenium cut.
 */
export function sceneLayout(): SceneCard[] {
  const wing = (id: SceneFlat["id"], side: -1 | 1): SceneFlat => {
    const inner = 6.2;
    const x = side < 0 ? [OPENING.x[0], -inner] as const : [inner, OPENING.x[1]] as const;
    return { id, kind: "wing", x, y: [OPENING.y[0], OPENING.y[1]], z: stagePlaneZ(id) };
  };
  return [
    { id: "deck", kind: "deck", x: [DECK.x[0], DECK.x[1]], y: DECK.y, z: [DECK.z[0], DECK.z[1]] },
    wing("groove-1", -1),
    wing("groove-1", 1),
    wing("groove-2", -1),
    wing("groove-2", 1),
    wing("scenic-flats", -1),
    wing("scenic-flats", 1),
    {
      id: "backcloth",
      kind: "cloth",
      x: [OPENING.x[0] - 0.3, OPENING.x[1] + 0.3],
      y: [OPENING.y[0] - 0.15, OPENING.y[1] + 0.15],
      z: stagePlaneZ("backcloth"),
    },
  ];
}

/** Curtain ratio while a rehearsal open or close is in progress. */
export function rehearsalCurtain(
  elapsedMs: number,
  from: number,
  to: number,
): { ratio: number; titleVisible: boolean; moving: boolean } {
  if (!(elapsedMs >= 0) || !Number.isFinite(elapsedMs)) throw new Error(`curtain elapsed: ${elapsedMs}`);
  if (!(from >= 0 && from <= 1 && to >= 0 && to <= 1)) throw new Error(`curtain span: ${from}..${to}`);
  const span = Math.abs(to - from);
  const duration = CURTAIN_TRAVEL_MS * span;
  const t = duration === 0 ? 1 : Math.min(1, elapsedMs / duration);
  const ratio = from + (to - from) * t;
  return { ratio, titleVisible: ratio < 0.02, moving: t < 1 && span > 0 };
}

export const LOBBY_LAYERS: readonly MultiplaneLayer[] = [
  { id: "grille", z: 1.5 },
  { id: "counter", z: 0.8 },
  { id: "page", z: -1.45 },
];

export const AISLE_HALF_WIDTH = 1.4;
/**
 * Immersive lens. The schematic profile stays at 40°. 64° is a moderate
 * audience lens: wide enough to hold the stage from the air behind three
 * stall rows, without the stretch of a fisheye.
 */
export const FOV_Y_DEGREES = 64;
/**
 * Three stall flats. The first clears the orchestra rail; each step is
 * the air between two paper rows. The box-office page sits just behind the
 * seat and turns away to reveal this picture.
 */
export const STALL_ROW_Z = [6.6, 8.1, 9.6] as const;
/**
 * Each stall row is one facing frame. Jambs run from the carpet line to the
 * facade top. The ceiling band fills the opening between them. The floor
 * band is a step farther from the camera so the seat cutouts draw in front.
 */
export const FRAME_SPAN_X = [-30, 22.6] as const;
export const FRAME_JAMB_INNER = 16;
export const FRAME_FLOOR_Y = -3.2;
export const FRAME_FLOOR_TOP = -0.6;
export const FRAME_CEILING_Y = 9.2;
export const FRAME_TOP_Y = 12.6;
export const ROW_FLOOR_OFFSET = 0.12;
/** House-right seat card, a little nearer than its row so the two halves overlap in depth. */
export const SEAT_STAGGER_Z = 0.45;

export interface StallFrame {
  z: number;
  leftJamb: Rect3;
  rightJamb: Rect3;
  ceiling: Rect3;
  floor: Rect3;
  runner: Rect3;
  seats: readonly [Rect3, Rect3];
}

const stallFrame = (z: number, index: number): StallFrame => {
  const floorZ = z - ROW_FLOOR_OFFSET;
  const sideY = [FRAME_FLOOR_Y, FRAME_TOP_Y] as const;
  // 0 is the row nearest the camera. Each row closer to the stage draws a
  // smaller opening, so its border shows inside the frame in front of it.
  const fromCamera = STALL_ROW_Z.length - 1 - index;
  const inner = FRAME_JAMB_INNER - fromCamera * 2.5;
  const seatOuter = inner - 0.5;
  const ceilingBottom = 11.2 - fromCamera * 1.5;
  const floorTop = -2.4 + fromCamera * 0.6;
  const floorY = [FRAME_FLOOR_Y, floorTop] as const;
  return {
    z,
    leftJamb: { x: [FRAME_SPAN_X[0], -inner], y: sideY, z },
    rightJamb: { x: [inner, FRAME_SPAN_X[1]], y: sideY, z },
    ceiling: { x: [-inner, inner], y: [ceilingBottom, FRAME_TOP_Y], z },
    floor: { x: [FRAME_SPAN_X[0], FRAME_SPAN_X[1]], y: floorY, z: floorZ },
    runner: { x: [-1.3, 1.3], y: floorY, z: floorZ + 0.04 },
    seats: [
      { x: [-seatOuter, -1.6], y: [FRAME_FLOOR_Y, 2.2], z },
      { x: [1.6, seatOuter], y: [FRAME_FLOOR_Y, 2.2], z: z + SEAT_STAGGER_Z },
    ],
  };
};
export const STALL_FRAMES: readonly StallFrame[] = STALL_ROW_Z.map(stallFrame);
/**
 * Front-row patron flats. They stand on the proscenium side of the stalls,
 * in the lower corners, so the short lens does not crop them into the carpet.
 */
export const PATRON_Z = PROSCENIUM_Z + 2.5;
export const PATRONS: readonly Rect3[] = [
  { x: [-18.5, -9.7], y: [-4.5, -1.3], z: PATRON_Z },
  { x: [9.7, 18.5], y: [-4.5, -1.3], z: PATRON_Z },
];
const TAN = Math.tan((FOV_Y_DEGREES * Math.PI) / 360);
const WIDE = 16 / 9;
/** Plaque growth step (world units), as in the bounded profile. */
export const GROWTH_STEP = 0.25;
/**
 * Resting viewpoint: the camera sits house-left of the centre line and turns
 * back toward the frame centre (rotation about +Y; negative looks toward +X),
 * so every flat card is seen at a slight angle, like standing paper.
 */
export const SIDE_VIEW_DEGREES = -7;
const SIDE = (SIDE_VIEW_DEGREES * Math.PI) / 180;

/** Contain-fit frame on a reference plane: always fully visible. */
export interface Frame {
  planeZ: number;
  top: number;
  bottom: number;
  halfWidth: number;
}

const frame = (planeZ: number, top: number, bottom: number): Frame => ({
  planeZ, top, bottom, halfWidth: ((top - bottom) / 2) * WIDE,
});

/** Lectern and rail authoring frame. The seated camera fits PROSCENIUM_FACADE. */
export const SEAT_FRAME = frame(PROSCENIUM_Z, 12.3, -5.65);
/**
 * The proscenium facade the seated camera contain-fits. Wider than it is
 * tall, so a narrow window fits the width and a wide one fits the height.
 */
export const PROSCENIUM_FACADE: Frame = {
  planeZ: PROSCENIUM_Z,
  top: 12.6,
  bottom: -3.2,
  halfWidth: 13,
};

interface Fit {
  aspect: number;
  centerY: number;
  cameraZ: number;
}

function fit(f: Frame, width: number, height: number, yaw = 0): Fit {
  if (!(width > 0 && height > 0)) throw new Error(`viewport bounds: ${width}x${height}`);
  const aspect = width / height;
  // Seen at an angle the near half is closer and larger: its corners bound
  // the view distance; cameraZ is that distance's depth component.
  const near = f.halfWidth * Math.abs(Math.sin(yaw));
  const distance = near + Math.max((f.top - f.bottom) / 2 / TAN, (f.halfWidth * Math.cos(yaw)) / (TAN * aspect));
  return { aspect, centerY: (f.top + f.bottom) / 2, cameraZ: f.planeZ + distance * Math.cos(yaw) };
}

/** Proscenium fit, extended downward when the read-along plaque grows. */
export function seatFrame(growthSteps: number): Frame {
  if (!Number.isSafeInteger(growthSteps) || growthSteps < 0) throw new Error(`growth steps: ${growthSteps}`);
  return {
    planeZ: PROSCENIUM_Z,
    top: PROSCENIUM_FACADE.top,
    bottom: PROSCENIUM_FACADE.bottom - growthSteps * GROWTH_STEP * 1.1,
    halfWidth: PROSCENIUM_FACADE.halfWidth,
  };
}

/** Air between the rearmost stall row and the resting seat camera. */
const SEAT_REAR_CLEARANCE = 1.2;
/** Foyer wall behind the furthest ordinary seat: a 16:9 window with a grown plaque. */
const FOYER_BEHIND_SEAT = 1.15;

/** Contain-fit of the proscenium facade, before the foyer clamp. */
export function seatContainFitDepth(width: number, height: number, growthSteps = 0): number {
  return fit(seatFrame(growthSteps), width, height, SIDE).cameraZ;
}

function preferredSeatDepth(width: number, height: number, growthSteps = 0): number {
  const fitted = seatContainFitDepth(width, height, growthSteps);
  const last = STALL_ROW_Z[STALL_ROW_Z.length - 1]!;
  return Math.max(fitted, last + SEAT_REAR_CLEARANCE);
}

/**
 * Design-space box office, the size the grille art was drawn for. The placed
 * sheet is this picture scaled down so it fills the view from just behind
 * the full-view seat. A quarter-turn of the right edge still clears the stalls.
 */
const LOBBY_OFFSET = 1.45;
const PAGE_FACE_OFFSET = 1.5;
const pageLayout = (foyerZ: number) => {
  const lobbyZ = foyerZ + LOBBY_OFFSET;
  const pageFaceZ = lobbyZ + PAGE_FACE_OFFSET;
  const lobbyFrame = frame(lobbyZ + 0.8, 7.7, 0.3);
  const wide = fit(lobbyFrame, 21, 9, SIDE);
  const tall = fit(lobbyFrame, 2, 3, SIDE);
  const halfW = (wide.cameraZ - pageFaceZ) * TAN * (21 / 9) + 0.8;
  const halfH = (tall.cameraZ - pageFaceZ) * TAN + 1;
  const tipZ = pageFaceZ - Math.hypot(halfW * 2, pageFaceZ - foyerZ);
  return { foyerZ, lobbyZ, pageFaceZ, lobbyFrame, halfW, halfH, centerY: tall.centerY, tipZ };
};
const anchoredPage = pageLayout(preferredSeatDepth(16, 9, 12) + FOYER_BEHIND_SEAT);
const lastStallZ = STALL_ROW_Z[STALL_ROW_Z.length - 1]!;
const designPage = pageLayout(anchoredPage.foyerZ + Math.max(0, lastStallZ + 0.4 - anchoredPage.tipZ));

/** Resting full-view seat: the 16:9 proscenium contain-fit. */
const FULL_VIEW_SEAT_Z = seatContainFitDepth(16, 9, 0);
/**
 * Booth camera behind that seat. The page turn is the trip; the dolly is
 * only this deep.
 */
export const BOOTH_FLIGHT = 1.65;
/** Placed sheet, just on the audience side of the resting seat. */
const PAGE_BEHIND_SEAT = 0.42;

const boothScale = (() => {
  const distance = BOOTH_FLIGHT - PAGE_BEHIND_SEAT;
  const visH = (2 * distance * TAN) / Math.cos(Math.abs(SIDE));
  // The delivered preview is a tall window. Fit the grille there; a 16:9
  // full view then shows the same sheet with a little more side margin.
  const visW = visH * (1497 / 1360);
  const grilleW = 16;
  const grilleH = 11.2;
  return Math.min((visW * 0.96) / grilleW, (visH * 0.96) / grilleH);
})();

/** Uniform scale from the design sheet onto the placed sheet. */
export const BOOTH_SCALE = boothScale;

export const BOOTH_DESIGN = {
  hinge: -designPage.halfW,
  edge: designPage.halfW,
  y0: designPage.centerY - designPage.halfH,
  y1: designPage.centerY + designPage.halfH,
  face: designPage.pageFaceZ,
  lobby: designPage.lobbyZ,
  foyer: designPage.foyerZ,
  centerX: 0,
  centerY: designPage.centerY,
} as const;

const placedCenterY = (PROSCENIUM_FACADE.top + PROSCENIUM_FACADE.bottom) / 2;
const placedFace = FULL_VIEW_SEAT_Z + PAGE_BEHIND_SEAT;
const placedW = designPage.halfW * 2 * boothScale;
const placedH = designPage.halfH * 2 * boothScale;

export const FOYER_WALL_Z = placedFace + (designPage.foyerZ - designPage.pageFaceZ) * boothScale;
export const LOBBY_Z = placedFace + (designPage.lobbyZ - designPage.pageFaceZ) * boothScale;

/**
 * The box-office sheet, shrunk onto the full-view seat. Its left edge is
 * the spine. The whole grille stays in the tall preview, and a quarter-turn
 * of the right edge still clears the stall cards.
 */
export const PAGE_FACE_Z = placedFace;
export const PAGE_HINGE_X = -placedW / 2;
export const PAGE_EDGE_X = placedW / 2;
export const PAGE_Y = [placedCenterY - placedH / 2, placedCenterY + placedH / 2] as const;

/** Box office view on the sheet. */
export const LOBBY_FRAME: Frame = {
  planeZ: PAGE_FACE_Z,
  top: PAGE_Y[1],
  bottom: PAGE_Y[0],
  halfWidth: placedW / 2,
};

/** Map a design-sheet point onto the placed sheet. */
export function boothMap(x: number, y: number, z: number): [number, number, number] {
  return [
    (PAGE_HINGE_X + PAGE_EDGE_X) / 2 + (x - BOOTH_DESIGN.centerX) * BOOTH_SCALE,
    (PAGE_Y[0] + PAGE_Y[1]) / 2 + (y - BOOTH_DESIGN.centerY) * BOOTH_SCALE,
    PAGE_FACE_Z + (z - BOOTH_DESIGN.face) * BOOTH_SCALE,
  ];
}

export interface Rect3 {
  x: readonly [number, number];
  y: readonly [number, number];
  z: number;
}

const mapRect = (x: readonly [number, number], y: readonly [number, number], z: number): Rect3 => {
  const a = boothMap(x[0], y[0], z);
  const b = boothMap(x[1], y[1], z);
  return { x: [a[0], b[0]], y: [a[1], b[1]], z: a[2] };
};
export const TICKET_SLOTS: readonly Rect3[] = [
  mapRect([-3.55, -0.45], [2.1, 3.7], BOOTH_DESIGN.lobby + 0.86),
  mapRect([0.45, 3.55], [2.1, 3.7], BOOTH_DESIGN.lobby + 0.86),
];
/** Inspection card raised from the counter toward the grille. */
export const INSPECT: Rect3 = mapRect([-3.9, 3.9], [2.2, 7.0], BOOTH_DESIGN.lobby + 1.2);
export const INSPECT_BACK: Rect3 = mapRect([-3.55, -0.25], [2.42, 3.17], BOOTH_DESIGN.lobby + 1.21);
export const INSPECT_SEAT: Rect3 = mapRect([0.25, 3.55], [2.42, 3.17], BOOTH_DESIGN.lobby + 1.21);

export interface InspectPlacement {
  card: { x: readonly [number, number]; y: readonly [number, number] };
  back: { x: readonly [number, number]; y: readonly [number, number] };
  seat: { x: readonly [number, number]; y: readonly [number, number] };
}

/**
 * Contain-fit the inspection card into the grille opening and center it.
 * The authored card is taller than the delivered window, so using it raw
 * hides the title behind the fanlight. Buttons keep their wells on the card.
 */
export function placeInspectCard(
  opening: { x0: number; y0: number; x1: number; y1: number },
  pad = 0.035,
): InspectPlacement {
  const bw = INSPECT.x[1] - INSPECT.x[0];
  const bh = INSPECT.y[1] - INSPECT.y[0];
  const ow = (opening.x1 - opening.x0) * (1 - pad * 2);
  const oh = (opening.y1 - opening.y0) * (1 - pad * 2);
  if (!(bw > 0 && bh > 0 && ow > 0 && oh > 0)) throw new Error("nonpositive inspect fit");
  const scale = Math.min(ow / bw, oh / bh);
  const cx = (opening.x0 + opening.x1) / 2;
  const cy = (opening.y0 + opening.y1) / 2;
  const contentCx = (INSPECT.x[0] + INSPECT.x[1]) / 2;
  const contentCy = (INSPECT.y[0] + INSPECT.y[1]) / 2;
  const span = (rect: Rect3) => ({
    x: [cx + (rect.x[0] - contentCx) * scale, cx + (rect.x[1] - contentCx) * scale] as const,
    y: [cy + (rect.y[0] - contentCy) * scale, cy + (rect.y[1] - contentCy) * scale] as const,
  });
  return { card: span(INSPECT), back: span(INSPECT_BACK), seat: span(INSPECT_SEAT) };
}

// Lectern and rail are authored as facade-plane design rects, converted
// once to world rects on their own planes at the base 16:9 seat fit.
const BASE = fit(SEAT_FRAME, 16, 9);
const onPlane = (x: readonly [number, number], y: readonly [number, number], z: number): Rect3 => {
  const s = (BASE.cameraZ - z) / (BASE.cameraZ - PROSCENIUM_Z);
  const wy = (v: number) => BASE.centerY + (v - BASE.centerY) * s;
  return { x: [x[0] * s, x[1] * s], y: [wy(y[0]), wy(y[1])], z };
};
/**
 * Read-along plaque face on the orchestra-pit lectern (+0.8).
 * The reading shift lifts that face onto the apron: above the front-row
 * patrons, and still below the stage floor, so the plaque can be read.
 */
const LECTERN_PIT_Y = [-3.6, -1.5] as const;
const LECTERN_READ_SHIFT = 1.55;
export const LECTERN: Rect3 = onPlane([-7, 7], LECTERN_PIT_Y, PROSCENIUM_Z + 0.8);
const LECTERN_READING: Rect3 = onPlane(
  [-7, 7],
  [LECTERN_PIT_Y[0] + LECTERN_READ_SHIFT, LECTERN_PIT_Y[1] + LECTERN_READ_SHIFT],
  PROSCENIUM_Z + 0.8,
);
/** World-Y distance from the pit rest to the reading pose. */
export const LECTERN_RAISE = LECTERN_READING.y[1] - LECTERN.y[1];
/**
 * How far a stall row slides off the aisle so its inner edge clears the
 * lectern. Left rows take the negative, right rows the positive.
 */
export const STALL_ASIDE = LECTERN.x[1] - (-STALL_FRAMES[0]!.seats[0].x[1]) + 0.8;
/** Time for a full rise or a full return. A partial trip takes the same rate. */
export const LECTERN_TRAVEL_MS = 450;
/**
 * Carpet on the facade plane, from the proscenium sill down through the
 * bottom of the seated view. It stays an XY card: the floor pattern
 * continues where the facade ends.
 */
export const PIT_FLOOR: Rect3 = {
  x: [-26, 36],
  y: [-14.5, -3.2],
  z: PROSCENIUM_Z + 0.04,
};
/** Playback controls in tab order. They hang on the proscenium banner. */
export const RAIL_CONTROLS = [
  { id: "play" },
  { id: "restart" },
  { id: "seek" },
  { id: "readalong" },
  { id: "return" },
] as const;
export type RailControlId = (typeof RAIL_CONTROLS)[number]["id"];

/** Design-sheet grille the ticket-window art was drawn on. */
export const GRILLE_CARD = { x: [-8, 8] as const, y: [-1, 10.2] as const };
/** Bottom of the brass window, on the design sheet. */
export const GRILLE_SILL_Y = 1.2;

/**
 * Green pediment field of the facade art, on the facade plane. The seated
 * camera contain-fits the whole facade, so this band is on screen in every
 * window; the controls hang on it as part of the proscenium.
 */
export const BANNER_FIELD = { x: [-11.8, 11.8] as const, y: [9.3, 11.76] as const };
/** Control cards stand this far in front of the facade, parallel to it. */
export const BANNER_Z = PROSCENIUM_Z + 0.07;
/**
 * Width of the cartouche line "PAPER THEATER". The facade sheet is 2048 px
 * across 26 world units and draws that line at 58 px in the bundled Noto
 * Serif face; measured advance is 462.1439208984375 px.
 */
export const BANNER_TITLE_WIDTH = (462.1439208984375 * 26) / 2048;
/** Round controls, half the previous 2.2 medallion. */
export const BANNER_MEDALLION = 1.1;
/**
 * Bottom of the control row. The facade's green field runs down to the gilt
 * knot at about y 8.9; the cards rest on that last strip of banner, just
 * above the crimson valance.
 */
export const BANNER_ROW_BOTTOM = 9.32;
/** Engraved scale of the gauge art, as fractions of the card width. */
export const BANNER_SEEK_TRACK = [0.14, 0.86] as const;

const BANNER_GAP = 0.24;
/** Narrowest the cards shrink to, as a fraction of their full size. */
const BANNER_SCALE_MIN = 0.5;
/** Screen edge kept clear of the cards, as a fraction of the viewport width. */
const BANNER_EDGE_CLEAR = 0.012;

export interface BannerControl {
  id: RailControlId;
  rect: Rect3;
}

/**
 * The five playback controls as one row of world cards on the bottom of the
 * proscenium banner, between the pillar tops and just above the valance.
 * Play and restart stand left of the seek gauge; read-along and box office
 * stand right of it. The gauge is exactly as wide as the cartouche line
 * "PAPER THEATER" and is centred on it. Round buttons are square cards at
 * half the previous medallion. `scale` is 1 whenever the row is on screen;
 * a narrow window shrinks the cards toward the centre (see `bannerScale`).
 */
export function bannerLayout(scale = 1): readonly BannerControl[] {
  const medallion = BANNER_MEDALLION * scale;
  const gap = BANNER_GAP * scale;
  const gauge = BANNER_TITLE_WIDTH * scale;
  const widths = [medallion, medallion, gauge, medallion, medallion];
  const ids: readonly RailControlId[] = ["play", "restart", "seek", "readalong", "return"];
  const row = widths.reduce((sum, width) => sum + width, 0) + gap * (widths.length - 1);
  let left = -row / 2;
  const y = [BANNER_ROW_BOTTOM, BANNER_ROW_BOTTOM + medallion] as const;
  return ids.map((id, index) => {
    const width = widths[index]!;
    const card: BannerControl = { id, rect: { x: [left, left + width], y, z: BANNER_Z } };
    left += width + gap;
    return card;
  });
}

/**
 * Card scale for a window: 1 when the banner's outer cards are on screen at
 * the resting seat, smaller only as far as a tall window's crop needs. A pure
 * function of the viewport, so the cards do not resize under pointer parallax.
 */
export function bannerScale(viewport: Viewport): number {
  const cam = worldCamera({ journey: 1, pointer: [0, 0], reducedMotion: true, width: viewport.width, height: viewport.height, growthSteps: 0 });
  const clear = viewport.width * BANNER_EDGE_CLEAR;
  const fits = (scale: number): boolean =>
    bannerLayout(scale).every(({ rect }) => {
      const css = projectRect(cam, rect, viewport, "outer");
      return css.left >= clear && css.left + css.width <= viewport.width - clear;
    });
  if (fits(1)) return 1;
  let [lo, hi] = [BANNER_SCALE_MIN, 1];
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    if (fits(mid)) lo = mid;
    else hi = mid;
  }
  return lo;
}

export interface BannerSign {
  id: RailControlId;
  rect: CssRect;
}

/** CSS rects of the banner controls for a camera. They move with the proscenium. */
export function bannerScreen(cam: WorldCamera, viewport: Viewport): readonly BannerSign[] {
  return bannerLayout(bannerScale(viewport)).map(({ id, rect }) => ({ id, rect: projectRect(cam, rect, viewport, "outer") }));
}

/**
 * Ease the lectern between two lifts in 0..1. `elapsedMs` is the time since
 * the trip started. A zero span is already there.
 */
export function lecternTravel(elapsedMs: number, from: number, to: number): { lift: number; moving: boolean } {
  if (!(elapsedMs >= 0) || !Number.isFinite(elapsedMs)) throw new Error(`lectern elapsed: ${elapsedMs}`);
  if (!(from >= 0 && from <= 1 && to >= 0 && to <= 1)) throw new Error(`lectern span: ${from}..${to}`);
  const span = Math.abs(to - from);
  const duration = LECTERN_TRAVEL_MS * span;
  const t = duration === 0 ? 1 : Math.min(1, elapsedMs / duration);
  const eased = t * t * (3 - 2 * t);
  return { lift: from + (to - from) * eased, moving: t < 1 && span > 0 };
}

/**
 * Stage column the read-along view contain-fits: the banner crown, just
 * outside the proscenium opening, down through the raised lectern.
 */
const READ_ALONG_HALF_WIDTH = 8.4;
const READ_ALONG_MARGIN = 0.02;

/** Closest straight-on camera that keeps the stage column inside the viewport. */
export function readAlongZoom(width: number, height: number, growthSteps = 0): { y: number; z: number } {
  if (!(width > 0 && height > 0)) throw new Error(`read-along zoom: ${width}x${height}`);
  if (!Number.isSafeInteger(growthSteps) || growthSteps < 0) throw new Error(`growth steps: ${growthSteps}`);
  const aspect = width / height;
  const limit = 1 - READ_ALONG_MARGIN * 2;
  const top = PROSCENIUM_FACADE.top;
  const lecternBottom = LECTERN.y[0] + LECTERN_RAISE - growthSteps * GROWTH_STEP;
  const lecternTop = LECTERN.y[1] + LECTERN_RAISE;
  const controls = bannerLayout();
  const play = controls[0]!.rect;
  const box = controls[controls.length - 1]!.rect;
  const points: [number, number, number][] = [
    [-READ_ALONG_HALF_WIDTH, top, PROSCENIUM_Z],
    [READ_ALONG_HALF_WIDTH, top, PROSCENIUM_Z],
    [play.x[0], play.y[1], play.z],
    [play.x[0], play.y[0], play.z],
    [box.x[1], box.y[1], box.z],
    [box.x[1], box.y[0], box.z],
    [LECTERN.x[0], lecternBottom, LECTERN.z],
    [LECTERN.x[1], lecternBottom, LECTERN.z],
    [LECTERN.x[0], lecternTop, LECTERN.z],
    [LECTERN.x[1], lecternTop, LECTERN.z],
  ];
  const fits = (cz: number): number | null => {
    let lo = -Infinity;
    let hi = Infinity;
    for (const [, y, z] of points) {
      const reach = (cz - z) * TAN * limit;
      if (!(reach > 0)) return null;
      lo = Math.max(lo, y - reach);
      hi = Math.min(hi, y + reach);
    }
    return lo <= hi ? (lo + hi) / 2 : null;
  };
  let zMin = Math.max(...points.map((point) => point[2])) + 0.35;
  for (const [x, , z] of points) zMin = Math.max(zMin, z + Math.abs(x) / (TAN * aspect * limit));
  let z = zMin;
  if (fits(z) === null) {
    let lo = zMin;
    let hi = zMin + 80;
    if (fits(hi) === null) throw new Error("read-along zoom has no fit");
    for (let i = 0; i < 48; i++) {
      const mid = (lo + hi) / 2;
      if (fits(mid) !== null) hi = mid;
      else lo = mid;
    }
    z = hi;
  }
  return { y: fits(z)!, z };
}

/** Signed slide for one stall row. `side` is -1 house-left and 1 house-right. */
export function stallShift(side: -1 | 1, lift: number): number {
  if (side !== -1 && side !== 1) throw new Error(`stall side: ${side}`);
  if (!(lift >= 0 && lift <= 1)) throw new Error(`stall lift: ${lift}`);
  if (lift === 0) return 0;
  return side * STALL_ASIDE * lift;
}

/** Pit pose at lift 0, reading pose at lift 1. Growth still extends downward. */
export function lecternWorld(growthSteps: number, lift = 0): Rect3 {
  if (!(lift >= 0 && lift <= 1)) throw new Error(`lectern lift: ${lift}`);
  const rise = lift * LECTERN_RAISE;
  return { ...LECTERN, y: [LECTERN.y[0] - growthSteps * GROWTH_STEP + rise, LECTERN.y[1] + rise] };
}
export interface WorldCamera {
  position: [number, number, number];
  fovYDegrees: number;
  aspect: number;
  /** Rotation about +Y (radians) turning the side view toward the frame centre. */
  yaw: number;
  /** Off-axis window shift in NDC that holds the frame centre under parallax. */
  shift: [number, number];
  focusZ: number;
}

export interface WorldView {
  /** 0 = box office, 1 = orchestra seat; between is the aisle dolly. */
  journey: number;
  /** Pointer in NDC (+Y up); clamped to [-1, 1]. */
  pointer: [number, number];
  reducedMotion: boolean;
  width: number;
  height: number;
  growthSteps: number;
  /** 0 is clear, 1 is a full black veil over the house. */
  veil?: number;
  /** 0 keeps the side view. 1 pans the camera onto the stage centre line. */
  readAlong?: number;
}

const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
const smooth = (t: number) => t * t * (3 - 2 * t);

/**
 * Resting seat depth. The picture is the 16:9 full-view contain-fit of the
 * proscenium. Taller windows keep that zoom. Growth eases back only until
 * the booth sheet.
 */
export function seatCameraDepth(width: number, height: number, growthSteps = 0): number {
  const fullView = seatContainFitDepth(16, 9, growthSteps);
  const fitted = preferredSeatDepth(width, height, growthSteps);
  const opening = fit({
    planeZ: PROSCENIUM_Z,
    top: OPENING.y[1],
    bottom: OPENING.y[0],
    halfWidth: Math.max(-OPENING.x[0], OPENING.x[1]),
  }, width, height, SIDE).cameraZ;
  // Full view is the 16:9 contain-fit. A tall window keeps that zoom instead
  // of dollying out to the whole facade. A very narrow window only eases
  // back until the stage opening is in frame. Growth refits the same way.
  return Math.max(Math.min(fitted, fullView), opening);
}

/** Just past edge-on: the sheet has left the view and the house is the next page. */
const PAGE_OPEN = Math.PI * 0.62;
/** The turn finishes here. The camera is still on the reader's side of the sheet. */
const TURN_END = 0.55;
/** How far in front of the page the camera holds until the sheet has left. */
const REVEAL_CLEARANCE = 0.4;

/** How far the box-office page has turned about its spine. 0 faces the reader. */
export function pageTurn(journey: number): number {
  if (!(journey >= 0 && journey <= 1)) throw new Error(`journey outside [0, 1]: ${journey}`);
  return PAGE_OPEN * smooth(clamp01(journey / TURN_END));
}

/**
 * Depth of the booth-to-seat trip. During the turn the camera eases up to the
 * page and no further. Once the sheet has left, it steps through to the seat.
 * It stays on the aisle: no pullback, no trip to one side.
 */
export function journeyDepth(lobbyZ: number, seatZ: number, journey: number): number {
  // Ease up to the sheet, then step through to the seat. A seat that is
  // already behind the sheet (a very narrow window) does not detour forward
  // and then back.
  const pageStop = PAGE_FACE_Z + REVEAL_CLEARANCE;
  const near = Math.min(lobbyZ, Math.max(pageStop, seatZ));
  const approached = lobbyZ + (near - lobbyZ) * smooth(clamp01(journey / TURN_END));
  const seated = smooth(clamp01((journey - (TURN_END + 0.03)) / 0.27));
  return approached + (seatZ - approached) * seated;
}

/**
 * Camera for a view. At rest it orbits SIDE_VIEW_DEGREES around the frame
 * centre. The box-office page turns while the camera eases up to it; the
 * camera then steps through to the seat. The side view holds the whole way.
 * Read-along eases onto the centre line and dollies until the stage column
 * fits the viewport.
 * Pointer parallax translates the camera in its own view plane (lobby up to
 * 0.6/0.3, seat up to the frozen pointer envelope) and shifts the window so
 * the frame centre holds; it fades out during the turn and is zero under
 * reduced motion. No history-dependent smoothing.
 */
export function worldCamera(view: WorldView): WorldCamera {
  const { journey, width, height } = view;
  if (!(journey >= 0 && journey <= 1)) throw new Error(`journey outside [0, 1]: ${journey}`);
  const seatFit = fit(seatFrame(view.growthSteps), width, height, SIDE);
  const seatRestZ = seatCameraDepth(width, height, view.growthSteps);
  const aim = view.readAlong ?? 0;
  if (!(aim >= 0 && aim <= 1)) throw new Error(`read-along aim: ${aim}`);
  const zoom = readAlongZoom(width, height, view.growthSteps);
  const seatZ = seatRestZ + (zoom.z - seatRestZ) * aim;
  const lobbyCamZ = seatRestZ + BOOTH_FLIGHT;
  const lobbyY = (PAGE_Y[0] + PAGE_Y[1]) / 2;
  const z = journeyDepth(lobbyCamZ, seatZ, journey);
  const span = seatZ - lobbyCamZ;
  const e = Math.abs(span) < 1e-6 ? clamp01(journey) : clamp01((z - lobbyCamZ) / span);
  const seatY = seatFit.centerY + (zoom.y - seatFit.centerY) * aim;
  const y = lobbyY + (seatY - lobbyY) * e;
  const wl = 1 - smooth(clamp01(journey / 0.15));
  const ws = smooth(clamp01((journey - 0.85) / 0.15));
  const c = (v: number) => Math.max(-1, Math.min(1, v));
  const [px, py] = view.reducedMotion ? [0, 0] : [c(view.pointer[0]), c(view.pointer[1])];
  const ox = px * (wl * 0.6 + ws * CAMERA.pointer_x);
  const oy = py * (wl * 0.3 + ws * CAMERA.pointer_y);
  const focusZ = PAGE_FACE_Z + (PROSCENIUM_Z - PAGE_FACE_Z) * e;
  const df = z - focusZ;
  const yaw = aim >= 1 ? 0 : SIDE * (1 - aim);
  const standX = df * Math.tan(yaw);
  const [sin, cos] = [Math.sin(yaw), Math.cos(yaw)];
  // Parallax moves along the camera's own axes.
  const reach = df / cos;
  const aspect = width / height;
  return {
    position: [standX + ox * cos, y + oy, z - ox * sin],
    fovYDegrees: FOV_Y_DEGREES,
    aspect,
    yaw,
    shift: [ox === 0 ? 0 : ox / (reach * TAN * aspect), oy === 0 ? 0 : oy / (reach * TAN)],
    focusZ,
  };
}

export interface TicketZoom {
  /** World centre of the hovered ticket. */
  at: [number, number, number];
  /** 0 rests at the booth camera, 1 is zoomed onto the ticket. */
  amount: number;
}

/**
 * Dolly toward a ticket and centre it. Amount 0 returns the booth camera
 * unchanged, so the resting view stays on the tested path.
 */
export function applyTicketZoom(camera: WorldCamera, zoom: TicketZoom | null): WorldCamera {
  if (!zoom || !(zoom.amount > 0)) return camera;
  const a = smooth(clamp01(zoom.amount));
  const [tx, ty, tz] = zoom.at;
  const restDistance = camera.position[2] - tz;
  if (!(restDistance > 0.2)) return camera;
  const zoomDistance = Math.max(0.34, restDistance / 2.1);
  const zoomZ = tz + zoomDistance;
  const zoomX = tx + (zoomZ - tz) * Math.tan(camera.yaw);
  const zoomY = ty;
  const mix = (from: number, to: number) => from + (to - from) * a;
  return {
    ...camera,
    position: [mix(camera.position[0], zoomX), mix(camera.position[1], zoomY), mix(camera.position[2], zoomZ)],
    shift: [camera.shift[0] * (1 - a), camera.shift[1] * (1 - a)],
    focusZ: mix(camera.focusZ, tz),
  };
}

/** How long a ticket stays zoomed after the pointer leaves it. */
export const TICKET_ZOOM_HOLD_MS = 2000;

export interface TicketZoomClock {
  hovering: boolean;
  journey: number;
  now: number;
  offSince: number | null;
  /** True once a zoom is already held, so leaving can start the dwell. */
  holding: boolean;
}

/**
 * Stay zoomed while a ticket is under the pointer. After the pointer leaves,
 * hold for two seconds, then release. A journey out of the booth releases
 * immediately.
 */
export function ticketZoomIntent(input: TicketZoomClock): { want: 0 | 1; offSince: number | null } {
  if (input.journey >= 0.12) return { want: 0, offSince: null };
  if (input.hovering) return { want: 1, offSince: null };
  if (!input.holding) return { want: 0, offSince: null };
  const since = input.offSince ?? input.now;
  if (input.now - since >= TICKET_ZOOM_HOLD_MS) return { want: 0, offSince: since };
  return { want: 1, offSince: since };
}

/** A pan counts as arrived once the aim is this close to its ticket. */
export const TICKET_PAN_ARRIVE = 0.02;

export interface TicketPan {
  /** Point the zoom eases toward. Null keeps the current aim. */
  at: [number, number, number] | null;
  /** Set for the whole move from one ticket to the next. */
  locked: [number, number, number] | null;
}

const panDistance = (a: [number, number, number], b: [number, number, number]): number =>
  Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);

/**
 * Once a zoomed camera starts toward another ticket, that move finishes
 * there. Pointer changes during the pan do not retarget it.
 */
export function ticketPanLock(
  zoomAt: [number, number, number] | null,
  anchor: [number, number, number] | null,
  locked: [number, number, number] | null,
): TicketPan {
  if (locked && (!zoomAt || panDistance(zoomAt, locked) > TICKET_PAN_ARRIVE)) {
    return { at: locked, locked };
  }
  if (zoomAt && anchor && panDistance(zoomAt, anchor) > TICKET_PAN_ARRIVE) {
    return { at: anchor, locked: [...anchor] };
  }
  return { at: anchor, locked: null };
}

/** Each half of the booth/seat change: fade out, then fade in. */
export const BOOTH_SEAT_FADE_MS = 1000;

/**
 * Fade the booth to black, then fade the seat in. The switch happens on
 * the black frame so the camera can already be in place.
 */
export function boothSeatCrossfade(elapsedMs: number): { veil: number; switched: boolean; done: boolean } {
  const fade = BOOTH_SEAT_FADE_MS;
  const t = Math.max(0, elapsedMs);
  if (t >= fade * 2) return { veil: 0, switched: true, done: true };
  if (t >= fade) return { veil: Math.max(0, 1 - (t - fade) / fade), switched: true, done: false };
  return { veil: Math.min(1, t / fade), switched: false, done: false };
}

export interface Viewport {
  width: number;
  height: number;
}

/** Project a world point to CSS pixels (top-left origin) for a camera. */
export function projectWorld(cam: WorldCamera, point: readonly [number, number, number], viewport: Viewport): [number, number] {
  const [sin, cos] = [Math.sin(cam.yaw), Math.cos(cam.yaw)];
  const vx = point[0] - cam.position[0];
  const vz = point[2] - cam.position[2];
  const d = -vx * sin - vz * cos;
  if (!(d > 0)) throw new Error(`point behind camera: ${point[2]}`);
  const nx = (vx * cos - vz * sin) / (d * TAN * cam.aspect) + cam.shift[0];
  const ny = (point[1] - cam.position[1]) / (d * TAN) + cam.shift[1];
  return [((nx + 1) / 2) * viewport.width, ((1 - ny) / 2) * viewport.height];
}

export interface CssRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * CSS rect of a world rect facing +Z. Under yaw (no pitch or roll) it
 * projects as a trapezoid with vertical sides: "outer" bounds it, "inner"
 * is the largest axis-aligned rect inside it.
 */
export function projectRect(cam: WorldCamera, rect: Rect3, viewport: Viewport, bound: "outer" | "inner" = "outer"): CssRect {
  const at = (x: number, y: number) => projectWorld(cam, [x, y, rect.z], viewport);
  const [[l, lt], [, lb]] = [at(rect.x[0], rect.y[1]), at(rect.x[0], rect.y[0])];
  const [[r, rt], [, rb]] = [at(rect.x[1], rect.y[1]), at(rect.x[1], rect.y[0])];
  const inner = bound === "inner";
  const top = inner ? Math.max(lt, rt) : Math.min(lt, rt);
  const bottom = inner ? Math.min(lb, rb) : Math.max(lb, rb);
  return { left: l, top, width: r - l, height: bottom - top };
}

/** Plaque area: the part of the lectern face that is a screen rectangle. */
export function lecternRect(cam: WorldCamera, viewport: Viewport, growthSteps: number, lift = 0): CssRect {
  return projectRect(cam, lecternWorld(growthSteps, lift), viewport, "inner");
}
/** Bounding box of the banner controls for a camera. */
export function railRect(cam: WorldCamera, viewport: Viewport): CssRect {
  const signs = bannerScreen(cam, viewport);
  const left = Math.min(...signs.map((sign) => sign.rect.left));
  const top = Math.min(...signs.map((sign) => sign.rect.top));
  const right = Math.max(...signs.map((sign) => sign.rect.left + sign.rect.width));
  const bottom = Math.max(...signs.map((sign) => sign.rect.top + sign.rect.height));
  return { left, top, width: right - left, height: bottom - top };
}

/**
 * World span of the lobby view where it meets the foyer wall. The corners
 * are the screen edges at that plane, so a card this size fills the view.
 */
export function foyerWallBounds(
  width: number,
  height: number,
  pointer: [number, number] = [0, 0],
): { x: readonly [number, number]; y: readonly [number, number] } {
  if (!(width > 0 && height > 0)) throw new Error(`viewport bounds: ${width}x${height}`);
  const cam = worldCamera({ journey: 0, width, height, growthSteps: 0, pointer, reducedMotion: false });
  const vz = PAGE_FACE_Z - cam.position[2];
  const sin = Math.sin(cam.yaw);
  const cos = Math.cos(cam.yaw);
  const k = TAN * cam.aspect;
  const corners: [number, number][] = [];
  for (const nx of [-1, 1]) for (const ny of [-1, 1]) {
    const s = nx - cam.shift[0];
    const vx = vz * (sin - s * k * cos) / (cos + s * k * sin);
    const d = -vx * sin - vz * cos;
    const sy = ny - cam.shift[1];
    corners.push([cam.position[0] + vx, cam.position[1] + sy * d * TAN]);
  }
  const xs = corners.map((corner) => corner[0]);
  const ys = corners.map((corner) => corner[1]);
  return { x: [Math.min(...xs), Math.max(...xs)], y: [Math.min(...ys), Math.max(...ys)] };
}

const FOYER_CHECK_VIEWS = [
  [800, 1200],
  [960, 540],
  [1280, 720],
  [1497, 1360],
  [1920, 1080],
  [2560, 1080],
] as const;

const foyerViewSpan = (() => {
  let x0 = Infinity;
  let x1 = -Infinity;
  let y0 = Infinity;
  let y1 = -Infinity;
  const pointers: [number, number][] = [[0, 0], [1, 1], [-1, -1], [1, -1], [-1, 1]];
  for (const [width, height] of FOYER_CHECK_VIEWS) for (const pointer of pointers) {
    const bounds = foyerWallBounds(width, height, pointer);
    x0 = Math.min(x0, bounds.x[0]);
    x1 = Math.max(x1, bounds.x[1]);
    y0 = Math.min(y0, bounds.y[0]);
    y1 = Math.max(y1, bounds.y[1]);
  }
  const pad = 0.05;
  return { x: [x0 - pad, x1 + pad] as const, y: [y0 - pad, y1 + pad] as const };
})();

/** Lobby wallpaper and carpet span: every supported view, plus pointer travel. */
export const FOYER_VIEW = foyerViewSpan;

export function viewportPixelRatio(devicePixelRatio: number | undefined): number {
  return Math.min(devicePixelRatio ?? 1, RENDER_DEFAULTS.dpr_max);
}

/** Stage opening in world space (frozen profile), for house framing. */
export const STAGE_OPENING = OPENING;
