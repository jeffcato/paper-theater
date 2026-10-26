// Deterministic capture interface for Agent 3's automated creative review.
// Each capture names exact manifest/candidate, profile, player/checker,
// sample and contributor identities. No provider calls from the player.
import { PROFILE_ID } from "./profile.js";
import { evaluate } from "./evaluate.js";
import type { CompiledTimeline, RevealGroup } from "./types.js";

export interface CaptureRequest {
  manifest_digest: string;
  candidate_id: string;
  checker_sha256: string;
  player_sha256: string;
  profile_id: string;
  sample: number;
  width: number;
  height: number;
}

export interface CaptureResult {
  manifest_digest: string;
  candidate_id: string;
  profile_id: string;
  checker_sha256: string;
  player_sha256: string;
  sample: number;
  phase: string;
  scene_id: string | null;
  curtain_open_ratio: number;
  stage_gain: number;
  contributor_ids: string[];
  width: number;
  height: number;
  /** Deterministic state digest (JSON of the evaluated snapshot). */
  state_digest: string;
  note: string;
}

export async function describeCapture(
  timeline: CompiledTimeline,
  reveals: RevealGroup[],
  request: CaptureRequest,
): Promise<CaptureResult> {
  if (request.profile_id !== PROFILE_ID) {
    throw new Error(`Unsupported profile ${request.profile_id}: expected ${PROFILE_ID}`);
  }
  const state = evaluate(timeline, request.sample, reveals);
  const bytes = new TextEncoder().encode(
    JSON.stringify({
      sample: state.sample,
      phase: state.phase,
      scene: state.scene_id,
      curtain: state.curtain_open_ratio,
      gain: state.stage_gain,
      actors: state.actors.map((a) => [a.instance_id, a.position, a.mirrored]),
      eligible: state.eligible_contributor_ids,
    }),
  );
  const digestBytes = await crypto.subtle.digest("SHA-256", bytes as unknown as ArrayBuffer);
  const state_digest = [...new Uint8Array(digestBytes)]
    .map((n) => n.toString(16).padStart(2, "0"))
    .join("");
  return {
    manifest_digest: request.manifest_digest,
    candidate_id: request.candidate_id,
    profile_id: request.profile_id,
    checker_sha256: request.checker_sha256,
    player_sha256: request.player_sha256,
    sample: state.sample,
    phase: state.phase,
    scene_id: state.scene_id,
    curtain_open_ratio: state.curtain_open_ratio,
    stage_gain: state.stage_gain,
    contributor_ids: state.eligible_contributor_ids,
    width: request.width,
    height: request.height,
    state_digest,
    note: "state-only capture; browser pixel capture runs under check:player -- integration",
  };
}

/** Required sampling inventory helper: boundaries at sample-1/sample/sample+1. */
export function boundarySamples(sample: number, sampleCount: number): number[] {
  return [sample - 1, sample, sample + 1].filter(
    (s) => Number.isSafeInteger(s) && s >= 0 && s <= sampleCount,
  );
}
