// House-art manifest record types, authoritative brief geometry, and mid-grey
// keying / aperture processing for immersive house surfaces.
import type { Digest } from "./types.js";
import type { HouseArtAlpha, HouseArtBriefAsset } from "./house-brief.js";
import { HOUSE_ART_BRIEF } from "./house-brief.js";

export type { HouseArtAlpha, HouseArtBriefAsset };
export { HOUSE_ART_BRIEF };

/** Proposed house-art manifest record shape per asset. */
export interface HouseArtAssetRecord {
  readonly id?: string;
  readonly url: string;
  readonly sha256: Digest;
  readonly deliver_px?: readonly [number, number];
  readonly alpha?: HouseArtAlpha;
  readonly nine_slice_px?: readonly [number, number, number, number] | null;
  readonly regions?: Readonly<Record<string, unknown>> | null;
}

/** Proposed house-art manifest shape. */
export interface HouseArtManifest {
  readonly schema_version?: number;
  readonly brief?: string;
  readonly diagnostic?: boolean;
  readonly accepted?: boolean;
  readonly assets: Readonly<Record<string, HouseArtAssetRecord>> | readonly HouseArtAssetRecord[];
}

/** Verified loaded house art asset handle. */
export interface LoadedHouseAsset {
  readonly id: string;
  readonly url: string;
  readonly sha256: Digest;
  readonly deliver_px: readonly [number, number];
  readonly alpha: HouseArtAlpha;
  readonly nine_slice_px: readonly [number, number, number, number] | null;
  readonly regions: Readonly<Record<string, unknown>> | null;
  readonly bytes: Uint8Array;
}

/** Verified immutable loaded house art handle. */
export interface LoadedHouseArt {
  readonly manifest_digest?: Digest | undefined;
  readonly assets: ReadonlyMap<string, LoadedHouseAsset>;
}

/** Pre-decoded house asset ready for canvas/texture drawing. */
export interface PreparedHouseAsset extends LoadedHouseAsset {
  readonly image?: CanvasImageSource | undefined;
}

/** Pre-decoded house art set. */
export interface PreparedHouseArt {
  readonly manifest_digest?: Digest | undefined;
  readonly assets: ReadonlyMap<string, PreparedHouseAsset>;
}

/**
 * Pre-decodes house-art asset bytes into drawable images when in a browser environment.
 * Throws before any render if an image fails to decode.
 */
export async function prepareHouseArt(
  houseArt: LoadedHouseArt,
): Promise<PreparedHouseArt> {
  const prepared = new Map<string, PreparedHouseAsset>();
  for (const [id, asset] of houseArt.assets) {
    let image: CanvasImageSource | undefined;
    if (typeof createImageBitmap !== "undefined") {
      const blob = new Blob([asset.bytes as unknown as BlobPart], { type: "image/png" });
      try {
        image = await createImageBitmap(blob);
      } catch (err) {
        throw new Error(
          `house_art_unavailable: ${id} decode failed (${err instanceof Error ? err.message : String(err)})`,
        );
      }
    } else if (typeof Image !== "undefined" && typeof URL !== "undefined") {
      const blob = new Blob([asset.bytes as unknown as BlobPart], { type: "image/png" });
      const url = URL.createObjectURL(blob);
      const img = new Image();
      img.src = url;
      await new Promise<void>((resolve, reject) => {
        img.onload = () => resolve();
        img.onerror = (e) => reject(new Error(`house_art_unavailable: ${id} (${String(e)})`));
      });
      image = img;
    }
    prepared.set(id, {
      ...asset,
      image,
    });
  }
  return {
    manifest_digest: houseArt.manifest_digest,
    assets: prepared,
  };
}

export function isMidGrey(r: number, g: number, b: number, tolerance = 18, neutralMax = 12): boolean {
  return (
    Math.abs(r - 128) <= tolerance &&
    Math.abs(g - 128) <= tolerance &&
    Math.abs(b - 128) <= tolerance &&
    Math.max(r, g, b) - Math.min(r, g, b) <= neutralMax
  );
}

export function keyOutMidGrey(
  data: Uint8ClampedArray | Uint8Array,
  _width: number,
  _height: number,
  tolerance = 18,
  neutralMax = 12,
): void {
  for (let i = 0; i < data.length; i += 4) {
    const r = data[i]!;
    const g = data[i + 1]!;
    const b = data[i + 2]!;
    if (isMidGrey(r, g, b, tolerance, neutralMax)) {
      data[i + 3] = 0;
    }
  }
}

/** Image-space rectangle of a flat neutral opening, top-left origin. */
export interface ApertureUv {
  readonly u0: number;
  readonly v0: number;
  readonly u1: number;
  readonly v1: number;
}

function flatNeutral(r: number, g: number, b: number): boolean {
  const chroma = Math.max(r, g, b) - Math.min(r, g, b);
  const luma = (r + g + b) / 3;
  return chroma <= 16 && luma >= 120 && luma <= 200;
}

/**
 * Finds the largest flat neutral rectangle in an RGBA buffer. Generated booth
 * frames paint the window as a flat grey that does not match the brief's
 * nominal UV, so the opening has to be measured from the pixels.
 */
export function detectFlatAperture(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): ApertureUv | null {
  if (width < 8 || height < 8 || data.length < width * height * 4) return null;
  const step = Math.max(1, Math.floor(height / 240));
  let seedY = -1;
  let seedX0 = 0;
  let seedX1 = 0;
  let seedLen = 0;
  for (let y = 0; y < height; y += step) {
    let run = 0;
    let runStart = 0;
    let best = 0;
    let bestStart = 0;
    for (let x = 0; x <= width; x++) {
      let ok = false;
      if (x < width) {
        const i = (y * width + x) * 4;
        ok = flatNeutral(data[i]!, data[i + 1]!, data[i + 2]!);
      }
      if (ok) {
        if (run === 0) runStart = x;
        run++;
      } else if (run > 0) {
        if (run > best) {
          best = run;
          bestStart = runStart;
        }
        run = 0;
      }
    }
    if (best > seedLen) {
      seedLen = best;
      seedY = y;
      seedX0 = bestStart;
      seedX1 = bestStart + best;
    }
  }
  if (seedY < 0 || seedLen < width * 0.25) return null;
  const cover = (y: number): number => {
    let n = 0;
    let hit = 0;
    const stride = Math.max(1, Math.floor((seedX1 - seedX0) / 48));
    for (let x = seedX0; x < seedX1; x += stride) {
      n++;
      const i = (y * width + x) * 4;
      if (flatNeutral(data[i]!, data[i + 1]!, data[i + 2]!)) hit++;
    }
    return n === 0 ? 0 : hit / n;
  };
  let top = seedY;
  let bot = seedY;
  for (let y = seedY - 1; y >= 0; y--) {
    if (cover(y) < 0.72) break;
    top = y;
  }
  for (let y = seedY + 1; y < height; y++) {
    if (cover(y) < 0.72) break;
    bot = y;
  }
  const mid = (top + bot) >> 1;
  let x0 = seedX0;
  let x1 = seedX1;
  while (x0 > 0) {
    const i = (mid * width + (x0 - 1)) * 4;
    if (!flatNeutral(data[i]!, data[i + 1]!, data[i + 2]!)) break;
    x0--;
  }
  while (x1 < width) {
    const i = (mid * width + x1) * 4;
    if (!flatNeutral(data[i]!, data[i + 1]!, data[i + 2]!)) break;
    x1++;
  }
  if (x1 - x0 < width * 0.2 || bot - top < height * 0.12) return null;
  return { u0: x0 / width, v0: top / height, u1: x1 / width, v1: (bot + 1) / height };
}

/**
 * Clears the flat neutral region that holds the largest flat run, following
 * its outline. The proscenium's painted arch is narrower than the brief
 * rectangle, so a rectangle would erase the columns and the valance.
 */
export function cutConnectedFlat(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): ApertureUv | null {
  const seed = detectFlatAperture(data, width, height);
  if (!seed) return null;
  const n = width * height;
  const seen = new Uint8Array(n);
  const stack = new Int32Array(n);
  let sp = 0;
  const push = (x: number, y: number): void => {
    if (x < 0 || y < 0 || x >= width || y >= height) return;
    const p = y * width + x;
    if (seen[p]) return;
    const i = p * 4;
    if (!flatNeutral(data[i]!, data[i + 1]!, data[i + 2]!)) return;
    seen[p] = 1;
    stack[sp++] = p;
  };
  push(
    Math.min(width - 1, Math.max(0, Math.floor(((seed.u0 + seed.u1) / 2) * width))),
    Math.min(height - 1, Math.max(0, Math.floor(((seed.v0 + seed.v1) / 2) * height))),
  );
  if (sp === 0) return null;
  let x0 = width;
  let y0 = height;
  let x1 = 0;
  let y1 = 0;
  while (sp > 0) {
    const p = stack[--sp]!;
    data[p * 4 + 3] = 0;
    const x = p % width;
    const y = (p - x) / width;
    if (x < x0) x0 = x;
    if (y < y0) y0 = y;
    if (x > x1) x1 = x;
    if (y > y1) y1 = y;
    push(x - 1, y);
    push(x + 1, y);
    push(x, y - 1);
    push(x, y + 1);
  }
  return { u0: x0 / width, v0: y0 / height, u1: (x1 + 1) / width, v1: (y1 + 1) / height };
}

export interface NamePlate {
  /** Center of the blank plate, in texture UV. */
  u: number;
  v: number;
  /** Plate width and height, in texture UV. */
  uw: number;
  vh: number;
}

const platePale = (r: number, g: number, b: number, a: number): boolean =>
  a >= 200 && r >= 200 && g >= 155 && b >= 70 && b <= 175 && r > b + 40;

/**
 * The blank name-plate above a ticket window. A short pale lip sits under
 * the plate, so the first tall band above the window is the plate.
 */
export function measureNamePlate(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
  windowTopV: number,
): NamePlate | null {
  if (width < 8 || height < 8 || data.length < width * height * 4) return null;
  if (!(windowTopV > 0 && windowTopV < 1)) return null;
  const yStart = Math.min(height - 1, Math.floor(windowTopV * height) - 2);
  const yStop = Math.max(0, yStart - Math.floor(height * 0.22));
  const minSpan = width * 0.18;
  const hits: { y: number; x0: number; x1: number }[] = [];
  for (let y = yStart; y >= yStop; y--) {
    let x0 = width;
    let x1 = -1;
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      if (!platePale(data[i]!, data[i + 1]!, data[i + 2]!, data[i + 3] ?? 255)) continue;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
    }
    if (x1 - x0 + 1 >= minSpan) hits.push({ y, x0, x1: x1 + 1 });
  }
  interface Band { y0: number; y1: number; rows: { y: number; x0: number; x1: number }[] }
  const bands: Band[] = [];
  let band: Band | null = null;
  let prevY = -1;
  for (const row of hits) {
    if (!band || prevY - row.y > 16) {
      band = { y0: row.y, y1: row.y, rows: [row] };
      bands.push(band);
    } else {
      band.y1 = row.y;
      band.rows.push(row);
    }
    prevY = row.y;
  }
  const substantial = bands.find((candidate) => candidate.y0 - candidate.y1 + 1 >= height * 0.022);
  if (!substantial) return null;
  const x0 = Math.min(...substantial.rows.map((row) => row.x0));
  const x1 = Math.max(...substantial.rows.map((row) => row.x1));
  const plateH = substantial.y0 - substantial.y1 + 1;
  return {
    u: (x0 + x1) / 2 / width,
    v: (substantial.y0 + substantial.y1) / 2 / height,
    uw: (x1 - x0) / width,
    vh: plateH / height,
  };
}

function openingNeutral(r: number, g: number, b: number): boolean {
  const chroma = Math.max(r, g, b) - Math.min(r, g, b);
  const luma = (r + g + b) / 3;
  return chroma <= 18 && luma >= 100 && luma <= 170;
}

/**
 * The arch's painted opening, ignoring the flat grey that also rings the
 * sheet. A full-width border run touches the image edge, so it is skipped.
 * The seed is the widest interior run in the middle of the picture; the
 * span grows while that opening continues, and stops at the stone gap under
 * a separate keystone tablet.
 */
export function detectInteriorOpening(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): ApertureUv | null {
  if (width < 8 || height < 8 || data.length < width * height * 4) return null;
  const interiorRun = (y: number): { x0: number; x1: number } | null => {
    let best = 0;
    let bestStart = 0;
    let run = 0;
    let runStart = 0;
    for (let x = 0; x <= width; x++) {
      let ok = false;
      if (x < width) {
        const i = (y * width + x) * 4;
        ok = openingNeutral(data[i]!, data[i + 1]!, data[i + 2]!);
      }
      if (ok) {
        if (run === 0) runStart = x;
        run++;
      } else if (run > 0) {
        const touchesEdge = runStart === 0 || x >= width;
        if (!touchesEdge && run > best) {
          best = run;
          bestStart = runStart;
        }
        run = 0;
      }
    }
    if (best < width * 0.12) return null;
    return { x0: bestStart, x1: bestStart + best };
  };
  let seed: { x0: number; x1: number; y: number } | null = null;
  const step = Math.max(1, Math.floor(height / 180));
  for (let y = Math.floor(height * 0.35); y < Math.floor(height * 0.75); y += step) {
    const run = interiorRun(y);
    if (run && (!seed || run.x1 - run.x0 > seed.x1 - seed.x0)) seed = { ...run, y };
  }
  if (!seed) return null;
  const seedW = seed.x1 - seed.x0;
  const continues = (run: { x0: number; x1: number }): boolean => {
    const overlap = Math.min(run.x1, seed.x1) - Math.max(run.x0, seed.x0);
    return run.x1 - run.x0 >= seedW * 0.18 && overlap >= seedW * 0.4;
  };
  let top = seed.y;
  let bot = seed.y;
  for (let y = seed.y - 1; y >= 0; y--) {
    const run = interiorRun(y);
    if (!run || !continues(run)) break;
    top = y;
  }
  for (let y = seed.y + 1; y < height; y++) {
    const run = interiorRun(y);
    if (!run || !continues(run)) break;
    bot = y;
  }
  if (bot - top < height * 0.2) return null;
  let x0 = seed.x0;
  let x1 = seed.x1;
  for (let y = top; y <= bot; y += step) {
    const run = interiorRun(y);
    if (!run) continue;
    if (run.x0 < x0) x0 = run.x0;
    if (run.x1 > x1) x1 = run.x1;
  }
  return { u0: x0 / width, v0: top / height, u1: x1 / width, v1: (bot + 1) / height };
}

/** Near-white paper: the printed margin around a rectangular plate. */
function isPaperWhite(r: number, g: number, b: number): boolean {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  return (r + g + b) / 3 >= 236 && max - min <= 28;
}

/** How far a near-white margin runs in from each edge, as UV. Empty when the art is full-bleed. */
export function paperEdgeInset(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): ApertureUv {
  const full = { u0: 0, v0: 0, u1: 1, v1: 1 };
  if (width < 8 || height < 8 || data.length < width * height * 4) return full;
  const paperAt = (x: number, y: number): boolean => {
    const i = (y * width + x) * 4;
    return isPaperWhite(data[i]!, data[i + 1]!, data[i + 2]!);
  };
  const strideY = Math.max(1, Math.floor(height / 48));
  const strideX = Math.max(1, Math.floor(width / 48));
  const column = (x: number): boolean => {
    let n = 0;
    let hit = 0;
    for (let y = 0; y < height; y += strideY) {
      n++;
      if (paperAt(x, y)) hit++;
    }
    return n > 0 && hit / n >= 0.72;
  };
  const row = (y: number): boolean => {
    let n = 0;
    let hit = 0;
    for (let x = 0; x < width; x += strideX) {
      n++;
      if (paperAt(x, y)) hit++;
    }
    return n > 0 && hit / n >= 0.72;
  };
  const maxX = Math.floor(width * 0.12);
  const maxY = Math.floor(height * 0.12);
  let left = 0;
  while (left < maxX && column(left)) left++;
  let right = width - 1;
  while (width - 1 - right < maxX && right > left && column(right)) right--;
  let top = 0;
  while (top < maxY && row(top)) top++;
  let bottom = height - 1;
  while (height - 1 - bottom < maxY && bottom > top && row(bottom)) bottom--;
  return { u0: left / width, v0: top / height, u1: (right + 1) / width, v1: (bottom + 1) / height };
}

/**
 * Punches out the flat ground that touches the image border: mid-grey and
 * the cream paper rim. Interior colour that is not connected to that rim
 * stays, so a flood cannot eat a figure that merely shares a pale tint.
 */
export function keyOutBorderGround(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): void {
  if (width < 2 || height < 2 || data.length < width * height * 4) return;
  const ground = (p: number): boolean => {
    const i = p * 4;
    const r = data[i]!;
    const g = data[i + 1]!;
    const b = data[i + 2]!;
    if (isMidGrey(r, g, b, 36, 22)) return true;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    return (r + g + b) / 3 >= 198 && max - min <= 58;
  };
  const n = width * height;
  const seen = new Uint8Array(n);
  const stack = new Int32Array(n);
  let sp = 0;
  const push = (x: number, y: number): void => {
    const p = y * width + x;
    if (seen[p]) return;
    if (!ground(p)) return;
    seen[p] = 1;
    stack[sp++] = p;
  };
  for (let x = 0; x < width; x++) {
    push(x, 0);
    push(x, height - 1);
  }
  for (let y = 0; y < height; y++) {
    push(0, y);
    push(width - 1, y);
  }
  while (sp > 0) {
    const p = stack[--sp]!;
    data[p * 4 + 3] = 0;
    const x = p % width;
    const y = (p - x) / width;
    if (x > 0) push(x - 1, y);
    if (x + 1 < width) push(x + 1, y);
    if (y > 0) push(x, y - 1);
    if (y + 1 < height) push(x, y + 1);
  }
  // One pixel of pale fringe left against the cleared ground.
  const pale = (p: number): boolean => {
    const i = p * 4;
    if (data[i + 3] === 0) return false;
    const r = data[i]!;
    const g = data[i + 1]!;
    const b = data[i + 2]!;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    return (r + g + b) / 3 >= 185 && max - min <= 62;
  };
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const p = y * width + x;
      if (!seen[p]) continue;
      if (x > 0 && pale(p - 1)) data[(p - 1) * 4 + 3] = 0;
      if (x + 1 < width && pale(p + 1)) data[(p + 1) * 4 + 3] = 0;
      if (y > 0 && pale(p - width)) data[(p - width) * 4 + 3] = 0;
      if (y + 1 < height && pale(p + width)) data[(p + width) * 4 + 3] = 0;
    }
  }
}

/**
 * Removes the paper margin around a curtain painting and stretches the
 * velvet to the full panel, so the two halves meet without a pale frame.
 */
export function fillCurtainVelvet(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): boolean {
  if (width < 8 || height < 8 || data.length < width * height * 4) return false;
  keyOutBorderGround(data, width, height);
  let x0 = width;
  let y0 = height;
  let x1 = -1;
  let y1 = -1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (data[(y * width + x) * 4 + 3] === 0) continue;
      if (x < x0) x0 = x;
      if (y < y0) y0 = y;
      if (x > x1) x1 = x;
      if (y > y1) y1 = y;
    }
  }
  if (x1 < 0) return false;
  const bw = x1 - x0 + 1;
  const bh = y1 - y0 + 1;
  const inset = x0 > width * 0.01 || y0 > height * 0.01
    || width - 1 - x1 > width * 0.01 || height - 1 - y1 > height * 0.01;
  if (!inset) return true;
  const out = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y++) {
    const sy = y0 + Math.min(bh - 1, Math.floor(((y + 0.5) * bh) / height));
    for (let x = 0; x < width; x++) {
      const sx = x0 + Math.min(bw - 1, Math.floor(((x + 0.5) * bw) / width));
      const s = (sy * width + sx) * 4;
      const d = (y * width + x) * 4;
      out[d] = data[s]!;
      out[d + 1] = data[s + 1]!;
      out[d + 2] = data[s + 2]!;
      out[d + 3] = data[s + 3] === 0 ? 0 : 255;
    }
  }
  data.set(out);
  const n = width * height;
  for (let pass = 0; pass < 24; pass++) {
    const alpha = new Uint8Array(n);
    for (let p = 0; p < n; p++) alpha[p] = data[p * 4 + 3]!;
    let changed = false;
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const p = y * width + x;
        if (alpha[p] !== 0) continue;
        let r = 0;
        let g = 0;
        let b = 0;
        let count = 0;
        const take = (q: number): void => {
          if (alpha[q] === 0) return;
          r += data[q * 4]!;
          g += data[q * 4 + 1]!;
          b += data[q * 4 + 2]!;
          count++;
        };
        if (x > 0) take(p - 1);
        if (x + 1 < width) take(p + 1);
        if (y > 0) take(p - width);
        if (y + 1 < height) take(p + width);
        if (count === 0) continue;
        data[p * 4] = Math.round(r / count);
        data[p * 4 + 1] = Math.round(g / count);
        data[p * 4 + 2] = Math.round(b / count);
        data[p * 4 + 3] = 255;
        changed = true;
      }
    }
    if (!changed) break;
  }
  for (let p = 0; p < n; p++) if (data[p * 4 + 3] === 0) data[p * 4 + 3] = 255;
  return true;
}

/**
 * Clears the flat grey field that touches the image border. The cream
 * paper flashing around a die-cut standee stays, including its color.
 */
export function keyOutBorderGrey(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): void {
  if (width < 2 || height < 2 || data.length < width * height * 4) return;
  const grey = (p: number): boolean => {
    const i = p * 4;
    return isMidGrey(data[i]!, data[i + 1]!, data[i + 2]!, 36, 22);
  };
  const n = width * height;
  const seen = new Uint8Array(n);
  const stack = new Int32Array(n);
  let sp = 0;
  const push = (x: number, y: number): void => {
    if (x < 0 || y < 0 || x >= width || y >= height) return;
    const p = y * width + x;
    if (seen[p] || !grey(p)) return;
    seen[p] = 1;
    stack[sp++] = p;
  };
  for (let x = 0; x < width; x++) {
    push(x, 0);
    push(x, height - 1);
  }
  for (let y = 0; y < height; y++) {
    push(0, y);
    push(width - 1, y);
  }
  while (sp > 0) {
    const p = stack[--sp]!;
    data[p * 4 + 3] = 0;
    const x = p % width;
    const y = (p - x) / width;
    push(x - 1, y);
    push(x + 1, y);
    push(x, y - 1);
    push(x, y + 1);
  }
}

/**
 * Copies the ink color into cleared neighbors. Linear filtering would
 * otherwise blend the old paper color back onto the cut edge.
 */
export function extrudeCutoutColor(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): void {
  if (width < 2 || height < 2 || data.length < width * height * 4) return;
  const n = width * height;
  const rgb = new Uint8Array(n * 3);
  const alpha = new Uint8Array(n);
  for (let p = 0; p < n; p++) {
    rgb[p * 3] = data[p * 4]!;
    rgb[p * 3 + 1] = data[p * 4 + 1]!;
    rgb[p * 3 + 2] = data[p * 4 + 2]!;
    alpha[p] = data[p * 4 + 3]!;
  }
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const p = y * width + x;
      if (alpha[p] !== 0) continue;
      let r = 0;
      let g = 0;
      let b = 0;
      let count = 0;
      const take = (q: number): void => {
        if (alpha[q] === 0) return;
        r += rgb[q * 3]!;
        g += rgb[q * 3 + 1]!;
        b += rgb[q * 3 + 2]!;
        count++;
      };
      if (x > 0) take(p - 1);
      if (x + 1 < width) take(p + 1);
      if (y > 0) take(p - width);
      if (y + 1 < height) take(p + width);
      if (count === 0) continue;
      data[p * 4] = Math.round(r / count);
      data[p * 4 + 1] = Math.round(g / count);
      data[p * 4 + 2] = Math.round(b / count);
    }
  }
}

/** Axis-aligned world rectangle. y0 is the bottom edge. */
export interface WorldRect {
  readonly x0: number;
  readonly y0: number;
  readonly x1: number;
  readonly y1: number;
}

/** Uniform contain-fit that maps `content` into `frame`, centred. */
export function containFit(content: WorldRect, frame: WorldRect, pad = 0): {
  scale: number;
  cx: number;
  cy: number;
  contentCx: number;
  contentCy: number;
} {
  const bw = content.x1 - content.x0;
  const bh = content.y1 - content.y0;
  const ow = (frame.x1 - frame.x0) * (1 - pad * 2);
  const oh = (frame.y1 - frame.y0) * (1 - pad * 2);
  if (!(bw > 0 && bh > 0 && ow > 0 && oh > 0)) throw new Error("nonpositive fit");
  return {
    scale: Math.min(ow / bw, oh / bh),
    cx: (frame.x0 + frame.x1) / 2,
    cy: (frame.y0 + frame.y1) / 2,
    contentCx: (content.x0 + content.x1) / 2,
    contentCy: (content.y0 + content.y1) / 2,
  };
}

export function cutAperture(
  data: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
  u0: number,
  v0: number,
  u1: number,
  v1: number,
): void {
  const x0 = Math.max(0, Math.floor(u0 * width));
  const x1 = Math.min(width, Math.ceil(u1 * width));
  const y0 = Math.max(0, Math.floor(v0 * height));
  const y1 = Math.min(height, Math.ceil(v1 * height));
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const idx = (y * width + x) * 4;
      data[idx + 3] = 0;
    }
  }
}
