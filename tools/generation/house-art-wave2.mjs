#!/usr/bin/env node
// Wave 2 diagnostic exception (operator-approved 2026-10-26/27): regenerate
// house.facade anchored on candidate B, then pilot three further assets with
// the accepted facade attached as a STYLE reference only. Same rules as
// wave 1: direct Muse Image calls, tools disabled, outputs diagnostic and
// unregistered, key never printed or serialized.
// Usage:
//   node tools/generation/house-art-wave2.mjs --facade-v2 [--dry-run]
//   node tools/generation/house-art-wave2.mjs --asset lobby.grille [--dry-run]
//   ... [--note "<edit instruction>"] [--out-suffix <suffix>] [--request-size WxH]
// (--request-size overrides the API size hint when the deliver aspect exceeds
// the API limit; deliver size stays exact via stretch-resize.)
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const run = promisify(execFile);
const root = resolve(join(dirname(fileURLToPath(import.meta.url)), "..", ".."));
const MODEL = "muse-image-1.0";
const GENERATIONS = "https://api.meta.ai/v1/images/generations";
const EDITS = "https://api.meta.ai/v1/images/edits";
const DRY = process.argv.includes("--dry-run");

const brief = JSON.parse(await readFile(join(root, "docs/implementation/player/immersive-art-brief.json"), "utf8"));
const byId = (id) => {
  const a = brief.assets.find((x) => x.id === id);
  if (!a) throw new Error(`${id} missing from brief`);
  return a;
};

const anchorB = join(root, "fixtures/generation/immersive-house-test-v1/house.facade-b.png");
const anchorIdx = process.argv.indexOf("--anchor");
const anchorPath = anchorIdx >= 0 ? resolve(process.argv[anchorIdx + 1]) : anchorB;
// Legal-size hint override for assets whose deliver aspect the API rejects
// (e.g. house.footlights 2048x77 exceeds the 16:1 limit). Deliver size is
// still exact via the sips stretch-resize; both sizes land in the manifest.
const reqIdx = process.argv.indexOf("--request-size");
const requestSize = reqIdx >= 0 ? process.argv[reqIdx + 1] : null;

const STYLE_REF = "Use the attached image ONLY as a style reference for palette, ink line weight, engraved hatching, watercolour wash handling and paper tone. Do not copy its composition, subjects, layout, geometry, lettering or any element. ";
const NO_MARGIN = "The piece fills the entire canvas edge to edge with no paper margin, border or surround outside the piece. ";

let mode, asset, prompt, attach, outName, editCall;
if (process.argv.includes("--facade-v2")) {
  mode = "facade-v2";
  asset = byId("house.facade");
  // Same composition as B, corrected: bottle-green blank cartouche (cream
  // typeset text needs the dark field), neutral #808080 opening, full bleed.
  prompt = asset.prompt + " " + STYLE_REF + NO_MARGIN
    + "REGENERATION FIXES, keeping the reference's composition, towers, three spectators per box, valance and green pediment: "
    + "the oval cartouche field is deep bottle-green and blank; the stage opening is one exact rectangle from u 0.185 to u 0.815 and from v 0.225 to the bottom edge, filled perfectly flat, even, neutral mid-grey (#808080); "
    + "the crimson scalloped valance sits entirely above v 0.225 and never enters the rectangle.";
  attach = anchorB;
  outName = "house.facade-v2";
  editCall = true;
} else if (process.argv.includes("--asset")) {
  const id = process.argv[process.argv.indexOf("--asset") + 1];
  asset = byId(id);
  mode = `pilot:${id}`;
  const noteIdx = process.argv.indexOf("--note");
  prompt = noteIdx >= 0
    ? process.argv[noteIdx + 1]
    : STYLE_REF + NO_MARGIN + asset.prompt;
  attach = anchorPath;
  const outIdx = process.argv.indexOf("--out-suffix");
  outName = id.replace(/\./g, "-") + (outIdx >= 0 ? process.argv[outIdx + 1] : "");
  editCall = true;
} else {
  throw new Error("usage: --facade-v2 | --asset <id> [--dry-run]");
}

const promptSha = createHash("sha256").update(prompt).digest("hex");
if (DRY) {
  console.log(JSON.stringify({
    wave: 2, diagnostic: true, model: MODEL, mode, asset: asset.id,
    deliver_px: asset.deliver_px,
    requested_size: requestSize ?? asset.deliver_px.join("x"),
    prompt_sha256: promptSha,
    anchor_sha256: createHash("sha256").update(await readFile(anchorB)).digest("hex"),
    endpoint: EDITS,
  }, null, 2));
  process.exit(0);
}

const env = await readFile(join(root, ".env"), "utf8").catch(() => "");
const key = env.match(/^META_API_KEY=(.+)$/m)?.[1]?.trim() || process.env.META_API_KEY;
if (!key) throw new Error("META_API_KEY not found in .env or environment");

const outDir = join(root, "fixtures/generation/immersive-house-test-v2");
await mkdir(outDir, { recursive: true });

const form = new FormData();
form.set("model", MODEL);
form.set("prompt", prompt);
form.set("size", requestSize ?? asset.deliver_px.join("x"));
form.set("response_format", "b64_json");
form.set("output_format", "png");
form.set("tool_enablement", JSON.stringify({ enable_image_search: false, enable_web_search: false, enable_shell: false }));
form.set("image", new File([await readFile(attach)], "anchor.png", { type: "image/png" }));

const started = new Date().toISOString();
const res = await fetch(EDITS, { method: "POST", headers: { Authorization: `Bearer ${key}` }, body: form });
const text = await res.text();
if (!res.ok) {
  await writeFile(join(outDir, `${outName}-error.json`), JSON.stringify({ at: started, status: res.status, body: text.slice(0, 2000) }, null, 2) + "\n");
  throw new Error(`Muse Image edits request failed: HTTP ${res.status} (see fixtures/generation/immersive-house-test-v2/${outName}-error.json)`);
}
const reply = JSON.parse(text);
const b64 = reply?.data?.[0]?.b64_json;
if (!b64) throw new Error("Muse Image reply carried no b64_json image");

const raw = Buffer.from(b64, "base64");
const rawPath = join(outDir, `${outName}.raw.png`);
await writeFile(rawPath, raw);
const rawInfo = await run("sips", ["-g", "pixelWidth", "-g", "pixelHeight", rawPath]).then((r) => r.stdout).catch(() => "");
const returnedPx = [...rawInfo.matchAll(/pixel(?:Width|Height): (\d+)/g)].map((m) => Number(m[1]));

const outPath = join(outDir, `${outName}.png`);
await run("sips", ["-s", "format", "png", "-z", String(asset.deliver_px[1]), String(asset.deliver_px[0]), rawPath, "--out", outPath]);

const sha = (buf) => createHash("sha256").update(buf).digest("hex");
await writeFile(join(outDir, `${outName}.manifest.json`), JSON.stringify({
  batch: "immersive-house-test-v2", wave: 2, diagnostic: true, accepted: false, registered: false,
  dispatch: "diagnostic-exception-direct (operator approved; runtime broker has no live image dispatch)",
  asset: asset.id, mode, model: MODEL, endpoint: EDITS,
  requested_size: requestSize ?? asset.deliver_px.join("x"),
  returned_px: returnedPx, deliver_px: asset.deliver_px,
  transform: "sips stretch-resize of returned image to exact deliver size",
  prompt_sha256: promptSha,
  anchor: { file: "v1:fixtures/generation/immersive-house-test-v1/house.facade-b.png", role: "style reference only" },
  tools_disabled: ["image_search", "web_search", "shell"],
  calls: 1, started_at: started,
  raw_file: `${outName}.raw.png`, raw_sha256: sha(raw),
  file: `${outName}.png`, file_sha256: sha(await readFile(outPath)),
}, null, 2) + "\n");
console.log(`${outName}: returned ${returnedPx.join("x") || "?"} -> ${asset.deliver_px.join("x")} (diagnostic, 1 call)`);
