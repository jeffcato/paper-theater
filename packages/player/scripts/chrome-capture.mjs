#!/usr/bin/env node
// check:chrome — render the visible diagnostic fixture in headless Chrome
// and capture the final composited pixels. Asserts real rendered output:
// visible oriented content at full gain, true blackout at zero gain, and a
// clean console. Mirrors the studio lane's proven CDP pattern; SwiftShader
// supplies software WebGL on machines without a GPU.
// Evidence: docs/implementation/player/chrome-capture.png + .json.
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(pkgDir, "..", "..");
const evidencePng = join(root, "docs", "implementation", "player", "chrome-capture.png");
const evidenceJson = join(root, "docs", "implementation", "player", "chrome-capture.json");
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script></head>
<body><canvas id="stage" width="320" height="180"></canvas>
<script type="module">
import * as api from "/dist/src/index.js";
window.__errors = [];
addEventListener("error", (e) => window.__errors.push(String(e.message)));
addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason?.message ?? e.reason)));
async function sha256(bytes) {
  const d = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(d)].map((n) => n.toString(16).padStart(2, "0")).join("");
}
async function load(name, maxBytes) {
  const res = await fetch("/fixture/" + name);
  if (!res.ok) throw new Error("fixture fetch failed: " + name);
  const buf = new Uint8Array(await res.arrayBuffer());
  if (buf.length > maxBytes) throw new Error("fixture over bound: " + name);
  return buf;
}
function stats(pixels, W, H) {
  // readRenderTargetPixels rows run bottom-up; flip y for display regions.
  const at = (x, y) => { const r = (H - 1 - y) * W + x; return [pixels[r * 4] / 255, pixels[r * 4 + 1] / 255, pixels[r * 4 + 2] / 255]; };
  // Bounding box of rendered content: orientation is measured relative to
  // what actually rendered, so fixture scale never defeats the assertions.
  let x0 = W, x1 = -1, y0 = H, y1 = -1, nonblack = 0, max = 0;
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const [R, G, B] = at(x, y);
    max = Math.max(max, R, G, B);
    if (R > 8 / 255 || G > 8 / 255 || B > 8 / 255) {
      nonblack++;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
    }
  }
  let lR = 0, lN = 0, rR = 0, rN = 0, topMax = 0, bot = 0, botN = 0;
  if (x1 >= x0 && y1 >= y0) {
    // Thin edge bands across the full content width: white strips are narrow
    // (means dilute), so the top assertion uses the band maximum while the
    // bottom uses the mean over dark strips and dim deck. A vertical flip
    // would swap bright top for dim bottom.
    const midX = (x0 + x1) / 2, topBand = y0 + (y1 - y0) * 0.08, botBand = y1 - (y1 - y0) * 0.08;
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
      const [R, G, B] = at(x, y);
      if (x < midX) { lR += R; lN++; } else { rR += R; rN++; }
      const lum = 0.2126 * R + 0.7152 * G + 0.0722 * B;
      if (y <= topBand) topMax = Math.max(topMax, lum);
      if (y >= botBand) { bot += lum; botN++; }
    }
  }
  const n = W * H;
  return {
    nonblackFraction: nonblack / n, maxChannel: max,
    bbox: x1 >= x0 ? [x0, y0, x1, y1] : null,
    leftRed: lN ? lR / lN : 0, rightRed: rN ? rR / rN : 0,
    topMax, bottomLum: botN ? bot / botN : 0,
  };
}
window.__run = async () => {
  const manifestBytes = await load("manifest.diagnostic.json", 1024 * 1024);
  const digest = await sha256(manifestBytes);
  const read = async (path) => load(path, 32 * 1024 * 1024);
  const policy = { player_sha256: "0".repeat(64), checker_sha256: "0".repeat(64), recipe_sha256: "0".repeat(64), allow_diagnostic: false };
  const handle = await api.loadDiagnostic(manifestBytes, digest, read, policy);
  const timeline = JSON.parse(new TextDecoder().decode(await load("timeline.json", 1024 * 1024)));
  const sink = await api.createThreeSink(320, 180);
  document.querySelector("#stage").replaceWith(sink.canvas);
  const mount = api.mountPlayer(document.body, { handle, timeline }, { sink, candidate_id: "chrome-diagnostic-1" });
  const out = {};
  mount.seek(200000);
  mount.tick();
  const lit = await mount.captureFrame(320, 180);
  out.lit = { ...stats(lit.pixels, 320, 180), sample: lit.sample, phase: lit.phase, accepted: lit.accepted, provenance: lit.provenance };
  mount.seek(432000);
  mount.tick();
  const dark = await mount.captureFrame(320, 180);
  let darkMax = 0;
  for (let i = 0; i < dark.pixels.length; i += 4) darkMax = Math.max(darkMax, dark.pixels[i] / 255, dark.pixels[i + 1] / 255, dark.pixels[i + 2] / 255);
  out.dark = { maxChannel: darkMax, sample: dark.sample };
  out.errors = window.__errors;
  // Leave the lit frame on screen for the evidence screenshot; the Node
  // driver calls window.__teardown() after capturing it.
  mount.seek(200000);
  mount.tick();
  window.__teardown = () => {
    mount.dispose();
    sink.dispose();
  };
  return out;
};
</script></body></html>`;

const MIME = { ".html": "text/html", ".js": "text/javascript", ".json": "application/json", ".mjs": "text/javascript" };
async function serveFile(res, path, type) {
  try {
    const data = await readFile(path);
    res.writeHead(200, { "content-type": type, "content-length": data.length });
    res.end(data);
  } catch {
    res.writeHead(404);
    res.end("missing");
  }
}

const temp = await mkdtemp(join(tmpdir(), "pt-player-chrome-"));
const children = [];
let ws;
function until(fn, message, timeout = 30000) {
  const start = Date.now();
  return (async () => {
    while (Date.now() - start < timeout) {
      try {
        const result = await fn();
        if (result) return result;
      } catch { /* retry */ }
      await delay(100);
    }
    throw new Error(message);
  })();
}
let server;
try {
  server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://127.0.0.1");
    if (url.pathname === "/") return serveInline(res);
    if (url.pathname.startsWith("/vendor/")) {
      const name = url.pathname.slice("/vendor/".length);
      if (!/^[A-Za-z0-9_.-]+\.js$/.test(name)) {
        res.writeHead(404);
        res.end("missing");
        return;
      }
      return serveFile(res, join(pkgDir, "node_modules", "three", "build", name), MIME[".js"]);
    }
    if (url.pathname.startsWith("/dist/")) {
      return serveFile(res, join(pkgDir, url.pathname), MIME[".js"]);
    }
    if (url.pathname.startsWith("/fixture/")) {
      return serveFile(res, join(root, "fixtures", "player", "two-scene-local", url.pathname.slice("/fixture/".length)), "application/octet-stream");
    }
    res.writeHead(404);
    res.end("missing");
    function serveInline(r) {
      const data = Buffer.from(PAGE);
      r.writeHead(200, { "content-type": "text/html", "content-length": data.length });
      r.end(data);
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  const pageUrl = `http://127.0.0.1:${port}/`;

  const chrome = spawn(CHROME, [
    "--headless=new", "--no-first-run", "--no-default-browser-check",
    "--disable-background-networking",
    "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
    "--remote-debugging-port=0", `--user-data-dir=${join(temp, "chrome")}`,
    "about:blank",
  ], { stdio: "ignore" });
  children.push(chrome);
  chrome.on("error", (error) => {
    console.error(`chrome spawn error: ${error.message}`);
  });
  const debugPort = await until(async () =>
    String(await readFile(join(temp, "chrome", "DevToolsActivePort"))).split("\n")[0],
  "Chrome did not start");
  const page = await (await fetch(`http://127.0.0.1:${debugPort}/json/new?${encodeURIComponent(pageUrl)}`, { method: "PUT" })).json();
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  let sequence = 0;
  const pending = new Map();
  ws.onmessage = ({ data }) => {
    const m = JSON.parse(data);
    if (m.id) {
      const p = pending.get(m.id);
      pending.delete(m.id);
      if (m.error) p.reject(new Error(m.error.message));
      else p.resolve(m.result);
    }
  };
  const call = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async (expression) => {
    const result = await call("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) {
      throw new Error(`page exception: ${result.exceptionDetails.text ?? JSON.stringify(result.exceptionDetails.exception?.description ?? result.exceptionDetails).slice(0, 300)}`);
    }
    return result.result.value;
  };
  await call("Page.enable");
  await call("Emulation.setDeviceMetricsOverride", { width: 360, height: 220, deviceScaleFactor: 1, mobile: false });
  await until(() => evaluate(`typeof window.__run === "function"`), "harness module did not boot");
  const out = await evaluate(`window.__run()`);
  if (out.errors?.length) throw new Error(`console errors: ${out.errors.join("; ").slice(0, 400)}`);
  const failures = [];
  if (!(out.lit.nonblackFraction > 0.02)) failures.push(`lit frame has no visible content (nonblack=${out.lit.nonblackFraction})`);
  if (!(out.lit.leftRed > out.lit.rightRed * 1.5)) failures.push(`orientation flipped (leftRed=${out.lit.leftRed}, rightRed=${out.lit.rightRed})`);
  if (!(out.lit.topMax > 0.5 && out.lit.bottomLum < 0.12)) failures.push(`vertical markers wrong (topMax=${out.lit.topMax}, bottom=${out.lit.bottomLum})`);
  if (!(out.dark.maxChannel <= 1 / 255 + 1e-9)) failures.push(`blackout leaks (max=${out.dark.maxChannel})`);
  if (out.lit.accepted !== false || out.lit.provenance !== "diagnostic") failures.push("capture stamp wrong");
  const shot = await call("Page.captureScreenshot", { format: "png" });
  await evaluate(`window.__teardown()`);
  await writeFile(evidencePng, Buffer.from(shot.data, "base64"));
  await writeFile(evidenceJson, JSON.stringify({
    tool: "check:chrome", profile: "paper-player-r186-v1", sink: [320, 180],
    lit: out.lit, dark: out.dark, failures,
    browser: "headless Chrome (SwiftShader)",
  }, null, 2));
  if (failures.length) throw new Error(`chrome capture failed: ${failures.join("; ")}`);
  console.log(`check:chrome pass (lit nonblack=${out.lit.nonblackFraction.toFixed(3)}, dark max=${out.dark.maxChannel})`);
} catch (error) {
  console.error(`check:chrome failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
} finally {
  try {
    ws?.close();
  } catch { /* ignore */ }
  for (const child of children.reverse()) {
    try {
      child.kill("SIGKILL");
    } catch { /* already exited */ }
  }
  await delay(200);
  if (server) await new Promise((resolve) => server.close(resolve));
  await rm(temp, { recursive: true, force: true });
}
