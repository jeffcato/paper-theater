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
import type { CompiledTimeline, EvaluatedState, FrameSink } from "../src/index.js";

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

  it("reports webgl_unavailable instead of crashing headless", async () => {
    await assert.rejects(createThreeSink(128, 72), /webgl_unavailable/);
  });
});
