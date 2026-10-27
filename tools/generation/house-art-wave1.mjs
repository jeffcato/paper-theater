#!/usr/bin/env node
// Wave 1 diagnostic exception (operator-approved 2026-10-26): generate ONLY
// house.facade from the immersive art brief via Muse Image, bypassing the
// runtime broker this once because it has no live image dispatch. Output is
// diagnostic, unregistered, and never consumed by the player. The brief's
// hand-coloured lithograph profile is used as-is; the operator's earlier
// painterly reference images are deliberately NOT attached or described.
// Reads META_API_KEY from the repo .env; never prints or serializes it.
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const run = promisify(execFile);
const root = resolve(join(dirname(fileURLToPath(import.meta.url)), "..", ".."));
const MODEL = "muse-image-1.0";
const ENDPOINT = "https://api.meta.ai/v1/images/generations";
const MAX_CALLS = 2;

const brief = JSON.parse(await readFile(join(root, "docs/implementation/player/immersive-art-brief.json"), "utf8"));
const asset = brief.assets.find((a) => a.id === "house.facade");
if (!asset) throw new Error("house.facade missing from brief");
const promptSha = createHash("sha256").update(asset.prompt).digest("hex");

if (process.argv.includes("--dry-run")) {
  console.log(JSON.stringify({
    wave: 1, diagnostic: true, model: MODEL, max_calls: MAX_CALLS,
    assets: [{ id: asset.id, deliver_px: asset.deliver_px, prompt_sha256: promptSha }],
  }, null, 2));
  process.exit(0);
}

const env = await readFile(join(root, ".env"), "utf8").catch(() => "");
const key = env.match(/^META_API_KEY=(.+)$/m)?.[1]?.trim() || process.env.META_API_KEY;
if (!key) throw new Error("META_API_KEY not found in .env or environment");

const outDir = join(root, "fixtures/generation/immersive-house-test-v1");
await mkdir(outDir, { recursive: true });

const body = {
  model: MODEL,
  prompt: asset.prompt,
  n: 1,
  size: asset.deliver_px.join("x"),
  response_format: "b64_json",
  output_format: "png",
  tool_enablement: { enable_image_search: false, enable_web_search: false, enable_shell: false },
};

const candidateB = process.argv.includes("--candidate-b");
if (candidateB) {
  body.prompt = asset.prompt + " GEOMETRY LOCK: the facade piece fills the entire canvas edge to edge with no paper margin, border or surround outside the piece. The stage opening is one exact rectangle from u 0.185 to u 0.815 and from v 0.225 to the bottom edge, filled perfectly flat mid-grey inside those bounds; the crimson scalloped valance sits entirely above v 0.225 and never enters the rectangle.";
}
const outName = candidateB ? "house.facade-b" : "house.facade";
const started = new Date().toISOString();
const res = await fetch(ENDPOINT, {
  method: "POST",
  headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
  body: JSON.stringify(body),
});
const text = await res.text();
if (!res.ok) {
  await writeFile(join(outDir, "wave1-error.json"), JSON.stringify({ at: started, status: res.status, body: text.slice(0, 2000) }, null, 2) + "\n");
  throw new Error(`Muse Image request failed: HTTP ${res.status} (see fixtures/generation/immersive-house-test-v1/wave1-error.json)`);
}
const reply = JSON.parse(text);
const b64 = reply?.data?.[0]?.b64_json;
if (!b64) throw new Error("Muse Image reply carried no b64_json image");

const raw = Buffer.from(b64, "base64");
const rawPath = join(outDir, `${outName}.raw.png`);
await writeFile(rawPath, raw);
const rawInfo = await run("sips", ["-g", "pixelWidth", "-g", "pixelHeight", rawPath]).then((r) => r.stdout).catch(() => "");
const returnedPx = [...rawInfo.matchAll(/pixel(?:Width|Height): (\d+)/g)].map((m) => Number(m[1]));

// Deterministic exact-size delivery: centre-crop to the card aspect is a
// resize in sips (it stretches); record it as a stretch-resize transform.
const outPath = join(outDir, `${outName}.png`);
await run("sips", ["-s", "format", "png", "-z", String(asset.deliver_px[1]), String(asset.deliver_px[0]), rawPath, "--out", outPath]);

const sha = (buf) => createHash("sha256").update(buf).digest("hex");
const manifest = {
  batch: "immersive-house-test-v1",
  wave: 1,
  diagnostic: true,
  accepted: false,
  registered: false,
  dispatch: "diagnostic-exception-direct (operator approved; runtime broker has no live image dispatch)",
  asset: asset.id,
  model: MODEL,
  endpoint: ENDPOINT,
  requested_size: asset.deliver_px.join("x"),
  returned_px: returnedPx,
  deliver_px: asset.deliver_px,
  transform: "sips stretch-resize of returned image to exact deliver size",
  prompt_sha256: candidateB ? createHash("sha256").update(body.prompt).digest("hex") : promptSha,
  brief_prompt_sha256_note: candidateB
    ? "brief prompt verbatim plus an appended GEOMETRY LOCK suffix (see tool source)"
    : "prompt is brief.assets[house.facade].prompt verbatim",
  tools_disabled: ["image_search", "web_search", "shell"],
  references_attached: [],
  calls: 1,
  candidate: candidateB ? "B (geometry-lock repair)" : "A (brief prompt verbatim)",
  max_calls: MAX_CALLS,
  started_at: started,
  raw_file: `${outName}.raw.png`,
  raw_sha256: sha(raw),
  file: `${outName}.png`,
  file_sha256: sha(await readFile(outPath)),
};
await writeFile(join(outDir, candidateB ? "manifest-b.json" : "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`${outName}: returned ${returnedPx.join("x") || "?"} -> ${asset.deliver_px.join("x")} (diagnostic, 1 call)`);
