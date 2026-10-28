// Immersive house dressing for the full-viewport multiplane world: the box
// office (a ticket window set into the foyer wall, tickets, the papered
// page), the aisle rows, the auditorium shell around the frozen stage
// (facade with side boxes, footlights, orchestra railing, read-along
// lectern, foreground patrons) and the playback controls hung on the
// proscenium banner. Every house element is a flat die-cut card at a discrete
// depth, painted in code in a hand-coloured toy-theatre manner. All of it
// renders in the independent house pass: none of it enters the narrative
// stage target, so blackout and reveal rules are unchanged. Story-neutral:
// labels come from the host.
import {
  INSPECT,
  INSPECT_BACK,
  INSPECT_SEAT,
  LECTERN,
  BOOTH_DESIGN,
  BOOTH_SCALE,
  FOYER_WALL_Z,
  FOYER_VIEW,
  GRILLE_CARD,
  PAGE_EDGE_X,
  PAGE_FACE_Z,
  PAGE_HINGE_X,
  PAGE_Y,
  boothMap,
  PIT_FLOOR,
  PROSCENIUM_Z,
  PATRONS,
  STALL_FRAMES,
  BANNER_SEEK_TRACK,
  BANNER_Z,
  TICKET_SLOTS,
  bannerLayout,
  bannerScale,
  lecternTravel,
  lecternWorld,
  pageTurn,
  stallShift,
  placeInspectCard,
  worldCamera,
} from "./multiplane.js";
import type { WorldCamera } from "./multiplane.js";
import type { RailControlId } from "./multiplane.js";
import { HOUSE_ART_BRIEF } from "./house-brief.js";
import type { LoadedHouseArt, PreparedHouseArt, PreparedHouseAsset } from "./house-art.js";
import { keyOutMidGrey, keyOutBorderGround, keyOutBorderGrey, extrudeCutoutColor, detectFlatAperture, cutConnectedFlat, fillCurtainVelvet } from "./house-art.js";
import type { ApertureUv, WorldRect } from "./house-art.js";

type Three = typeof import("three");
type Ctx = CanvasRenderingContext2D;

export interface TicketView {
  id: string;
  title: string;
  subtitle: string;
  ready: boolean;
  punched?: boolean;
}
export interface InspectView {
  id: string;
  title: string;
  subtitle: string;
  synopsis: string;
  excerpt: string;
  readiness: string;
  seatLabel: string;
}
export interface ControlsView {
  /** A performance is loaded: transport levers and gauge respond. */
  enabled: boolean;
  playing: boolean;
  readAlong: boolean;
  /** Playhead as a 0..1 fraction of the master. */
  progress: number;
  fullscreen: boolean;
  /** Empty-stage curtain lever while no performance is mounted. */
  rehearsal?: boolean;
}

export interface HouseRig {
  nodes: import("three").Object3D[];
  resources: { dispose(): void }[];
  /** Pickable meshes; `userData.pick` holds the id. */
  pickables: import("three").Mesh[];
  curtainMap: import("three").Texture;
  setTickets(tickets: TicketView[], notice: string | null): void;
  setInspect(card: InspectView | null): void;
  setControls(state: ControlsView): void;
  setHighlight(id: string | null): void;
  /** World centre of the highlighted ticket, or null when none is hovered. */
  ticketAnchor(): [number, number, number] | null;
  /** 0 hides the ticket spotlight, 1 holds it at full. `at` aims the pool. */
  setHoverZoom(amount: number, at?: [number, number, number] | null): void;
  /** Footlight halo, 0..1, so the house lamps can dim without a blackout. */
  setGlow(amount: number): void;
  /** 0 is the pit rest, 1 is the reading pose. */
  lecternLift(): number;
  /** Advance the read-along trip and return its lift. A repeat at the same time stays put. */
  syncReadAlong(motion?: { now: number; reduced: boolean }): number;
  update(journey: number, width: number, height: number, growthSteps: number, camera?: WorldCamera, motion?: { now: number; reduced: boolean }): void;
  /** Owned texture bytes (RGBA8, no mipmaps) for the memory account. */
  textureBytes(): number;
}

const SERIF = '"Noto Serif", Georgia, serif';
const INK = "#21160d";
const PAPER = "#efe2c2";
const CRIMSON = "#8f1d1f";
const GOLD = "#c9a24a";
const GOLD_DARK = "#7d5d22";
const BRASS = "#b8903c";

function rng(seed: number): () => number {
  let s = seed >>> 0;
  return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 4294967296);
}

/** Hand-coloured edge: flat fill, ink outline, a few hatch strokes. */
function inked(ctx: Ctx, path: () => void, fill: string, line = 3): void {
  ctx.beginPath();
  path();
  ctx.fillStyle = fill;
  ctx.fill();
  ctx.lineWidth = line;
  ctx.strokeStyle = INK;
  ctx.stroke();
}

function hatch(ctx: Ctx, x: number, y: number, w: number, h: number, gap = 7, alpha = 0.22): void {
  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, w, h);
  ctx.clip();
  ctx.strokeStyle = `rgba(33,22,13,${alpha})`;
  ctx.lineWidth = 1.2;
  for (let i = -h; i < w; i += gap) {
    ctx.beginPath();
    ctx.moveTo(x + i, y + h);
    ctx.lineTo(x + i + h, y);
    ctx.stroke();
  }
  ctx.restore();
}

function damask(ctx: Ctx, size: number, base: string, motif: string): void {
  ctx.fillStyle = base;
  ctx.fillRect(0, 0, size, size);
  ctx.fillStyle = motif;
  for (const [cx, cy] of [[size / 2, size / 2], [0, 0], [size, 0], [0, size], [size, size]] as const) {
    ctx.beginPath();
    ctx.ellipse(cx, cy, size * 0.12, size * 0.26, 0, 0, Math.PI * 2);
    ctx.fill();
    for (const s of [-1, 1]) {
      ctx.beginPath();
      ctx.ellipse(cx + s * size * 0.17, cy + size * 0.06, size * 0.07, size * 0.15, s * 0.6, 0, Math.PI * 2);
      ctx.fill();
    }
  }
  ctx.strokeStyle = motif;
  ctx.lineWidth = size * 0.012;
  ctx.strokeRect(size * 0.02, size * 0.02, size * 0.96, size * 0.96);
}

function patron(ctx: Ctx, x: number, base: number, s: number, kind: number): void {
  ctx.fillStyle = "#140d08";
  ctx.beginPath();
  ctx.ellipse(x, base, 44 * s, 30 * s, 0, Math.PI, 0);
  ctx.fill();
  ctx.fillRect(x - 44 * s, base, 88 * s, 60 * s);
  ctx.beginPath();
  ctx.ellipse(x, base - 46 * s, 17 * s, 21 * s, 0, 0, Math.PI * 2);
  ctx.fill();
  if (kind === 0) {
    ctx.fillRect(x - 15 * s, base - 102 * s, 30 * s, 40 * s);
    ctx.fillRect(x - 26 * s, base - 64 * s, 52 * s, 6 * s);
  } else if (kind === 1) {
    ctx.beginPath();
    ctx.ellipse(x, base - 54 * s, 27 * s, 22 * s, 0, Math.PI, 0);
    ctx.fill();
    ctx.beginPath();
    ctx.moveTo(x + 10 * s, base - 70 * s);
    ctx.bezierCurveTo(x + 40 * s, base - 120 * s, x + 64 * s, base - 96 * s, x + 50 * s, base - 82 * s);
    ctx.lineWidth = 6 * s;
    ctx.strokeStyle = "#140d08";
    ctx.stroke();
  } else if (kind === 2) {
    ctx.beginPath();
    ctx.ellipse(x, base - 60 * s, 24 * s, 9 * s, 0, 0, Math.PI * 2);
    ctx.fill();
  }
}

function seatRow(ctx: Ctx, w: number, h: number, seed: number): void {
  const r = rng(seed);
  const seatW = 92;
  for (let x = 6 + (seed % 2) * 46; x < w - seatW; x += seatW + 8) {
    if (r() < 0.55) patron(ctx, x + seatW / 2, h * 0.42, 0.9 + r() * 0.25, Math.floor(r() * 4));
    const top = h * 0.36;
    inked(ctx, () => {
      ctx.moveTo(x, h);
      ctx.lineTo(x, top + 22);
      ctx.quadraticCurveTo(x, top, x + 22, top);
      ctx.lineTo(x + seatW - 22, top);
      ctx.quadraticCurveTo(x + seatW, top, x + seatW, top + 22);
      ctx.lineTo(x + seatW, h);
    }, CRIMSON, 3);
    ctx.strokeStyle = GOLD;
    ctx.lineWidth = 3;
    ctx.strokeRect(x + 10, top + 14, seatW - 20, h - top - 30);
    hatch(ctx, x, h * 0.7, seatW, h * 0.3, 6, 0.35);
  }
}

function plaqueText(ctx: Ctx, text: string, x: number, y: number, px: number, color: string, weight = ""): void {
  ctx.fillStyle = color;
  ctx.font = `${weight} ${px}px ${SERIF}`.trim();
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(text, x, y);
}

function wrap(ctx: Ctx, text: string, x: number, y: number, maxW: number, lineH: number, maxLines: number): number {
  const words = text.split(/\s+/);
  let line = "";
  let lines = 0;
  for (let i = 0; i < words.length; i++) {
    const next = line ? `${line} ${words[i]}` : words[i]!;
    if (ctx.measureText(next).width > maxW && line) {
      if (lines === maxLines - 1) {
        ctx.fillText(`${line}…`, x, y + lines * lineH);
        return lines + 1;
      }
      ctx.fillText(line, x, y + lines * lineH);
      lines++;
      line = words[i]!;
    } else {
      line = next;
    }
  }
  if (line) ctx.fillText(line, x, y + lines * lineH);
  return lines + 1;
}

function brassPlate(ctx: Ctx, w: number, h: number, glow: boolean, dim: boolean): void {
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, dim ? "#7b6a48" : "#e2c27a");
  g.addColorStop(0.5, dim ? "#5c4d33" : BRASS);
  g.addColorStop(1, dim ? "#3e3322" : "#6e5320");
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.roundRect(3, 3, w - 6, h - 6, h * 0.18);
  ctx.fill();
  ctx.lineWidth = glow ? 7 : 4;
  ctx.strokeStyle = glow ? "#fff1c4" : INK;
  ctx.stroke();
  ctx.fillStyle = "rgba(33,22,13,0.55)";
  for (const [x, y] of [[14, 14], [w - 14, 14], [14, h - 14], [w - 14, h - 14]] as const) {
    ctx.beginPath();
    ctx.arc(x, y, 4, 0, Math.PI * 2);
    ctx.fill();
  }
}

export interface BuildHouseOptions {
  houseArt?: LoadedHouseArt | PreparedHouseArt | null;
}

export function buildHouse(
  THREE: Three,
  options?: BuildHouseOptions | LoadedHouseArt | null,
): HouseRig {
  const houseArt: LoadedHouseArt | PreparedHouseArt | null =
    options && "assets" in options ? options : options?.houseArt ?? null;
  const resources: { dispose(): void }[] = [];
  const textures: { w: number; h: number }[] = [];
  const track = <T extends { dispose(): void }>(r: T): T => {
    resources.push(r);
    return r;
  };
  interface Painted {
    texture: import("three").CanvasTexture;
    canvas: HTMLCanvasElement;
    ctx: Ctx;
  }
  const paint = (w: number, h: number, draw: (ctx: Ctx) => void, repeat?: [number, number]): Painted => {
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("2d canvas unavailable");
    draw(ctx);
    const texture = track(new THREE.CanvasTexture(canvas));
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = false;
    if (repeat) {
      texture.wrapS = THREE.RepeatWrapping;
      texture.wrapT = THREE.RepeatWrapping;
      texture.repeat.set(repeat[0], repeat[1]);
    }
    textures.push({ w, h });
    return { texture, canvas, ctx };
  };
  const paintAsset = (
    id: string,
    fallbackW: number,
    fallbackH: number,
    fallbackDraw: (c: Ctx) => void,
    opts?: {
      repeat?: [number, number];
      overlay?: (c: Ctx, w: number, h: number) => void;
    },
  ): Painted => {
    const asset = houseArt?.assets.get(id);
    const named = (painted: Painted): Painted => {
      painted.texture.name = id;
      return painted;
    };
    if (!asset) {
      return named(paint(fallbackW, fallbackH, fallbackDraw, opts?.repeat));
    }
    const [w, h] = asset.deliver_px;
    return named(paint(
      w,
      h,
      (c) => {
        const img = (asset as PreparedHouseAsset).image;
        if (img) {
          c.drawImage(img, 0, 0, w, h);
        } else {
          fallbackDraw(c);
        }

        if (img && (asset.alpha === "cutout" || id === "house.pit-rail")) {
          try {
            const imgData = c.getImageData(0, 0, w, h);
            const standee = id === "aisle.row-a" || id === "aisle.row-b" || id.startsWith("house.patrons") || id === "lobby.palm" || id.startsWith("banner.");
            if (id === "house.pit-rail") keyOutMidGrey(imgData.data, w, h);
            else if (standee) keyOutBorderGrey(imgData.data, w, h);
            else if (asset.alpha === "cutout") keyOutBorderGround(imgData.data, w, h);
            extrudeCutoutColor(imgData.data, w, h);
            c.putImageData(imgData, 0, 0);
          } catch {
            // Environments without full getImageData support
          }
        }

        if (id === "house.curtain" && img) {
          try {
            const imgData = c.getImageData(0, 0, w, h);
            if (fillCurtainVelvet(imgData.data, w, h)) c.putImageData(imgData, 0, 0);
          } catch {
            // An unreadable bitmap keeps the painted velvet.
          }
        }

        if (asset.alpha === "aperture") {
          if (id === "house.facade") {
            let opened = false;
            try {
              const imgData = c.getImageData(0, 0, w, h);
              if (cutConnectedFlat(imgData.data, w, h)) {
                c.putImageData(imgData, 0, 0);
                opened = true;
              }
            } catch {
              // Detection needs a readable bitmap; the brief window remains.
            }
            if (!opened) {
              const x0 = Math.floor(0.1846 * w);
              const x1 = Math.ceil(0.8154 * w);
              const y0 = Math.floor(0.2247 * h);
              c.save();
              c.globalCompositeOperation = "destination-out";
              c.fillRect(x0, y0, x1 - x0, h - y0);
              c.restore();
            }
          } else if (id === "lobby.grille") {
            let u0 = 0.1625;
            let v0 = 0.2813;
            let u1 = 0.8375;
            let v1 = 0.8036;
            try {
              const imgData = c.getImageData(0, 0, w, h);
              const found = detectFlatAperture(imgData.data, w, h);
              if (found) {
                u0 = found.u0;
                v0 = found.v0;
                u1 = found.u1;
                v1 = found.v1;
                grilleOpening = apertureOnGrille(found);
              }
            } catch {
              // Detection needs a readable bitmap; the brief window remains.
            }
            const x0 = Math.floor(u0 * w);
            const x1 = Math.ceil(u1 * w);
            const y0 = Math.floor(v0 * h);
            const y1 = Math.ceil(v1 * h);
            c.save();
            c.globalCompositeOperation = "destination-out";
            c.fillRect(x0, y0, x1 - x0, y1 - y0);
            c.restore();
          }
        }

        if (opts?.overlay) {
          opts.overlay(c, w, h);
        }
      },
      opts?.repeat,
    ));
  };
  const repaint = (p: Painted, draw: (ctx: Ctx) => void): void => {
    p.ctx.clearRect(0, 0, p.canvas.width, p.canvas.height);
    draw(p.ctx);
    p.texture.needsUpdate = true;
  };
  const material = (map: import("three").Texture, cutout = true): import("three").MeshLambertMaterial =>
    track(new THREE.MeshLambertMaterial({ map, alphaTest: cutout ? 0.5 : 0 }));
  const card = (
    x: readonly [number, number], y: readonly [number, number], z: number,
    mat: import("three").Material,
    shade = true,
  ): import("three").Mesh => {
    const mesh = new THREE.Mesh(track(new THREE.PlaneGeometry(x[1] - x[0], y[1] - y[0])), mat);
    mesh.position.set((x[0] + x[1]) / 2, (y[0] + y[1]) / 2, z);
    mesh.castShadow = shade;
    mesh.receiveShadow = shade;
    const lambert = mat as import("three").MeshLambertMaterial;
    if (shade && lambert.isMeshLambertMaterial && lambert.alphaTest > 0 && lambert.map) {
      mesh.customDepthMaterial = track(new THREE.MeshDepthMaterial({
        depthPacking: THREE.RGBADepthPacking,
        map: lambert.map,
        alphaTest: lambert.alphaTest,
      }));
    }
    return mesh;
  };
  /** Grille card in world units: the booth frame. */
  const grilleCorner = (x: number, y: number) => boothMap(x, y, BOOTH_DESIGN.face);
  const GRILLE_X: readonly [number, number] = [grilleCorner(-8, 0)[0], grilleCorner(8, 0)[0]];
  const GRILLE_Y: readonly [number, number] = [grilleCorner(0, -1)[1], grilleCorner(0, 10.2)[1]];
  let grilleOpening: WorldRect | null = null;
  const passageRight = 22;
  const apertureOnGrille = (uv: ApertureUv): WorldRect => {
    const width = GRILLE_X[1] - GRILLE_X[0];
    const height = GRILLE_Y[1] - GRILLE_Y[0];
    return {
      x0: GRILLE_X[0] + uv.u0 * width,
      x1: GRILLE_X[0] + uv.u1 * width,
      y1: GRILLE_Y[1] - uv.v0 * height,
      y0: GRILLE_Y[1] - uv.v1 * height,
    };
  };
  const nodes: import("three").Object3D[] = [];
  const pickables: import("three").Mesh[] = [];
  const pick = (mesh: import("three").Mesh, id: string): import("three").Mesh => {
    mesh.userData.pick = id;
    pickables.push(mesh);
    return mesh;
  };
  const lobbyGroup = new THREE.Group();
  const houseGroup = new THREE.Group();
  const seatGroup = new THREE.Group();
  nodes.push(lobbyGroup, houseGroup, seatGroup);

  // ---- Shared papers -------------------------------------------------
  const wallTile = paintAsset("lobby.wallpaper", 256, 256, (c) => damask(c, 256, "#3a171c", "#572a2c"), { repeat: [1, 1] });
  // World size of one wallpaper sheet at the foyer wall. Small enough that
  // the 512 painting is not stretched soft. Farther walls grow the sheet
  // so the pattern stays this size on screen.
  const WALLPAPER_SHEET = 0.5;
  const lobbyDistance = worldCamera({
    journey: 0, width: 16, height: 9, growthSteps: 0, pointer: [0, 0], reducedMotion: true,
  }).position[2] - PAGE_FACE_Z;
  const seatZ = worldCamera({
    journey: 1, width: 16, height: 9, growthSteps: 0, pointer: [0, 0], reducedMotion: true,
  }).position[2];
  const sheetAt = (distance: number) => WALLPAPER_SHEET * distance / lobbyDistance;
  const wallMat = (w: number, h: number, distance: number, tint = 0xffffff): import("three").MeshLambertMaterial => {
    const t = track(wallTile.texture.clone());
    t.wrapS = THREE.RepeatWrapping;
    t.wrapT = THREE.RepeatWrapping;
    const sheet = sheetAt(distance);
    t.repeat.set(w / sheet, h / sheet);
    t.needsUpdate = true;
    return track(new THREE.MeshLambertMaterial({ map: t, color: tint }));
  };
  const wall = (
    x: readonly [number, number],
    y: readonly [number, number],
    z: number,
    group: import("three").Group,
    distance: number,
  ) => {
    const mesh = card(x, y, z, wallMat(x[1] - x[0], y[1] - y[0], distance));
    group.add(mesh);
    return mesh;
  };
  // ---- Lobby: the box office is one paper page in front of the house ---
  const [pageBottom, pageTop] = PAGE_Y;
  const pageBack = wall([PAGE_HINGE_X, PAGE_EDGE_X], [pageBottom, pageTop], FOYER_WALL_Z, lobbyGroup, lobbyDistance + PAGE_FACE_Z - FOYER_WALL_Z);
  pageBack.layers.enable(1);
  pageBack.castShadow = false;
  pageBack.userData.role = "booth-page";

  const wallZ = PAGE_FACE_Z;
  const grilleZ = wallZ + 0.012;
  const baseY = GRILLE_Y[0];
  const grille = paintAsset("lobby.grille", 1600, 1120, (c) => {
    const g = c.createLinearGradient(0, 0, 0, 1120);
    g.addColorStop(0, "#d7b56a");
    g.addColorStop(1, "#8a6427");
    c.fillStyle = g;
    c.fillRect(0, 0, 1600, 1120);
    hatch(c, 0, 0, 1600, 1120, 10, 0.14);
    // Window: x +-5.4, y 1.2..8.3 within the x +-8, y -1..10.2 card.
    const px = (x: number) => ((x + 8) / 16) * 1600;
    const py = (y: number) => ((10.2 - y) / 11.2) * 1120;
    const win = () => {
      c.moveTo(px(-5.4), py(1.2));
      c.lineTo(px(-5.4), py(7.05));
      c.ellipse(px(0), py(7.05), px(5.4) - px(0), py(7.05) - py(8.9), 0, Math.PI, 0);
      c.lineTo(px(5.4), py(1.2));
      c.closePath();
    };
    c.beginPath();
    win();
    c.lineWidth = 26;
    c.strokeStyle = GOLD_DARK;
    c.stroke();
    c.lineWidth = 6;
    c.strokeStyle = INK;
    c.stroke();
    c.save();
    c.globalCompositeOperation = "destination-out";
    c.beginPath();
    win();
    c.fill();
    c.restore();
    // Fan of brass bars in the arch head only; the centre stays clear.
    c.save();
    c.beginPath();
    win();
    c.clip();
    c.strokeStyle = BRASS;
    c.lineWidth = 8;
    for (let a = 0; a <= 12; a++) {
      const ang = Math.PI + (a / 12) * Math.PI;
      c.beginPath();
      c.moveTo(px(0), py(7.05));
      c.lineTo(px(0) + Math.cos(ang) * 900, py(7.05) + Math.sin(ang) * 900);
      c.stroke();
    }
    c.restore();
    c.save();
    c.globalCompositeOperation = "destination-out";
    c.fillRect(px(-5.4), py(7.05), px(5.4) - px(-5.4), py(1.2) - py(7.05));
    c.restore();
    // Engraved crossbar over the window: the counter's name board.
    inked(c, () => c.rect(px(-5.4), py(7.4), px(5.4) - px(-5.4), py(7.05) - py(7.4)), BRASS, 4);
    inked(c, () => c.rect(px(-2.2), py(9.95), px(2.2) - px(-2.2), 70), "#2b1a10", 4);
  });
  const frameTex = track(grille.texture.clone());
  frameTex.name = "lobby.grille";
  frameTex.needsUpdate = true;
  const grilleMesh = card(GRILLE_X, [GRILLE_Y[0], GRILLE_Y[1]], grilleZ, material(frameTex));
  grilleMesh.userData.role = "booth-grille";
  lobbyGroup.add(grilleMesh);
  const opening = grilleOpening ?? apertureOnGrille({ u0: 0.1625, v0: 0.2813, u1: 0.8375, v1: 0.8036 });
  const wallX: readonly [number, number] = [FOYER_VIEW.x[0], FOYER_VIEW.x[1]];
  const wallTop = Math.max(pageTop, FOYER_VIEW.y[1]);
  // One wallpaper sheet across the booth. The delivered painting does not
  // tile, so a repeat would cut a seam through the pattern.
  const sheet = { x: -1.45, y: 3.85, size: WALLPAPER_SHEET };
  const rects: [number, number, number, number][] = [
    [wallX[0], opening.x0, baseY, wallTop],
    [opening.x1, wallX[1], baseY, wallTop],
    [opening.x0, opening.x1, opening.y1, wallTop],
    [opening.x0, opening.x1, baseY, opening.y0],
  ];
  const positions: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  for (const [left, right, bottom, top] of rects) {
    if (!(right - left > 1e-4 && top - bottom > 1e-4)) continue;
    const base = positions.length / 3;
    for (const [x, y] of [[left, bottom], [right, bottom], [right, top], [left, top]] as const) {
      positions.push(x, y, 0);
      uvs.push((x - sheet.x) / sheet.size, (y - sheet.y) / sheet.size);
    }
    indices.push(base, base + 1, base + 2, base, base + 2, base + 3);
  }
  const wallGeo = track(new THREE.BufferGeometry());
  wallGeo.setAttribute("position", new THREE.BufferAttribute(new Float32Array(positions), 3));
  wallGeo.setAttribute("uv", new THREE.BufferAttribute(new Float32Array(uvs), 2));
  wallGeo.setIndex(indices);
  wallGeo.computeVertexNormals();
  const foyerTex = track(wallTile.texture.clone());
  foyerTex.name = "lobby.wallpaper";
  foyerTex.wrapS = THREE.RepeatWrapping;
  foyerTex.wrapT = THREE.RepeatWrapping;
  foyerTex.repeat.set(1, 1);
  foyerTex.needsUpdate = true;
  const foyerWall = new THREE.Mesh(wallGeo, track(new THREE.MeshLambertMaterial({ map: foyerTex })));
  foyerWall.position.z = wallZ;
  foyerWall.castShadow = true;
  foyerWall.receiveShadow = true;
  foyerWall.userData.role = "booth-wall";
  lobbyGroup.add(foyerWall);

  // Tickets, paper shadows, rehearsal signs, notice and inspection card.
  const lobbyUi = new THREE.Group();
  lobbyGroup.add(lobbyUi);
  interface TicketRig {
    paper: Painted;
    mesh: import("three").Mesh;
    shadow: import("three").Mesh;
    sign: import("three").Mesh;
    view: TicketView | null;
  }
  const signTex = paint(512, 112, (c) => {
    brassPlate(c, 512, 112, false, false);
    plaqueText(c, "UNDER REHEARSAL", 256, 58, 40, INK, "bold");
  });
  const shadowTex = paint(64, 64, (c) => {
    const g = c.createRadialGradient(32, 32, 8, 32, 32, 32);
    g.addColorStop(0, "rgba(0,0,0,0.55)");
    g.addColorStop(1, "rgba(0,0,0,0)");
    c.fillStyle = g;
    c.fillRect(0, 0, 64, 64);
  });
  const shadowMat = track(new THREE.MeshBasicMaterial({ map: shadowTex.texture, transparent: true, depthWrite: false }));
  const ticketCanvasPx = (): readonly [number, number] =>
    houseArt?.assets.get("lobby.ticket")?.deliver_px ?? [640, 330];
  const tickets: TicketRig[] = TICKET_SLOTS.map((slot, i) => {
    const [tw, th] = ticketCanvasPx();
    const paper = paint(tw, th, () => {});
    const mesh = pick(card(slot.x, slot.y, slot.z, material(paper.texture)), `ticket:${i}`);
    mesh.layers.enable(1);
    const pad = 0.3 * BOOTH_SCALE;
    const shadow = card([slot.x[0] - pad, slot.x[1] + pad], [slot.y[0] - pad, slot.y[1] + pad], slot.z - 0.03 * BOOTH_SCALE, shadowMat, false);
    const signTop = boothMap(0, 2.3, slot.z)[1];
    const signBottom = boothMap(0, 1.75, slot.z)[1];
    const sign = card(
      [slot.x[0] + 0.2 * BOOTH_SCALE, slot.x[1] - 0.2 * BOOTH_SCALE],
      [signBottom, signTop],
      slot.z + 0.05 * BOOTH_SCALE,
      material(signTex.texture),
    );
    lobbyUi.add(shadow, mesh, sign);
    return { paper, mesh, shadow, sign, view: null };
  });
  const ticketSpot = new THREE.SpotLight("#fff6e4", 0, 3.2, 0.32, 0.35, 0);
  // The view camera is layer 0, so the light has to stay there or it never
  // renders. Layer 1 is only for the shadow pass, which drops the grille.
  ticketSpot.layers.enable(1);
  ticketSpot.castShadow = false;
  ticketSpot.shadow.mapSize.set(2048, 2048);
  ticketSpot.shadow.camera.layers.set(1);
  ticketSpot.shadow.camera.near = 0.04;
  ticketSpot.shadow.bias = -0.0004;
  ticketSpot.shadow.normalBias = 0.012;
  ticketSpot.visible = false;
  ticketSpot.userData.role = "ticket-spot";
  nodes.push(ticketSpot, ticketSpot.target);
  const ticketCentre = (index: number): [number, number, number] => {
    const slot = TICKET_SLOTS[index]!;
    return [(slot.x[0] + slot.x[1]) / 2, (slot.y[0] + slot.y[1]) / 2, slot.z];
  };
  const aimTicketSpotAt = (tx: number, ty: number, tz: number): void => {
    // Between the camera and the card, only slightly above it, so the
    // bright center lands on the ticket instead of the notice overhead.
    const lift = 0.16;
    const front = 0.55;
    ticketSpot.position.set(tx, ty + lift, tz + front);
    ticketSpot.target.position.set(tx, ty, tz);
    ticketSpot.angle = 0.32;
    ticketSpot.penumbra = 0.35;
    ticketSpot.distance = 3.2;
    ticketSpot.target.updateMatrixWorld();
  };
  const aimTicketSpot = (index: number): void => {
    const [tx, ty, tz] = ticketCentre(index);
    aimTicketSpotAt(tx, ty, tz);
  };
  const drawTicketLabels = (c: Ctx, t: TicketView, w: number, h: number, lifted: boolean): void => {
    const sx = w / 640;
    const sy = h / 330;
    if (t.punched) {
      c.save();
      c.globalCompositeOperation = "destination-out";
      c.beginPath();
      c.arc(92 * sx, 60 * sy, 18 * Math.min(sx, sy), 0, Math.PI * 2);
      c.fill();
      c.restore();
    }
    if (lifted) {
      c.save();
      c.globalAlpha = 0.08;
      c.fillStyle = "#fff8e8";
      c.fillRect(0, 0, w, h);
      c.restore();
    }
    c.save();
    c.translate(92 * sx, 165 * sy);
    c.rotate(-Math.PI / 2);
    plaqueText(c, "ADMIT ONE", 0, 0, 30 * sy, CRIMSON, "bold");
    c.restore();
    plaqueText(c, "PAPER THEATER · STALLS", 380 * sx, 58 * sy, 20 * sy, "#5a4630", "bold");
    c.font = `bold ${32 * sy}px ${SERIF}`;
    c.fillStyle = INK;
    c.textAlign = "center";
    wrap(c, t.title, 380 * sx, 118 * sy, 420 * sx, 38 * sy, 2);
    plaqueText(c, t.subtitle, 380 * sx, 214 * sy, 22 * sy, INK);
    plaqueText(c, t.ready ? "Performance ready" : "Not yet published", 380 * sx, 262 * sy, 20 * sy, t.ready ? "#2f5d50" : CRIMSON);
  };
  const drawTicket = (c: Ctx, t: TicketView, lifted: boolean): void => {
    inked(c, () => c.roundRect(6, 6, 628, 318, 18), lifted ? "#f7ecd0" : PAPER, 4);
    c.strokeStyle = CRIMSON;
    c.lineWidth = 3;
    c.strokeRect(30, 26, 580, 278);
    c.setLineDash([8, 8]);
    c.strokeStyle = INK;
    c.beginPath();
    c.moveTo(150, 26);
    c.lineTo(150, 304);
    c.stroke();
    c.setLineDash([]);
    drawTicketLabels(c, t, 640, 330, lifted);
  };
  const notice = paint(1024, 128, () => {});
  const noticeA = boothMap(-3.4, 4.3, BOOTH_DESIGN.lobby + 0.9);
  const noticeB = boothMap(3.4, 5.15, BOOTH_DESIGN.lobby + 0.9);
  const noticeMesh = pick(card([noticeA[0], noticeB[0]], [noticeA[1], noticeB[1]], noticeA[2], material(notice.texture)), "notice");
  noticeMesh.visible = false;
  lobbyUi.add(noticeMesh);

  const inspectGroup = new THREE.Group();
  inspectGroup.visible = false;
  lobbyGroup.add(inspectGroup);
  const inspectPaper = paint(1024, 630, () => {});
  const briefWindow = apertureOnGrille({ u0: 0.1625, v0: 0.2813, u1: 0.8375, v1: 0.8036 });
  const placedInspect = placeInspectCard(grilleOpening ?? briefWindow);
  inspectGroup.add(card(placedInspect.card.x, placedInspect.card.y, INSPECT.z, material(inspectPaper.texture)));
  const buttonTex = (label: string): Painted[] => [false, true].map((glow) => paint(440, 106, (c) => {
    brassPlate(c, 440, 106, glow, false);
    plaqueText(c, label, 220, 55, 36, INK, "bold");
  }));
  const backTex = buttonTex("Return Ticket");
  const seatTex = buttonTex("Take Your Seat");
  const backMats = backTex.map((p) => material(p.texture));
  const seatMats = seatTex.map((p) => material(p.texture));
  const backBtn = pick(card(placedInspect.back.x, placedInspect.back.y, INSPECT_BACK.z, backMats[0]!), "inspect:back");
  const seatBtn = pick(card(placedInspect.seat.x, placedInspect.seat.y, INSPECT_SEAT.z, seatMats[0]!), "inspect:seat");
  inspectGroup.add(backBtn, seatBtn);

  // ---- Aisle: one facing frame per stall row, seats in front of the floor ----
  const rows = [0, 1].map((seed) =>
    paintAsset(seed ? "aisle.row-b" : "aisle.row-a", 1024, 400, (c) => seatRow(c, 1024, 400, seed + 7)),
  );
  const carpet = paintAsset("floor.carpet", 256, 256, (c) => {
    c.fillStyle = "#5c1416";
    c.fillRect(0, 0, 256, 256);
    c.fillStyle = "#7a2022";
    for (let y = 0; y < 256; y += 64) for (let x = (y / 64) % 2 * 32; x < 256; x += 64) {
      c.beginPath();
      c.moveTo(x + 32, y);
      c.lineTo(x + 64, y + 32);
      c.lineTo(x + 32, y + 64);
      c.lineTo(x, y + 32);
      c.fill();
    }
  }, { repeat: [1, 1] });
  const runner = paintAsset("floor.runner", 256, 512, (c) => {
    c.fillStyle = "#2c3f5c";
    c.fillRect(0, 0, 256, 512);
    c.fillStyle = "#c6a15a";
    c.fillRect(0, 0, 18, 512);
    c.fillRect(238, 0, 18, 512);
  }, { repeat: [1, 1] });
  const bandMat = (
    source: import("three").Texture,
    x: readonly [number, number],
    y: readonly [number, number],
    tile: readonly [number, number],
  ): import("three").MeshLambertMaterial => {
    const t = track(source.clone());
    t.wrapS = THREE.RepeatWrapping;
    t.wrapT = THREE.RepeatWrapping;
    t.repeat.set((x[1] - x[0]) / tile[0], (y[1] - y[0]) / tile[1]);
    t.needsUpdate = true;
    return track(new THREE.MeshLambertMaterial({ map: t }));
  };
  const floorX: readonly [number, number] = [FOYER_VIEW.x[0], FOYER_VIEW.x[1]];
  const floorBottom = Math.min(pageBottom, FOYER_VIEW.y[0]);
  const boothFloor = card(
    floorX,
    [floorBottom, baseY],
    wallZ + 0.006,
    bandMat(carpet.texture, floorX, [floorBottom, baseY], [0.42, 0.42]),
  );
  boothFloor.userData.role = "booth-floor";
  lobbyGroup.add(boothFloor);
  // Palms stand on the carpet, in front of the grille, one on each side.
  const palm = paintAsset("lobby.palm", 864, 1152, (c) => {
    const w = 864;
    const h = 1152;
    c.fillStyle = "#808080";
    c.fillRect(0, 0, w, h);
    c.fillStyle = "#e7d7b4";
    c.beginPath();
    c.ellipse(w * 0.5, h * 0.78, w * 0.16, h * 0.16, 0, 0, Math.PI * 2);
    c.fill();
    c.strokeStyle = "#1c140c";
    c.lineWidth = 4;
    c.stroke();
    c.fillStyle = "#c6a15a";
    c.fillRect(w * 0.36, h * 0.7, w * 0.28, h * 0.035);
    c.fillStyle = "#2f5d3a";
    for (let i = -3; i <= 3; i++) {
      c.beginPath();
      c.moveTo(w * 0.5, h * 0.72);
      c.quadraticCurveTo(w * (0.5 + i * 0.08), h * 0.4, w * (0.5 + i * 0.12), h * 0.16);
      c.lineWidth = 10;
      c.strokeStyle = "#2f5d3a";
      c.stroke();
    }
  });
  const grilleH = GRILLE_Y[1] - GRILLE_Y[0];
  // Still tall enough for the fronds to cross the arch, a step smaller
  // than the first foreground plants. The vase sits on the carpet in
  // front of the plinth.
  const palmH = grilleH * 1.28;
  const palmW = palmH * 0.75;
  const plantZ = grilleZ + 0.16;
  const centers = ([-1, 1] as const).map((side) => {
    const edge = side < 0 ? GRILLE_X[0] : GRILLE_X[1];
    return edge + side * palmW * 0.02;
  });
  // The band is the floor in front of the booth: its top stays at the
  // grille's foot. The plant card is lowered so the drawn pedestal
  // crosses that edge and the vase reads as standing on it.
  const bandTop = baseY;
  const bandOverlap = palmH * 0.12;
  const vaseY = bandTop - bandOverlap;
  const bandH = grilleH * 0.34;
  const bandX: readonly [number, number] = [centers[0]! - palmW / 2, centers[1]! + palmW / 2];
  const bandY: readonly [number, number] = [bandTop - bandH, bandTop];
  const palmFloor = card(
    bandX,
    bandY,
    plantZ - 0.02,
    bandMat(carpet.texture, bandX, bandY, [0.42, 0.42]),
  );
  palmFloor.userData.role = "booth-palm-floor";
  lobbyGroup.add(palmFloor);
  const rightTex = track(palm.texture.clone());
  rightTex.wrapS = THREE.RepeatWrapping;
  rightTex.repeat.x = -1;
  rightTex.offset.x = 1;
  rightTex.needsUpdate = true;
  for (const [index, side] of ([-1, 1] as const).entries()) {
    const center = centers[index]!;
    const mesh = card(
      [center - palmW / 2, center + palmW / 2],
      [vaseY, vaseY + palmH],
      plantZ,
      material(side < 0 ? palm.texture : rightTex),
    );
    mesh.userData.role = "booth-palm";
    lobbyGroup.add(mesh);
  }
  const stallSeats: import("three").Mesh[] = [];
  for (const [i, frame] of STALL_FRAMES.entries()) {
    const right = Math.max(frame.rightJamb.x[1], passageRight);
    const floorX: readonly [number, number] = [frame.floor.x[0], Math.max(frame.floor.x[1], right)];
    const rightJambX: readonly [number, number] = [frame.rightJamb.x[0], right];
    houseGroup.add(card(floorX, frame.floor.y, frame.floor.z, bandMat(carpet.texture, floorX, frame.floor.y, [3.2, 3.2])));
    houseGroup.add(card(frame.runner.x, frame.runner.y, frame.runner.z, bandMat(runner.texture, frame.runner.x, frame.runner.y, [2.6, 5.2])));
    wall(frame.leftJamb.x, frame.leftJamb.y, frame.leftJamb.z, houseGroup, seatZ - frame.leftJamb.z);
    wall(rightJambX, frame.rightJamb.y, frame.rightJamb.z, houseGroup, seatZ - frame.rightJamb.z);
    frame.seats.forEach((seat, side) => {
      const mesh = card(seat.x, seat.y, seat.z, material(rows[(i + side) % 2]!.texture));
      const hand = seat.x[0] < 0 ? -1 : 1;
      mesh.userData.role = "stall-seat";
      mesh.userData.side = hand;
      mesh.userData.restX = mesh.position.x;
      stallSeats.push(mesh);
      houseGroup.add(mesh);
    });
  }

  // ---- Proscenium facade (0) with side boxes, pediment and cartouche ---
  const facade = paintAsset("house.facade", 2048, 1244, (c) => {
    const W = 2048;
    const H = 1244;
    const px = (x: number) => ((x + 13) / 26) * W;
    const py = (y: number) => ((12.6 - y) / 15.8) * H;
    const g = c.createLinearGradient(0, 0, 0, H);
    g.addColorStop(0, "#f1e2bb");
    g.addColorStop(1, "#d9c18d");
    c.fillStyle = g;
    c.fillRect(0, 0, W, H);
    hatch(c, 0, 0, W, H, 11, 0.1);
    // Pediment band, garlands and title cartouche.
    inked(c, () => c.rect(0, 0, W, py(9.25)), "#e8d3a2", 4);
    c.strokeStyle = GOLD_DARK;
    c.lineWidth = 8;
    for (const s of [-1, 1]) {
      for (let k = 0; k < 3; k++) {
        c.beginPath();
        const x0 = px(s * (3.6 + k * 2.6));
        c.moveTo(x0, py(11.3));
        c.quadraticCurveTo(x0 + s * 100, py(10.0), x0 + s * 200, py(11.3));
        c.stroke();
      }
    }
    inked(c, () => c.ellipse(px(0), py(10.75), px(3.4) - px(0), py(10.75) - py(12.05), 0, 0, Math.PI * 2), GOLD, 6);
    inked(c, () => c.ellipse(px(0), py(10.75), px(3.0) - px(0), py(10.75) - py(11.85), 0, 0, Math.PI * 2), "#2f5d50", 4);
    plaqueText(c, "PAPER THEATER", px(0), py(11.12), 58, "#f3e6c4", "bold");
    plaqueText(c, "· ROYAL JUVENILE DRAMA ·", px(0), py(10.58), 22, GOLD);
    // Side towers: pilasters and two tiers of boxes with paper patrons.
    for (const s of [-1, 1]) {
      const inner = s * 8.2;
      const outer = s * 13;
      const left = Math.min(px(inner), px(outer));
      const width = Math.abs(px(outer) - px(inner));
      for (const [top, bottom] of [[8.4, 5.0], [4.0, 0.6]] as const) {
        const bx = left + width * 0.14;
        const bw = width * 0.72;
        inked(c, () => {
          c.moveTo(bx, py(bottom));
          c.lineTo(bx, py(top - 0.9));
          c.quadraticCurveTo(bx + bw / 2, py(top + 0.3), bx + bw, py(top - 0.9));
          c.lineTo(bx + bw, py(bottom));
          c.closePath();
        }, "#2a1210", 5);
        const r = rng(top * 10 + s + 20);
        for (let k = 0; k < 3; k++) patron(c, bx + bw * (0.2 + k * 0.3), py(bottom + 1.0), 0.95 + r() * 0.2, Math.floor(r() * 4));
        inked(c, () => c.rect(bx - 10, py(bottom + 0.55), bw + 20, py(bottom) - py(bottom + 0.55)), GOLD, 4);
        c.fillStyle = CRIMSON;
        for (let k = 0; k < 4; k++) {
          c.beginPath();
          c.arc(bx + (k + 0.5) * (bw / 4), py(top - 0.85), bw / 8, 0, Math.PI);
          c.fill();
        }
      }
      for (const x of [inner + s * 0.25, outer - s * 0.35]) {
        inked(c, () => c.rect(px(x) - 22, py(9.2), 44, py(-3.2) - py(9.2)), "#e9d6a6", 4);
        c.strokeStyle = GOLD_DARK;
        c.lineWidth = 3;
        for (const dx of [-10, 0, 10]) {
          c.beginPath();
          c.moveTo(px(x) + dx, py(9.0));
          c.lineTo(px(x) + dx, py(-3.0));
          c.stroke();
        }
      }
    }
    // Gilt molding and scalloped valance around the opening (above y=9).
    c.lineWidth = 22;
    c.strokeStyle = GOLD;
    c.strokeRect(px(-8.2), py(9.25), px(8.2) - px(-8.2), py(-3.2) - py(9.25));
    c.fillStyle = CRIMSON;
    for (let x = -8.2; x < 8.2; x += 0.8) {
      c.beginPath();
      c.arc(px(x + 0.4), py(9.25), (px(0.4) - px(0)), 0, Math.PI);
      c.fill();
    }
    c.save();
    c.globalCompositeOperation = "destination-out";
    c.fillRect(px(-8.2), py(9.05), px(8.2) - px(-8.2), H);
    c.restore();
  }, {
    overlay: (c, w, h) => {
      const px = (x: number) => ((x + 13) / 26) * w;
      const py = (y: number) => ((12.6 - y) / 15.8) * h;
      plaqueText(c, "PAPER THEATER", px(0), py(11.12), 58 * (w / 2048), "#f3e6c4", "bold");
      plaqueText(c, "· ROYAL JUVENILE DRAMA ·", px(0), py(10.58), 22 * (w / 2048), GOLD);
    },
  });
  houseGroup.add(card([-13, 13], [-3.2, 12.6], PROSCENIUM_Z, material(facade.texture)));
  const houseLeft = -30;
  const surroundZ = PROSCENIUM_Z - 0.05;
  wall([houseLeft, -8.3], [-3.2, 24], surroundZ, houseGroup, seatZ - surroundZ);
  wall([8.3, passageRight], [-3.2, 24], surroundZ, houseGroup, seatZ - surroundZ);
  wall([-8.3, 8.3], [9.2, 24], surroundZ, houseGroup, seatZ - surroundZ);

  const footlights = paintAsset("house.footlights", 1024, 96, (c) => {
    inked(c, () => c.rect(0, 52, 1024, 44), "#3d2414", 3);
    for (let k = 0; k < 9; k++) {
      const x = 56 + k * 114;
      inked(c, () => {
        c.moveTo(x - 30, 70);
        c.quadraticCurveTo(x, 30, x + 30, 70);
        c.closePath();
      }, BRASS, 3);
    }
  });
  houseGroup.add(card([-8.2, 8.2], [-0.62, 0.0], PROSCENIUM_Z + 0.1, track(new THREE.MeshLambertMaterial({ map: footlights.texture, transparent: true, depthWrite: false }))));
  const footGlow = paint(1024, 256, (c) => {
    for (let k = 0; k < 9; k++) {
      const x = 56 + k * 114;
      const g = c.createRadialGradient(x, 170, 4, x, 170, 90);
      g.addColorStop(0, "rgba(255,226,160,0.95)");
      g.addColorStop(1, "rgba(255,190,90,0)");
      c.fillStyle = g;
      c.fillRect(x - 100, 40, 200, 200);
    }
  });
  const glowMat = track(new THREE.MeshBasicMaterial({
    map: footGlow.texture,
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    opacity: 0.85,
  }));
  const footHalo = card([-8.2, 8.2], [-0.2, 1.35], PROSCENIUM_Z + 0.18, glowMat, false);
  footHalo.userData.role = "footlight-glow";
  houseGroup.add(footHalo);
  const apron = paintAsset("house.apron", 1024, 192, (c) => {
    c.fillStyle = "#3a2112";
    c.fillRect(0, 0, 1024, 192);
    for (let x = 0; x < 1024; x += 64) {
      c.strokeStyle = "rgba(0,0,0,0.4)";
      c.strokeRect(x, 0, 64, 192);
    }
  });
  houseGroup.add(card([-8.2, 8.2], [-3.2, -0.62], PROSCENIUM_Z + 0.05, track(new THREE.MeshBasicMaterial({ map: apron.texture }))));
  const pitFloor = card(
    PIT_FLOOR.x,
    PIT_FLOOR.y,
    PIT_FLOOR.z,
    bandMat(carpet.texture, PIT_FLOOR.x, PIT_FLOOR.y, [3.2, 3.2]),
  );
  pitFloor.userData.role = "pit-floor";
  houseGroup.add(pitFloor);

  // Orchestra pit (+0.8): railing either side of the read-along lectern.
  const rail = paintAsset("house.pit-rail", 512, 96, (c) => {
    c.fillStyle = BRASS;
    c.fillRect(0, 0, 512, 14);
    c.fillRect(0, 82, 512, 14);
    for (let x = 16; x < 512; x += 48) inked(c, () => c.ellipse(x, 48, 10, 34, 0, 0, Math.PI * 2), GOLD, 2);
  }, { repeat: [1, 1] });
  const lecternZ = LECTERN.z;
  for (const s of [-1, 1]) {
    const t = track(rail.texture.clone());
    t.repeat.set(4, 1);
    t.needsUpdate = true;
    houseGroup.add(card(s < 0 ? [-16, LECTERN.x[0] - 0.15] : [LECTERN.x[1] + 0.15, 16], [-3.6, -2.75], lecternZ, track(new THREE.MeshLambertMaterial({ map: t, alphaTest: 0.5 }))));
  }
  const lectern = paintAsset("house.lectern", 1024, 448, (c) => {
    inked(c, () => c.roundRect(4, 4, 1016, 440, 26), GOLD, 5);
    inked(c, () => c.roundRect(22, 22, 980, 404, 18), "#17231f", 4);
    c.strokeStyle = GOLD_DARK;
    c.lineWidth = 3;
    c.strokeRect(36, 36, 952, 376);
    for (const [x, y] of [[36, 36], [988, 36], [36, 412], [988, 412]] as const) inked(c, () => c.arc(x, y, 12, 0, Math.PI * 2), GOLD, 2);
  });
  const lecternMesh = card(LECTERN.x, LECTERN.y, lecternZ, material(lectern.texture));
  lecternMesh.userData.role = "lectern";
  seatGroup.add(lecternMesh);
  let lift = 0;
  let liftFrom = 0;
  let liftTo = 0;
  let liftAt: number | null = null;
  const stepReadAlong = (motion?: { now: number; reduced: boolean }): number => {
    const now = motion?.now ?? performance.now();
    const target = controlState.readAlong ? 1 : 0;
    if (liftAt === null) liftAt = now;
    if (motion?.reduced) {
      lift = target;
      liftFrom = target;
      liftTo = target;
      liftAt = now;
    } else if (target !== liftTo) {
      liftFrom = lift;
      liftTo = target;
      liftAt = now;
    }
    lift = lecternTravel(Math.max(0, now - liftAt), liftFrom, liftTo).lift;
    return lift;
  };

  // Front-row patrons on the proscenium side of the stalls, lower corners.
  const crowd = [0, 1].map((seed) =>
    paintAsset(seed ? "house.patrons-right" : "house.patrons-left", 1024, 320, (c) => {
      const r = rng(seed + 41);
      for (let k = 0; k < 5; k++) patron(c, 90 + k * 210 + r() * 40, 230, 1.8 + r() * 0.3, Math.floor(r() * 4));
    }),
  );
  crowd.forEach((p, i) => {
    const rect = PATRONS[i]!;
    seatGroup.add(card(rect.x, rect.y, rect.z, material(p.texture)));
  });

  // ---- Playback controls: lithographed cards hung on the proscenium -----
  // banner. They are world cards on the facade's own plane, just in front
  // of it, so they move with the stage; only the state painting changes.
  const bannerGroup = new THREE.Group();
  bannerGroup.userData.role = "banner-controls";
  bannerGroup.visible = false;
  seatGroup.add(bannerGroup);
  interface ControlRig {
    id: RailControlId;
    art: Painted;
    paper: Painted;
    /** Transparent paper margin around the art, in art pixels, for the halo. */
    pad: number;
    mesh: import("three").Mesh;
    mat: import("three").MeshLambertMaterial;
  }
  /** Where the blank ribbon sits in each lithograph, as fractions of its art. */
  const RIBBON: Record<RailControlId, { u: number; v: number; width: number }> = {
    play: { u: 0.5, v: 0.795, width: 0.4 },
    restart: { u: 0.5, v: 0.795, width: 0.4 },
    seek: { u: 0.5, v: 0.735, width: 0.28 },
    readalong: { u: 0.5, v: 0.795, width: 0.4 },
    return: { u: 0.5, v: 0.795, width: 0.4 },
  };
  /** Blank scroll the player typesets its label on. */
  const scroll = (c: Ctx, cx: number, cy: number, w: number, h: number): void => {
    const x0 = cx - w / 2;
    const x1 = cx + w / 2;
    for (const side of [-1, 1] as const) {
      const edge = side < 0 ? x0 : x1;
      inked(c, () => {
        c.moveTo(edge, cy - h * 0.3);
        c.lineTo(edge + side * h * 0.95, cy - h * 0.05);
        c.lineTo(edge + side * h * 0.6, cy + h * 0.32);
        c.lineTo(edge + side * h * 0.95, cy + h * 0.7);
        c.lineTo(edge, cy + h * 0.5);
        c.closePath();
      }, "#d9c79b", 3);
    }
    inked(c, () => {
      c.moveTo(x0, cy - h / 2);
      c.quadraticCurveTo(cx, cy - h * 0.2, x1, cy - h / 2);
      c.lineTo(x1, cy + h / 2);
      c.quadraticCurveTo(cx, cy + h * 0.8, x0, cy + h / 2);
      c.closePath();
    }, PAPER, 3);
  };
  /** Round medallion: paper rim, gilt ring, coloured ground, emblem, blank ribbon. */
  const medallion = (c: Ctx, s: number, ground: string, emblem: (c: Ctx, s: number) => void): void => {
    const m = s / 2;
    inked(c, () => c.arc(m, m, s * 0.47, 0, Math.PI * 2), PAPER, 4);
    inked(c, () => c.arc(m, m, s * 0.4, 0, Math.PI * 2), GOLD, 3);
    inked(c, () => c.arc(m, m, s * 0.37, 0, Math.PI * 2), ground, 3);
    emblem(c, s);
    scroll(c, m, s * 0.795, s * 0.4, s * 0.1);
  };
  const playEmblem = (c: Ctx, s: number): void => {
    const p = (v: number) => v * s;
    inked(c, () => {
      c.moveTo(p(0.32), p(0.22));
      c.lineTo(p(0.74), p(0.44));
      c.lineTo(p(0.32), p(0.66));
      c.closePath();
    }, "#f3e6c4", 4);
    c.strokeStyle = GOLD_DARK;
    c.lineWidth = s * 0.012;
    c.beginPath();
    c.moveTo(p(0.36), p(0.3));
    c.lineTo(p(0.36), p(0.58));
    c.stroke();
  };
  const restartEmblem = (c: Ctx, s: number): void => {
    const m = s / 2;
    const r = s * 0.19;
    c.lineCap = "round";
    for (const [width, colour] of [[s * 0.085, INK], [s * 0.06, GOLD]] as const) {
      c.lineWidth = width;
      c.strokeStyle = colour;
      c.beginPath();
      c.arc(m, s * 0.47, r, Math.PI * 0.15, Math.PI * 1.8);
      c.stroke();
    }
    const tip = [m + Math.cos(Math.PI * 1.8) * r, s * 0.47 + Math.sin(Math.PI * 1.8) * r] as const;
    inked(c, () => {
      c.moveTo(tip[0] + s * 0.07, tip[1] + s * 0.04);
      c.lineTo(tip[0] - s * 0.06, tip[1] - s * 0.08);
      c.lineTo(tip[0] - s * 0.07, tip[1] + s * 0.09);
      c.closePath();
    }, GOLD, 3);
  };
  const bookEmblem = (c: Ctx, s: number): void => {
    const p = (v: number) => v * s;
    for (const side of [-1, 1] as const) {
      inked(c, () => {
        c.moveTo(p(0.5), p(0.28));
        c.quadraticCurveTo(p(0.5 + side * 0.12), p(0.24), p(0.5 + side * 0.24), p(0.3));
        c.lineTo(p(0.5 + side * 0.24), p(0.58));
        c.quadraticCurveTo(p(0.5 + side * 0.12), p(0.54), p(0.5), p(0.6));
        c.closePath();
      }, "#efe3c1", 3);
    }
    c.strokeStyle = INK;
    c.lineWidth = Math.max(1, s * 0.008);
    for (const side of [-1, 1] as const) {
      for (let k = 0; k < 4; k++) {
        c.beginPath();
        c.moveTo(p(0.5 + side * 0.05), p(0.34 + k * 0.05));
        c.lineTo(p(0.5 + side * 0.2), p(0.32 + k * 0.05));
        c.stroke();
      }
    }
    inked(c, () => c.rect(p(0.47), p(0.28), p(0.06), p(0.34)), "#6b3f22", 2);
  };
  const ticketEmblem = (c: Ctx, s: number): void => {
    const p = (v: number) => v * s;
    inked(c, () => c.roundRect(p(0.22), p(0.28), p(0.56), p(0.36), p(0.03)), "#f4ead0", 4);
    c.strokeStyle = CRIMSON;
    c.lineWidth = Math.max(1, s * 0.008);
    c.strokeRect(p(0.4), p(0.32), p(0.34), p(0.28));
    c.strokeStyle = INK;
    c.setLineDash([s * 0.012, s * 0.01]);
    c.beginPath();
    c.moveTo(p(0.36), p(0.3));
    c.lineTo(p(0.36), p(0.62));
    c.stroke();
    c.setLineDash([]);
  };
  /** Gauge: paper plaque, gilt rule, green field, engraved scale, blank ribbon. */
  const gaugeFallback = (c: Ctx, w: number, h: number): void => {
    inked(c, () => c.roundRect(3, 3, w - 6, h - 6, h * 0.22), PAPER, 4);
    inked(c, () => c.roundRect(w * 0.025, h * 0.08, w * 0.95, h * 0.84, h * 0.16), GOLD, 3);
    inked(c, () => c.roundRect(w * 0.04, h * 0.14, w * 0.92, h * 0.72, h * 0.12), "#2f5a3e", 3);
    inked(c, () => c.rect(w * 0.13, h * 0.32, w * 0.74, h * 0.25), "#eadcb2", 3);
    c.fillStyle = INK;
    for (let k = 0; k <= 20; k++) c.fillRect(w * (BANNER_SEEK_TRACK[0] + (BANNER_SEEK_TRACK[1] - BANNER_SEEK_TRACK[0]) * (k / 20)) - 1, h * 0.34, 2, h * (k % 5 ? 0.07 : 0.12));
    scroll(c, w / 2, h * 0.735, w * 0.28, h * 0.19);
  };
  const FALLBACK: Record<RailControlId, (c: Ctx, w: number, h: number) => void> = {
    play: (c, w) => medallion(c, w, "#2f5a3e", playEmblem),
    restart: (c, w) => medallion(c, w, CRIMSON, restartEmblem),
    seek: gaugeFallback,
    readalong: (c, w) => medallion(c, w, "#1f3f73", bookEmblem),
    return: (c, w) => medallion(c, w, "#7a5a2a", ticketEmblem),
  };
  const CONTROL_PAD = 0.06;
  const controls: ControlRig[] = bannerLayout().map(({ id, rect }) => {
    const [dw, dh] = HOUSE_ART_BRIEF[`banner.${id}`]!.deliver_px;
    const art = paintAsset(`banner.${id}`, dw, dh, (c) => FALLBACK[id](c, dw, dh));
    const aw = art.canvas.width;
    const ah = art.canvas.height;
    const pad = Math.round(Math.min(aw, ah) * CONTROL_PAD);
    const paper = paint(aw + pad * 2, ah + pad * 2, () => {});
    const worldW = (rect.x[1] - rect.x[0]) * paper.canvas.width / aw;
    const worldH = (rect.y[1] - rect.y[0]) * paper.canvas.height / ah;
    const mat = track(new THREE.MeshLambertMaterial({
      map: paper.texture,
      emissive: 0xffffff,
      emissiveMap: paper.texture,
      emissiveIntensity: 0.28,
      transparent: true,
      alphaTest: 0.02,
      depthWrite: false,
    }));
    const mesh = pick(new THREE.Mesh(track(new THREE.PlaneGeometry(worldW, worldH)), mat), `rail:${id}`);
    mesh.position.set((rect.x[0] + rect.x[1]) / 2, (rect.y[0] + rect.y[1]) / 2, BANNER_Z);
    mesh.renderOrder = 20;
    mesh.castShadow = true;
    mesh.customDepthMaterial = track(new THREE.MeshDepthMaterial({
      depthPacking: THREE.RGBADepthPacking,
      map: paper.texture,
      alphaTest: 0.5,
    }));
    bannerGroup.add(mesh);
    return { id, art, paper, pad, mesh, mat };
  });
  const seekRig = controls.find((r) => r.id === "seek")!;
  // Pick fractions across the plane that land on the gauge's engraved scale.
  seekRig.mesh.userData.track = BANNER_SEEK_TRACK.map(
    (t) => (seekRig.pad + t * seekRig.art.canvas.width) / seekRig.paper.canvas.width,
  );
  /** Banner cards at full size; a narrow window scales them in `placeBanner`. */
  let seekRect = bannerLayout().find((c) => c.id === "seek")!.rect;
  const SEEK_STRIP_V = 0.447;
  const needlePaper = paint(32, 128, (c) => {
    inked(c, () => {
      c.moveTo(16, 6);
      c.lineTo(29, 2);
      c.lineTo(3, 2);
      c.closePath();
    }, CRIMSON, 2);
    inked(c, () => c.rect(12, 6, 8, 118), CRIMSON, 3);
    c.fillStyle = "rgba(255,230,190,0.6)";
    c.fillRect(14, 10, 2, 108);
  });
  const needleMat = track(new THREE.MeshBasicMaterial({ map: needlePaper.texture, transparent: true, depthWrite: false }));
  const needleHeight = (seekRect.y[1] - seekRect.y[0]) * 0.36;
  let bannerScaleNow = 1;
  let bannerViewport = { width: 0, height: 0 };
  const needle = new THREE.Mesh(track(new THREE.PlaneGeometry(needleHeight * 0.25, needleHeight)), needleMat);
  needle.userData.role = "seek-needle";
  needle.renderOrder = 21;
  needle.visible = false;
  needle.position.set(seekRect.x[0], seekRect.y[1] - SEEK_STRIP_V * (seekRect.y[1] - seekRect.y[0]), BANNER_Z + 0.03);
  bannerGroup.add(needle);
  let controlState: ControlsView = { enabled: false, playing: false, readAlong: false, progress: 0, fullscreen: false };
  let highlight: string | null = null;
  const drawControl = (rig: ControlRig): void => {
    const s = controlState;
    const lit = highlight === `rail:${rig.id}`;
    const live = rig.id === "readalong" || rig.id === "return" || s.enabled || (s.rehearsal === true && rig.id === "play");
    const on = rig.id === "play" ? s.playing : rig.id === "readalong" ? s.readAlong : false;
    const label = {
      play: s.playing ? "PAUSE" : "PLAY",
      restart: "RESTART",
      seek: "SEEK",
      readalong: "READ-ALONG",
      return: "BOX OFFICE",
    }[rig.id];
    const { pad } = rig;
    const aw = rig.art.canvas.width;
    const ah = rig.art.canvas.height;
    const ribbon = RIBBON[rig.id];
    repaint(rig.paper, (c) => {
      c.drawImage(rig.art.canvas, pad, pad);
      c.fillStyle = INK;
      c.textAlign = "center";
      c.textBaseline = "middle";
      let px = Math.round(aw * 0.07);
      do c.font = `bold ${px}px ${SERIF}`; while (c.measureText(label).width > ribbon.width * aw && --px > 10);
      c.fillText(label, pad + ribbon.u * aw, pad + ribbon.v * ah);
      if (rig.id === "play" || rig.id === "readalong") {
        // Pilot jewel set in the gilt ring: lit while the control is on.
        const jx = pad + 0.815 * aw;
        const jy = pad + 0.185 * ah;
        const jr = aw * 0.042;
        inked(c, () => c.arc(jx, jy, jr * 1.4, 0, Math.PI * 2), GOLD, 3);
        const jewel = c.createRadialGradient(jx - jr * 0.3, jy - jr * 0.3, jr * 0.1, jx, jy, jr);
        jewel.addColorStop(0, on ? "#ffc2a0" : "#7a3a30");
        jewel.addColorStop(1, on ? "#d8332f" : "#2a0d0d");
        c.beginPath();
        c.arc(jx, jy, jr, 0, Math.PI * 2);
        c.fillStyle = jewel;
        c.fill();
      }
      if (!live) {
        // A card that cannot act yet is tinted down, not repainted.
        c.globalCompositeOperation = "source-atop";
        c.fillStyle = "rgba(46,34,22,0.55)";
        c.fillRect(0, 0, rig.paper.canvas.width, rig.paper.canvas.height);
        c.globalCompositeOperation = "source-over";
      }
      if (lit) {
        // Halo behind the card, drawn from its own die-cut silhouette.
        c.save();
        c.globalCompositeOperation = "destination-over";
        c.shadowColor = "rgba(255,226,150,0.95)";
        c.shadowBlur = pad * 0.9;
        c.drawImage(rig.art.canvas, pad, pad);
        c.drawImage(rig.art.canvas, pad, pad);
        c.restore();
      }
    });
  };
  const bannerMats = [...controls.map((rig) => rig.mat), needleMat];
  const placeNeedle = (): void => {
    const progress = Number.isFinite(controlState.progress) ? Math.max(0, Math.min(1, controlState.progress)) : 0;
    const along = BANNER_SEEK_TRACK[0] + (BANNER_SEEK_TRACK[1] - BANNER_SEEK_TRACK[0]) * progress;
    needle.position.set(
      seekRect.x[0] + along * (seekRect.x[1] - seekRect.x[0]),
      seekRect.y[1] - SEEK_STRIP_V * (seekRect.y[1] - seekRect.y[0]),
      BANNER_Z + 0.03,
    );
    needle.scale.set(bannerScaleNow, bannerScaleNow, 1);
    needle.visible = controlState.enabled;
  };
  /** A tall window crops the banner: the cards shrink toward the cartouche to stay on screen. */
  const placeBanner = (scale: number): void => {
    bannerScaleNow = scale;
    for (const { id, rect } of bannerLayout(scale)) {
      const rig = controls.find((r) => r.id === id)!;
      rig.mesh.position.set((rect.x[0] + rect.x[1]) / 2, (rect.y[0] + rect.y[1]) / 2, BANNER_Z);
      rig.mesh.scale.set(scale, scale, 1);
      if (id === "seek") seekRect = rect;
    }
    placeNeedle();
  };
  for (const rig of controls) drawControl(rig);

  // Velvet curtain for the frozen curtain panels (slide geometry unchanged).
  const velvet = paintAsset("house.curtain", 512, 576, (c) => {
    const g = c.createLinearGradient(0, 0, 512, 0);
    for (let i = 0; i <= 12; i++) g.addColorStop(i / 12, i % 2 ? "#4e0b10" : "#a3242a");
    c.fillStyle = g;
    c.fillRect(0, 0, 512, 576);
    const shade = c.createLinearGradient(0, 0, 0, 576);
    shade.addColorStop(0, "rgba(0,0,0,0.35)");
    shade.addColorStop(0.2, "rgba(0,0,0,0)");
    shade.addColorStop(1, "rgba(0,0,0,0.25)");
    c.fillStyle = shade;
    c.fillRect(0, 0, 512, 576);
    c.fillStyle = GOLD;
    c.fillRect(0, 540, 512, 10);
    for (let x = 4; x < 512; x += 10) c.fillRect(x, 550, 4, 22);
    inked(c, () => c.rect(0, 0, 512, 22), GOLD_DARK, 2);
  });
  velvet.texture.wrapS = THREE.RepeatWrapping;
  velvet.texture.wrapS = THREE.RepeatWrapping;

  let inspectView: InspectView | null = null;
  let ticketList: TicketView[] = [];
  let noticeText: string | null = null;
  const redrawTickets = (): void => {
    tickets.forEach((rig, i) => {
      const t = ticketList[i] ?? null;
      rig.view = t;
      rig.mesh.visible = t !== null && inspectView === null;
      rig.shadow.visible = rig.mesh.visible;
      rig.sign.visible = rig.mesh.visible && !t!.ready;
      rig.mesh.userData.pick = t ? `ticket:${t.id}` : "";
      if (!t) return;
      const ticketAsset = houseArt?.assets.get("lobby.ticket");
      const ticketImg = ticketAsset ? (ticketAsset as PreparedHouseAsset).image : undefined;
      repaint(rig.paper, (c) => {
        const w = rig.paper.canvas.width;
        const h = rig.paper.canvas.height;
        if (ticketImg) {
          c.drawImage(ticketImg, 0, 0, w, h);
          drawTicketLabels(c, t, w, h, false);
        } else {
          drawTicket(c, t, false);
        }
      });
      const slot = TICKET_SLOTS[i]!;
      rig.mesh.position.y = (slot.y[0] + slot.y[1]) / 2;
      rig.mesh.rotation.z = 0;
      rig.shadow.position.set(
        (slot.x[0] + slot.x[1]) / 2 + 0.08 * BOOTH_SCALE,
        (slot.y[0] + slot.y[1]) / 2 - 0.06 * BOOTH_SCALE,
        slot.z - 0.03 * BOOTH_SCALE,
      );
      rig.shadow.scale.setScalar(1);
    });
    noticeMesh.visible = noticeText !== null;
    if (noticeText) {
      const text = noticeText;
      repaint(notice, (c) => {
        brassPlate(c, 1024, 128, highlight === "notice", false);
        plaqueText(c, text, 512, 66, 34, INK, "bold");
      });
    }
  };
  const redrawInspect = (): void => {
    inspectGroup.visible = inspectView !== null;
    for (const r of tickets) {
      r.mesh.visible = inspectView === null && r.view !== null;
      r.shadow.visible = r.mesh.visible;
      r.sign.visible = r.mesh.visible && r.view !== null && !r.view.ready;
    }
    backBtn.material = backMats[highlight === "inspect:back" ? 1 : 0]!;
    seatBtn.material = seatMats[highlight === "inspect:seat" ? 1 : 0]!;
    if (!inspectView) return;
    const v = inspectView;
    repaint(inspectPaper, (c) => {
      inked(c, () => c.roundRect(6, 6, 1012, 618, 20), PAPER, 5);
      c.strokeStyle = CRIMSON;
      c.lineWidth = 3;
      c.strokeRect(28, 28, 968, 574);
      hatch(c, 28, 28, 968, 574, 14, 0.06);
      c.textAlign = "center";
      c.fillStyle = INK;
      c.font = `bold 44px ${SERIF}`;
      wrap(c, v.title, 512, 80, 900, 50, 1);
      plaqueText(c, v.subtitle, 512, 138, 28, "#5a4630");
      c.font = `26px ${SERIF}`;
      c.fillStyle = INK;
      wrap(c, v.synopsis, 512, 196, 880, 34, 2);
      c.font = `italic 25px ${SERIF}`;
      c.fillStyle = "#3d2a18";
      wrap(c, `“${v.excerpt}”`, 512, 318, 860, 33, 3);
      plaqueText(c, v.readiness, 512, 432, 27, v.seatLabel ? "#2f5d50" : CRIMSON, "bold");
    });
  };

  // Spine on the left. Subtracting the hinge leaves the rest pose unchanged.
  for (const child of lobbyGroup.children) {
    child.position.x -= PAGE_HINGE_X;
    child.position.z -= PAGE_FACE_Z;
  }
  lobbyGroup.position.set(PAGE_HINGE_X, 0, PAGE_FACE_Z);
  lobbyGroup.userData.hinge = "page";

  return {
    nodes,
    resources,
    pickables,
    curtainMap: velvet.texture,
    setTickets(list, notice) {
      ticketList = list.slice(0, TICKET_SLOTS.length);
      noticeText = notice;
      redrawTickets();
    },
    setInspect(cardView) {
      inspectView = cardView;
      redrawInspect();
    },
    setControls(state) {
      const prev = controlState;
      controlState = { ...state };
      // Progress only moves the needle; textures repaint on state changes.
      if (prev.enabled !== state.enabled || prev.playing !== state.playing ||
        prev.readAlong !== state.readAlong || prev.fullscreen !== state.fullscreen ||
        prev.rehearsal !== state.rehearsal) {
        for (const rig of controls) drawControl(rig);
      }
      placeNeedle();
    },
    ticketAnchor() {
      if (!highlight?.startsWith("ticket:")) return null;
      const index = tickets.findIndex((rig) => rig.mesh.userData.pick === highlight && rig.mesh.visible);
      return index < 0 ? null : ticketCentre(index);
    },
    setHoverZoom(amount, at) {
      const level = Math.max(0, Math.min(1, amount));
      // Lambert divides by π. This pool has to outshine the booth lamps
      // so the wall shadow is a dark shape inside a bright patch.
      ticketSpot.intensity = level * 1.8 * Math.PI;
      ticketSpot.visible = level > 0.02;
      ticketSpot.castShadow = ticketSpot.visible;
      if (at) aimTicketSpotAt(at[0], at[1], at[2]);
    },
    setGlow(amount) {
      const level = Math.max(0, Math.min(1, amount));
      glowMat.opacity = 0.85 * level;
    },
    lecternLift() {
      return lift;
    },
    syncReadAlong(motion) {
      return stepReadAlong(motion);
    },
    setHighlight(id) {
      if (id === highlight) return;
      const prev = highlight;
      highlight = id;
      if (id?.startsWith("ticket:")) {
        const index = tickets.findIndex((rig) => rig.mesh.userData.pick === id && rig.mesh.visible);
        if (index >= 0) aimTicketSpot(index);
      }
      for (const key of [prev, id]) {
        if (!key) continue;
        if (key.startsWith("rail:")) {
          const rig = controls.find((r) => `rail:${r.id}` === key);
          if (rig) drawControl(rig);
        } else if (key === "notice") {
          redrawTickets();
        } else if (key.startsWith("inspect:")) {
          redrawInspect();
        }
      }
    },
    update(journey, width, height, growthSteps, camera, motion) {
      const turn = pageTurn(journey);
      lobbyGroup.rotation.y = turn;
      const fade = Math.max(0, Math.min(1, (turn - Math.PI * 0.45) / (Math.PI * 0.1)));
      const shown = 1 - fade;
      lobbyGroup.visible = shown > 0;
      lobbyGroup.traverse((obj) => {
        const mesh = obj as import("three").Mesh;
        const mat = mesh.material as import("three").Material | import("three").Material[] | undefined;
        if (!mat || Array.isArray(mat)) return;
        const basic = mat as import("three").MeshBasicMaterial & { userData: { pageFade?: { transparent: boolean; depthWrite: boolean; opacity: number } } };
        if (typeof basic.opacity !== "number") return;
        // Ticket shadows and other cutouts already rely on transparency.
        // Remember that, and only multiply it by the page fade.
        if (!basic.userData.pageFade) {
          basic.userData.pageFade = {
            transparent: basic.transparent,
            depthWrite: basic.depthWrite,
            opacity: basic.opacity,
          };
        }
        const base = basic.userData.pageFade;
        basic.opacity = base.opacity * shown;
        basic.transparent = shown < 1 || base.transparent;
        basic.depthWrite = shown > 0.99 && base.depthWrite;
      });
      inspectGroup.visible = journey < 0.25 && inspectView !== null;
      seatGroup.visible = true;
      stepReadAlong(motion);
      for (const mesh of stallSeats) {
        const side = mesh.userData.side as -1 | 1;
        mesh.position.x = (mesh.userData.restX as number) + stallShift(side, lift);
      }
      const lw = lecternWorld(growthSteps, lift);
      lecternMesh.scale.y = (lw.y[1] - lw.y[0]) / (LECTERN.y[1] - LECTERN.y[0]);
      lecternMesh.position.y = (lw.y[0] + lw.y[1]) / 2;
      const arriveT = Math.max(0, Math.min(1, (journey - 0.78) / 0.22));
      const arrive = arriveT * arriveT * (3 - 2 * arriveT);
      if (width !== bannerViewport.width || height !== bannerViewport.height) {
        bannerViewport = { width, height };
        const scale = bannerScale(bannerViewport);
        if (scale !== bannerScaleNow) placeBanner(scale);
      }
      bannerGroup.visible = arrive > 0.04;
      for (const mat of bannerMats) mat.opacity = arrive;
    },
    textureBytes() {
      return textures.reduce((sum, t) => sum + t.w * t.h * 4, 0);
    },
  };
}
