// Wave 2 diagnostic test: --request-size overrides the API size hint while
// deliver size stays exact (needed when deliver aspect exceeds the API limit).
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const run = promisify(execFile);
const root = new URL("../..", import.meta.url).pathname;

test("dry-run reports requested_size override, deliver_px unchanged", async () => {
  const { stdout } = await run(
    "node",
    [
      "tools/generation/house-art-wave2.mjs",
      "--asset", "house.footlights",
      "--anchor", "fixtures/generation/immersive-house-test-v2/house.facade-v2.png",
      "--request-size", "2048x128",
      "--dry-run",
    ],
    { cwd: root },
  );
  const plan = JSON.parse(stdout);
  assert.equal(plan.asset, "house.footlights");
  assert.deepEqual(plan.deliver_px, [2048, 77]);
  assert.equal(plan.requested_size, "2048x128");
  assert.equal(plan.diagnostic, true);
});

test("dry-run defaults requested_size to deliver_px", async () => {
  const { stdout } = await run(
    "node",
    [
      "tools/generation/house-art-wave2.mjs",
      "--asset", "house.rope",
      "--anchor", "fixtures/generation/immersive-house-test-v2/house.facade-v2.png",
      "--dry-run",
    ],
    { cwd: root },
  );
  const plan = JSON.parse(stdout);
  assert.equal(plan.requested_size, "64x512");
  assert.deepEqual(plan.deliver_px, [64, 512]);
});
