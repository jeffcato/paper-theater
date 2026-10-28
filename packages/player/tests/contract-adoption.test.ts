// contracts group: the public player boundary consumes the shared contract
// types (pt-contracts-v1.0.0) directly. Identity cases pass
// structurally-identical shared records into evaluate/validateTimeline and
// the cue/light adapters; strict cases assert the shared tagged-union
// timelines/lights reject incompatible shapes at compile time, and runtime
// cases prove hostile wire bytes are still diagnostically rejected.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type {
  Light,
  ReadAlongGroup,
  ReadAlongWord,
  StoryCue as SharedCue,
  Timeline,
} from "@paper-theater/contracts";
import {
  evaluate,
  groupWords,
  toCueView,
  toLightView,
  validateTimeline,
} from "../src/index.js";
import type {
  CompiledTimeline,
  FlatLight,
  StoryCue as PlayerCue,
} from "../src/index.js";

function sharedCamera() {
  return {
    position: [0, 4, 10] as [number, number, number],
    target: [0, 4, 0] as [number, number, number],
    up: [0, 1, 0] as [number, number, number],
    fov_y_degrees: 40,
    near: 1,
    far: 100,
  };
}

function sharedSnapshot(set_id: string, curtain_open_ratio: number) {
  return {
    set_id,
    instances: [],
    camera: sharedCamera(),
    lighting: {
      ambient: { color: "#ffffff", intensity: 1 },
      environment: null as null,
      lights: [],
    },
    curtain_open_ratio,
  };
}

/** Minimal valid shared timeline: one scene, one enter cue, closed terminal. */
function sharedTimeline(): Timeline {
  return {
    schema_version: 1,
    capability_version: "paper-player-v1",
    delivery_audio_sha256: "0".repeat(64),
    presentation_map_sha256: "0".repeat(64),
    sample_rate: 48000,
    sample_count: 456000,
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
        entry_snapshot: sharedSnapshot("set-a", 1),
        required_asset_ids: ["card-set-a"],
        reveal_group_ids: [],
      },
    ],
    transitions: [],
    cues: [
      {
        id: "cue-enter",
        scene_id: "scene-a",
        target_id: "em-1",
        start_sample: 168000,
        end_sample: 192000,
        easing: "linear",
        reveal_group_ids: [],
        action: "enter",
        params: { entrance_id: "door-a", target: [2, 0, 0], facing: "left" },
      },
    ],
    closing: {
      last_prose_action_sample: 192000,
      curtain_close: [384000, 456000],
      score_tail: null,
      terminal_sample: 456000,
    },
    terminal_snapshot: sharedSnapshot("set-a", 0),
  };
}

function sharedSpot(): Light {
  return {
    id: "key",
    enabled: true,
    position: [0, 8, 0],
    target: [0, 0, 0],
    color: "#ffffff",
    intensity: 1,
    cast_shadow: false,
    shadow: { map_size: 1024, bias: 0, normal_bias: 0, near: 1, far: 20 },
    type: "spot",
    angle_radians: 0.5,
    penumbra: 0.2,
    distance: 30,
    decay: 1,
  };
}

describe("contract adoption: shared timeline at the public boundary", () => {
  it("validates and evaluates an actual shared Timeline", () => {
    const shared = sharedTimeline();
    const asPlayer: CompiledTimeline = shared;
    assert.deepEqual(validateTimeline(asPlayer), []);
    const asShared: Timeline = asPlayer;
    assert.equal(asShared.sample_count, 456000);
    const state = evaluate(asShared, 180000);
    assert.equal(state.phase, "scene");
    assert.equal(state.scene_id, "scene-a");
    assert.deepEqual(state.active_cue_ids, ["cue-enter"]);
  });

  it("types player cues as shared tagged-union cues", () => {
    const playerEnter: PlayerCue = {
      id: "cue-enter",
      scene_id: "scene-a",
      target_id: "em-1",
      start_sample: 168000,
      end_sample: 192000,
      easing: "linear",
      reveal_group_ids: [],
      action: "enter",
      params: { entrance_id: "door-a", target: [2, 0, 0], facing: "left" },
    };
    const sharedEnter: SharedCue = playerEnter;
    const view = toCueView(sharedEnter);
    assert.equal(view.action, "enter");
    assert.equal(view.easing, "linear");
    if (playerEnter.action === "enter") {
      // @ts-expect-error - enter params carry no exit_id
      void playerEnter.params.exit_id;
    }
  });

  it("adapts an actual shared spot light with required extras", () => {
    const flat: FlatLight = toLightView(sharedSpot());
    assert.equal(flat.type, "spot");
    assert.equal(flat.angle_radians, 0.5);
    assert.equal(flat.penumbra, 0.2);
    assert.equal(flat.distance, 30);
    assert.equal(flat.decay, 1);
  });

  it("rejects a spot missing required extras at compile time", () => {
    const incompleteSpot = {
      id: "key",
      enabled: true,
      position: [0, 8, 0] as [number, number, number],
      target: [0, 0, 0] as [number, number, number],
      color: "#ffffff",
      intensity: 1,
      cast_shadow: false,
      shadow: { map_size: 1024, bias: 0, normal_bias: 0, near: 1, far: 20 },
      type: "spot" as const,
    };
    if (false) {
      // @ts-expect-error - the shared discriminated union requires spot extras
      toLightView(incompleteSpot);
    }
    assert.throws(
      () => toLightView(incompleteSpot as unknown as Light),
      /incomplete spot/,
    );
  });

  it("validates actual shared read-along words/groups", () => {
    const words: ReadAlongWord[] = [
      {
        id: "w1",
        source_span: [0, 5],
        text: "Hello",
        following_text: " ",
        start_sample: 168000,
        end_sample: 168000,
      },
      {
        id: "w2",
        source_span: [5, 11],
        text: "world.",
        following_text: "",
        start_sample: 168000,
        end_sample: 192000,
      },
    ];
    const groups: ReadAlongGroup[] = [
      {
        id: "g1",
        word_ids: ["w1", "w2"],
        timing_owner_word_id: "w2",
        start_sample: 168000,
        end_sample: 192000,
      },
    ];
    assert.doesNotThrow(() => groupWords(words, groups));
  });

  it("rejects hostile wire actions at runtime with a diagnostic", () => {
    assert.throws(
      () => toCueView({ action: "hop", params: {} } as unknown as SharedCue),
      /unsupported action/,
    );
  });
});
