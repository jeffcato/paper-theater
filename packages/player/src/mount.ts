// Diagnostic mounting: bind a loaded handle to one transport, the pure
// evaluator and an injected frame sink. Timeline cues pass through the
// existing shared-record adapters at mount time, so unsupported cues fail
// here — never mid-playback. Diagnostic handles mount and render, but every
// capture is stamped with their provenance/acceptance; the production path
// must pass through requireAccepted first.
import { describeCapture } from "./capture.js";
import { PlaqueView } from "./plaque.js";
import { PROFILE_ID } from "./profile.js";
import { evaluate, validateTimeline } from "./evaluate.js";
import { isAcceptedHandle } from "./loader.js";
import type { LoadedRelease } from "./loader.js";
import { groupWords } from "./readalong.js";
import type { MeasureFn } from "./readalong.js";
import { toCueView } from "./shared.js";
import { createTransport } from "./transport.js";
import type { ReadAlongGroup, ReadAlongWord } from "./types.js";
import type {
  ClockSource,
  IntermissionOptions,
  MountHandle,
  PauseReason,
  TransportEvents,
} from "./transport.js";
import type { EvaluatedState } from "./evaluate.js";
import type {
  CompiledTimeline,
  RevealGroup,
} from "./types.js";

/** Injected frame destination. The browser three.js sink implements pixel
 * capture; headless checks use recording doubles. Renderer construction
 * itself stays in shell.ts. */
export interface FrameSink {
  render(state: EvaluatedState): void;
  capturePixels?(width: number, height: number): Uint8Array;
  dispose?(): void;
  /** Browser sinks expose their canvas so the page can display it. */
  canvas?: unknown;
  /** Live camera refit (growth, pointer, authored moves). Optional. */
  setCameraPose?(pose: import("./shell.js").CameraPose): void;
}

export interface MountSource {
  handle: LoadedRelease;
  timeline: CompiledTimeline;
  reveals?: RevealGroup[];
}

export interface MountEvents extends TransportEvents {
  onState?: (state: EvaluatedState) => void;
}

/**
 * Verified read-along track for the mount-owned plaque. Studio passes the
 * package's read-along words/groups with the below-stage plaque container;
 * mount validates the grouping up front (invalid tracks throw before any
 * DOM is touched), feeds every delivered media sample to the view, routes
 * setReadAlong to it, and disposes it with the mount. `measure` is for
 * non-browser checks only — browsers measure the bundled Noto Serif face.
 */
export interface MountReadAlong {
  words: ReadAlongWord[];
  groups: ReadAlongGroup[];
  container: HTMLElement;
  basePx?: number;
  scale?: number;
  measure?: MeasureFn;
}

export interface MountOptions {
  clock?: ClockSource | null;
  sink?: FrameSink | null;
  readAlong?: MountReadAlong | null;
  candidate_id?: string;
  checker_sha256?: string;
  player_sha256?: string;
  events?: MountEvents;
  signOptions?: IntermissionOptions;
}

export interface FrameCapture {
  manifest_digest: string;
  candidate_id: string;
  profile_id: string;
  provenance: string;
  accepted: boolean;
  sample: number;
  phase: string;
  state_digest: string;
  contributor_ids: string[];
  width: number;
  height: number;
  pixels: Uint8Array;
}

export interface PlayerMount extends MountHandle {
  readonly diagnostic: boolean;
  readonly accepted: boolean;
  /** Mount-owned plaque, or null when no read-along track was supplied. */
  readonly plaque: PlaqueView | null;
  /** Advance one scheduler step: sample the clock once, evaluate, render. */
  tick(now?: number): EvaluatedState | null;
  /** Actual frame capture. Throws when no pixel sink is attached. */
  captureFrame(width: number, height: number): Promise<FrameCapture>;
}

/** Production gate: only loader-minted accepted handles pass. Diagnostic,
 * synthetic and imported handles throw instead of being coerced. */
export function requireAccepted(handle: LoadedRelease): void {
  const provenance = (handle as LoadedRelease).provenance;
  if (!isAcceptedHandle(handle)) {
    throw new Error(
      `Release blocked: provenance:${provenance} (diagnostic entry cannot authorize production capture)`,
    );
  }
}

export function mountPlayer(
  _container: unknown,
  source: MountSource,
  options: MountOptions = {},
): PlayerMount {
  const { handle, timeline } = source;
  const errors = validateTimeline(timeline);
  if (errors.length > 0) {
    throw new Error(`Invalid timeline: ${errors.join("; ")}`);
  }
  // Normalize once through the shared-record adapters. Unsupported cues
  // throw here, at mount time.
  const normalized: CompiledTimeline = {
    ...timeline,
    cues: timeline.cues.map((cue) => ({ ...cue, ...toCueView(cue) })),
  };
  const reveals = source.reveals ?? handle.reveals;
  const sink = options.sink ?? null;
  const events = options.events ?? {};
  let disposed = false;
  let lastState: EvaluatedState | null = null;

  // Verified track first: invalid grouping throws before any DOM exists.
  const track = options.readAlong ?? null;
  if (track) groupWords(track.words, track.groups);
  const plaque = track
    ? new PlaqueView(
      track.container,
      track.words,
      track.groups,
      {
        ...(track.basePx !== undefined ? { basePx: track.basePx } : {}),
        ...(track.scale !== undefined ? { scale: track.scale } : {}),
        ...(track.measure !== undefined ? { measure: track.measure } : {}),
      },
    )
    : null;

  const deliver = (sample: number): EvaluatedState => {
    const state = evaluate(normalized, sample, [...reveals]);
    lastState = state;
    sink?.render(state);
    plaque?.setSample(sample);
    events.onState?.(state);
    return state;
  };

  const transport = createTransport(
    timeline.sample_count,
    options.clock ?? null,
    {
      ...events,
      onSample: (sample) => {
        events.onSample?.(sample);
        deliver(sample);
      },
    },
    options.signOptions,
  );

  deliver(0);

  // Read provenance before the acceptance guard narrows the handle type.
  const provenance = handle.provenance;
  const diagnostic = !isAcceptedHandle(handle);
  const mount: PlayerMount = {
    diagnostic,
    accepted: !diagnostic,
    plaque,
    get epoch() {
      return transport.epoch;
    },
    get reason(): PauseReason {
      return transport.reason;
    },
    get sample() {
      return transport.sample;
    },
    tick(now?: number): EvaluatedState | null {
      if (disposed) return null;
      return deliver(transport.frame(now));
    },
    async captureFrame(width: number, height: number): Promise<FrameCapture> {
      if (disposed) throw new Error("capture after dispose");
      if (!Number.isSafeInteger(width) || width <= 0 || !Number.isSafeInteger(height) || height <= 0) {
        throw new Error(`capture bounds: ${width}x${height}`);
      }
      if (!sink?.capturePixels) {
        throw new Error("not_ready: capture (no pixel sink attached in this harness)");
      }
      const pixels = sink.capturePixels(width, height);
      const capture = await describeCapture(normalized, [...reveals], {
        manifest_digest: handle.manifest_digest,
        candidate_id: options.candidate_id ?? "unspecified-candidate",
        checker_sha256: options.checker_sha256 ?? "0".repeat(64),
        player_sha256: options.player_sha256 ?? "0".repeat(64),
        profile_id: PROFILE_ID,
        sample: transport.sample,
        width,
        height,
      });
      return {
        manifest_digest: capture.manifest_digest,
        candidate_id: capture.candidate_id,
        profile_id: capture.profile_id,
        provenance,
        accepted: !diagnostic,
        sample: capture.sample,
        phase: capture.phase,
        state_digest: capture.state_digest,
        contributor_ids: capture.contributor_ids,
        width,
        height,
        pixels,
      };
    },
    play: () => transport.play(),
    pause: (userInitiated = true) => transport.pause(userInitiated),
    seek: (sample: number) => transport.seek(sample),
    restart: () => transport.restart(),
    setVolume: (volume: number) => transport.setVolume(volume),
    setReadAlong: (enabled: boolean) => {
      transport.setReadAlong(enabled);
      plaque?.setEnabled(enabled);
    },
    dispose: () => {
      if (disposed) return;
      disposed = true;
      transport.dispose();
      plaque?.dispose();
      sink?.dispose?.();
    },
  };
  void lastState;
  return mount;
}
