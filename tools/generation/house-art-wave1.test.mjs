// Wave 1 diagnostic test: dry-run must select exactly house.facade from the
// brief, with its deliver size and a hash-bound prompt, and make no calls.
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const run = promisify(execFile);

test("dry-run selects house.facade only, with brief deliver size", async () => {
  const { stdout } = await run("node", ["tools/generation/house-art-wave1.mjs", "--dry-run"], {
    cwd: new URL("../..", import.meta.url).pathname,
  });
  const plan = JSON.parse(stdout);
  assert.equal(plan.wave, 1);
  assert.deepEqual(plan.assets.map((a) => a.id), ["house.facade"]);
  assert.deepEqual(plan.assets[0].deliver_px, [2048, 1245]);
  assert.match(plan.assets[0].prompt_sha256, /^[0-9a-f]{64}$/);
  assert.equal(plan.model, "muse-image-1.0");
  assert.equal(plan.max_calls, 2);
  assert.equal(plan.diagnostic, true);
});
