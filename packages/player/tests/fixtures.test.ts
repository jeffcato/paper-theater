// fixtures group (runs under check:player -- integration): owned
// fixtures/player cases travel through the real loader/evaluator.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import {
  evaluate,
  loadDiagnostic,
  loadRelease,
  sha256Hex,
  validateTimeline,
} from "../src/index.js";
import type { CompiledTimeline } from "../src/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const playerFixtures = join(here, "..", "..", "..", "..", "fixtures", "player");
const read = (p: string) => readFileSync(join(playerFixtures, p));

const policy = {
  player_sha256: "0".repeat(64),
  checker_sha256: "0".repeat(64),
  recipe_sha256: "0".repeat(64),
  allow_diagnostic: false,
};

describe("fixtures: two-scene diagnostic and negatives", () => {
  it("evaluates the two-scene timeline fixture at boundaries", () => {
    const timeline = JSON.parse(read("two-scene-local/timeline.json").toString("utf8")) as CompiledTimeline;
    assert.deepEqual(validateTimeline(timeline), []);
    assert.equal(evaluate(timeline, 431999).scene_id, "scene-a");
    assert.equal(evaluate(timeline, 432000).scene_id, "scene-b");
    assert.equal(evaluate(timeline, 864000).curtain_open_ratio, 0);
  });

  it("diagnostic manifest reports missing acceptance IDs; production rejects it", async () => {
    const manifestBytes = read("two-scene-local/manifest.diagnostic.json");
    const digest = await sha256Hex(manifestBytes);
    const reader = async (path: string) => {
      if (path === "acceptance.json") return read("two-scene-local/acceptance.json");
      throw new Error(`unexpected read: ${path}`);
    };
    const diag = await loadDiagnostic(manifestBytes, digest, reader, policy);
    assert.equal(diag.accepted, false);
    assert.ok(diag.missing_acceptance_ids.length > 0);
    await assert.rejects(loadRelease(manifestBytes, digest, reader, policy));
  });

  it("negative: short dark hold fails timeline validation", () => {
    const timeline = JSON.parse(read("negatives/bad-hold.json").toString("utf8")) as CompiledTimeline;
    const errors = validateTimeline(timeline);
    assert.ok(errors.some((e) => e.includes("96000")), errors.join("; "));
  });

  it("negative: unsupported action fails timeline validation", () => {
    const timeline = JSON.parse(
      read("negatives/unsupported-action.json").toString("utf8"),
    ) as CompiledTimeline;
    const errors = validateTimeline(timeline);
    assert.ok(errors.some((e) => e.includes("unsupported action")), errors.join("; "));
  });
});
