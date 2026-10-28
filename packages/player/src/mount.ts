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
import { normalizeCues } from "./shared.js";
import { createTransport } from "./transport.js";
import type { ReadAlongGroup, ReadAlongWord } from "./types.js";
import type {
  ClockSource,
  IntermissionOptions,
  MountHandle,
  PauseReason,
  SignPose,
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
  /** Pause-driven INTERMISSION sign pose from the transport UI clock. */
  setSign?(pose: SignPose, progress: number): void;
  /** Read-only live GL resource census for teardown proofs. Optional. */
  resourceInfo?(): { geometries: number; textures: number };
  /**
   * Diagnostic-only live geometry readout for conformance harnesses
   * (never part of audience flows): per-instance pivot/bottom-center world
   * positions in authoring coordinates plus static shell join points, all
   * read through the real scene-graph matrices.
   */
  debugGeometry?(): SinkGeometry;
}

/** Authoring-frame world points for geometry conformance. */
export interface SinkGeometry {
  instances: {
    instance_id: string;
    pivotWorld: [number, number, number];
    bottomWorld: [number, number, number];
  }[];
  joints: {
    deckFrontEdge: [number, number, number];
    apronTopEdge: [number, number, number];
    columnBaseL: [number, number, number];
    columnBaseR: [number, number, number];
  };
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
  /**
   * Caller-owned sink factory for graphics recovery: rebuilds the rendering
   * surface on the same canvas after context loss. Absent means recovery is
   * unavailable and retryGraphics rejects with a reload path.
   */
  recreateSink?: () => Promise<FrameSink>;
}

/** Actionable graphics-recovery state for context loss/restore. */
export interface GraphicsStatus {
  readonly state: "ready" | "lost";
  /** Actionable error while lost; null when ready. */
  readonly error: string | null;
  /** Media was playing when graphics were lost (never autoresumed: explicit Play required). */
  readonly suspendedWhilePlaying: boolean;
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
  /** Current graphics-recovery state (context loss/restore). */
  graphicsStatus(): GraphicsStatus;
  /**
   * Rebuild the rendering surface after context loss and re-render at the
   * retained sample. Resolves immediately when ready; joins an in-flight
   * recovery; rejects stale after a newer seek/restart/loss/disposal, or
   * with a reload path when no sink factory was supplied. Never autoplays.
   */
  retryGraphics(): Promise<void>;
}

/** Minimal canvas surface for context-loss monitoring (real or test double). */
interface CanvasMonitor {
  addEventListener(type: string, listener: (event: { preventDefault(): void }) => void): void;
  removeEventListener(type: string, listener: (event: { preventDefault(): void }) => void): void;
}

function asMonitoredCanvas(sink: FrameSink | null): CanvasMonitor | null {
  const canvas = (sink as { canvas?: unknown } | null)?.canvas;
  if (
    canvas !== null && typeof canvas === "object" &&
    typeof (canvas as CanvasMonitor).addEventListener === "function" &&
    typeof (canvas as CanvasMonitor).removeEventListener === "function"
  ) {
    return canvas as CanvasMonitor;
  }
  return null;
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
    cues: normalizeCues(timeline.cues),
  };
  const reveals = source.reveals ?? handle.reveals;
  let sink = options.sink ?? null;
  const clock: ClockSource | null = options.clock ?? null;
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
      onSign: (pose, progress) => {
        events.onSign?.(pose, progress);
        sink?.setSign?.(pose, progress);
      },
    },
    options.signOptions,
  );

  // Graphics loss/recovery: the mount watches the sink canvas (when it
  // carries listeners) and pauses media without touching user intent.
  // Recovery rebuilds the surface through the caller-owned factory and
  // re-renders at the retained sample; a graphics sequence number rejects
  // completions overtaken by a newer seek, restart, loss or disposal.
  let graphics: GraphicsStatus["state"] = "ready";
  let graphicsError: string | null = null;
  let suspendedWhilePlaying = false;
  let graphicsSeq = 0;
  let recoveryPromise: Promise<void> | null = null;
  // Playback-intent ownership across graphics suspension: each mount.play()
  // takes a generation; loss/pause/restart/disposal bump the suspension
  // generation. A settling intent that is still the latest play but was
  // overtaken by a suspension must not leave media playing.
  let playSeq = 0;
  let playInflight = 0;
  let suspendSeq = 0;
  let monitored = asMonitoredCanvas(sink);
  const attachMonitor = (): void => {
    monitored?.addEventListener("webglcontextlost", onGraphicsLost);
    monitored?.addEventListener("webglcontextrestored", onGraphicsRestored);
  };
  const detachMonitor = (): void => {
    monitored?.removeEventListener("webglcontextlost", onGraphicsLost);
    monitored?.removeEventListener("webglcontextrestored", onGraphicsRestored);
  };
  const onGraphicsLost = (event: { preventDefault(): void }): void => {
    try {
      event.preventDefault();
    } catch { /* listener doubles may not implement it */ }
    if (disposed) return;
    if (graphics !== "lost") {
      suspendedWhilePlaying = !!clock && !clock.paused;
    }
    graphicsSeq++;
    graphics = "lost";
    graphicsError = "graphics context lost; media paused, capture blocked — call retryGraphics() to rebuild";
    // Suspend outstanding playback intent even when the clock still reports
    // paused: a pending Play completion must not resume media into the loss.
    // A standing user pause with no outstanding intent is left untouched.
    if (clock && (!clock.paused || playInflight > 0)) {
      suspendSeq++;
      // Non-user pause: the INTERMISSION sign must not lower merely
      // because graphics failed.
      transport.pause(false);
    }
  };
  const recoverGraphics = (): Promise<void> => {
    if (graphics === "ready") return Promise.resolve();
    if (recoveryPromise) return recoveryPromise;
    const factory = options.recreateSink ?? null;
    if (!factory) {
      return Promise.reject(
        new Error("graphics_recovery_unavailable: no sink factory supplied; reload the theater"),
      );
    }
    const seq = graphicsSeq;
    recoveryPromise = (async (): Promise<void> => {
      try {
        const fresh = await factory();
        if (disposed || seq !== graphicsSeq) {
          try {
            await fresh.dispose?.();
          } catch { /* replaced-surface teardown is best effort */ }
          throw new Error("stale graphics recovery after newer seek/restart/loss/disposal");
        }
        try {
          sink?.dispose?.();
        } catch { /* replaced-surface teardown is best effort */ }
        detachMonitor();
        sink = fresh;
        monitored = asMonitoredCanvas(sink);
        attachMonitor();
        // Re-render at the retained sample through the existing evaluator;
        // a throwing render keeps graphics suspended (surface not ready).
        try {
          deliver(transport.frame());
        } catch (error) {
          graphicsError = `graphics recovery render failed (${error instanceof Error ? error.message : String(error)}); call retryGraphics() to try again`;
          throw error;
        }
        graphics = "ready";
        graphicsError = null;
      } finally {
        recoveryPromise = null;
      }
    })();
    return recoveryPromise;
  };
  const onGraphicsRestored = (): void => {
    if (disposed || graphics !== "lost") return;
    // Event-driven recovery has no caller: retain a rejection as actionable
    // status instead of an unhandled rejection. Explicit retryGraphics()
    // callers still receive the original rejection from the shared promise.
    recoverGraphics().catch((error: unknown) => {
      graphicsError = error instanceof Error ? error.message : String(error);
    });
  };
  attachMonitor();

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
      if (graphics === "lost") return null;
      return deliver(transport.frame(now));
    },
    async captureFrame(width: number, height: number): Promise<FrameCapture> {
      if (disposed) throw new Error("capture after dispose");
      if (graphics === "lost") {
        throw new Error("graphics_lost: capture blocked while graphics are suspended (retryGraphics)");
      }
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
    play: () => {
      if (graphics === "lost") {
        return Promise.reject(
          new Error("graphics_lost: playback blocked while graphics are suspended (retryGraphics, then Play)"),
        );
      }
      // A suspend/pause/restart/disposal that overtakes this intent owns the
      // media state: when it settles, pause instead of leaving media playing.
      // A newer play owns the state instead, so superseded intents stay silent.
      const mine = ++playSeq;
      const mySuspend = suspendSeq;
      playInflight++;
      const settle = (): void => {
        playInflight--;
        if (mine === playSeq && mySuspend !== suspendSeq && clock && !clock.paused) {
          clock.pause();
        }
      };
      return transport.play().then(
        () => { settle(); },
        (error: unknown) => { settle(); throw error; },
      );
    },
    pause: (userInitiated = true) => {
      suspendSeq++;
      transport.pause(userInitiated);
    },
    seek: (sample: number) => {
      transport.seek(sample);
      graphicsSeq++;
    },
    restart: () => {
      suspendSeq++;
      transport.restart();
      graphicsSeq++;
    },
    setVolume: (volume: number) => transport.setVolume(volume),
    setReadAlong: (enabled: boolean) => {
      transport.setReadAlong(enabled);
      plaque?.setEnabled(enabled);
    },
    dispose: () => {
      if (disposed) return;
      disposed = true;
      suspendSeq++;
      graphicsSeq++;
      transport.dispose();
      plaque?.dispose();
      sink?.dispose?.();
      detachMonitor();
    },
    graphicsStatus: () => ({
      state: graphics,
      error: graphicsError,
      suspendedWhilePlaying,
    }),
    retryGraphics: () => recoverGraphics(),
  };
  void lastState;
  return mount;
}
