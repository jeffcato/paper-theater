// Adapter boundary: shared contract records (pt-contracts-v1.0.0) into the
// player's private evaluation views.
//
// The shared package defines cues as a tagged union
// (`{action:'move', params:{...}}`) and lights as a discriminated union
// (`{type:'directional'} | {type:'spot', ...}`). The player normalizes both
// once, here, into flat views. Inputs are annotated with the shared types;
// hostile wire bytes still arrive through these adapters (adverse-shape
// callers cast them in), so every field is re-validated at runtime and
// anything outside the allowlist is rejected with a diagnostic. The flat
// views stay private to evaluation: nothing outside the player constructs
// them, and the shared records are never mutated.
import type { Easing, SupportedAction } from "./types.js";
import type { Light, StoryCue } from "@paper-theater/contracts";

/** Mount-time normalization: reject hostile cues now, at the boundary; the
 * validated shared cues flow on unchanged (evaluate reads their typed
 * params directly, so no flat copy is stored). */
export function normalizeCues(cues: StoryCue[]): StoryCue[] {
  for (const cue of cues) toCueView(cue);
  return [...cues];
}

const SUPPORTED_ACTIONS: readonly string[] = [
  "enter", "exit", "move", "face", "gesture", "camera", "lighting",
];

export interface SharedCueView {
  action: SupportedAction;
  easing: Easing;
  params: Record<string, unknown>;
}

/** Normalize one shared tagged-union cue; reject anything outside the allowlist. */
export function toCueView(cue: StoryCue): SharedCueView {
  if (!cue || typeof cue !== "object") {
    throw new Error("unsupported action: <missing cue>");
  }
  const { action, easing, params } = cue;
  if (typeof action !== "string" || !SUPPORTED_ACTIONS.includes(action)) {
    throw new Error(`unsupported action: ${String(action)}`);
  }
  if (!params || typeof params !== "object" || Array.isArray(params)) {
    throw new Error(`cue ${String(cue.id ?? "?")}: missing params`);
  }
  const allowedEasing = ["linear", "ease_in", "ease_out", "ease_in_out", "theatrical_wobble"];
  if (typeof easing !== "string" || !allowedEasing.includes(easing)) {
    throw new Error(`cue: unsupported easing ${String(easing)}`);
  }
  const flatParams: Record<string, unknown> = Object.fromEntries(Object.entries(params));
  return {
    action,
    easing,
    params: flatParams,
  };
}

export interface FlatLight {
  id: string;
  type: "directional" | "spot";
  enabled: boolean;
  position: [number, number, number];
  target: [number, number, number];
  color: string;
  intensity: number;
  cast_shadow: boolean;
  shadow: { map_size: number; bias: number; normal_bias: number; near: number; far: number };
  angle_radians?: number;
  penumbra?: number;
  distance?: number;
  decay?: number;
}

function isVec3(v: unknown): v is [number, number, number] {
  return Array.isArray(v) && v.length === 3 &&
    v.every((n) => typeof n === "number" && Number.isFinite(n));
}

/** Normalize one shared discriminated-union light; spot extras required for spots. */
export function toLightView(light: Light): FlatLight {
  if (!light || typeof light !== "object") throw new Error("lighting: missing light");
  const { type } = light;
  if (type !== "directional" && type !== "spot") {
    throw new Error(`lighting: unsupported light type ${String(type)}`);
  }
  const shadow = light.shadow;
  if (
    typeof light.id !== "string" || typeof light.enabled !== "boolean" ||
    !isVec3(light.position) || !isVec3(light.target) ||
    typeof light.color !== "string" || typeof light.intensity !== "number" ||
    typeof light.cast_shadow !== "boolean" || !shadow || typeof shadow !== "object" ||
    typeof shadow.map_size !== "number" || typeof shadow.bias !== "number" ||
    typeof shadow.normal_bias !== "number" || typeof shadow.near !== "number" ||
    typeof shadow.far !== "number"
  ) {
    throw new Error(`lighting: incomplete light ${String(light.id ?? "?")}`);
  }
  const flat: FlatLight = {
    id: light.id,
    type,
    enabled: light.enabled,
    position: light.position,
    target: light.target,
    color: light.color,
    intensity: light.intensity,
    cast_shadow: light.cast_shadow,
    shadow: {
      map_size: shadow.map_size,
      bias: shadow.bias,
      normal_bias: shadow.normal_bias,
      near: shadow.near,
      far: shadow.far,
    },
  };
  if (type === "spot") {
    const { angle_radians, penumbra, distance, decay } = light;
    if (
      typeof angle_radians !== "number" || !Number.isFinite(angle_radians) ||
      typeof penumbra !== "number" || !Number.isFinite(penumbra) ||
      typeof distance !== "number" || !Number.isFinite(distance) ||
      typeof decay !== "number" || !Number.isFinite(decay)
    ) {
      throw new Error(`lighting: incomplete spot ${flat.id}`);
    }
    flat.angle_radians = angle_radians;
    flat.penumbra = penumbra;
    flat.distance = distance;
    flat.decay = decay;
  }
  return flat;
}
