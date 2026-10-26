// contracts group: the shared player API surface from INTERFACES.md.
// Public names every consumer (studio, audience, Agent 3 capture) relies on.
import { describe, it } from "node:test";
import assert from "node:assert/strict";

describe("contracts: public API surface", () => {
  it("exposes loadRelease/mount-shaped transport/evaluate/capture", async () => {
    const api = await import("../src/index.js");
    for (const name of [
      "loadRelease",
      "loadDiagnostic",
      "createTransport",
      "evaluate",
      "describeCapture",
      "PROFILE_ID",
      "SAMPLE_RATE",
      "DARK_HOLD_SAMPLES",
    ]) {
      assert.ok(name in api, `missing export: ${name}`);
    }
  });

  it("exposes the pivot conversion (top-origin to engine bottom-origin)", async () => {
    const api = await import("../src/index.js") as Record<string, unknown>;
    assert.equal(typeof api["planeOffsetForPivot"], "function");
    assert.equal(typeof api["pivotTopToBottom"], "function");
  });

  it("rejects unknown wire versions instead of defaulting", async () => {
    const { loadRelease } = await import("../src/index.js");
    const manifest = {
      schema_version: 2,
      capability_version: "paper-player-v1",
      provenance: "autonomous",
      title: "t",
      author: "a",
      chapter_label: "c",
      source_sha256: "0".repeat(64),
      sample_rate: 48000,
      sample_count: 1,
      entries: {},
      assets: [],
      reveals: [],
      acceptance: { path: "a.bin", sha256: "0".repeat(64), bytes: 1, media_type: "application/octet-stream" },
    };
    const bytes = new TextEncoder().encode(JSON.stringify(manifest));
    const { sha256Hex } = await import("../src/index.js");
    const digest = await sha256Hex(bytes);
    await assert.rejects(
      loadRelease(bytes, digest, async () => new Uint8Array(), {
        player_sha256: "0".repeat(64),
        checker_sha256: "0".repeat(64),
        recipe_sha256: "0".repeat(64),
        allow_diagnostic: false,
      }),
      /schema/i,
    );
  });
});
