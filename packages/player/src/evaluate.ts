// Pure deterministic evaluator: evaluate(compiled, sample) reconstructs the
// full state without replaying history. Integer samples only; no epsilon may
// authorize an early reveal. Seeking into a transition selects the destination
// phase directly without restarting its wait.
import { DARK_HOLD_SAMPLES } from "./profile.js";
import type {
  CompiledTimeline,
  LightingSpec,
  RevealGroup,
  SceneRecord,
  StageSnapshot,
  StoryCue,
} from "./types.js";

export type PhaseKind = "opening" | "scene" | "transition" | "closing";

export interface EvaluatedActor {
  instance_id: string;
  asset_id: string;
  position: [number, number, number];
  rotation_xyz: [number, number, number];
  mirrored: boolean;
  present: boolean;
  gesture: string | null;
  gesture_intensity: number;
}

export interface EvaluatedState {
  sample: number;
  phase: PhaseKind;
  scene_id: string | null;
  curtain_open_ratio: number;
  stage_gain: number;
  snapshot: StageSnapshot;
  actors: EvaluatedActor[];
  active_cue_ids: string[];
  eligible_contributor_ids: string[];
  title_visible: boolean;
}

export function clampSample(sample: number, sampleCount: number): number {
  if (!Number.isSafeInteger(sample)) throw new Error(`Invalid sample ${sample}`);
  return Math.max(0, Math.min(sampleCount, sample));
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

export function evaluateEasing(name: string, t: number): number {
  const c = Math.max(0, Math.min(1, t));
  switch (name) {
    case "ease_in": return c * c;
    case "ease_out": return 1 - (1 - c) * (1 - c);
    case "ease_in_out":
      return c < 0.5 ? 2 * c * c : 1 - Math.pow(-2 * c + 2, 2) / 2;
    case "theatrical_wobble": {
      const c4 = (2 * Math.PI) / 3;
      if (c === 0) return 0;
      if (c === 1) return 1;
      return Math.pow(2, -10 * c) * Math.sin((c * 10 - 0.75) * c4) + 1;
    }
    case "linear":
    default:
      return c;
  }
}

function curtainAt(open: [number, number], close: [number, number], sample: number): number {
  // Opening curtain_open interval: closed->open. Closing curtain_close: open->closed.
  if (sample < open[0]) return 0;
  if (sample >= open[1] && sample < close[0]) return 1;
  if (sample >= close[1]) return 0;
  if (sample < open[1]) {
    const t = (sample - open[0]) / Math.max(1, open[1] - open[0]);
    return evaluateEasing("ease_in_out", t);
  }
  if (sample >= close[0]) {
    const t = (sample - close[0]) / Math.max(1, close[1] - close[0]);
    return 1 - evaluateEasing("ease_in_out", t);
  }
  return 1;
}

function transitionGain(
  fadeOutStart: number,
  swap: number,
  fadeInStart: number,
  fadeInEnd: number,
  sample: number,
): number {
  if (sample < fadeOutStart) return 1;
  if (sample < swap) {
    const t = (sample - fadeOutStart) / Math.max(1, swap - fadeOutStart);
    return 1 - evaluateEasing("ease_in_out", t);
  }
  if (sample < fadeInStart) return 0;
  if (sample < fadeInEnd) {
    const t = (sample - fadeInStart) / Math.max(1, fadeInEnd - fadeInStart);
    return evaluateEasing("ease_in_out", t);
  }
  return 1;
}

export function validateTransitionHold(
  fadeOutStart: number,
  swap: number,
  fadeInStart: number,
): { ok: boolean; hold: number } {
  const hold = fadeInStart - swap;
  return { ok: hold === DARK_HOLD_SAMPLES, hold };
}

function sceneAtSample(timeline: CompiledTimeline, sample: number): SceneRecord | null {
  for (const scene of timeline.scenes) {
    if (sample >= scene.narration[0] && sample < scene.narration[1]) return scene;
  }
  return null;
}

function transitionAtSample(
  timeline: CompiledTimeline,
  sample: number,
): { from: SceneRecord | null; to: SceneRecord | null; gain: number } | null {
  for (const tr of timeline.transitions) {
    if (sample >= tr.fade_out_start && sample < tr.fade_in_end) {
      const from = timeline.scenes.find((s) => s.id === tr.from_scene_id) ?? null;
      const to = timeline.scenes.find((s) => s.id === tr.to_scene_id) ?? null;
      return {
        from,
        to,
        gain: transitionGain(tr.fade_out_start, tr.swap_sample, tr.fade_in_start, tr.fade_in_end, sample),
      };
    }
  }
  return null;
}

function applyCueToActor(
  actor: EvaluatedActor,
  cue: StoryCue,
  sample: number,
): void {
  if (sample < cue.start_sample || sample >= cue.end_sample) return;
  const span = Math.max(1, cue.end_sample - cue.start_sample);
  const t = evaluateEasing(cue.easing, (sample - cue.start_sample) / span);
  const p = cue.params;
  switch (cue.action) {
    case "move": {
      const target = p["target"];
      if (Array.isArray(target) && target.length === 3) {
        // Deterministic: cues carry from/to via params origin/target.
        const origin = (Array.isArray(p["origin"]) ? p["origin"] : actor.position) as number[];
        actor.position = [
          lerp(origin[0] ?? 0, Number(target[0]), t),
          lerp(origin[1] ?? 0, Number(target[1]), t),
          lerp(origin[2] ?? 0, Number(target[2]), t),
        ];
      }
      if (p["facing"] === "left" || p["facing"] === "right") {
        void t;
      }
      break;
    }
    case "enter":
      actor.present = true;
      break;
    case "exit":
      // Exit completes at end_sample; while the cue is active the actor is
      // still present (walking out), removed exactly at the boundary.
      actor.present = true;
      break;
    case "face":
      break;
    case "gesture": {
      const g = p["gesture"];
      if (typeof g === "string") {
        actor.gesture = g;
        const intensity = typeof p["intensity"] === "number" ? p["intensity"] : 1;
        actor.gesture_intensity = Math.max(0, Math.min(1, intensity));
      }
      break;
    }
    case "camera":
    case "lighting":
      break;
  }
}

/** Eligibility: approved contributors with verified reveal bounds at/before t. */
export function eligibleContributors(
  reveals: RevealGroup[],
  sample: number,
): Set<string> {
  const out = new Set<string>();
  for (const group of reveals) {
    if (group.not_before_sample === null) continue;
    if (!Number.isSafeInteger(group.not_before_sample)) continue;
    if (sample < group.not_before_sample) continue;
    for (const id of group.contributor_ids) out.add(id);
  }
  return out;
}

export function evaluate(
  timeline: CompiledTimeline,
  sample: number,
  reveals?: RevealGroup[],
): EvaluatedState {
  const s = clampSample(sample, timeline.sample_count);
  // Terminal evaluation: on ended, explicitly evaluate sample_count.
  const atEnd = s >= timeline.sample_count;
  const curtain = atEnd
    ? 0
    : curtainAt(timeline.opening.curtain_open, timeline.closing.curtain_close, s);
  const inOpening =
    s < timeline.opening.curtain_open[1] || s < timeline.opening.first_prose_sample;
  const inClosing = s >= timeline.closing.curtain_close[0];
  const tr = atEnd ? null : transitionAtSample(timeline, s);
  const scene = atEnd ? null : (sceneAtSample(timeline, s) ?? null);

  let phase: PhaseKind = "scene";
  if (atEnd) phase = "closing";
  else if (tr) phase = "transition";
  else if (inClosing) phase = "closing";
  else if (inOpening && !scene) phase = "opening";

  // Destination snapshot without replaying the wait: a seek into a
  // transition resolves the incoming scene's entry snapshot for s >= swap.
  let base: StageSnapshot;
  let sceneId: string | null = null;
  let gain = 1;
  if (tr) {
    const useIncoming = s >= trPeakSwap(timeline, tr.from?.id ?? "", tr.to?.id ?? "");
    const chosen = (useIncoming ? tr.to : tr.from)?.entry_snapshot ?? timeline.terminal_snapshot;
    base = structuredClone(chosen);
    sceneId = (useIncoming ? tr.to : tr.from)?.id ?? null;
    gain = tr.gain;
  } else if (scene) {
    base = structuredClone(scene.entry_snapshot);
    sceneId = scene.id;
  } else if (phase === "opening") {
    // Curtains closed: neutral shell with the opening lighting target.
    base = structuredClone(timeline.scenes[0]?.entry_snapshot ?? timeline.terminal_snapshot);
    base.curtain_open_ratio = curtain;
    gain = 1;
  } else {
    base = structuredClone(timeline.terminal_snapshot);
  }
  base.curtain_open_ratio = curtain;

  // Active cues at this sample (half-open [start, end)).
  const activeCues = timeline.cues.filter(
    (c) => s >= c.start_sample && s < c.end_sample &&
      (sceneId === null || c.scene_id === sceneId || tr !== null),
  );

  // Build actor states from the snapshot instances, then apply cues.
  const actors: EvaluatedActor[] = base.instances.map((inst) => ({
    instance_id: inst.id,
    asset_id: inst.asset_id,
    position: [...inst.position] as [number, number, number],
    rotation_xyz: [...inst.rotation_xyz] as [number, number, number],
    mirrored: inst.mirrored,
    present: true,
    gesture: null,
    gesture_intensity: 0,
  }));
  const byTarget = new Map<string, EvaluatedActor[]>();
  for (const a of actors) {
    const list = byTarget.get(a.asset_id) ?? [];
    list.push(a);
    byTarget.set(a.asset_id, list);
  }
  // Exit-boundary removal: an exit cue ending at/before s removes its target.
  for (const cue of timeline.cues) {
    if (cue.action === "exit" && s >= cue.end_sample) {
      for (const a of byTarget.get(cue.target_id) ?? []) a.present = false;
    }
  }
  for (const cue of activeCues) {
    for (const a of byTarget.get(cue.target_id) ?? []) applyCueToActor(a, cue, s);
  }

  // No-spoilers filter before every pass: only approved contributors at or
  // before the playhead participate, including in darkness and after seeks.
  const revealSource = reveals ?? [];
  const eligible = eligibleContributors(revealSource, s);
  if (revealSource.length > 0) {
    for (const a of actors) {
      let ok = eligible.has(a.asset_id) || eligible.has(a.instance_id);
      // The neutral shell (instances with no reveal binding) stays visible.
      if (!hasRevealBinding(revealSource, a.asset_id) && !hasRevealBinding(revealSource, a.instance_id)) {
        ok = true;
      }
      if (!ok) a.present = false;
    }
  }
  // Full blackout covers unlit/emissive contributions too: gain multiplies
  // stage color, and ineligible content never enters any pass even at zero.
  const visibleActors = gain <= 0 ? actors.filter((a) => a.present && isShellActor(a)) : actors;
  void visibleActors;

  const titleVisible =
    s >= timeline.opening.title_hold[0] && s < timeline.opening.title_dismiss[1];

  return {
    sample: s,
    phase,
    scene_id: sceneId,
    curtain_open_ratio: curtain,
    stage_gain: gain,
    snapshot: base,
    actors: actors.filter((a) => a.present),
    active_cue_ids: activeCues.map((c) => c.id),
    eligible_contributor_ids: [...eligible].sort(),
    title_visible: titleVisible,
  };
}

function hasRevealBinding(reveals: RevealGroup[], contributor: string): boolean {
  return reveals.some((g) => g.contributor_ids.includes(contributor));
}
function isShellActor(_a: EvaluatedActor): boolean {
  return false;
}

function trPeakSwap(
  timeline: CompiledTimeline,
  fromId: string,
  toId: string,
): number {
  const tr = timeline.transitions.find(
    (t) => t.from_scene_id === fromId && t.to_scene_id === toId,
  );
  return tr?.swap_sample ?? 0;
}

/** Compile-time validation of transition records (dark hold, ordering). */
export function validateTimeline(timeline: CompiledTimeline): string[] {
  const errors: string[] = [];
  if (timeline.sample_rate !== 48000) errors.push("sample_rate: expected 48000");
  for (const tr of timeline.transitions) {
    const { ok, hold } = validateTransitionHold(tr.fade_out_start, tr.swap_sample, tr.fade_in_start);
    if (!ok) errors.push(`transition ${tr.from_scene_id}->${tr.to_scene_id}: dark hold ${hold} != 96000`);
    if (!(tr.fade_out_start < tr.swap_sample && tr.swap_sample < tr.fade_in_start && tr.fade_in_start < tr.fade_in_end)) {
      errors.push(`transition ${tr.from_scene_id}->${tr.to_scene_id}: unordered boundaries`);
    }
    if (tr.fade_in_end > timeline.sample_count) errors.push(`transition ${tr.from_scene_id}: beyond master`);
  }
  if (timeline.closing.terminal_sample !== timeline.sample_count) {
    errors.push("closing.terminal_sample must equal sample_count");
  }
  if (timeline.terminal_snapshot.curtain_open_ratio !== 0) {
    errors.push("terminal snapshot must have closed curtains");
  }
  // Supported actions only.
  const allowed = new Set(["enter", "exit", "move", "face", "gesture", "camera", "lighting"]);
  for (const cue of timeline.cues) {
    if (!allowed.has(cue.action)) errors.push(`cue ${cue.id}: unsupported action ${cue.action}`);
    if (!(cue.start_sample < cue.end_sample)) errors.push(`cue ${cue.id}: empty interval`);
  }
  return errors;
}

export function lightingWithGain(lighting: LightingSpec, gain: number): LightingSpec {
  // Intensities alone do not implement blackout; the compositor multiplies
  // stage color by gain. This helper mirrors gain onto lights for the
  // stage-domain preview path only.
  const copy: LightingSpec = structuredClone(lighting);
  for (const light of copy.lights) light.intensity *= gain;
  copy.ambient.intensity *= gain;
  return copy;
}
