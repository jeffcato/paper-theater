// contracts group: consume the frozen shared revision
// (contracts-ready.json: pt-contracts-v1.0.0 / pt-services-v1.0.0).
// The frozen canonical fixture travels through the real loader/evaluator.
//
// Boundary note: the contracts package now ships built `dist` with a root
// workspace link, so @paper-theater/contracts is directly importable for
// types (see tests/contract-adoption.test.ts). This check keeps reading the
// frozen JSON records as data so the canonical fixture bytes still travel
// the real loader/evaluator path byte-for-byte.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import {
  evaluate,
  loadDiagnostic,
  loadRelease,
  normalizeCues,
  sha256Hex,
  toCueView,
  toLightView,
  validateTimeline,
} from "../src/index.js";
import type { CompiledTimeline } from "../src/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const productRoot = join(here, "..", "..", "..", "..");
const sharedRoot = join(productRoot, "fixtures", "shared", "two-scene-v1");
const freeze = JSON.parse(
  readFileSync(join(productRoot, "docs", "implementation", "integration", "contracts-ready.json"), "utf8"),
) as { contract_revision: string; manifest_sha256: string };
const FROZEN_MANIFEST_SHA256 = "04f11a892cb775590120c371c7b8effbf6ea6460ca2d800e5f159d7ed46c1f1d";

const policy = {
  player_sha256: "0".repeat(64),
  checker_sha256: "0".repeat(64),
  recipe_sha256: "0".repeat(64),
  allow_diagnostic: false,
};

describe("contracts: frozen shared revision", () => {
  it("matches the published freeze record", () => {
    assert.equal(freeze.contract_revision, "pt-contracts-v1.0.0");
    assert.equal(freeze.manifest_sha256, FROZEN_MANIFEST_SHA256);
  });

  it("verifies the frozen manifest digest byte-for-byte", async () => {
    const bytes = readFileSync(join(sharedRoot, "manifest.json"));
    assert.equal(await sha256Hex(new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength)), FROZEN_MANIFEST_SHA256);
  });

  it("rejects the frozen diagnostic fixture in production, reports via diagnostic", async () => {
    const raw = readFileSync(join(sharedRoot, "manifest.json"));
    const bytes = new Uint8Array(raw.buffer, raw.byteOffset, raw.byteLength);
    const read = async (path: string, maxBytes: number) => {
      const data = readFileSync(join(sharedRoot, path));
      if (data.length > maxBytes) throw new Error(`over bound: ${path}`);
      return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    };
    const diag = await loadDiagnostic(bytes, FROZEN_MANIFEST_SHA256, read, policy);
    assert.equal(diag.accepted, false);
    assert.ok(diag.missing_acceptance_ids.length > 0);
    await assert.rejects(loadRelease(bytes, FROZEN_MANIFEST_SHA256, read, policy));
  });

  it("evaluates the frozen shared timeline at swap and terminal samples", () => {
    const timeline = JSON.parse(readFileSync(join(sharedRoot, "timeline.json"), "utf8")) as CompiledTimeline;
    const normalized: CompiledTimeline = {
      ...timeline,
      cues: normalizeCues(timeline.cues),
    };
    assert.deepEqual(validateTimeline(normalized), []);
    const tr = timeline.transitions[0]!;
    assert.equal(evaluate(normalized, tr.swap_sample - 1).scene_id, tr.from_scene_id);
    assert.equal(evaluate(normalized, tr.swap_sample).scene_id, tr.to_scene_id);
    const terminal = evaluate(normalized, timeline.sample_count);
    assert.equal(terminal.phase, "closing");
    assert.equal(terminal.curtain_open_ratio, 0);
  });

  it("flags the frozen negative inventory cases it owns", () => {
    const timeline = JSON.parse(readFileSync(join(sharedRoot, "timeline.json"), "utf8")) as CompiledTimeline;
    const base: CompiledTimeline = {
      ...timeline,
      cues: normalizeCues(timeline.cues),
    };
    // short-dark-hold: one sample early on fade_in_start.
    const shortHold = {
      ...base,
      transitions: base.transitions.map((t) => ({ ...t, fade_in_start: t.fade_in_start - 1 })),
    };
    assert.ok(validateTimeline(shortHold).some((e) => e.includes("96000")));
    // unknown-action: a cue outside the production allowlist. The cast is
    // confined to this negative construction: the wire value is genuinely
    // mistyped, and the validator (not the type system) must reject it.
    const badAction: CompiledTimeline = {
      ...base,
      cues: [...base.cues, { id: "neg", scene_id: base.scenes[0]!.id, target_id: "x", action: "hop", start_sample: 1, end_sample: 2, easing: "linear", reveal_group_ids: [], params: {} } as unknown as CompiledTimeline["cues"][number]],
    };
    assert.ok(validateTimeline(badAction).some((e) => e.includes("unsupported action")));
  });

  it("normalizes shared tagged-union cues and discriminated lights", () => {
    const timeline = JSON.parse(readFileSync(join(sharedRoot, "timeline.json"), "utf8")) as CompiledTimeline;
    for (const cue of timeline.cues) {
      const view = toCueView(cue);
      assert.equal(typeof view.action, "string");
      assert.equal(typeof view.params, "object");
    }
    const shell = JSON.parse(readFileSync(join(sharedRoot, "shell.json"), "utf8")) as {
      layers: unknown[];
    };
    void shell;
    const spot = {
      id: "s", type: "spot", enabled: true, position: [0, 0, 0], target: [0, 0, 0],
      color: "#ffffff", intensity: 1, cast_shadow: true,
      shadow: { map_size: 1024, bias: 0, normal_bias: 0, near: 0.1, far: 10 },
      angle_radians: 0.5, penumbra: 0.5, distance: 10, decay: 2,
    } as const;
    const flat = toLightView(spot as never);
    assert.equal(flat.angle_radians, 0.5);
    assert.throws(() => toCueView({ action: "hop", params: {} } as never), /unsupported action/);
  });
});
