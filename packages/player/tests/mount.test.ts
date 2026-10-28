// mount group (runs under check:player -- integration): diagnostic mounting
// binds a loaded handle to transport + evaluator + an injected frame sink.
// Uses existing adapters (toCueViewcue normalization, validateTimeline).
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import {
  createThreeSink,
  loadDiagnostic,
  mountPlayer,
  requireAccepted,
  sha256Hex,
} from "../src/index.js";
import type { CompiledTimeline, EvaluatedState, FrameSink, ReadAlongGroup, ReadAlongWord } from "../src/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const ownedRoot = join(here, "..", "..", "..", "..", "fixtures", "player");
const readOwned = (p: string) => readFileSync(join(ownedRoot, p));

const policy = {
  player_sha256: "0".repeat(64),
  checker_sha256: "0".repeat(64),
  recipe_sha256: "0".repeat(64),
  allow_diagnostic: false,
};

function fakeClock() {
  return {
    currentTime: 0,
    paused: true,
    seeking: false,
    ended: false,
    async play() {
      this.paused = false;
    },
    pause() {
      this.paused = true;
    },
  };
}

function recordingSink(): FrameSink & { states: EvaluatedState[]; disposed: boolean } {
  return {
    states: [],
    disposed: false,
    render(state: EvaluatedState) {
      if (this.disposed) throw new Error("render after dispose");
      this.states.push(state);
    },
    dispose() {
      this.disposed = true;
    },
  };
}

async function diagnosticMount() {
  const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
  const digest = await sha256Hex(manifestBytes);
  const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
  const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
  const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
  const clock = fakeClock();
  const sink = recordingSink();
  const mount = mountPlayer({}, { handle, timeline }, { clock, sink });
  return { mount, clock, sink, timeline };
}

describe("mount: diagnostic lifecycle", () => {
  it("flags diagnostic handles and validates the timeline through adapters", async () => {
    const { mount } = await diagnosticMount();
    assert.equal(mount.diagnostic, true);
    assert.equal(mount.accepted, false);
    mount.dispose();
  });

  it("delivers evaluated samples to the sink on tick, seek and dispose", async () => {
    const { mount, clock, sink } = await diagnosticMount();
    clock.currentTime = 0.5;
    mount.tick();
    assert.ok(sink.states.length >= 1);
    assert.equal(sink.states.at(-1)?.sample, 24000);
    mount.seek(48000);
    mount.tick();
    assert.equal(sink.states.at(-1)?.sample, 48000);
    const delivered = sink.states.length;
    mount.pause();
    assert.equal(mount.reason, "user");
    mount.dispose();
    mount.tick();
    assert.equal(sink.states.length, delivered);
    assert.equal(sink.disposed, true);
  });

  it("disposed mounts stay silent across play/pause/seek/restart/capture", async () => {
    const { mount, sink } = await diagnosticMount();
    mount.tick();
    const delivered = sink.states.length;
    const at = mount.sample;
    mount.dispose();
    await mount.play();
    mount.pause();
    mount.seek(100);
    mount.restart();
    assert.equal(mount.tick(), null);
    await assert.rejects(mount.captureFrame(960, 540), /after dispose/);
    assert.equal(sink.states.length, delivered);
    assert.equal(mount.sample, at);
    mount.dispose();
  });

  it("rejects unsupported cues at mount time, never mid-playback", async () => {
    const { mount, sink } = await diagnosticMount();
    mount.dispose();
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const bad = {
      ...timeline,
      cues: [...timeline.cues, { id: "neg", scene_id: "scene-a", target_id: "x", action: "hop", start_sample: 1, end_sample: 2, easing: "linear", reveal_group_ids: [], params: {} }],
    } as unknown as CompiledTimeline;
    assert.throws(() => mountPlayer({}, { handle, timeline: bad }, { sink }), /unsupported action/);
    void sink;
  });
});

describe("mount: actual frame capture", () => {
  function capturingSink(pixels: Uint8Array): FrameSink & { states: EvaluatedState[] } {
    return {
      states: [],
      render(state: EvaluatedState) {
        this.states.push(state);
      },
      capturePixels(_width: number, _height: number) {
        return pixels;
      },
    };
  }

  it("stamps captures with manifest/candidate/sample/provenance identities", async () => {
    const pixels = new Uint8Array([0, 0, 0, 255, 10, 20, 30, 255]);
    // Mount with a capturing sink for the capture assertions.
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const capturing = mountPlayer({}, { handle, timeline }, {
      sink: capturingSink(pixels),
      candidate_id: "cand-diagnostic-1",
      checker_sha256: "b".repeat(64),
      player_sha256: "c".repeat(64),
    });
    capturing.seek(48000);
    capturing.tick();
    const frame = await capturing.captureFrame(2, 1);
    assert.equal(frame.manifest_digest, digest);
    assert.equal(frame.candidate_id, "cand-diagnostic-1");
    assert.equal(frame.profile_id, "paper-player-r186-v1");
    assert.equal(frame.provenance, "diagnostic");
    assert.equal(frame.accepted, false);
    assert.equal(frame.sample, 48000);
    assert.equal(frame.state_digest.length, 64);
    assert.deepEqual(frame.pixels, pixels);
    assert.ok(Array.isArray(frame.contributor_ids));
    capturing.dispose();
  });

  it("fails closed without a pixel sink and after dispose", async () => {
    const { mount } = await diagnosticMount();
    await assert.rejects(mount.captureFrame(2, 1), /not_ready/);
    await assert.rejects(mount.captureFrame(0, 1), /bounds/);
    mount.dispose();
    await assert.rejects(mount.captureFrame(2, 1), /dispose/);
  });

  it("bars diagnostic handles from the production capture gate", async () => {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    assert.throws(() => requireAccepted(handle), /provenance/);
  });

  function plaqueTrack(): { words: ReadAlongWord[]; groups: ReadAlongGroup[] } {
    return {
      words: [
        { id: "w0", source_span: [0, 5], text: "hush", following_text: " ", start_sample: 0, end_sample: 24000 },
        { id: "w1", source_span: [5, 8], text: "now", following_text: ".", start_sample: 24000, end_sample: 48000 },
        { id: "w2", source_span: [8, 14], text: "listen", following_text: ".", start_sample: 48000, end_sample: 96000 },
      ],
      groups: [
        { id: "g0", word_ids: ["w0"], timing_owner_word_id: "w0", start_sample: 0, end_sample: 24000 },
        { id: "g1", word_ids: ["w1"], timing_owner_word_id: "w1", start_sample: 24000, end_sample: 48000 },
        { id: "g2", word_ids: ["w2"], timing_owner_word_id: "w2", start_sample: 48000, end_sample: 96000 },
      ],
    };
  }

  function fakeElement(): Record<string, unknown> & {
    children: unknown[];
    dataset: Record<string, string>;
    style: Record<string, string>;
    attrs: Map<string, string>;
  } {
    const el = {
      children: [] as unknown[],
      dataset: {} as Record<string, string>,
      style: {} as Record<string, string>,
      attrs: new Map<string, string>(),
      removed: false,
      clientWidth: 640,
      textContent: "",
      classList: { toggle() {}, add() {}, remove() {} },
      setAttribute(this: unknown, k: string, v: string) {
        (this as typeof el).attrs.set(k, v);
      },
      removeAttribute(this: unknown, k: string) {
        (this as typeof el).attrs.delete(k);
      },
      appendChild(this: unknown, child: unknown) {
        (this as typeof el).children.push(child);
        return child;
      },
      replaceChildren(this: unknown) {
        (this as typeof el).children = [];
      },
      remove(this: unknown) {
        (this as typeof el).removed = true;
      },
      ownerDocument: null as unknown,
    };
    el.ownerDocument = { createElement: () => fakeElement(), fonts: undefined };
    return el;
  }

  function plaqueSpans(container: { children: unknown[] }): Array<{ dataset: Record<string, string>; attrs: Map<string, string> }> {
    const chunk = container.children[0] as { children: unknown[] } | undefined;
    if (!chunk) return [];
    return chunk.children as Array<{ dataset: Record<string, string>; attrs: Map<string, string> }>;
  }

  it("mounts a verified plaque and feeds it the delivered sample", async () => {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const { words, groups } = plaqueTrack();
    const container = fakeElement();
    const mount = mountPlayer({}, { handle, timeline }, {
      clock: fakeClock(),
      sink: recordingSink(),
      readAlong: { words, groups, container: container as unknown as HTMLElement, measure: (t) => t.length * 10 },
    });
    assert.ok(mount.plaque, "mount exposes its plaque");
    mount.seek(30000);
    mount.tick();
    const active = plaqueSpans(container).filter((s) => s.attrs.get("aria-current") === "true");
    assert.equal(active.length, 1);
    assert.equal(active[0]!.dataset.groupId, "g1");
    mount.dispose();
  });

  it("rejects unverified track data before touching the plaque container", async () => {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const { words, groups } = plaqueTrack();
    const bad = groups.map((g) => ({ ...g }));
    bad[1]!.end_sample = -1;
    const container = fakeElement();
    assert.throws(
      () => mountPlayer({}, { handle, timeline }, {
        clock: fakeClock(),
        sink: recordingSink(),
        readAlong: { words, groups: bad, container: container as unknown as HTMLElement, measure: (t) => t.length * 10 },
      }),
      /interval/,
    );
    assert.deepEqual(container.children, []);
  });

  it("forwards the pause-driven sign to the sink", async () => {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const signCalls: Array<{ pose: string; progress: number }> = [];
    const sink = recordingSink() as FrameSink & { states: EvaluatedState[]; disposed: boolean };
    (sink as unknown as { setSign: (pose: string, progress: number) => void }).setSign = (pose, progress) => {
      signCalls.push({ pose, progress });
    };
    const mount = mountPlayer({}, { handle, timeline }, { clock: fakeClock(), sink });
    mount.pause();
    assert.deepEqual(signCalls.at(-1), { pose: "visible", progress: 1 });
    await mount.play();
    assert.deepEqual(signCalls.at(-1), { pose: "hidden", progress: 0 });
    mount.pause();
    mount.restart();
    assert.deepEqual(signCalls.at(-1), { pose: "hidden", progress: 0 });
    mount.dispose();
  });

  it("controls the plaque through setReadAlong and releases it on dispose", async () => {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const { words, groups } = plaqueTrack();
    const container = fakeElement();
    const mount = mountPlayer({}, { handle, timeline }, {
      clock: fakeClock(),
      sink: recordingSink(),
      readAlong: { words, groups, container: container as unknown as HTMLElement, measure: (t) => t.length * 10 },
    });
    assert.ok(plaqueSpans(container).length > 0);
    mount.setReadAlong(false);
    assert.equal(container.style.display, "none");
    assert.deepEqual(plaqueSpans(container), []);
    mount.setReadAlong(true);
    assert.equal(container.style.display, "");
    assert.ok(plaqueSpans(container).length > 0);
    mount.dispose();
    const chunk = container.children[0] as { removed?: boolean; children: unknown[] };
    assert.equal(chunk.removed, true);
    assert.deepEqual(chunk.children, []);
  });

  it("reports webgl_unavailable instead of crashing headless", async () => {
    await assert.rejects(createThreeSink(128, 72), /webgl_unavailable/);
  });
});

describe("mount: graphics loss and recovery", () => {
  interface FakeCanvasEvent {
    preventDefault(): void;
  }
  interface FakeCanvas {
    listeners: Map<string, Array<(event: FakeCanvasEvent) => void>>;
    addEventListener(type: string, listener: (event: FakeCanvasEvent) => void): void;
    removeEventListener(type: string, listener: (event: FakeCanvasEvent) => void): void;
    fire(type: string): { prevented: boolean };
  }
  function fakeCanvas(): FakeCanvas {
    const listeners = new Map<string, Array<(event: FakeCanvasEvent) => void>>();
    return {
      listeners,
      addEventListener(type, listener) {
        listeners.set(type, [...(listeners.get(type) ?? []), listener]);
      },
      removeEventListener(type, listener) {
        listeners.set(type, (listeners.get(type) ?? []).filter((l) => l !== listener));
      },
      fire(type) {
        let prevented = false;
        for (const l of listeners.get(type) ?? []) {
          l({ preventDefault: () => { prevented = true; } });
        }
        return { prevented };
      },
    };
  }

  interface GraphicsSink extends FrameSink {
    renders: number;
    disposed: boolean;
  }
  function graphicsSink(canvas: FakeCanvas): GraphicsSink {
    return {
      canvas,
      renders: 0,
      disposed: false,
      render() {
        if (this.disposed) throw new Error("render after dispose");
        this.renders++;
      },
      dispose() {
        this.disposed = true;
      },
    };
  }

  /** Clock whose play() stays pending until released (browser autoplay-policy shape). */
  function deferredClock() {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    return {
      currentTime: 0,
      paused: true,
      seeking: false,
      ended: false,
      async play() {
        await gate;
        this.paused = false;
      },
      pause() {
        this.paused = true;
      },
      release,
    };
  }

  /** Collect unhandled rejections across a bounded flush; the repair must leave none. */
  async function withoutUnhandled(fn: () => Promise<void> | void): Promise<unknown[]> {
    const seen: unknown[] = [];
    const listener = (reason: unknown) => { seen.push(reason); };
    process.on("unhandledRejection", listener);
    try {
      await fn();
      await new Promise((resolve) => setTimeout(resolve, 25));
    } finally {
      process.removeListener("unhandledRejection", listener);
    }
    return seen;
  }

  async function graphicsMount(factory?: () => Promise<FrameSink>, rawOverride?: ReturnType<typeof fakeClock>) {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const raw = rawOverride ?? fakeClock();
    let plays = 0;
    const clock = {
      get currentTime(): number {
        return raw.currentTime;
      },
      set currentTime(v: number) {
        raw.currentTime = v;
      },
      get paused(): boolean {
        return raw.paused;
      },
      get seeking(): boolean {
        return raw.seeking;
      },
      get ended(): boolean {
        return raw.ended;
      },
      async play(): Promise<void> {
        plays++;
        return raw.play();
      },
      pause(): void {
        return raw.pause();
      },
    };
    const canvas = fakeCanvas();
    const created: GraphicsSink[] = [graphicsSink(canvas)];
    const reasons: string[] = [];
    const poses: string[] = [];
    const errors: unknown[] = [];
    const mount = mountPlayer({}, { handle, timeline }, {
      clock,
      sink: created[0]!,
      recreateSink: factory ?? (async () => {
        const fresh = graphicsSink(canvas);
        created.push(fresh);
        return fresh;
      }),
      events: {
        onReason: (r) => reasons.push(r),
        onSign: (pose) => poses.push(pose),
        onError: (e) => errors.push(e),
      },
    });
    return {
      mount, clock: raw, canvas, created, reasons, poses, errors,
      plays: () => plays,
      status: () => mount.graphicsStatus(),
    };
  }

  it("loss while playing pauses media without lowering the sign and blocks delivery", async () => {
    const g = await graphicsMount();
    g.clock.currentTime = 0.5;
    g.mount.tick();
    assert.equal(g.mount.sample, 24000);
    await g.mount.play();
    assert.equal(g.mount.reason, "playing");
    const renders = g.created[0]!.renders;
    assert.ok(renders > 0);
    const posesAfterPlay = g.poses.length;
    const lost = g.canvas.fire("webglcontextlost");
    assert.equal(lost.prevented, true);
    assert.equal(g.clock.paused, true);
    assert.equal(g.mount.reason, "readiness");
    assert.equal(g.mount.sample, 24000);
    // The loss itself lowers nothing: pose traffic is frozen at the raise.
    assert.equal(g.poses.length, posesAfterPlay);
    assert.equal(g.poses[g.poses.length - 1], "hidden");
    assert.equal(g.mount.tick(), null);
    assert.equal(g.created[0]!.renders, renders);
    await assert.rejects(g.mount.captureFrame(960, 540), /graphics_lost/);
    assert.deepEqual(g.status(), { state: "lost", error: g.status().error, suspendedWhilePlaying: true });
    assert.ok((g.status().error ?? "").length > 0);
    g.mount.dispose();
  });

  it("loss while user-paused keeps the pause intent and the visible sign", async () => {
    const g = await graphicsMount();
    g.mount.pause(true);
    assert.equal(g.mount.reason, "user");
    g.canvas.fire("webglcontextlost");
    assert.equal(g.mount.reason, "user");
    assert.equal(g.poses[g.poses.length - 1], "visible");
    assert.deepEqual(g.status().state, "lost");
    assert.equal(g.status().suspendedWhilePlaying, false);
    g.mount.dispose();
  });

  it("restored event rebuilds a fresh surface at the retained sample", async () => {
    const g = await graphicsMount();
    g.clock.currentTime = 0.5;
    g.mount.tick();
    await g.mount.play();
    g.canvas.fire("webglcontextlost");
    g.canvas.fire("webglcontextrestored");
    await g.mount.retryGraphics();
    assert.equal(g.created.length, 2);
    assert.equal(g.created[0]!.disposed, true);
    assert.ok(g.created[1]!.renders > 0);
    assert.equal(g.mount.sample, 24000);
    assert.notEqual(g.mount.tick(), null);
    assert.deepEqual(g.status().state, "ready");
    assert.equal(g.status().error, null);
    // Recovery never autoplays: explicit Play still required.
    assert.equal(g.mount.reason, "readiness");
    assert.equal(g.clock.paused, true);
    await g.mount.play();
    assert.equal(g.mount.reason, "playing");
    // One listener pair per canvas, not doubled by the swap.
    assert.equal(g.canvas.listeners.get("webglcontextlost")?.length, 1);
    assert.equal(g.canvas.listeners.get("webglcontextrestored")?.length, 1);
    g.mount.dispose();
  });

  it("retryGraphics while ready resolves without rebuilding", async () => {
    const g = await graphicsMount();
    await g.mount.retryGraphics();
    assert.equal(g.created.length, 1);
    assert.deepEqual(g.status().state, "ready");
    g.mount.dispose();
  });

  it("retryGraphics without a factory rejects with a reload path", async () => {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const canvas = fakeCanvas();
    const mount = mountPlayer({}, { handle, timeline }, { clock: fakeClock(), sink: graphicsSink(canvas) });
    canvas.fire("webglcontextlost");
    await assert.rejects(mount.retryGraphics(), /graphics_recovery_unavailable/);
    assert.deepEqual(mount.graphicsStatus().state, "lost");
    mount.dispose();
  });

  it("seek during in-flight recovery rejects stale and retries at the new sample", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const g = await graphicsMount(async () => {
      await gate;
      const fresh = graphicsSink(g.canvas);
      g.created.push(fresh);
      return fresh;
    });
    g.clock.currentTime = 0.5;
    g.mount.tick();
    await g.mount.play();
    g.canvas.fire("webglcontextlost");
    g.canvas.fire("webglcontextrestored");
    g.mount.seek(48000);
    release();
    await assert.rejects(g.mount.retryGraphics(), /stale/);
    assert.deepEqual(g.status().state, "lost");
    assert.equal(g.created[1]!.disposed, true);
    await g.mount.retryGraphics();
    assert.deepEqual(g.status().state, "ready");
    assert.equal(g.mount.sample, 48000);
    assert.ok(g.created[2]!.renders > 0);
    g.mount.dispose();
  });

  it("restart during in-flight recovery also goes stale", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const g = await graphicsMount(async () => {
      await gate;
      const fresh = graphicsSink(g.canvas);
      g.created.push(fresh);
      return fresh;
    });
    g.mount.tick();
    g.canvas.fire("webglcontextlost");
    g.canvas.fire("webglcontextrestored");
    g.mount.restart();
    release();
    await assert.rejects(g.mount.retryGraphics(), /stale/);
    assert.equal(g.mount.sample, 0);
    await g.mount.retryGraphics();
    assert.deepEqual(g.status().state, "ready");
    g.mount.dispose();
  });

  it("dispose during in-flight recovery stays silent with listeners removed", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const g = await graphicsMount(async () => {
      await gate;
      const fresh = graphicsSink(g.canvas);
      g.created.push(fresh);
      return fresh;
    });
    g.mount.tick();
    g.canvas.fire("webglcontextlost");
    g.canvas.fire("webglcontextrestored");
    g.mount.dispose();
    // Teardown hides the sign exactly once; everything after is silent.
    assert.deepEqual(g.poses, ["hidden"]);
    const reasons = g.reasons.length;
    const poses = g.poses.length;
    release();
    await assert.rejects(g.mount.retryGraphics(), /stale|disposed/);
    assert.equal(g.mount.tick(), null);
    assert.equal(g.reasons.length, reasons);
    assert.equal(g.poses.length, poses);
    assert.equal(g.created[1]!.disposed, true);
    assert.deepEqual(g.canvas.listeners.get("webglcontextlost"), []);
    assert.deepEqual(g.canvas.listeners.get("webglcontextrestored"), []);
  });

  it("play while lost throws actionable without touching the clock", async () => {
    const g = await graphicsMount();
    g.mount.tick();
    g.canvas.fire("webglcontextlost");
    await assert.rejects(g.mount.play(), /graphics_lost/);
    assert.equal(g.plays(), 0);
    assert.equal(g.clock.paused, true);
    g.mount.dispose();
  });

  it("loss during pending Play leaves media paused with graphics lost", async () => {
    const raw = deferredClock();
    const g = await graphicsMount(undefined, raw);
    const play = g.mount.play();
    g.canvas.fire("webglcontextlost");
    raw.release();
    await play;
    // The pre-loss intent completes into a suspension, never into playback.
    assert.equal(g.clock.paused, true);
    assert.deepEqual(g.status().state, "lost");
    assert.equal(g.mount.reason, "readiness");
    // Graphics failure lowers nothing as a new user pause.
    assert.deepEqual(g.poses, []);
    // Recovery still requires an explicit new Play.
    await assert.rejects(g.mount.play(), /graphics_lost/);
    g.mount.dispose();
  });

  it("loss during pending Play after a user pause keeps media paused and the sign", async () => {
    const raw = deferredClock();
    const g = await graphicsMount(undefined, raw);
    g.mount.pause(true);
    assert.equal(g.poses[g.poses.length - 1], "visible");
    const play = g.mount.play();
    g.canvas.fire("webglcontextlost");
    raw.release();
    await play;
    assert.equal(g.clock.paused, true);
    assert.deepEqual(g.status().state, "lost");
    assert.equal(g.poses[g.poses.length - 1], "visible");
    g.mount.dispose();
  });

  it("pending Play stays suspended until an explicit new Play after recovery", async () => {
    const raw = deferredClock();
    const g = await graphicsMount(undefined, raw);
    const play = g.mount.play();
    g.canvas.fire("webglcontextlost");
    raw.release();
    await play;
    g.canvas.fire("webglcontextrestored");
    await g.mount.retryGraphics();
    assert.deepEqual(g.status().state, "ready");
    // No autoplay from the pre-loss intent: still paused until Play.
    assert.equal(g.clock.paused, true);
    assert.equal(g.mount.reason, "readiness");
    await g.mount.play();
    assert.equal(g.mount.reason, "playing");
    assert.equal(g.clock.paused, false);
    g.mount.dispose();
  });

  it("restored event without a factory retains the error with no unhandled rejection", async () => {
    const manifestBytes = readOwned("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const acceptanceBytes = readOwned("two-scene-local/acceptance.json");
    const handle = await loadDiagnostic(manifestBytes, digest, async () => acceptanceBytes, policy);
    const timeline = JSON.parse(readOwned("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    const canvas = fakeCanvas();
    const mount = mountPlayer({}, { handle, timeline }, { clock: fakeClock(), sink: graphicsSink(canvas) });
    canvas.fire("webglcontextlost");
    const unhandled = await withoutUnhandled(() => {
      canvas.fire("webglcontextrestored");
    });
    assert.deepEqual(unhandled, []);
    assert.deepEqual(mount.graphicsStatus().state, "lost");
    assert.match(mount.graphicsStatus().error ?? "", /graphics_recovery_unavailable|reload/);
    mount.dispose();
  });

  it("restored event with a rejecting factory keeps status silent and explicit retry rejects", async () => {
    const g = await graphicsMount(async () => {
      throw new Error("factory boom");
    });
    g.canvas.fire("webglcontextlost");
    const unhandled = await withoutUnhandled(() => {
      g.canvas.fire("webglcontextrestored");
    });
    assert.deepEqual(unhandled, []);
    assert.deepEqual(g.status().state, "lost");
    assert.match(g.status().error ?? "", /factory boom/);
    await assert.rejects(g.mount.retryGraphics(), /factory boom/);
    g.mount.dispose();
  });

  it("dispose during unjoined event-driven recovery stays silent", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const g = await graphicsMount(async () => {
      await gate;
      const fresh = graphicsSink(g.canvas);
      g.created.push(fresh);
      return fresh;
    });
    g.mount.tick();
    g.canvas.fire("webglcontextlost");
    const unhandled = await withoutUnhandled(() => {
      g.canvas.fire("webglcontextrestored");
      g.mount.dispose();
      release();
    });
    assert.deepEqual(unhandled, []);
    await assert.rejects(g.mount.retryGraphics(), /stale|disposed/);
  });
});
