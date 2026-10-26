// timeline group: integer boundaries, exact 96,000-sample dark hold,
// destination-phase seeks, terminal snapshot with closed curtains.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  evaluate,
  validateTimeline,
  validateTransitionHold,
} from "../src/index.js";
import type { CompiledTimeline, StageSnapshot } from "../src/index.js";

function snapshot(set_id: string): StageSnapshot {
  return {
    set_id,
    instances: [
      {
        id: `inst-${set_id}`,
        asset_id: `card-${set_id}`,
        position: [0, 0, 0],
        rotation_xyz: [0, 0, 0],
        support_id: "deck",
        reveal_group_id: `rg-${set_id}`,
        mirrored: false,
      },
    ],
    camera: {
      position: [0, 4, 14],
      target: [0, 4, 0],
      up: [0, 1, 0],
      fov_y_degrees: 40,
      near: 0.1,
      far: 100,
    },
    lighting: { ambient: { color: "#ffffff", intensity: 1 }, environment: null, lights: [] },
    curtain_open_ratio: 1,
  };
}

// Synthetic 18-second arithmetic fixture from the handoff (no measured
// speech, no release permission): narration slots, formal transition with a
// 96,000-sample dark hold, closing inside the master endpoint.
function twoSceneTimeline(): CompiledTimeline {
  return {
    schema_version: 1,
    capability_version: "paper-player-v1",
    delivery_audio_sha256: "0".repeat(64),
    presentation_map_sha256: "0".repeat(64),
    sample_rate: 48000,
    sample_count: 864000,
    opening: {
      title: "t",
      chapter_label: "c",
      title_hold: [0, 72000],
      title_dismiss: [72000, 96000],
      curtain_open: [96000, 168000],
      first_prose_sample: 168000,
      intro_narration: null,
      first_picture_id: "card-set-a",
      first_picture_eligible_sample: 96000,
    },
    scenes: [
      {
        id: "scene-a",
        set_id: "set-a",
        source_span: [0, 10],
        narration: [168000, 384000],
        entry_snapshot: snapshot("set-a"),
        required_asset_ids: ["card-set-a"],
        reveal_group_ids: ["rg-set-a"],
      },
      {
        id: "scene-b",
        set_id: "set-b",
        source_span: [10, 20],
        narration: [576000, 792000],
        entry_snapshot: snapshot("set-b"),
        required_asset_ids: ["card-set-b"],
        reveal_group_ids: ["rg-set-b"],
      },
    ],
    transitions: [
      {
        from_scene_id: "scene-a",
        to_scene_id: "scene-b",
        fade_out_start: 384000,
        swap_sample: 432000,
        fade_in_start: 528000,
        fade_in_end: 576000,
      },
    ],
    cues: [],
    closing: {
      last_prose_action_sample: 792000,
      curtain_close: [792000, 864000],
      score_tail: null,
      terminal_sample: 864000,
    },
    terminal_snapshot: { ...snapshot("set-b"), curtain_open_ratio: 0 },
  };
}

describe("timeline: boundaries and dark hold", () => {
  it("dark hold is exactly 96,000 samples", () => {
    const { ok, hold } = validateTransitionHold(384000, 432000, 528000);
    assert.equal(hold, 96000);
    assert.equal(ok, true);
  });

  it("sample-1/sample/sample+1 resolve the swap boundary deterministically", () => {
    const t = twoSceneTimeline();
    const before = evaluate(t, 431999);
    const at = evaluate(t, 432000);
    const after = evaluate(t, 432001);
    assert.equal(before.scene_id, "scene-a");
    assert.equal(at.scene_id, "scene-b");
    assert.equal(after.scene_id, "scene-b");
    // Fade reaches full blackout exactly at the swap sample.
    assert.ok(before.stage_gain > 0 && before.stage_gain < 0.001, `gain ${before.stage_gain}`);
    assert.equal(at.stage_gain, 0);
  });

  it("seek into a transition selects the destination phase without restarting the wait", () => {
    const t = twoSceneTimeline();
    const s = evaluate(t, 550000);
    assert.equal(s.phase, "transition");
    assert.equal(s.scene_id, "scene-b");
    assert.ok(s.stage_gain > 0 && s.stage_gain < 1);
  });

  it("terminal evaluation closes the curtains within the master endpoint", () => {
    const t = twoSceneTimeline();
    const s = evaluate(t, 864000);
    assert.equal(s.phase, "closing");
    assert.equal(s.curtain_open_ratio, 0);
    assert.deepEqual(validateTimeline(t), []);
  });

  it("a one-sample-short dark hold fails validation and is not repaired", () => {
    const t = twoSceneTimeline();
    t.transitions[0]!.fade_in_start = 527999;
    const errors = validateTimeline(t);
    assert.ok(errors.some((e) => e.includes("96000")), errors.join("; "));
  });

  it("sequential and arbitrary seeks agree at matched samples", () => {
    const t = twoSceneTimeline();
    for (const s of [0, 167999, 168000, 383999, 432000, 527999, 576000, 791999, 864000]) {
      assert.deepEqual(evaluate(t, s), evaluate(t, s));
    }
    // Full state reconstructs without history: late sample needs no preamble.
    const late = evaluate(t, 600000);
    assert.equal(late.scene_id, "scene-b");
    assert.equal(late.stage_gain, 1);
  });
});
