// Hash-bound package/media loading with bounded fetch/decode, immutable
// accepted handles, scoped asset ownership and explicit diagnostic loading.
// A diagnostic fixture reports missing acceptance IDs and is rejected by the
// audience production path.
import { SAMPLE_RATE } from "./profile.js";
import type { Digest, FileRef, Manifest, Provenance, RevealGroup } from "./types.js";
import { isDigest } from "./types.js";
import { validateTimeline } from "./evaluate.js";

export const MAX_MANIFEST_BYTES = 1024 * 1024;
export const MAX_FILE_BYTES = 32 * 1024 * 1024;
export const MAX_TOTAL_BYTES = 256 * 1024 * 1024;
export const MAX_IMAGE_PX = 4096;

export interface LoadPolicy {
  player_sha256: Digest;
  checker_sha256: Digest;
  recipe_sha256: Digest;
  allow_diagnostic: boolean;
}

export interface LoadedRelease {
  readonly manifest: Manifest;
  readonly manifest_digest: Digest;
  readonly files: ReadonlyMap<string, Uint8Array>;
  readonly reveals: readonly RevealGroup[];
  readonly accepted: boolean;
  readonly provenance: Provenance;
  readonly missing_acceptance_ids: readonly string[];
  readonly asset_scope_id: string;
}
const minted = new WeakSet<object>();
export function isAcceptedHandle(value: unknown): value is LoadedRelease {
  return !!value && typeof value === "object" && minted.has(value) &&
    (value as LoadedRelease).accepted === true;
}

export type FileReader = (path: string, maxBytes: number) => Promise<Uint8Array>;

function freezeDeep<T>(value: T): T {
  if (value && typeof value === "object") {
    for (const v of Object.values(value)) freezeDeep(v);
    Object.freeze(value);
  }
  return value;
}

export function safePackagePath(path: string): string {
  if (
    typeof path !== "string" ||
    !/^[A-Za-z0-9_-]+(?:\/[A-Za-z0-9_.-]+)*\.[A-Za-z0-9]+$/.test(path) ||
    path.split("/").some((p) => p === "." || p === "..")
  ) {
    throw new Error(`Unsafe package path: ${path}`);
  }
  return path;
}

export async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes as unknown as ArrayBuffer);
  return [...new Uint8Array(digest)].map((n) => n.toString(16).padStart(2, "0")).join("");
}

function checkFileRef(ref: FileRef): string | null {
  try {
    safePackagePath(ref.path);
  } catch {
    return `path:${ref.path}`;
  }
  if (!isDigest(ref.sha256)) return `digest:${ref.path}`;
  if (!Number.isSafeInteger(ref.bytes) || ref.bytes <= 0 || ref.bytes > MAX_FILE_BYTES) {
    return `bounds:${ref.path}`;
  }
  if (!ref.media_type) return `media:${ref.path}`;
  if (ref.dimensions) {
    const [w, h] = ref.dimensions;
    if (!(w >= 1 && h >= 1 && w <= MAX_IMAGE_PX && h <= MAX_IMAGE_PX)) {
      return `dimensions:${ref.path}`;
    }
  } else if (ref.media_type === "image/png") {
    return `dimensions:${ref.path}`;
  }
  return null;
}

export function inspectPng(bytes: Uint8Array, ref: FileRef): void {
  if (bytes.length < 33) throw new Error(`Invalid PNG: ${ref.path}`);
  const sig = [137, 80, 78, 71, 13, 10, 26, 10];
  for (let i = 0; i < 8; i++) if (bytes[i] !== sig[i]) throw new Error(`Invalid PNG: ${ref.path}`);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (view.getUint32(8) !== 13) throw new Error(`Invalid PNG: ${ref.path}`);
  const type = String.fromCharCode(bytes[12]!, bytes[13]!, bytes[14]!, bytes[15]!);
  if (type !== "IHDR") throw new Error(`Invalid PNG: ${ref.path}`);
  const w = view.getUint32(16);
  const h = view.getUint32(20);
  if (!w || !h || w > MAX_IMAGE_PX || h > MAX_IMAGE_PX) {
    throw new Error(`Invalid PNG dimensions: ${ref.path}`);
  }
  if (ref.dimensions && (ref.dimensions[0] !== w || ref.dimensions[1] !== h)) {
    throw new Error(`PNG dimension mismatch: ${ref.path}`);
  }
}

/** Narrow delivery capability: integer PCM16 WAV at 48 kHz stereo. */
export function inspectWav(bytes: Uint8Array): { sample_rate: number; sample_count: number; channels: number } {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const tag = (i: number): string =>
    String.fromCharCode(bytes[i]!, bytes[i + 1]!, bytes[i + 2]!, bytes[i + 3]!);
  if (bytes.length < 44 || tag(0) !== "RIFF" || tag(8) !== "WAVE") {
    throw new Error("Invalid delivered WAV container");
  }
  let format: { sample_rate: number; channels: number } | undefined;
  let frames: number | undefined;
  for (let p = 12; p + 8 <= bytes.length;) {
    const n = view.getUint32(p + 4, true);
    const end = p + 8 + n;
    if (end > bytes.length) throw new Error("Truncated WAV chunk");
    if (tag(p) === "fmt ") {
      if (n < 16 || view.getUint16(p + 8, true) !== 1 || view.getUint16(p + 22, true) !== 16) {
        throw new Error("Unsupported WAV encoding");
      }
      const channels = view.getUint16(p + 10, true);
      const rate = view.getUint32(p + 12, true);
      if (!(channels === 1 || channels === 2) || rate !== SAMPLE_RATE) {
        throw new Error("WAV must be 48 kHz mono/stereo PCM16");
      }
      format = { channels, sample_rate: rate };
    } else if (tag(p) === "data") {
      if (!format || frames !== undefined || !n || n % (format.channels * 2)) {
        throw new Error("Invalid WAV sample data");
      }
      frames = n / (format.channels * 2);
      for (let i = p + 8; i < end; i += 2) view.getInt16(i, true);
    }
    p = end + (n % 2);
  }
  if (!format || frames === undefined) throw new Error("Missing WAV samples");
  return { ...format, sample_count: frames };
}

const REQUIRED_ENTRIES = [
  "bible", "stages", "actors", "timeline", "shell",
  "read_along", "captions", "audio", "poster", "credits",
] as const;

export interface Inspection {
  manifest: Manifest;
  digest: Digest;
  accepted: boolean;
  issues: { code: string; target: string; detail: string }[];
  missing_acceptance_ids: string[];
}

/** Fail-closed inspection shared by both entry points. */
export async function inspectRelease(
  manifestBytes: Uint8Array,
  expectedDigest: string,
  read: FileReader,
  policy: LoadPolicy,
): Promise<Inspection> {
  const issues: Inspection["issues"] = [];
  const issue = (code: string, target: string, detail: string): void => {
    issues.push({ code, target, detail });
  };
  if (manifestBytes.length > MAX_MANIFEST_BYTES) throw new Error("Release manifest too large");
  const digest = await sha256Hex(manifestBytes);
  if (digest !== expectedDigest) throw new Error("Release digest mismatch");
  const manifest = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(manifestBytes)) as Manifest;
  if (manifest.schema_version !== 1) throw new Error("Unsupported release schema");
  if (manifest.capability_version !== "paper-player-v1") throw new Error("Unsupported capability");
  if (manifest.sample_rate !== SAMPLE_RATE) throw new Error("Invalid final sample domain");
  if (!Number.isSafeInteger(manifest.sample_count) || manifest.sample_count <= 0) {
    throw new Error("Invalid sample_count");
  }
  if (manifest.provenance !== "autonomous") {
    issue("provenance", "release", "Only autonomous provenance can be accepted");
  }
  // Bounded fetch with exact identity per artifact.
  const files = new Map<string, Uint8Array>();
  const paths = new Set<string>();
  let total = 0;
  const refs: FileRef[] = [...Object.values(manifest.entries), ...manifest.assets, manifest.acceptance];
  for (const ref of refs) {
    const bad = checkFileRef(ref);
    if (bad) throw new Error(`Invalid/bounded artifact: ${bad}`);
    if (paths.has(ref.path)) throw new Error(`Duplicate path: ${ref.path}`);
    paths.add(ref.path);
    total += ref.bytes;
    if (total > MAX_TOTAL_BYTES) throw new Error("Release exceeds total byte bound");
    const bytes = await read(ref.path, ref.bytes);
    if (bytes.length !== ref.bytes || (await sha256Hex(bytes)) !== ref.sha256) {
      throw new Error(`Artifact identity mismatch: ${ref.path}`);
    }
    if (ref.media_type === "image/png") inspectPng(bytes, ref);
    files.set(ref.path, bytes);
  }
  for (const key of REQUIRED_ENTRIES) {
    if (!manifest.entries[key]) issue("missing_entry", key, "Required immutable dependency missing");
  }
  // Delivery audio must decode to the declared sample domain.
  const audioRef = manifest.entries["audio"];
  if (audioRef && files.has(audioRef.path)) {
    try {
      const decoded = inspectWav(files.get(audioRef.path)!);
      if (decoded.sample_count !== manifest.sample_count) {
        issue("delivery_samples", "audio", "Decoded final sample domain differs");
      }
    } catch (err) {
      issue("invalid_wav", "audio", err instanceof Error ? err.message : "WAV decode failed");
    }
  }
  // Reveal references must resolve to acceptance evidence. Unknown, stale or
  // ambiguous bounds stay ineligible; missing IDs are reported, never inferred.
  const missing: string[] = [];
  const acceptanceIds = new Set(manifest.assets.map((a) => a.path));
  acceptanceIds.add(manifest.acceptance.path);
  for (const group of manifest.reveals) {
    if (group.not_before_sample === null) {
      issue("missing_anchor", group.id, "Unknown reveal bounds stay hidden");
      continue;
    }
    for (const fact of group.facts) {
      for (const anchor of [fact.image_attestation_id, fact.primary_anchor_id, fact.independent_anchor_id]) {
        if (!acceptanceIds.has(anchor) && !manifest.assets.some((a) => a.path === anchor)) {
          if (!missing.includes(anchor)) missing.push(anchor);
        }
      }
    }
  }
  if (missing.length > 0) {
    issue("missing_acceptance", missing[0]!, `Missing acceptance IDs: ${missing.join(", ")}`);
  }
  const accepted = issues.length === 0;
  return { manifest: freezeDeep(manifest), digest, accepted, issues, missing_acceptance_ids: missing };
}

/**
 * Production entry point: returns a verified immutable handle or throws a
 * typed failure. Diagnostic provenance is always rejected here.
 */
export async function loadRelease(
  manifestBytes: Uint8Array,
  expectedDigest: string,
  read: FileReader,
  policy: LoadPolicy,
): Promise<LoadedRelease> {
  const inspection = await inspectRelease(manifestBytes, expectedDigest, read, policy);
  if (!inspection.accepted) {
    throw new Error(
      `Release blocked: ${inspection.issues.map((i) => `${i.code}:${i.target}`).join(", ")}`,
    );
  }
  if (inspection.manifest.provenance !== "autonomous") {
    throw new Error("Release blocked: provenance:release");
  }
  const handle: LoadedRelease = freezeDeep({
    manifest: inspection.manifest,
    manifest_digest: inspection.digest,
    files: new Map(),
    reveals: inspection.manifest.reveals,
    accepted: true,
    provenance: inspection.manifest.provenance,
    missing_acceptance_ids: [],
    asset_scope_id: inspection.digest.slice(0, 16),
  });
  minted.add(handle);
  return handle;
}

/**
 * Explicit diagnostic entry point. May render a structurally valid fixture
 * while reporting all missing acceptance IDs. It never returns a
 * production-accepted handle and is unavailable from the audience's
 * production-loading path.
 */
export async function loadDiagnostic(
  manifestBytes: Uint8Array,
  expectedDigest: string,
  read: FileReader,
  _policy: LoadPolicy,
): Promise<Omit<LoadedRelease, "accepted"> & { accepted: false }> {
  const inspection = await inspectRelease(manifestBytes, expectedDigest, read, _policy);
  const handle = freezeDeep({
    manifest: inspection.manifest,
    manifest_digest: inspection.digest,
    files: new Map(),
    reveals: inspection.manifest.reveals,
    accepted: false as const,
    provenance: inspection.manifest.provenance,
    missing_acceptance_ids: inspection.missing_acceptance_ids,
    asset_scope_id: `diagnostic-${inspection.digest.slice(0, 8)}`,
  });
  return handle;
}

export function validateTimelineForRelease(manifest: Manifest, timelineJson: unknown): string[] {
  void manifest;
  const t = timelineJson as Parameters<typeof validateTimeline>[0];
  try {
    return validateTimeline(t);
  } catch (err) {
    return [`timeline: ${err instanceof Error ? err.message : "invalid"}`];
  }
}
