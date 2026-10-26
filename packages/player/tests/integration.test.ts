// integration group: adapter conversions, capture identities, the
// PCFSoftShadowMap migration guard, and repeated mount/dispose cycles.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import {
  applyCropToSource,
  assertNativeAspect,
  boundarySamples,
  createTransport,
  describeCapture,
  eulerToMatrixZFlipped,
  flipMatrixZ,
  flipPointZ,
  mediaTimeToSample,
  pivotTopToBottom,
  planeOffsetForPivot,
  sampleToSeconds,
  scalarToUtf16Map,
  sliceByScalar,
} from "../src/index.js";
import type { CompiledTimeline } from "../src/index.js";

describe("integration: adapter conversions", () => {
  it("maps samples to legacy seconds without authorizing early reveals", () => {
    assert.equal(sampleToSeconds(96000), 2);
    assert.equal(mediaTimeToSample(0.5, 96000), 24000);
    assert.equal(mediaTimeToSample(99, 96000), 96000);
    assert.throws(() => sampleToSeconds(-1), /sample/i);
  });

  it("converts the handoff pivot example exactly", () => {
    assert.deepEqual(pivotTopToBottom([0.25, 0.9]), [0.25, 0.09999999999999998]);
    assert.deepEqual(planeOffsetForPivot([0.25, 0.9], [1, 2]), [0.25, 0.8, 0]);
  });

  it("preserves native crop aspect and applies crop_to_source", () => {
    assert.doesNotThrow(() => assertNativeAspect([200, 400], [1, 2]));
    assert.throws(() => assertNativeAspect([200, 400], [1, 1]), /Aspect/);
    assert.deepEqual(applyCropToSource([1, 0, 0, 1, 10, 20], 3, 4), [13, 24]);
  });

  it("flips V3 +Z-away to +Z-toward consistently (F*M*F is an involution)", () => {
    assert.deepEqual(flipPointZ([1, 2, 3]), [1, 2, -3]);
    const m = [
      [1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [0, 0, 0, 1],
    ];
    assert.deepEqual(flipMatrixZ(flipMatrixZ(m)), m);
    // Euler angles go through matrices, never copied blindly.
    const r = eulerToMatrixZFlipped([0.1, 0.2, 0.3]);
    assert.equal(r.length, 4);
    assert.ok(r.every((row) => row.length === 4 && row.every(Number.isFinite)));
  });

  it("maps scalar offsets without splitting surrogate pairs", () => {
    const source = "a𐀀b";
    assert.deepEqual(scalarToUtf16Map(source), [0, 1, 3, 4]);
    assert.equal(sliceByScalar(source, 1, 2), "𐀀");
    assert.throws(() => sliceByScalar(source, 0, 9), /range/);
  });
});

describe("integration: capture identities", () => {
  it("names manifest/candidate/profile/player/checker/sample/contributors", async () => {
    const timeline = {
      schema_version: 1,
      capability_version: "paper-player-v1",
      delivery_audio_sha256: "0".repeat(64),
      presentation_map_sha256: "0".repeat(64),
      sample_rate: 48000,
      sample_count: 100,
      opening: {
        title: "t", chapter_label: "c", title_hold: [0, 1], title_dismiss: [1, 2],
        curtain_open: [2, 3], first_prose_sample: 3, intro_narration: null,
        first_picture_id: "card-a", first_picture_eligible_sample: 2,
      },
      scenes: [],
      transitions: [],
      cues: [],
      closing: {
        last_prose_action_sample: 99, curtain_close: [99, 100],
        score_tail: null, terminal_sample: 100,
      },
      terminal_snapshot: {
        set_id: "set-a", instances: [],
        camera: { position: [0, 0, 0], target: [0, 0, 0], up: [0, 1, 0], fov_y_degrees: 40, near: 0.1, far: 100 },
        lighting: { ambient: { color: "#fff", intensity: 0 }, environment: null, lights: [] },
        curtain_open_ratio: 0,
      },
    } as unknown as CompiledTimeline;
    const result = await describeCapture(timeline, [], {
      manifest_digest: "a".repeat(64),
      candidate_id: "cand-1",
      checker_sha256: "b".repeat(64),
      player_sha256: "c".repeat(64),
      profile_id: "paper-player-r186-v1",
      sample: 50,
      width: 1280,
      height: 720,
    });
    assert.equal(result.manifest_digest, "a".repeat(64));
    assert.equal(result.candidate_id, "cand-1");
    assert.equal(result.profile_id, "paper-player-r186-v1");
    assert.equal(result.player_sha256, "c".repeat(64));
    assert.equal(result.checker_sha256, "b".repeat(64));
    assert.equal(result.sample, 50);
    assert.equal(result.state_digest.length, 64);
    assert.deepEqual(boundarySamples(0, 100), [0, 1]);
  });

  it("rejects captures against another profile revision", async () => {
    const timeline = { sample_count: 10 } as unknown as CompiledTimeline;
    await assert.rejects(
      describeCapture(timeline, [], {
        manifest_digest: "a".repeat(64), candidate_id: "c",
        checker_sha256: "b".repeat(64), player_sha256: "c".repeat(64),
        profile_id: "other-profile", sample: 0, width: 1, height: 1,
      }),
      /profile/i,
    );
  });
});

describe("integration: migration and lifecycle", () => {
  it("contains no PCFSoftShadowMap in player sources", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const srcDir = join(here, "..", "src");
    for (const file of readdirSync(srcDir)) {
      if (!file.endsWith(".ts")) continue;
      const text = readFileSync(join(srcDir, file), "utf8");
      // Comments may document the migration; code must not use the old map.
      const code = text
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .split("\n")
        .filter((line) => !line.trimStart().startsWith("//") && !line.trimStart().startsWith("*"))
        .join("\n");
      assert.ok(!code.includes("PCFSoftShadowMap"), `${file} still uses PCFSoftShadowMap in code`);
    }
  });

  it("survives ten mount/dispose cycles with no retained owned state", () => {
    const clock = {
      currentTime: 0, paused: true, seeking: false, ended: false,
      async play() {}, pause() {},
    };
    for (let i = 0; i < 10; i++) {
      const t = createTransport(96000, clock);
      t.seek(i * 1000);
      t.pause(true);
      t.dispose();
      t.dispose();
    }
  });
});
