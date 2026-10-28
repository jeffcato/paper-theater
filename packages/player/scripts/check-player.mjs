#!/usr/bin/env node
// check:player — clean TypeScript build followed by tests against freshly
// compiled modules. Groups: contracts, loader, transport, shell, timeline,
// readalong, integration, then the aggregate (no group argument).
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(here, "..");
const GROUPS = ["contracts", "loader", "transport", "shell", "timeline", "readalong", "integration"];

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { cwd: pkgDir, stdio: "inherit", ...opts });
  if (r.status !== 0) process.exit(r.status ?? 1);
}

const requested = process.argv[2];
if (requested && !GROUPS.includes(requested)) {
  console.error(`Unknown check:player group: ${requested}\nGroups: ${GROUPS.join(", ")}`);
  process.exit(2);
}

// Clean build first: tests run against freshly compiled modules.
run("./node_modules/.bin/tsc", ["-p", "tsconfig.json"]);

// Owned fixture cases run under the integration group; the frozen shared
// revision check runs under contracts.
const GROUP_FILES = Object.fromEntries(GROUPS.map((g) => [g, [`dist/tests/${g}.test.js`]]));
GROUP_FILES["integration"].push("dist/tests/fixtures.test.js");
GROUP_FILES["integration"].push("dist/tests/mount.test.js");
GROUP_FILES["integration"].push("dist/tests/assets.test.js");
GROUP_FILES["integration"].push("dist/tests/plaque.test.js");
GROUP_FILES["contracts"].push("dist/tests/shared-contracts.test.js");
GROUP_FILES["contracts"].push("dist/tests/contract-adoption.test.js");
GROUP_FILES["loader"].push("dist/tests/house-art.test.js");
GROUP_FILES["shell"].push("dist/tests/geometry.test.js");
GROUP_FILES["shell"].push("dist/tests/multiplane.test.js");
GROUP_FILES["shell"].push("dist/tests/banner-controls.test.js");
GROUP_FILES["shell"].push("dist/tests/lighting.test.js");
GROUP_FILES["shell"].push("dist/tests/viewport.test.js");
const targets = (requested ? [requested] : GROUPS).flatMap((g) => GROUP_FILES[g]);
for (const t of targets) {
  if (!existsSync(join(pkgDir, t))) {
    console.error(`Missing compiled check output: ${t}`);
    process.exit(1);
  }
}
run("node", ["--test", ...targets]);
console.log(`check:player ${requested ?? "aggregate"}: pass`);
