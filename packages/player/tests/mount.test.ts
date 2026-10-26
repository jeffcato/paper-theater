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
