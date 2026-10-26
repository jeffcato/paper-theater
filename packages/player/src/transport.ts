// Transport: one media element drives the delivered master. Read
// currentTime once per rendered frame; derive a clamped integer sample and
// use it for imagery/text. On ended, explicitly evaluate sample_count.
// Seek/load epochs reject stale completions; rewind removes future
// contributors synchronously. INTERMISSION uses UI time with interruptible
// sign movement and latest-intent ownership.
import { SIGN_DOWN_MS, SIGN_UP_MS } from "./profile.js";
import { mediaTimeToSample } from "./convert.js";

export type PauseReason =
  | "user" | "buffering" | "readiness" | "seeking" | "suspended" | "ended" | "playing";
export type SignPose = "hidden" | "lowering" | "visible" | "raising";

export interface TransportEvents {
  onSample?: (sample: number) => void;
  onReason?: (reason: PauseReason) => void;
  onSign?: (pose: SignPose, progress: number) => void;
  onError?: (error: unknown) => void;
}

export interface IntermissionOptions {
  reducedMotion?: boolean;
  downMs?: number;
  upMs?: number;
}

/** Frame scheduler indirection: browsers supply requestAnimationFrame. */
type FrameCallback = (now: number) => void;
const frameScheduler: ((cb: FrameCallback) => number) | undefined =
  typeof (globalThis as { requestAnimationFrame?: unknown }).requestAnimationFrame === "function"
    ? (cb: FrameCallback) =>
      (globalThis as unknown as { requestAnimationFrame: (cb: FrameCallback) => number })
        .requestAnimationFrame(cb)
    : undefined;
function cancelFrame(id: number): void {
  const caf = (globalThis as { cancelAnimationFrame?: unknown }).cancelAnimationFrame;
  if (typeof caf === "function") {
    (globalThis as unknown as { cancelAnimationFrame: (id: number) => void }).cancelAnimationFrame(id);
  }
  void id;
}

/** Interruptible UI-clock sign animation with latest-intent ownership. */
export class IntermissionSign {
  private pose: SignPose = "hidden";
  private progress = 0; // 0 hidden .. 1 fully visible
  private intent = 0;
  private raf = 0;
  private running = false;
  private lastTick = 0;
  private tilt = 0;
  constructor(
    private events: TransportEvents = {},
    private options: IntermissionOptions = {},
  ) {}
  get state(): { pose: SignPose; progress: number } {
    return { pose: this.pose, progress: this.progress };
  }
  /** Lower on explicit user pause; retargets from the current pose. */
  lower(): void {
    const intent = ++this.intent;
    void intent;
    if (this.options.reducedMotion) {
      this.pose = "visible";
      this.progress = 1;
      this.events.onSign?.("visible", 1);
      return;
    }
    this.pose = "lowering";
    this.startLoop();
  }
  /** Retract on resume; latest intent wins over stale callbacks. */
  raise(): void {
    const intent = ++this.intent;
    void intent;
    if (this.options.reducedMotion) {
      this.pose = "hidden";
      this.progress = 0;
      this.events.onSign?.("hidden", 0);
      return;
    }
    this.pose = "raising";
    this.startLoop();
  }
  cancel(): void {
    this.intent++;
    this.running = false;
    cancelFrame(this.raf);
    this.pose = "hidden";
    this.progress = 0;
    this.events.onSign?.("hidden", 0);
  }
  private startLoop(): void {
    if (this.running) return;
    // Without a frame scheduler (non-browser check harness) there is no UI
    // clock to animate on: settle immediately, like reduced motion.
    if (typeof frameScheduler !== "function") {
      this.running = false;
      if (this.pose === "lowering") {
        this.pose = "visible";
        this.progress = 1;
      } else if (this.pose === "raising") {
        this.pose = "hidden";
        this.progress = 0;
      }
      this.events.onSign?.(this.pose, this.progress);
      return;
    }
    this.running = true;
    this.lastTick = performance.now();
    const step = (now: number): void => {
      if (!this.running) return;
      const dt = Math.max(0, now - this.lastTick);
      this.lastTick = now;
      const downMs = this.options.downMs ?? SIGN_DOWN_MS;
      const upMs = this.options.upMs ?? SIGN_UP_MS;
      if (this.pose === "lowering") {
        this.progress = Math.min(1, this.progress + dt / downMs);
        // Bounded deterministic tilt/bounce (<=3 deg / 0.15 units).
        this.tilt = Math.sin(this.progress * Math.PI) * (3 * Math.PI / 180);
        if (this.progress >= 1) {
          this.pose = "visible";
          this.running = false;
        }
      } else if (this.pose === "raising") {
        this.progress = Math.max(0, this.progress - dt / upMs);
        this.tilt = Math.sin(this.progress * Math.PI) * (3 * Math.PI / 180);
        if (this.progress <= 0) {
          this.pose = "hidden";
          this.running = false;
        }
      } else {
        this.running = false;
      }
      this.events.onSign?.(this.pose, this.progress);
      if (this.running) this.raf = frameScheduler(step);
    };
    this.raf = frameScheduler(step);
  }
  get bounce(): number {
    return Math.abs(Math.sin(this.progress * Math.PI * 2)) * 0.15 * (1 - this.progress);
  }
  get tiltRadians(): number {
    return this.tilt;
  }
}

export interface MountHandle {
  play(): Promise<void>;
  pause(userInitiated?: boolean): void;
  seek(sample: number): void;
  restart(): void;
  setVolume(volume: number): void;
  setReadAlong(enabled: boolean): void;
  dispose(): void;
  readonly epoch: number;
  readonly reason: PauseReason;
  readonly sample: number;
}

interface ClockSource {
  readonly currentTime: number;
  readonly paused: boolean;
  readonly seeking: boolean;
  readonly ended: boolean;
  play(): Promise<void>;
  pause(): void;
}

/**
 * Mount a transport to a clock source (HTMLAudioElement in the browser; a
 * test double in checks). One render scheduler lives in the renderer; the
 * transport only samples the clock once per frame via frame().
 */
export function createTransport(
  sampleCount: number,
  clock: ClockSource | null,
  events: TransportEvents = {},
  signOptions: IntermissionOptions = {},
): MountHandle & { frame(now?: number): number; sign: IntermissionSign } {
  let epoch = 0;
  let reason: PauseReason = "readiness";
  let sample = 0;
  let disposed = false;
  let playIntent = 0;
  const sign = new IntermissionSign(events, signOptions);
  let readAlong = false;
  void readAlong;
  let volume = 1;
  void volume;

  const setReason = (next: PauseReason): void => {
    if (reason === next) return;
    reason = next;
    events.onReason?.(next);
  };
  const emitSample = (next: number): void => {
    const clamped = Math.max(0, Math.min(sampleCount, next));
    if (clamped !== sample) {
      sample = clamped;
      events.onSample?.(sample);
    }
  };

  const frame = (_now = 0): number => {
    if (disposed) return sample;
    if (!clock) return sample;
    if (clock.seeking) {
      setReason("seeking");
      return sample;
    }
    if (clock.ended) {
      // On ended, explicitly evaluate sample_count.
      emitSample(sampleCount);
      setReason("ended");
      return sample;
    }
    // Read currentTime once per rendered frame.
    const next = mediaTimeToSample(clock.currentTime, sampleCount);
    // Rewind removes future contributors synchronously: the evaluator
    // reconstructs from the destination sample, so just publish it.
    emitSample(next);
    if (clock.paused) {
      if (reason === "playing") setReason("readiness");
    } else {
      setReason("playing");
    }
    return sample;
  };

  const handle: MountHandle & { frame(now?: number): number; sign: IntermissionSign } = {
    sign,
    frame,
    get epoch() { return epoch; },
    get reason() { return reason; },
    get sample() { return sample; },
    async play(): Promise<void> {
      if (disposed) return;
      const intent = ++playIntent;
      if (sample >= sampleCount) {
        epoch++;
        emitSample(0);
      }
      if (!clock) {
        setReason("playing");
        sign.raise();
        return;
      }
      try {
        await clock.play();
        if (disposed || intent !== playIntent) return;
        setReason("playing");
        sign.raise();
      } catch (error) {
        // A failed resume preserves the sign/retry state.
        if (disposed || intent !== playIntent) return;
        setReason(reason === "user" ? "user" : "readiness");
        events.onError?.(error);
        throw error;
      }
    },
    pause(userInitiated = true): void {
      if (disposed) return;
      playIntent++;
      clock?.pause();
      epoch++;
      if (userInitiated) {
        // Explicit user pause lowers the sign even mid-transition.
        setReason("user");
        sign.lower();
      } else {
        setReason("readiness");
      }
      emitSample(sample);
    },
    seek(target: number): void {
      if (disposed) return;
      if (!Number.isSafeInteger(target) || target < 0 || target > sampleCount) {
        throw new Error(`Seek out of bounds: ${target}`);
      }
      const myEpoch = ++epoch;
      setReason("seeking");
      // Synchronous cleanup: publish the destination immediately so future
      // contributors are removed before async media/texture work completes.
      emitSample(target);
      if (clock && "currentTime" in clock) {
        try {
          (clock as { currentTime: number }).currentTime = target / 48000;
        } catch (error) {
          events.onError?.(error);
        }
      }
      // Stale completions check their epoch before applying.
      queueMicrotask(() => {
        if (disposed || myEpoch !== epoch) return;
        setReason(clock?.paused === false ? "playing" : "seeking");
      });
    },
    restart(): void {
      if (disposed) return;
      playIntent++;
      sign.cancel();
      epoch++;
      emitSample(0);
      setReason("readiness");
    },
    setVolume(v: number): void {
      if (!Number.isFinite(v) || v < 0 || v > 1) throw new Error("Volume must be 0..1");
      volume = v;
    },
    setReadAlong(enabled: boolean): void {
      readAlong = enabled;
    },
    dispose(): void {
      disposed = true;
      playIntent++;
      epoch++;
      sign.cancel();
    },
  };
  return handle;
}
