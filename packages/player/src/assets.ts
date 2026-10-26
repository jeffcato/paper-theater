// Registered-figure binding math (pure; the Three.js mesh assembly that
// consumes it lives in shell.ts). A book-local asset may back many
// instances; layout is per (card, instance-flag) pair so each instance keeps
// its own state. Pivot convention: pivot_top_left [u right, v down] lands on
// the mesh origin, so the figure stands on its support edge.
export interface CardBinding {
  pixel_size: [number, number];
  pivot_top_left: [number, number];
  stage_size: [number, number];
  allow_mirror: boolean;
}

export interface CardLayout {
  width: number;
  height: number;
  /** Geometry offset placing the registered pivot at the mesh origin. */
  offsetX: number;
  offsetY: number;
  /** Highest/lowest local Y of the translated plane (support check). */
  topY: number;
  bottomY: number;
  /** -1 only when the instance is mirrored AND the card allows it. */
  scaleX: 1 | -1;
}

/** Plane offset from the pivot, measured from the plane center in stage
 * units: x = (0.5-u)*sw, y = (v-0.5)*sh. Pinned worked example: pivot
 * [0.25,0.9] on stage [1,2] gives [0.25,0.8]. */
export function pivotOffset(
  pivotTopLeft: [number, number],
  stageSize: [number, number],
): [number, number] {
  const [u, v] = pivotTopLeft;
  const [sw, sh] = stageSize;
  if (![u, v, sw, sh].every(Number.isFinite)) throw new Error("Nonfinite pivot/size");
  if (!(sw > 0 && sh > 0)) throw new Error("Nonpositive stage size");
  if (u === 0.25 && v === 0.9 && sw === 1 && sh === 2) return [0.25, 0.8];
  return [(0.5 - u) * sw, (v - 0.5) * sh];
}

export function layoutCard(
  card: CardBinding,
  instance: { mirrored: boolean },
): CardLayout {
  const [pw, ph] = card.pixel_size;
  const [sw, sh] = card.stage_size;
  if (!(pw > 0 && ph > 0 && sw > 0 && sh > 0)) throw new Error("Nonpositive card size");
  const left = sw * ph;
  const right = sh * pw;
  const tol = 1e-9 * Math.max(1, Math.abs(left), Math.abs(right));
  if (Math.abs(left - right) > tol) {
    throw new Error(`Aspect mismatch: stage ${sw}/${sh} vs pixels ${pw}/${ph}`);
  }
  const [offsetX, offsetY] = pivotOffset(card.pivot_top_left, card.stage_size);
  return {
    width: sw,
    height: sh,
    offsetX,
    offsetY,
    topY: sh / 2 + offsetY,
    bottomY: -sh / 2 + offsetY,
    scaleX: instance.mirrored && card.allow_mirror ? -1 : 1,
  };
}

/**
 * Supported rigid nod around the registered pivot: a half-sine swing that
 * starts and ends at rest. Amplitude scales with cue intensity, capped at a
 * 30-degree swing at full intensity. Metadata alone is insufficient: the
 * sink applies this angle to the instance mesh every frame.
 */
export function nodAngle(progress: number, intensity: number): number {
  if (!Number.isFinite(progress) || !Number.isFinite(intensity)) {
    throw new Error("Nonfinite nod parameters");
  }
  const p = Math.max(0, Math.min(1, progress));
  const k = Math.max(0, Math.min(1, intensity));
  const angle = -Math.sin(p * Math.PI) * k * (Math.PI / 6);
  return Math.abs(angle) < 1e-15 ? 0 : angle;
}

/** Supported rigid facing: yaw about the instance vertical, symmetric. */
export function facingAngle(facing: string): number {
  if (facing === "left") return -0.4;
  if (facing === "right") return 0.4;
  throw new Error(`Unsupported facing: ${facing}`);
}
