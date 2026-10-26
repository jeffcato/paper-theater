#!/usr/bin/env node
// check:plaque — verify the mounted read-along plaque's actual DOM/layout
// behavior in headless Chrome: active two-to-three-line chunk display,
// measured group highlight from media samples, toggle, seek/rewind, resize
// reflow, and 100%/200% text. The real player PlaqueView runs against the
// real 176-word shared track. Evidence: plaque.png + plaque.json.
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(pkgDir, "..", "..");
const evidencePng = join(root, "docs", "implementation", "player", "plaque.png");
const evidencePng2 = join(root, "docs", "implementation", "player", "plaque-200.png");
const evidenceJson = join(root, "docs", "implementation", "player", "plaque.json");
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<style>
body { margin: 0; background: #f6efe2; font-family: Georgia, serif; }
#plaque { width: 640px; margin: 16px; border: 2px solid #8a6d2f; border-radius: 8px; background: #1d1712; color: #f6efe2; padding: 12px 16px; min-height: 64px; }
.pl-group { border-radius: 3px; padding: 0 0.1em; }
.pl-group.pl-active { background: #8a6d2f; color: #1d1712; }
</style></head>
<body><div id="plaque"></div>
<script type="module">
import * as api from "/dist/src/index.js";
window.__errors = [];
addEventListener("error", (e) => window.__errors.push(String(e.message)));
addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason?.message ?? e.reason)));
function snapshot(plaque, el) {
  const chunk = el.querySelector(".pl-chunk");
  const groups = chunk ? [...chunk.querySelectorAll(".pl-group")] : [];
  const active = chunk ? chunk.querySelector(".pl-group.pl-active") : null;
  const cs = getComputedStyle(chunk ?? el);
  return {
    containerChildren: el.childElementCount,
    chunkChildren: groups.length,
    chunkText: chunk ? chunk.textContent : "",
    activeGroup: active ? active.dataset.groupId : null,
    ariaCount: chunk ? chunk.querySelectorAll('[aria-current="true"]').length : 0,
    fontSize: chunk ? cs.fontSize : null,
    display: getComputedStyle(el).display,
  };
}
window.__run = async () => {
  const res = await fetch("/shared/read-along.json");
  if (!res.ok) throw new Error("track fetch failed");
  const track = await res.json();
  const el = document.querySelector("#plaque");
  const plaque = new api.PlaqueView(el, track.words, track.groups, {});
  await plaque.ready;
  const out = { steps: [] };
  const step = (name, extra = {}) => {
    const snap = snapshot(plaque, el);
    snap.name = name;
    Object.assign(snap, extra);
    out.steps.push(snap);
    return snap;
  };
  const firstGroup = track.groups[0];
  plaque.setSample(firstGroup.start_sample);
  const s0 = step("first-sample", { expectGroup: firstGroup.id });
  // Seeks across the track, then a rewind to the first sample.
  const mid = track.groups[Math.floor(track.groups.length / 2)];
  plaque.setSample(mid.start_sample);
  step("seek-mid", { expectGroup: mid.id });
  const last = track.groups[track.groups.length - 1];
  plaque.setSample(last.start_sample);
  step("seek-last", { expectGroup: last.id });
  plaque.setSample(firstGroup.start_sample);
  const rewound = step("rewind-first", { expectGroup: firstGroup.id });
  rewound.rewoundTextMatches = rewound.chunkText === s0.chunkText;
  // Toggle off: nothing in the tree. Toggle on: same chunk returns.
  plaque.setEnabled(false);
  step("toggle-off");
  plaque.setEnabled(true);
  plaque.setSample(firstGroup.start_sample);
  step("toggle-on", { expectGroup: firstGroup.id });
  // Resize reflow: active group survives, word samples untouched.
  el.style.width = "320px";
  plaque.reflow();
  const resized = step("resize-320", { expectGroup: firstGroup.id });
  resized.activePreserved = resized.activeGroup === firstGroup.id;
  el.style.width = "640px";
  plaque.reflow();
  // 200% text: 44px serif, active group survives.
  plaque.setScale(2);
  plaque.setSample(mid.start_sample);
  const scaled = step("scale-200", { expectGroup: mid.id });
  scaled.activePreserved = scaled.activeGroup === mid.id;
  // Silence past the last group: highlight cleared, chunk retained.
  plaque.setSample(last.end_sample + 1);
  step("silence");
  out.errors = window.__errors;
  out.firstGroupId = firstGroup.id;
  window.__shot = (scale) => {
    plaque.setEnabled(true);
    plaque.setScale(scale);
    plaque.setSample(mid.start_sample);
    return snapshot(plaque, el);
  };
  window.__teardown = () => plaque.dispose();
  return out;
};
</script></body></html>`;

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

const temp = await mkdtemp(join(tmpdir(), "pt-player-plaque-"));
const children = [];
let ws;
let server;
try {
  server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://127.0.0.1");
    if (url.pathname === "/") {
      const data = Buffer.from(PAGE);
      res.writeHead(200, { "content-type": "text/html", "content-length": data.length });
      res.end(data);
      return;
    }
    if (url.pathname.startsWith("/dist/")) return serveFile(res, join(pkgDir, url.pathname), "text/javascript");
    if (url.pathname.startsWith("/shared/")) {
      const name = url.pathname.slice("/shared/".length);
      if (name.includes("..")) {
        res.writeHead(404);
        res.end("missing");
        return;
      }
      return serveFile(res, join(root, "fixtures", "shared", "two-scene-v1", name), "application/octet-stream");
    }
    res.writeHead(404);
    res.end("missing");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  const pageUrl = `http://127.0.0.1:${port}/`;

  const chrome = spawn(CHROME, [
    "--headless=new", "--no-first-run", "--no-default-browser-check",
    "--disable-background-networking",
    "--remote-debugging-port=0", `--user-data-dir=${join(temp, "chrome")}`,
    "about:blank",
  ], { stdio: "ignore" });
  children.push(chrome);
  chrome.on("error", (error) => console.error(`chrome spawn error: ${error.message}`));
  const debugPort = await (async () => {
    const start = Date.now();
    while (Date.now() - start < 30000) {
      try {
        return String(await readFile(join(temp, "chrome", "DevToolsActivePort"))).split("\n")[0];
      } catch { await delay(100); }
    }
    throw new Error("Chrome did not start");
  })();
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
      throw new Error(`page exception: ${JSON.stringify(result.exceptionDetails).slice(0, 400)}`);
    }
    return result.result.value;
  };
  await call("Page.enable");
  await call("Emulation.setDeviceMetricsOverride", { width: 720, height: 420, deviceScaleFactor: 1, mobile: false });
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 60000) {
      try {
        if (await evaluate(`typeof window.__run === "function"`)) return;
      } catch { /* retry */ }
      await delay(200);
    }
    throw new Error("harness module did not boot");
  })();
  const out = await evaluate(`window.__run()`);
  if (out.errors?.length) throw new Error(`console errors: ${out.errors.join("; ").slice(0, 400)}`);
  const byName = Object.fromEntries(out.steps.map((s) => [s.name, s]));
  const firstGroupId = out.firstGroupId;
  const failures = [];
  const check = (cond, message) => {
    if (!cond) failures.push(message);
  };
  // Exactly one chunk in the tree; only measured groups inside it.
  for (const step of out.steps) {
    if (step.name === "toggle-off") continue;
    check(step.containerChildren === 1, `${step.name}: want 1 container child, got ${step.containerChildren}`);
    check(step.chunkChildren >= 1, `${step.name}: empty chunk`);
    check(step.ariaCount <= 1, `${step.name}: ${step.ariaCount} aria-current marks`);
  }
  check(byName["first-sample"].activeGroup === firstGroupId, "first sample highlights first group");
  check((byName["first-sample"].chunkText ?? "").length > 0, "active chunk renders measured text");
  check(byName["seek-mid"].activeGroup === byName["seek-mid"].expectGroup, "seek moves highlight");
  check(byName["seek-last"].activeGroup === byName["seek-last"].expectGroup, "late seek highlights");
  check(byName["rewind-first"].rewoundTextMatches === true, "rewind returns earlier chunk text");
  check(byName["toggle-off"].display === "none", "toggle hides plaque");
  check(byName["toggle-off"].chunkChildren === 0, "hidden plaque keeps no text");
  check(byName["toggle-on"].activeGroup === firstGroupId, "toggle restores chunk");
  check(byName["resize-320"].activePreserved === true, "resize reflow preserves active group");
  check(byName["scale-200"].fontSize === "44px", `200% text is 44px (got ${byName["scale-200"].fontSize})`);
  check(byName["scale-200"].activePreserved === true, "200% reflow preserves active group");
  check(byName.silence.activeGroup === null && byName.silence.ariaCount === 0, "silence clears highlight");
  check(byName.silence.chunkChildren >= 1, "silence retains last chunk");
  // Evidence: mid-track plaque at 100% and 200%.
  await evaluate(`window.__shot(1)`);
  const shot = await call("Page.captureScreenshot", { format: "png" });
  await writeFile(evidencePng, Buffer.from(shot.data, "base64"));
  await evaluate(`window.__shot(2)`);
  const shot2 = await call("Page.captureScreenshot", { format: "png" });
  await writeFile(evidencePng2, Buffer.from(shot2.data, "base64"));
  await evaluate(`window.__teardown()`);
  await writeFile(evidenceJson, JSON.stringify({
    tool: "check:plaque", track: "fixtures/shared/two-scene-v1/read-along.json",
    steps: out.steps.map(({ name, containerChildren, chunkChildren, chunkText, activeGroup, ariaCount, fontSize, display, expectGroup, rewoundTextMatches, activePreserved }) => (
      { name, containerChildren, chunkChildren, chunkText, activeGroup, ariaCount, fontSize, display, expectGroup, rewoundTextMatches, activePreserved })),
    failures, browser: "headless Chrome",
  }, null, 2));
  if (failures.length) throw new Error(`plaque run failed: ${failures.join("; ")}`);
  console.log("check:plaque pass (chunk/highlight/toggle/seek/resize/scale verified in DOM)");
} catch (error) {
  console.error(`check:plaque failed: ${error instanceof Error ? error.message : String(error)}`);
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
  const { rm } = await import("node:fs/promises");
  await rm(temp, { recursive: true, force: true });
}
