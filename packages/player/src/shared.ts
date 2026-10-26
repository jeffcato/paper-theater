// Adapter boundary for the frozen shared records (pt-contracts-v1.0.0).
//
// The frozen TS mirror defines cues as a tagged union
// (`{action:'move', params:{...}}`) and lights as a discriminated union
// (`{type:'directional'} | {type:'spot', ...}`). The player normalizes both
// once, here, into flat views. Field names match the frozen mirror exactly;
// annotating the inputs with the shared types is a one-line change per
// adapter once the contracts package interface is directly importable
// (see the boundary note in tests/shared-contracts.test.ts).
import type { Easing, SupportedAction } from "./types.js";

const SUPPORTED_ACTIONS: readonly string[] = [
  "enter", "exit", "move", "face", "gesture", "camera", "lighting",
];

export interface SharedCueView {
  action: SupportedAction;
  easing: Easing;
  params: Record<string, unknown>;
}

/** Normalize one shared tagged-union cue; reject anything outside the allowlist. */
export function toCueView(cue: unknown): SharedCueView {
  if (!cue || typeof cue !== "object") {
    throw new Error("unsupported action: <missing cue>");
  }
  const record = cue as Record<string, unknown>;
  const { action, easing, params } = record;
  if (typeof action !== "string" || !SUPPORTED_ACTIONS.includes(action)) {
    throw new Error(`unsupported action: ${String(action)}`);
  }
  if (!params || typeof params !== "object" || Array.isArray(params)) {
    throw new Error(`cue ${String((record as { id?: unknown }).id ?? "?")}: missing params`);
  }
  const allowedEasing = ["linear", "ease_in", "ease_out", "ease_in_out", "theatrical_wobble"];
  if (typeof easing !== "string" || !allowedEasing.includes(easing)) {
    throw new Error(`cue: unsupported easing ${String(easing)}`);
  }
  return {
    action: action as SupportedAction,
    easing: easing as Easing,
    params: params as Record<string, unknown>,
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
export function toLightView(light: unknown): FlatLight {
  if (!light || typeof light !== "object") throw new Error("lighting: missing light");
  const record = light as Record<string, unknown>;
  const { type } = record;
  if (type !== "directional" && type !== "spot") {
    throw new Error(`lighting: unsupported light type ${String(type)}`);
  }
  const shadow = record["shadow"] as Record<string, unknown> | undefined;
  if (
    typeof record["id"] !== "string" || typeof record["enabled"] !== "boolean" ||
    !isVec3(record["position"]) || !isVec3(record["target"]) ||
    typeof record["color"] !== "string" || typeof record["intensity"] !== "number" ||
    typeof record["cast_shadow"] !== "boolean" || !shadow || typeof shadow !== "object" ||
    typeof shadow["map_size"] !== "number" || typeof shadow["bias"] !== "number" ||
    typeof shadow["normal_bias"] !== "number" || typeof shadow["near"] !== "number" ||
    typeof shadow["far"] !== "number"
  ) {
    throw new Error(`lighting: incomplete light ${String(record["id"] ?? "?")}`);
  }
  const flat: FlatLight = {
    id: record["id"] as string,
    type,
    enabled: record["enabled"] as boolean,
    position: record["position"] as [number, number, number],
    target: record["target"] as [number, number, number],
    color: record["color"] as string,
    intensity: record["intensity"] as number,
    cast_shadow: record["cast_shadow"] as boolean,
    shadow: {
      map_size: shadow["map_size"] as number,
      bias: shadow["bias"] as number,
      normal_bias: shadow["normal_bias"] as number,
      near: shadow["near"] as number,
      far: shadow["far"] as number,
    },
  };
  if (type === "spot") {
    for (const key of ["angle_radians", "penumbra", "distance", "decay"] as const) {
      const value = record[key];
      if (typeof value !== "number" || !Number.isFinite(value)) {
        throw new Error(`lighting: incomplete spot ${flat.id}`);
      }
      flat[key] = value;
    }
  }
  return flat;
}
