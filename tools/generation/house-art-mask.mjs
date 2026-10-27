#!/usr/bin/env node
// Diagnostic SAM 3.1 masking (operator-approved 2026-10-27): cut the exterior
// white background off two geometric-cut pieces (house.facade-v2,
// lobby-grille) via Meta Model API sam-3.1, preserving the grey apertures.
// Wire format: https://dev.meta.ai/docs/sam/segmenting and
// https://dev.meta.ai/docs/sam/reading-segmentation. Mask decode/prompts use
// the vendored zero-dependency @meta-sam/parser (see vendor-meta-sam/).
// Outputs are diagnostic and unregistered; key never printed or serialized.
// Usage:
//   node tools/generation/house-art-mask.mjs --image <png> --mode box
//     --box x1,y1,x2,y2 [--pos "x,y;x,y"] --neg "x,y;x,y"
//     [--object-id 1] [--out-suffix <s>] [--dry-run]
//   node tools/generation/house-art-mask.mjs --image <png> --mode concept
//     --concept "<noun phrase>" [--out-suffix <s>] [--dry-run]
//   ... [--replay <saved .sam-stream.txt>] re-parse a saved stream offline
//     (no key, no new call).
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import {
  buildObjectPrompt,
  parseImageStream,
  recordsOfKind,
  decodeMaskToRaster,
} from "./vendor-meta-sam/index.js";

const run = promisify(execFile);
const root = resolve(join(dirname(fileURLToPath(import.meta.url)), "..", ".."));
const MODEL = "sam-3.1";
const RESPONSES = "https://api.meta.ai/v1/responses";
const DRY = process.argv.includes("--dry-run");

const arg = (name, dflt = null) => {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : dflt;
};

// Box edges are inclusive display pixels; the wire grammar takes an
// exclusive right/bottom, hence +1 (see vendor-meta-sam/object-prompt.js).
export function buildRequestInput({ mode, size, objectId, box, posPoints, negPoints, concept, imageDataUrl }) {
  let text;
  if (mode === "box") {
    text = buildObjectPrompt({
      size: { width: size.w, height: size.h },
      objects: [{
        id: objectId,
        prompts: [{
          box: { left: box.x1, top: box.y1, right: box.x2 + 1, bottom: box.y2 + 1 },
          points: [
            ...(posPoints ?? []).map(([x, y]) => ({ x, y, label: "positive" })),
            ...(negPoints ?? []).map(([x, y]) => ({ x, y, label: "negative" })),
          ],
        }],
      }],
    });
  } else if (mode === "concept") {
    text = concept;
  } else {
    throw new Error("mode must be box or concept");
  }
  return [{
    type: "message",
    role: "user",
    content: [
      { type: "input_text", text },
      { type: "input_image", image_url: imageDataUrl },
    ],
  }];
}

// Minimal SSE accumulator for logging/raw capture: concatenates
// response.output_text.delta payloads, ignoring anything else.
export function accumulateStreamText(sseText) {
  let out = "";
  for (const line of sseText.split("\n")) {
    const t = line.trim();
    if (!t.startsWith("data:")) continue;
    const data = t.slice(5).trim();
    if (!data || data === "[DONE]") continue;
    try {
      const ev = JSON.parse(data);
      if (ev.type === "response.output_text.delta" && typeof ev.delta === "string") out += ev.delta;
    } catch { /* keep sick lines out of the text */ }
  }
  return out;
}

// The lib consumes an async-iterable of OpenAI-SDK-style event objects:
// each yield must BE the event (the protocol wraps it in {value, done}).
async function* sseEventSource(sseText) {
  for (const line of sseText.split("\n")) {
    const t = line.trim();
    if (!t.startsWith("data:")) continue;
    const data = t.slice(5).trim();
    if (!data || data === "[DONE]") continue;
    yield JSON.parse(data);
  }
}

const invokedDirectly = process.argv[1] !== undefined
  && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (!invokedDirectly) {
  // Imported by tests: exports only.
} else {
await main();
}
async function main() {
const imageRel = arg("--image");
if (!imageRel) throw new Error("usage: --image <png> --mode box|concept [...] [--dry-run]");
const mode = arg("--mode", "box");
const objectId = Number(arg("--object-id", "1"));
const imagePath = resolve(join(root, imageRel));
const stem = basename(imageRel, ".png") + (arg("--out-suffix") ?? "");
const outDir = dirname(imagePath);

const dimInfo = await run("sips", ["-g", "pixelWidth", "-g", "pixelHeight", imagePath]).then((r) => r.stdout);
const [W, H] = [...dimInfo.matchAll(/pixel(?:Width|Height): (\d+)/g)].map((m) => Number(m[1]));
const imageBytes = await readFile(imagePath);
const sha = (buf) => createHash("sha256").update(buf).digest("hex");

let input;
if (mode === "box") {
  const [x1, y1, x2, y2] = arg("--box").split(",").map(Number);
  const splitPts = (s) => (s ?? "").split(";").filter(Boolean).map((p) => p.split(",").map(Number));
  input = buildRequestInput({
    mode, size: { w: W, h: H }, objectId,
    box: { x1, y1, x2, y2 }, posPoints: splitPts(arg("--pos")), negPoints: splitPts(arg("--neg")),
    imageDataUrl: `data:image/png;base64,${imageBytes.toString("base64")}`,
  });
} else {
  input = buildRequestInput({
    mode, concept: arg("--concept"),
    imageDataUrl: `data:image/png;base64,${imageBytes.toString("base64")}`,
  });
}
const promptText = input[0].content[0].text;
const promptSha = sha(promptText);

if (DRY) {
  console.log(JSON.stringify({
    task: "sam-mask", diagnostic: true, model: MODEL, endpoint: RESPONSES,
    image: imageRel, size: [W, H], image_sha256: sha(imageBytes),
    mode, objectId, prompt_text: promptText, prompt_sha256: promptSha,
  }, null, 2));
  process.exit(0);
}

const env = await readFile(join(root, ".env"), "utf8").catch(() => "");
const key = process.env.MODEL_API_KEY || process.env.META_API_KEY
  || env.match(/^MODEL_API_KEY=(.+)$/m)?.[1]?.trim()
  || env.match(/^META_API_KEY=(.+)$/m)?.[1]?.trim();
if (!key && !arg("--replay")) throw new Error("no MODEL_API_KEY/META_API_KEY in environment or .env");

const started = new Date().toISOString();
const replay = arg("--replay");
let sseText;
if (replay) {
  sseText = await readFile(resolve(join(root, replay)), "utf8");
} else {
  const res = await fetch(RESPONSES, {
    method: "POST",
    headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
    body: JSON.stringify({ model: MODEL, input, stream: true }),
  });
  sseText = await res.text();
  await writeFile(join(outDir, `${stem}.sam-stream.txt`), sseText);
  if (!res.ok) {
  await writeFile(join(outDir, `${stem}-error.json`), JSON.stringify(
    { at: started, status: res.status, body: sseText.slice(0, 2000) }, null, 2) + "\n");
  if (res.status === 404) {
    throw new Error(`SAM request failed: HTTP 404 model_not_found — key lacks sam-3.1 entitlement; not retried (see ${stem}-error.json)`);
  }
  throw new Error(`SAM request failed: HTTP ${res.status} (see ${stem}-error.json)`);
  }
}

const parsed = parseImageStream(sseEventSource(sseText));
const result = await parsed.finalResult;
const masks = recordsOfKind(result.records, "mask")
  .filter((r) => String(r.objectId) === String(objectId));
await writeFile(join(outDir, `${stem}.sam-text.txt`), accumulateStreamText(sseText));
if (masks.length === 0) {
  await writeFile(join(outDir, `${stem}-empty.json`), JSON.stringify(
    { at: started, note: "zero matches: normal per docs, not an error", prompt_sha256: promptSha }, null, 2) + "\n");
  throw new Error("SAM returned zero matches for this prompt (normal result); retry with a different prompt, max 1 retry");
}
masks.sort((a, b) => ((b.bounds.right - b.bounds.left) * (b.bounds.bottom - b.bounds.top))
  - ((a.bounds.right - a.bounds.left) * (a.bounds.bottom - a.bounds.top)));
const record = masks[0];
const raster = Array.from(decodeMaskToRaster(record.mask));
await writeFile(join(outDir, `${stem}.sam-mask.json`), JSON.stringify({
  height: record.mask.height, width: record.mask.width, raster,
}, null, 2));

const maskedPath = join(outDir, `${stem}.sam-masked.png`);
const applyOut = await run("python3",
  [join(root, "tools/generation/sam_mask_apply.py"), imagePath,
    join(outDir, `${stem}.sam-mask.json`), maskedPath]).then((r) => r.stdout);
const stats = JSON.parse(applyOut);
const gates = {
  grey_kept: stats.grey_kept_frac >= 0.995,
  white_removed: stats.white_removed_frac >= 0.95,
  mask_agreement: stats.notwhite_agreement_frac >= 0.95,
};
await writeFile(join(outDir, `${stem}.sam-manifest.json`), JSON.stringify({
  task: "sam-mask", diagnostic: true, accepted: false, registered: false,
  image: imageRel, image_sha256: sha(imageBytes), size: [W, H],
  model: MODEL, endpoint: RESPONSES, mode, objectId, prompt_sha256: promptSha,
  request_sha256: sha(JSON.stringify({ model: MODEL, input })),
  stream_sha256: sha(sseText),
  mask_records: masks.length, mask_hw: stats.mask_hw,
  mask_sha256: sha(await readFile(join(outDir, `${stem}.sam-mask.json`))),
  file: `${stem}.sam-masked.png`, file_sha256: sha(await readFile(maskedPath)),
  stats, gates, gates_pass: Object.values(gates).every(Boolean),
  calls: 1, started_at: started,
}, null, 2) + "\n");
if (!Object.values(gates).every(Boolean)) {
  throw new Error(`mask gates failed: ${JSON.stringify(gates)} (see ${stem}.sam-manifest.json)`);
}
console.log(`${stem}: mask ${stats.mask_hw.join("x")} coverage ${stats.coverage_frac}, grey kept ${stats.grey_kept_frac}, white removed ${stats.white_removed_frac} (diagnostic, 1 call)`);
}
