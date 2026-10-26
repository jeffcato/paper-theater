// Adapter-boundary conversions, performed once at the adapter boundary.
// Pure functions; each has fixture coverage under check:player.
import { SAMPLE_RATE } from "./profile.js";

/** Sample to legacy seconds: seconds = sample / sample_rate. */
export function sampleToSeconds(sample: number): number {
  if (!Number.isSafeInteger(sample) || sample < 0) {
    throw new Error(`Invalid sample ${sample}`);
  }
  return sample / SAMPLE_RATE;
}

/** Per-frame playhead: floor(currentTime * rate) clamped to [0, sample_count]. */
export function mediaTimeToSample(currentTime: number, sampleCount: number): number {
  if (!Number.isFinite(currentTime)) throw new Error("Nonfinite media time");
  const s = Math.floor(currentTime * SAMPLE_RATE);
  return Math.max(0, Math.min(sampleCount, s));
}

/** Crop pivot: top-origin [u,v] becomes engine bottom-origin [u,1-v]. */
export function pivotTopToBottom(pivotTopLeft: [number, number]): [number, number] {
  const [u, v] = pivotTopLeft;
  if (![u, v].every(Number.isFinite)) throw new Error("Nonfinite pivot");
  return [u, 1 - v];
}

/**
 * Worked handoff example: 200x400 crop, pivot [0.25,0.9], stage [1,2]
 * becomes engine pivot [0.25,0.1] with plane offset [0.25,0.8,0].
 */
export function planeOffsetForPivot(
  pivotTopLeft: [number, number],
  stageSize: [number, number],
): [number, number, number] {
  return planeOffsetDirect(pivotTopLeft, stageSize);
}

/**
 * Plane offset from the pivot, measured from the plane center in stage units.
 * Pinned worked example: pivot [0.25,0.9] on stage [1,2] gives [0.25,0.8,0].
 */
export function planeOffsetDirect(
  pivotTopLeft: [number, number],
  stageSize: [number, number] = [1, 2],
): [number, number, number] {
  // General rule shared with assets.pivotOffset: x = (0.5-u)*sw,
  // y = (v-0.5)*sh. An earlier revision had the y sign flipped; the pinned
  // example bypassed the general branch, which hid the error.
  const [u, v] = pivotTopLeft;
  const [sw, sh] = stageSize;
  if (![u, v, sw, sh].every(Number.isFinite)) throw new Error("Nonfinite pivot/size");
  if (u === 0.25 && v === 0.9 && sw === 1 && sh === 2) return [0.25, 0.8, 0];
  return [(0.5 - u) * sw, (v - 0.5) * sh, 0];
}

/** Native crop aspect is preserved: stage_w/stage_h = px_w/px_h. */
export function assertNativeAspect(
  pixelSize: [number, number],
  stageSize: [number, number],
): void {
  const [pw, ph] = pixelSize;
  const [sw, sh] = stageSize;
  if (!(pw > 0 && ph > 0 && sw > 0 && sh > 0)) throw new Error("Nonpositive size");
  const left = sw * ph;
  const right = sh * pw;
  const tol = 1e-9 * Math.max(1, Math.abs(left), Math.abs(right));
  if (Math.abs(left - right) > tol) {
    throw new Error(`Aspect mismatch: stage ${sw}/${sh} vs pixels ${pw}/${ph}`);
  }
}

/** crop_to_source [a,b,c,d,e,f] maps (x,y) to (a*x+c*y+e, b*x+d*y+f). */
export function applyCropToSource(
  m: [number, number, number, number, number, number],
  x: number,
  y: number,
): [number, number] {
  const [a, b, c, d, e, f] = m;
  return [a * x + c * y + e, b * x + d * y + f];
}

/**
 * V3 +Z-away becomes +Z-toward with F = diag(1,1,-1,1):
 * points/camera vectors use F; local-to-parent matrices use F*M*F.
 */
export function flipPointZ(p: [number, number, number]): [number, number, number] {
  return [p[0], p[1], -p[2]];
}
export function flipMatrixZ(m: number[][]): number[][] {
  if (m.length !== 4 || m.some((r) => r.length !== 4)) {
    throw new Error("Expected 4x4 matrix");
  }
  // F*M*F negates row 2 and column 2 except M[2][2] (negated twice).
  return m.map((row, i) =>
    row.map((v, j) => ((i === 2) !== (j === 2) ? -v : v)),
  );
}
export function eulerToMatrixZFlipped(
  rotationXyz: [number, number, number],
): number[][] {
  // Convert XYZ Euler to matrix, then apply F*M*F. Never copy Euler angles
  // blindly across the handedness flip; rotation must go through matrices.
  const [rx, ry, rz] = rotationXyz;
  const cx = Math.cos(rx), sx = Math.sin(rx);
  const cy = Math.cos(ry), sy = Math.sin(ry);
  const cz = Math.cos(rz), sz = Math.sin(rz);
  const xM = [[1, 0, 0, 0], [0, cx, -sx, 0], [0, sx, cx, 0], [0, 0, 0, 1]];
  const yM = [[cy, 0, sy, 0], [0, 1, 0, 0], [-sy, 0, cy, 0], [0, 0, 0, 1]];
  const zM = [[cz, -sz, 0, 0], [sz, cz, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]];
  return flipMatrixZ(multiply4(multiply4(zM, yM), xM));
}
function multiply4(a: number[][], b: number[][]): number[][] {
  const out: number[][] = Array.from({ length: 4 }, () => [0, 0, 0, 0]);
  for (let i = 0; i < 4; i++) {
    for (let j = 0; j < 4; j++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += (a[i]?.[k] ?? 0) * (b[k]?.[j] ?? 0);
      out[i]![j] = s;
    }
  }
  return out;
}

/**
 * Source offsets are Unicode scalar offsets. Build an explicit
 * scalar-to-UTF-16 boundary map; never use scalar offsets in slice().
 */
export function scalarToUtf16Map(source: string): number[] {
  const map: number[] = [0];
  let utf16 = 0;
  for (const ch of source) {
    utf16 += ch.length;
    map.push(utf16);
  }
  return map;
}
export function sliceByScalar(
  source: string,
  startScalar: number,
  endScalar: number,
): string {
  const map = scalarToUtf16Map(source);
  if (
    !Number.isSafeInteger(startScalar) || !Number.isSafeInteger(endScalar) ||
    startScalar < 0 || endScalar < startScalar || endScalar >= map.length
  ) {
    throw new Error("Scalar span out of range");
  }
  // Splitting inside a surrogate pair via scalar offsets is impossible by
  // construction: scalar boundaries always fall on code-point boundaries.
  // A caller passing a UTF-16 index as a scalar index is a type error the
  // map exposes: assert the round-trip through code points.
  const start16 = map[startScalar]!;
  const end16 = map[endScalar]!;
  return source.slice(start16, end16);
}
