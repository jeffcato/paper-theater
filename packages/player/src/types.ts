// Public wire records: shared contract types (pt-contracts-v1.0.0) consumed
// directly from @paper-theater/contracts. The player keeps its legacy record
// names as aliases so the public package surface is unchanged; every alias
// below is the identical shared type, not a fork. See
// docs/implementation/player/shared-types.md for the full mapping, the two
// intentional shape changes (tagged cue params, discriminated lights), and
// the retained private evaluation views in ./shared.js.
import { SAMPLE_RATE } from "./profile.js";
import type { CueAction, Digest, Id, Interval, Sample } from "@paper-theater/contracts";

export type {
  Digest,
  Id,
  Sample,
  Interval,
  SourceSpan,
  FileRef,
  ReleaseRef,
  Provenance,
  Manifest,
  RegisteredCard,
  SceneInstance,
  RevealFact,
  RevealGroup,
  Camera as CameraSpec,
  Light as LightSpec,
  Lighting as LightingSpec,
  Snapshot as StageSnapshot,
  Easing,
  Facing,
  CueAction,
  StoryCue,
  Scene as SceneRecord,
  SceneTransition,
  Opening as OpeningRecord,
  Closing as ClosingRecord,
  Timeline as CompiledTimeline,
  ReadAlongWord,
  ReadAlongGroup,
  ReadAlong as ReadAlongTrack,
} from "@paper-theater/contracts";

/** Flat action-name view of the shared tagged-union cues; derived, never forked. */
export type SupportedAction = CueAction["action"];

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
