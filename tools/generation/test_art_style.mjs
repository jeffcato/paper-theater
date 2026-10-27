#!/usr/bin/env node
// Style-direction guard: every image asset family in the product must be
// documented as illustrated die-cut paper. Red until the scene style doc
// exists and the handoffs point at it.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(join(dirname(fileURLToPath(import.meta.url)), "..", ".."));
const read = (p) => readFile(join(root, p), "utf8");

test("scene style doc exists and requires illustrated die-cut paper", async () => {
  const doc = await read("docs/implementation/generation/scene-art-style.md");
  assert.match(doc, /die-cut/i);
  assert.match(doc, /illustrat/i);
});

test("house brief prompts require die-cut paper construction", async () => {
  const brief = JSON.parse(await read("docs/implementation/player/immersive-art-brief.json"));
  assert.ok(brief.assets.length > 0);
  for (const a of brief.assets) {
    assert.match(a.prompt, /die-cut/i, a.id);
  }
});

test("handoff docs point at the illustrated die-cut scene direction", async () => {
  for (const p of [
    "docs/agent-handoffs/03-generation.md",
    "docs/agent-handoffs/04-player-and-audience.md",
    "docs/agent-handoffs/COMMON.md",
  ]) {
    const doc = await read(p);
    assert.match(doc, /scene-art-style|die-cut/i, p);
  }
});
