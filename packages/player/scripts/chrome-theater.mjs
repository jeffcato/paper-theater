#!/usr/bin/env node
// check:theater — combined theater+plaque browser harness (P4). Mounts the
// readiness-v2 diagnostic performance with the real <audio> clock, the
// three.js stage sink and the mount-owned PlaqueView fed by mountPlayer's
// sampled clock. At 960x540, 1280x720 and 1920x1080 (DPR 1) it asserts:
// actual audio play/pause/seek/rewind with visible group agreement,
// toggle/seek/rewind/silence, projected plaque placement, 2-3 measured
// lines, >=8px floor clearance, future-text exclusion, bundled Noto Serif
// readiness (fails loudly on fallback), and a P12 frame capture per
// viewport. Evidence: theater.json + theater-{WxH}[-200].png.
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(pkgDir, "..", "..");
const fixtureRoot = join(root, "fixtures", "generation", "two-scene-readiness-v2");
const evidenceJson = join(root, "docs", "implementation", "player", "theater.json");
const shotName = (w, h, s) => join(
  root, "docs", "implementation", "player",
  s === 2 ? `theater-${w}x${h}-200.png` : s === "extreme" ? `theater-${w}x${h}-extreme.png` : `theater-${w}x${h}.png`,
);
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const VIEWPORTS = [[960, 540], [1280, 720], [1920, 1080]];

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<link rel="stylesheet" href="/fonts/noto-serif.css">
<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script>
<style>
html, body { margin: 0; padding: 0; background: #141210; }
#theater { position: relative; overflow: hidden; }
#theater canvas { position: absolute; left: 0; top: 0; display: block; }
#positioner { position: absolute; box-sizing: border-box; padding: 6px 10px;
  background: #1d1712; border: 2px solid #8a6d2f; border-radius: 8px; }
#plaque { color: #f6efe2; }
.pl-group { border-radius: 3px; padding: 0 0.1em; }
.pl-group.pl-active { background: #8a6d2f; color: #1d1712; }
</style></head>
<body><div id="theater"><audio id="audio" muted preload="auto" src="/fx/audio/master.wav"></audio></div>
<script type="module">
import * as api from "/dist/src/index.js";
window.__errors = [];
window.__roLoopNotices = 0;
addEventListener("error", (e) => {
  // Benign settled-layout notice from rapid programmatic resizes (each
  // place() settles before we measure); counted in the report, never mixed
  // with real failures.
  if (String(e.message).includes("ResizeObserver loop completed with undelivered notifications")) {
    window.__roLoopNotices++;
    return;
  }
  window.__errors.push(String(e.message));
});
addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason?.message ?? e.reason)));
async function sha256(bytes) {
  const d = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(d)].map((n) => n.toString(16).padStart(2, "0")).join("");
}
async function load(path, maxBytes) {
  const res = await fetch(path);
  if (!res.ok) throw new Error("fetch failed: " + path);
  const buf = new Uint8Array(await res.arrayBuffer());
  if (buf.length > maxBytes) throw new Error("over bound: " + path);
  return buf;
}
const text = (v) => new TextDecoder().decode(v);
function domActive() {
  const el = document.querySelector("#plaque .pl-group.pl-active");
  return el ? el.dataset.groupId : null;
}
function lineBoxes() {
  const spans = [...document.querySelectorAll("#plaque .pl-group")];
  const boxes = [];
  for (const span of spans) {
    const range = document.createRange();
    range.selectNodeContents(span);
    for (const rect of range.getClientRects()) {
      boxes.push({ x: rect.left, y: rect.top, w: rect.width, h: rect.height });
    }
  }
  // Cluster into rows by vertical center.
  const rows = [];
  for (const box of boxes.sort((a, b) => a.y - b.y)) {
    const row = rows.find((r) => Math.abs((r.y + r.h / 2) - (box.y + box.h / 2)) < 3);
    if (row) {
      row.w = Math.max(row.x + row.w, box.x + box.w) - Math.min(row.x, box.x);
      row.x = Math.min(row.x, box.x);
      row.y = Math.min(row.y, box.y);
      row.h = Math.max(row.h, box.h);
    } else rows.push({ ...box });
  }
  return rows;
}
function measureLayout(projected) {
  const positioner = document.querySelector("#positioner");
  const container = document.querySelector("#plaque");
  const chunk = container.querySelector(".pl-chunk");
  const pos = positioner.getBoundingClientRect();
  const box = container.getBoundingClientRect();
  const cs = getComputedStyle(container);
  const content = {
    left: box.left + parseFloat(cs.paddingLeft),
    right: box.right - parseFloat(cs.paddingRight),
  };
  const rows = lineBoxes();
  const chunkText = chunk ? chunk.textContent : "";
  const fontSize = chunk ? getComputedStyle(chunk).fontSize : null;
  return {
    rows,
    lineCount: rows.length,
    chunkText,
    fontSize,
    chunkTop: rows.length ? Math.min(...rows.map((r) => r.y)) : null,
    chunkBottom: rows.length ? Math.max(...rows.map((r) => r.y + r.h)) : null,
    overlay: { left: pos.left, top: pos.top, width: pos.width, height: pos.height },
    inset: content,
    insetWidth: content.right - content.left,
  };
}
window.__boot = async (W, H) => {
  const out = { width: W, height: H };
  const theater = document.querySelector("#theater");
  theater.style.width = W + "px";
  theater.style.height = H + "px";
  const manifestBytes = await load("/fx/manifest.json", 1024 * 1024);
  const manifestDigest = await sha256(manifestBytes);
  const read = async (path) => load("/fx/" + path, 64 * 1024 * 1024);
  const policy = { player_sha256: "0".repeat(64), checker_sha256: "0".repeat(64), recipe_sha256: "0".repeat(64), allow_diagnostic: false };
  const handle = await api.loadDiagnostic(manifestBytes, manifestDigest, read, policy);
  const timeline = JSON.parse(text(await load("/fx/timeline.json", 1024 * 1024)));
  const actorsDoc = JSON.parse(text(await load("/fx/actors.json", 1024 * 1024)));
  const figures = actorsDoc.map((card) => ({
    asset_id: card.id, url: "/fx/" + card.image.path, pixel_size: card.pixel_size,
    pivot_top_left: card.pivot_top_left, stage_size: card.stage_size, allow_mirror: card.allow_mirror,
  }));
  const track = JSON.parse(text(await load("/fx/read-along.json", 4 * 1024 * 1024)));
  const model = new api.ReadAlongModel(track.words, track.groups, 48000);
  const sink = await api.createThreeSink(W, H, { figures });
  sink.canvas.style.width = W + "px";
  sink.canvas.style.height = H + "px";
  theater.appendChild(sink.canvas);
  const audio = document.querySelector("#audio");
  // Projected below-stage placement from the profile geometry.
  const projected = api.projectPlaqueSafe(api.baseCameraPose(), { width: W, height: H });
  out.projected = projected;
  const positioner = document.createElement("div");
  positioner.id = "positioner";
  positioner.style.left = projected.left + "px";
  positioner.style.top = projected.top + "px";
  positioner.style.width = projected.width + "px";
  positioner.style.minHeight = projected.height + "px";
  const plaqueEl = document.createElement("div");
  plaqueEl.id = "plaque";
  positioner.appendChild(plaqueEl);
  theater.appendChild(positioner);
  const mount = api.mountPlayer(theater, { handle, timeline }, {
    clock: audio, sink, candidate_id: "theater-run-1", readAlong: { words: track.words, groups: track.groups, container: plaqueEl },
  });
  window.__timeline = timeline;
  window.__mount = mount;
  window.__sink = sink;
  window.__pose = api.baseCameraPose();
  window.__W = W;
  window.__H = H;
  window.__model = model;
  window.__track = track;
  window.__audio = audio;
  await mount.plaque.ready;
  out.fontReady = await mount.plaque.fontReady;
  out.fontsCheckFull = document.fonts.check('22px "Noto Serif"', track.words.map((w) => w.text + w.following_text).join(""));
  const firstGroup = track.groups[0];
  const midGroup = track.groups[Math.floor(track.groups.length / 2)];
  const lastGroup = track.groups[track.groups.length - 1];
  window.__stops = { first: firstGroup.start_sample, mid: midGroup.start_sample, lastEnd: lastGroup.end_sample };
  // Actual audio playback with the sampled clock feeding stage + plaque.
  mount.seek(firstGroup.start_sample);
  mount.tick();
  out.firstChunkText = plaqueEl.textContent;
  await audio.play();
  const t0 = audio.currentTime;
  const frames = [];
  await new Promise((resolve) => {
    const start = performance.now();
    const step = () => {
      mount.tick();
      const sample = mount.sample;
      frames.push({ sample, active: domActive(), expect: model.groupAt(sample)?.id ?? null });
      if (performance.now() - start < 1500) requestAnimationFrame(step);
      else resolve();
    };
    requestAnimationFrame(step);
  });
  out.playedT0 = t0;
  out.playedT1 = audio.currentTime;
  out.playFrames = frames.length;
  out.mismatches = frames.filter((f) => f.active !== f.expect).slice(0, 5);
  out.mismatchCount = frames.filter((f) => f.active !== f.expect).length;
  // Pause freezes the delivered sample; seek jumps; restart returns.
  mount.pause();
  mount.tick();
  out.pausedSample = mount.sample;
  out.pauseReason = mount.reason;
  mount.seek(midGroup.start_sample);
  mount.tick();
  out.seekSample = mount.sample;
  out.seekActive = domActive();
  out.seekExpect = midGroup.id;
  mount.restart();
  audio.currentTime = 0;
  mount.tick();
  out.restartSample = mount.sample;
  // Rewind returns the earlier chunk text; silence clears the highlight.
  mount.seek(firstGroup.start_sample);
  mount.tick();
  out.rewoundTextMatches = plaqueEl.textContent === out.firstChunkText;
  mount.seek(lastGroup.end_sample + 48000);
  mount.tick();
  out.silenceActive = domActive();
  out.silenceChunkKept = (plaqueEl.querySelectorAll(".pl-group").length ?? 0) > 0;
  // Toggle through the mount control.
  mount.setReadAlong(false);
  mount.tick();
  out.toggleOff = { display: getComputedStyle(plaqueEl).display, groups: plaqueEl.querySelectorAll(".pl-group").length };
  mount.setReadAlong(true);
  mount.seek(midGroup.start_sample);
  mount.tick();
  out.toggleOnActive = domActive();
  out.toggleOnExpect = midGroup.id;
  // Future-text exclusion at the mid stop.
  const futureText = track.groups.slice(-3).map((g) => g.word_ids.map((id) => track.words.find((w) => w.id === id).text).join(" ")).join(" ");
  out.futureTextExcluded = !plaqueEl.textContent.includes(futureText.split(" ")[0]);
  // Refit flow per scale: measure, grow, refit the camera to the grown
  // shell, reposition the overlay from the refit projection, re-measure.
  const raf2 = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  const place = (proj) => {
    positioner.style.left = proj.left + "px";
    positioner.style.top = proj.top + "px";
    positioner.style.width = proj.width + "px";
    positioner.style.minHeight = proj.height + "px";
    positioner.style.height = "";
  };
  const extend = (proj, layout) => {
    const chunkHeight = (layout.chunkBottom ?? 0) - (layout.chunkTop ?? 0);
    positioner.style.height = Math.max(proj.height, chunkHeight + 12) + "px";
  };
  window.__layout = async (scale) => {
    mount.plaque.setScale(scale);
    await raf2();
    mount.seek(midGroup.start_sample);
    mount.tick();
    const before = measureLayout();
    const projBefore = api.projectPlaqueSafe(window.__pose, { width: W, height: H });
    const worldPerPx = 13 / projBefore.width;
    const needed = before.lineCount * parseFloat(before.fontSize) * 1.3 * worldPerPx;
    const growth = api.plaqueGrowthSteps(Math.max(needed, 0.01), 1.9);
    const shell = api.refitShell(growth);
    const pose = api.envelopeCameraPose([0, 0], 0, [0, 0], false, shell);
    window.__pose = pose;
    window.__shell = shell;
    window.__sink.setCameraPose(pose);
    const proj = api.projectPlaqueSafe(pose, { width: W, height: H });
    window.__proj = proj;
    place(proj);
    await raf2();
    mount.seek(midGroup.start_sample);
    mount.tick();
    extend(proj, measureLayout());
    const layout = measureLayout();
    return { layout, growth, projected: proj, shell, distance: shell.distance };
  };
  const EXTREMES = [
    { offset: [0.5, 0.25], dolly: 2, pointer: [0.8, 0.4], reduced: false },
    { offset: [-0.5, -0.25], dolly: 2, pointer: [-0.8, -0.4], reduced: false },
    { offset: [0.5, -0.25], dolly: 0, pointer: [0.8, -0.4], reduced: false },
    { offset: [0, 0], dolly: 0, pointer: [0.8, 0.4], reduced: true },
  ];
  window.__extremes = async () => {
    const rows = [];
    for (const def of EXTREMES) {
      const pose = api.envelopeCameraPose(def.offset, def.dolly, def.pointer, def.reduced, window.__shell);
      window.__sink.setCameraPose(pose);
      const proj = api.projectPlaqueSafe(pose, { width: W, height: H });
      place(proj);
      await raf2();
      mount.seek(midGroup.start_sample);
      mount.tick();
      extend(proj, measureLayout());
      const layout = measureLayout();
      rows.push({
        def: def.offset + "+d" + def.dolly + "+p" + def.pointer + (def.reduced ? "+rm" : ""),
        active: domActive(),
        expect: midGroup.id,
        layout,
        projected: proj,
      });
    }
    // Restore the refit pose for screenshots.
    window.__sink.setCameraPose(window.__pose);
    place(window.__proj);
    await raf2();
    mount.seek(midGroup.start_sample);
    mount.tick();
    return rows;
  };
  const first = await window.__layout(1);
  out.layout1b = first.layout;
  out.growthSteps1 = first.growth;
  out.projected1 = first.projected;
  out.refitDistance1 = first.distance;
  out.extremes1 = await window.__extremes();
  const second = await window.__layout(2);
  out.layout2b = second.layout;
  out.growthSteps2 = second.growth;
  out.projected2 = second.projected;
  out.refitDistance2 = second.distance;
  out.extremes2 = await window.__extremes();
  out.projected = out.projected2;
  // Blackout retention: scan the pure evaluator for a zero-gain sample and
  // prove the composite pipeline renders it black. A fixture with no
  // full-blackout sample reports the skip instead of certifying one.
  const reveals = handle.reveals ?? [];
  let blackSample = -1;
  for (let s = 0; s <= timeline.sample_count; s += 24000) {
    if (api.evaluate(timeline, s, [...reveals]).stage_gain === 0) {
      blackSample = s;
      break;
    }
  }
  out.blackoutSample = blackSample;
  if (blackSample >= 0) {
    mount.seek(blackSample);
    mount.tick();
    const dark = await mount.captureFrame(W, H);
    let darkMax = 0;
    for (let i = 0; i < dark.pixels.length; i += 4) {
      darkMax = Math.max(darkMax, dark.pixels[i] / 255, dark.pixels[i + 1] / 255, dark.pixels[i + 2] / 255);
    }
    out.blackoutMax = darkMax;
    out.blackoutPhase = dark.phase;
  }
  out.errors = window.__errors;
  return out;
};
// Screenshot poses on the live mount (refit applied inside __layout).
window.__poseForShot = async (scale) => {
  const mount = window.__mount;
  mount.setReadAlong(true);
  const applied = await window.__layout(scale);
  await new Promise((r) => requestAnimationFrame(r));
  return { active: domActive(), fontSize: getComputedStyle(document.querySelector("#plaque .pl-chunk")).fontSize };
};
window.__poseForExtreme = async () => {
  const mount = window.__mount;
  mount.setReadAlong(true);
  await window.__layout(2);
  const def = { offset: [0.5, 0.25], dolly: 2, pointer: [0.8, 0.4], reduced: false };
  const pose = api.envelopeCameraPose(def.offset, def.dolly, def.pointer, def.reduced, window.__shell);
  window.__sink.setCameraPose(pose);
  const proj = api.projectPlaqueSafe(pose, { width: window.__W, height: window.__H });
  const positioner = document.querySelector("#positioner");
  positioner.style.left = proj.left + "px";
  positioner.style.top = proj.top + "px";
  positioner.style.width = proj.width + "px";
  positioner.style.minHeight = proj.height + "px";
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  mount.seek(window.__stops.mid);
  mount.tick();
  await new Promise((r) => requestAnimationFrame(r));
  return { active: domActive() };
};
// P12 retention + release, after screenshots.
window.__finish = async (W, H) => {
  const out = {};
  const mount = window.__mount;
  const plaqueEl = document.querySelector("#plaque");
  const frame = await mount.captureFrame(W, H);
  let nonblack = 0;
  for (let i = 0; i < frame.pixels.length; i += 4) {
    if (frame.pixels[i] > 8 || frame.pixels[i + 1] > 8 || frame.pixels[i + 2] > 8) nonblack++;
  }
  out.capture = {
    sample: frame.sample, phase: frame.phase, provenance: frame.provenance,
    pixels_sha256: await sha256(frame.pixels),
    nonblackFraction: nonblack / (frame.pixels.length / 4),
  };
  const timeline = window.__timeline;
  const nodCue = timeline.cues.find((c) => c.id === "nod-em");
  out.nodGestures = {};
  if (nodCue) {
    for (const d of [6000, 12000, 18000]) {
      mount.seek(nodCue.start_sample + d);
      const state = mount.tick();
      out.nodGestures[nodCue.start_sample + d] = (state.actors.find((a) => a.instance_id === "em-2") ?? {}).gesture ?? null;
    }
  }
  mount.dispose();
  out.disposeClean = plaqueEl.childElementCount === 0 && document.querySelectorAll(".pl-chunk").length === 0;
  out.errors = window.__errors;
  out.roLoopNotices = window.__roLoopNotices;
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

const failures = [];
const check = (cond, message) => {
  if (!cond) failures.push(message);
};

const temp = await mkdtemp(join(tmpdir(), "pt-player-theater-"));
const children = [];
let ws;
let server;
const report = { tool: "check:theater", fixture: "fixtures/generation/two-scene-readiness-v2", viewports: [] };
try {
  server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://127.0.0.1");
    if (url.pathname === "/") {
      const data = Buffer.from(PAGE);
      res.writeHead(200, { "content-type": "text/html", "content-length": data.length });
      res.end(data);
      return;
    }
    if (url.pathname.startsWith("/vendor/")) {
      const name = url.pathname.slice("/vendor/".length);
      if (!/^[A-Za-z0-9_.-]+\.js$/.test(name)) {
        res.writeHead(404);
        res.end("missing");
        return;
      }
      return serveFile(res, join(pkgDir, "node_modules", "three", "build", name), "text/javascript");
    }
    if (url.pathname.startsWith("/dist/")) return serveFile(res, join(pkgDir, url.pathname), "text/javascript");
    if (url.pathname.startsWith("/fonts/")) {
      const name = url.pathname.slice("/fonts/".length);
      if (name.includes("..") || name.includes("/")) {
        res.writeHead(404);
        res.end("missing");
        return;
      }
      const type = name.endsWith(".css") ? "text/css" : name.endsWith(".woff2") ? "font/woff2" : "text/plain";
      return serveFile(res, join(pkgDir, "fonts", name), type);
    }
    if (url.pathname.startsWith("/fx/")) {
      const name = url.pathname.slice("/fx/".length);
      if (name.includes("..")) {
        res.writeHead(404);
        res.end("missing");
        return;
      }
      return serveFile(res, join(fixtureRoot, name), "application/octet-stream");
    }
    res.writeHead(404);
    res.end("missing");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;

  const chrome = spawn(CHROME, [
    "--headless=new", "--no-first-run", "--no-default-browser-check",
    "--disable-background-networking", "--remote-debugging-port=0",
    "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
    "--autoplay-policy=no-user-gesture-required", "--mute-audio",
    `--user-data-dir=${join(temp, "chrome")}`, "about:blank",
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
  const debuggerUrl = `http://127.0.0.1:${debugPort}`;

  for (const [W, H] of VIEWPORTS) {
    const tag = `${W}x${H}`;
    const target = await (await fetch(`${debuggerUrl}/json/new?${encodeURIComponent(`http://127.0.0.1:${port}/`)}`, { method: "PUT" })).json();
    const socket = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      socket.onopen = resolve;
      socket.onerror = reject;
    });
    let sequence = 0;
    const pending = new Map();
    socket.onmessage = ({ data }) => {
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
      socket.send(JSON.stringify({ id, method, params }));
    });
    const evaluate = async (expression) => {
      const result = await call("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
      if (result.exceptionDetails) {
        throw new Error(`page exception: ${JSON.stringify(result.exceptionDetails).slice(0, 500)}`);
      }
      return result.result.value;
    };
    try {
      await call("Page.enable");
      await call("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
      await (async () => {
        const start = Date.now();
        while (Date.now() - start < 90000) {
          try {
            if (await evaluate(`typeof window.__boot === "function"`)) return;
          } catch { /* retry */ }
          await delay(200);
        }
        throw new Error("theater harness did not boot");
      })();
      const out = await evaluate(`window.__boot(${W}, ${H})`);
      const pose1 = await evaluate(`window.__poseForShot(1)`);
      const shot1 = await call("Page.captureScreenshot", { format: "png" });
      await writeFile(shotName(W, H, 1), Buffer.from(shot1.data, "base64"));
      const pose2 = await evaluate(`window.__poseForShot(2)`);
      const shot2 = await call("Page.captureScreenshot", { format: "png" });
      await writeFile(shotName(W, H, 2), Buffer.from(shot2.data, "base64"));
      const poseX = await evaluate(`window.__poseForExtreme()`);
      const shotX = await call("Page.captureScreenshot", { format: "png" });
      await writeFile(shotName(W, H, "extreme"), Buffer.from(shotX.data, "base64"));
      const tail = await evaluate(`window.__finish(${W}, ${H})`);
      Object.assign(out, tail, { pose1, pose2, poseX });

      // Assertions: every number below is measured in the browser.
      const at = (name) => `${tag} ${name}`;
      check(out.fontReady === true, at("Noto Serif readiness (fallback is not certified)"));
      check(out.fontsCheckFull === true, at("bundled face covers the full track text"));
      check((out.errors ?? []).length === 0, at(`console errors: ${(out.errors ?? []).join("; ").slice(0, 200)}`));
      check(out.playedT1 - out.playedT0 >= 0.5, at(`audio clock advanced (${out.playedT0} -> ${out.playedT1})`));
      check(out.playFrames >= 20, at(`enough playback frames (${out.playFrames})`));
      check(out.mismatchCount === 0, at(`sample/group agreement (${out.mismatchCount} mismatches)`));
      check(out.pauseReason === "user", at(`pause reason (${out.pauseReason})`));
      check(out.seekActive === out.seekExpect, at("seek lands on the measured group"));
      check(out.restartSample === 0, at(`restart returns to 0 (${out.restartSample})`));
      check(out.rewoundTextMatches === true, at("rewind returns the earlier chunk text"));
      check(out.silenceActive === null || out.silenceActive === undefined, at("silence clears the highlight"));
      check(out.silenceChunkKept === true, at("silence retains the last chunk"));
      check(out.toggleOff.display === "none" && out.toggleOff.groups === 0, at("toggle hides the plaque"));
      check(out.toggleOnActive === out.toggleOnExpect, at("toggle restores the measured group"));
      check(out.futureTextExcluded === true, at("no future text in the tree"));
      // The refit eliminates growth overflow: every scale stays in frame.
      for (const [scale, layout, growth, proj] of [
        [1, out.layout1b, out.growthSteps1, out.projected1],
        [2, out.layout2b, out.growthSteps2, out.projected2],
      ]) {
        check(Math.abs(layout.overlay.left - proj.left) <= 2, at(`scale ${scale}: overlay left tracks refit`));
        check(Math.abs(layout.overlay.top - proj.top) <= 2, at(`scale ${scale}: overlay top tracks refit`));
        check(Math.abs(layout.overlay.width - proj.width) <= 2, at(`scale ${scale}: overlay width tracks refit`));
        check(layout.overlay.height + 0.5 >= proj.height, at(`scale ${scale}: overlay covers the safe inset`));
        check(layout.lineCount >= 2 && layout.lineCount <= 3, at(`scale ${scale}: 2-3 measured lines (${layout.lineCount})`));
        const widest = layout.rows.reduce((m, r) => Math.max(m, r.w), 0);
        check(widest <= layout.insetWidth + 1, at(`scale ${scale}: lines fit the inset (${widest} <= ${layout.insetWidth})`));
        for (const row of layout.rows) {
          check(
            row.x >= layout.inset.left - 1 && row.x + row.w <= layout.inset.right + 1,
            at(`scale ${scale}: line inside overlay`),
          );
        }
        check(layout.chunkTop - proj.floorY >= 8, at(`scale ${scale}: floor clearance (${layout.chunkTop} vs ${proj.floorY})`));
        check(
          layout.overlay.top + layout.overlay.height + 1 >= (layout.chunkBottom ?? 0),
          at(`scale ${scale}: extended plaque covers the chunk`),
        );
        const overflow = layout.overlay.top + layout.overlay.height - H;
        check(overflow <= 0, at(`scale ${scale}: refit keeps the plaque in frame (overflow ${overflow})`));
        check(layout.overlay.top >= 0, at(`scale ${scale}: plaque top on screen`));
        check(Number.isSafeInteger(growth) && growth >= 0, at(`scale ${scale}: growth steps recorded (${growth})`));
      }
      // Camera/pointer extremes hold the same contract through the envelope.
      const extremeRows = [...(out.extremes1 ?? []), ...(out.extremes2 ?? [])];
      check(extremeRows.length === 8, at(`extreme matrix complete (${extremeRows.length})`));
      for (const row of extremeRows) {
        const proj = row.projected;
        const layout = row.layout;
        check(Math.abs(layout.overlay.left - proj.left) <= 2, at(`extreme ${row.def}: tracks projection`));
        check(Math.abs(layout.overlay.top - proj.top) <= 2, at(`extreme ${row.def}: tracks projection`));
        check(row.active === row.expect, at(`extreme ${row.def}: highlight agreement`));
        check(layout.lineCount >= 2 && layout.lineCount <= 3, at(`extreme ${row.def}: 2-3 lines (${layout.lineCount})`));
        check(layout.chunkTop - proj.floorY >= 8, at(`extreme ${row.def}: floor clearance`));
        check(layout.overlay.top >= 0, at(`extreme ${row.def}: top on screen`));
        check(layout.overlay.top + layout.overlay.height <= H, at(`extreme ${row.def}: in frame`));
      }
      if ((out.blackoutSample ?? -1) >= 0) {
        check(out.blackoutMax <= 8 / 255, at(`dark hold composites to black (${out.blackoutMax} in ${out.blackoutPhase})`));
      } else {
        // No zero-gain sample in this fixture: blackout stays covered by
        // the inventory run, and the skip is recorded, not certified.
        report.blackoutSkipped = true;
      }
      check(out.layout1b.fontSize === "22px", at(`100% text is 22px (${out.layout1b.fontSize})`));
      check(out.layout2b.fontSize === "44px", at(`200% text is 44px (${out.layout2b.fontSize})`));
      check(typeof out.capture.pixels_sha256 === "string" && out.capture.pixels_sha256.length === 64, at("frame capture hash"));
      check(out.capture.nonblackFraction > 0.01, at(`stage renders (${out.capture.nonblackFraction})`));
      check(out.capture.provenance === "diagnostic", at("capture provenance"));
      check(Object.values(out.nodGestures ?? {}).every((g) => g === "nod"), at(`nod-em gestures (${JSON.stringify(out.nodGestures)})`));
      check(out.disposeClean === true, at("dispose releases plaque DOM"));
      const overflowOf = (layout) => +((layout.overlay.top + layout.overlay.height - H).toFixed(1));
      report.viewports.push({
        viewport: tag,
        fontReady: out.fontReady,
        playSeconds: +(out.playedT1 - out.playedT0).toFixed(2),
        playFrames: out.playFrames,
        mismatchCount: out.mismatchCount,
        lines1: out.layout1b.lineCount,
        lines2: out.layout2b.lineCount,
        clearance1: +((out.layout1b.chunkTop ?? 0) - out.projected1.floorY).toFixed(1),
        clearance2: +((out.layout2b.chunkTop ?? 0) - out.projected2.floorY).toFixed(1),
        growthSteps1: out.growthSteps1,
        growthSteps2: out.growthSteps2,
        refitDistance1: +out.refitDistance1.toFixed(2),
        refitDistance2: +out.refitDistance2.toFixed(2),
        bottomOverflowPx1: overflowOf(out.layout1b),
        bottomOverflowPx2: overflowOf(out.layout2b),
        extremes: [...(out.extremes1 ?? []), ...(out.extremes2 ?? [])].map((row) => ({
          def: row.def,
          active: row.active,
          lines: row.layout.lineCount,
          clearance: +((row.layout.chunkTop ?? 0) - row.projected.floorY).toFixed(1),
          bottomOverflowPx: +((row.layout.overlay.top + row.layout.overlay.height - H).toFixed(1)),
        })),
        blackoutSample: out.blackoutSample,
        blackoutMax: out.blackoutMax,
        blackoutPhase: out.blackoutPhase,
        roLoopNotices: out.roLoopNotices ?? 0,
        captureSha: out.capture.pixels_sha256,
        nonblackFraction: +out.capture.nonblackFraction.toFixed(4),
        nodGestures: out.nodGestures,
        disposeClean: out.disposeClean,
      });
    } finally {
      try {
        await call("Page.close");
      } catch { /* ignore */ }
      try {
        socket.close();
      } catch { /* ignore */ }
    }
  }
  report.failures = failures;
  await writeFile(evidenceJson, JSON.stringify(report, null, 2));
  if (failures.length) throw new Error(`check:theater failed:\n- ${failures.join("\n- ")}`);
  console.log(`check:theater pass (${VIEWPORTS.length} viewports: audio/sample/group, placement, lines, clearance)`);
} catch (error) {
  console.error(`check:theater failed: ${error instanceof Error ? error.message : String(error)}`);
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

