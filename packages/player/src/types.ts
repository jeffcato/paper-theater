// Local provisional wire records mirroring the player handoff
// "Interfaces and conversions" section. These are INTERNAL structures for
// isolated Agent 4 work before Agent 1 publishes contracts-ready.json.
// They must not be treated as the shared public schema: do not copy them
// into another lane, and consume Agent 1's exact revision before exposing
// stable public package interfaces. Field names/meanings below follow the
// handoff snake_case JSON so the later migration is mechanical.
import { SAMPLE_RATE } from "./profile.js";

export type Digest = string;
export type Id = string;
export type Sample = number;
export type Interval = [Sample, Sample];
export type SourceSpan = [number, number];

export const MAX_SAFE_INTEGER = Number.MAX_SAFE_INTEGER;

export function isDigest(s: unknown): s is Digest {
  return typeof s === "string" && /^[a-f0-9]{64}$/.test(s);
}
export function isId(s: unknown): s is Id {
  return typeof s === "string" && s.trim().length > 0 && s.length <= 256 &&
    ![...s].some((c) => /\p{Cc}/u.test(c));
}
export function isSample(n: unknown): n is Sample {
  return typeof n === "number" && Number.isSafeInteger(n) && n >= 0 &&
    n <= MAX_SAFE_INTEGER;
}
export function isInterval(v: unknown): v is Interval {
  return Array.isArray(v) && v.length === 2 && isSample(v[0]) && isSample(v[1]) &&
    (v[0] as number) <= (v[1] as number);
}
export function isFiniteVec(v: unknown, n: number): v is number[] {
  return Array.isArray(v) && v.length === n &&
    v.every((x) => typeof x === "number" && Number.isFinite(x));
}

export interface FileRef {
  path: string;
  sha256: Digest;
  bytes: number;
  media_type: string;
  dimensions?: [number, number];
}
export interface ReleaseRef {
  manifest_path: string;
  manifest_sha256: Digest;
}
export type Provenance = "autonomous" | "imported" | "synthetic" | "diagnostic";

export interface Manifest {
  schema_version: 1;
  capability_version: "paper-player-v1";
  provenance: Provenance;
  title: string;
  author: string;
  chapter_label: string;
  source_sha256: Digest;
  sample_rate: 48000;
  sample_count: Sample;
  entries: Record<"bible" | "stages" | "actors" | "timeline" | "shell" | "read_along" | "captions" | "audio" | "poster" | "credits", FileRef>;
  assets: FileRef[];
  reveals: RevealGroup[];
  acceptance: FileRef;
}

export interface RegisteredCard {
  id: Id;
  image: FileRef;
  pixel_size: [number, number];
  crop_to_source: [number, number, number, number, number, number];
  pivot_top_left: [number, number];
  stage_size: [number, number];
  body_height: number;
  support_id: Id;
  member_ids: Id[];
  allow_mirror: boolean;
}

export interface SceneInstance {
  id: Id;
  asset_id: Id;
  position: [number, number, number];
  rotation_xyz: [number, number, number];
  support_id: Id;
  reveal_group_id: Id;
  mirrored: boolean;
}

export interface RevealFact {
  source_span: SourceSpan;
  image_attestation_id: Id;
  primary_anchor_id: Id;
  independent_anchor_id: Id;
}
export interface RevealGroup {
  id: Id;
  contributor_ids: Id[];
  not_before_sample: Sample | null;
  facts: RevealFact[];
}

export interface CameraSpec {
  position: [number, number, number];
  target: [number, number, number];
  up: [number, number, number];
  fov_y_degrees: number;
  near: number;
  far: number;
}
export interface LightSpec {
  id: Id;
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
export interface LightingSpec {
  ambient: { color: string; intensity: number };
  environment: null;
  lights: LightSpec[];
}
export interface StageSnapshot {
  set_id: Id;
  instances: SceneInstance[];
  camera: CameraSpec;
  lighting: LightingSpec;
  curtain_open_ratio: number;
}

export type SupportedAction =
  | "enter" | "exit" | "move" | "face" | "gesture" | "camera" | "lighting";
export type Easing = "linear" | "ease_in" | "ease_out" | "ease_in_out" | "theatrical_wobble";
export interface StoryCue {
  id: Id;
  scene_id: Id;
  target_id: Id;
  action: SupportedAction;
  start_sample: Sample;
  end_sample: Sample;
  easing: Easing;
  reveal_group_ids: Id[];
  params: Record<string, unknown>;
}

export interface SceneRecord {
  id: Id;
  set_id: Id;
  source_span: SourceSpan;
  narration: Interval;
  entry_snapshot: StageSnapshot;
  required_asset_ids: Id[];
  reveal_group_ids: Id[];
}
export interface SceneTransition {
  from_scene_id: Id;
  to_scene_id: Id;
  fade_out_start: Sample;
  swap_sample: Sample;
  fade_in_start: Sample;
  fade_in_end: Sample;
}
export interface OpeningRecord {
  title: string;
  chapter_label: string;
  title_hold: Interval;
  title_dismiss: Interval;
  curtain_open: Interval;
  first_prose_sample: Sample;
  intro_narration: Interval | null;
  first_picture_id: Id;
  first_picture_eligible_sample: Sample;
}
export interface ClosingRecord {
  last_prose_action_sample: Sample;
  curtain_close: Interval;
  score_tail: Interval | null;
  terminal_sample: Sample;
}
export interface CompiledTimeline {
  schema_version: 1;
  capability_version: "paper-player-v1";
  delivery_audio_sha256: Digest;
  presentation_map_sha256: Digest;
  sample_rate: 48000;
  sample_count: Sample;
  opening: OpeningRecord;
  scenes: SceneRecord[];
  transitions: SceneTransition[];
  cues: StoryCue[];
  closing: ClosingRecord;
  terminal_snapshot: StageSnapshot;
}

export interface ReadAlongWord {
  id: Id;
  source_span: SourceSpan;
  text: string;
  following_text: string;
  start_sample: Sample;
  end_sample: Sample;
}
export interface ReadAlongGroup {
  id: Id;
  word_ids: Id[];
  timing_owner_word_id: Id;
  start_sample: Sample;
  end_sample: Sample;
}
export interface ReadAlongTrack {
  schema_version: 1;
  policy: "point-following-same-boundary-v1";
  language: string;
  source_sha256: Digest;
  narration_sha256: Digest;
  alignment_sha256: Digest;
  presentation_map_sha256: Digest;
  delivery_audio_sha256: Digest;
  leading_text: string;
  words: ReadAlongWord[];
  groups: ReadAlongGroup[];
  preferred_break_after: Id[];
}

export type PlayerFailureCode =
  | "digest_mismatch" | "manifest_too_large" | "schema" | "capability"
  | "sample_domain" | "missing_entry" | "artifact_identity" | "artifact_bounds"
  | "unsupported_media" | "invalid_png" | "invalid_wav" | "reveal_ineligible"
  | "unsupported_action" | "unsupported_version" | "missing_acceptance"
  | "provenance" | "load_bounds" | "seek_bounds" | "not_ready";
export interface PlayerFailure {
  code: PlayerFailureCode;
  field: string;
  message: string;
}
export function fail(code: PlayerFailureCode, field: string, message: string): PlayerFailure {
  return { code, field, message };
}

export function assertSampleRate(rate: number): void {
  if (rate !== SAMPLE_RATE) {
    throw new Error(`Unsupported sample rate ${rate}: v1 requires ${SAMPLE_RATE}`);
  }
}
