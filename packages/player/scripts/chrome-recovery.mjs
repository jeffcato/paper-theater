#!/usr/bin/env node
// check:recovery — prove WebGL context loss/recovery in headless Chrome with
// the real player build. Uses the mounted readiness-v2 package, figure
// textures, plaque and audio clock. Forces real loss/restoration with the
// WEBGL_lose_context extension (records the branch unverified if the
// extension is unavailable): loss while playing at blackout, loss while
// user-paused at opening with a backward rewind mid-rebuild (stale abort +
// retry at the new sample), loss at a revealed scene with restart during
// the outage, rejected post-recovery play, disposal before restoration and
// a replacement mount — plus two posed screenshots and full teardown
// accounting. Zero provider calls: every request is same-origin localhost.
// Evidence: docs/implementation/player/recovery.json + recovery-*.png.
// Headless functional evidence only — not demo-device performance.
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
const evidenceJson = join(root, "docs", "implementation", "player", "recovery.json");
const shotName = (name) => join(root, "docs", "implementation", "player", `recovery-${name}.png`);
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const W = 960;
const H = 540;

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script>
<style>html, body { margin: 0; padding: 0; background: #000; }</style></head>
<body><div id="theater"></div>
<script type="module">
import * as THREE from "three";
import * as api from "/dist/src/index.js";
window.__errors = [];
addEventListener("error", (e) => window.__errors.push(String(e.message)));
addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason && e.reason.message ? e.reason.message : e.reason)));
// Owned-resource accounting: harness-side wrappers only, no player changes.
const res = {
  listenerNet: 0, roLive: 0, urlsCreated: 0, urlsRevoked: 0, targetDisposals: 0,
};
const rafPending = new Set();
const rafLive = () => rafPending.size;
const rafOrig = window.requestAnimationFrame.bind(window);
const cafOrig = window.cancelAnimationFrame.bind(window);
window.requestAnimationFrame = (cb) => {
  const id = rafOrig((t) => { rafPending.delete(id); cb(t); });
  rafPending.add(id);
  return id;
};
window.cancelAnimationFrame = (id) => { rafPending.delete(id); cafOrig(id); };
const addOrig = EventTarget.prototype.addEventListener;
const remOrig = EventTarget.prototype.removeEventListener;
EventTarget.prototype.addEventListener = function (...a) { res.listenerNet++; return addOrig.apply(this, a); };
EventTarget.prototype.removeEventListener = function (...a) { res.listenerNet--; return remOrig.apply(this, a); };
const ROOrig = window.ResizeObserver;
window.ResizeObserver = function (cb) {
  res.roLive++;
  const inst = new ROOrig(cb);
  const origDisc = inst.disconnect.bind(inst);
  inst.disconnect = () => { res.roLive--; origDisc(); };
  return inst;
};
window.ResizeObserver.prototype = ROOrig.prototype;
const createOrig = URL.createObjectURL.bind(URL);
const revokeOrig = URL.revokeObjectURL.bind(URL);
URL.createObjectURL = (o) => { res.urlsCreated++; return createOrig(o); };
URL.revokeObjectURL = (u) => { res.urlsRevoked++; return revokeOrig(u); };
const targetDisposeOrig = THREE.WebGLRenderTarget.prototype.dispose;
THREE.WebGLRenderTarget.prototype.dispose = function () {
  res.targetDisposals++;
  return targetDisposeOrig.apply(this);
};
async function load(path, maxBytes) {
  const res2 = await fetch(path);
  if (!res2.ok) throw new Error("fetch failed: " + path);
  const buf = new Uint8Array(await res2.arrayBuffer());
  if (buf.length > maxBytes) throw new Error("over bound: " + path);
  return buf;
}
async function sha256(bytes) {
  const d = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(d)].map((n) => n.toString(16).padStart(2, "0")).join("");
}
const text = (v) => new TextDecoder().decode(v);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function waitMedia(el, kind, ms) {
  await new Promise((res3, rej) => {
    const to = setTimeout(() => rej(new Error(kind + " timeout")), ms);
    if (kind === "canplay") el.oncanplay = () => { clearTimeout(to); res3(null); };
    else el.onseeked = () => { clearTimeout(to); res3(null); };
    el.onerror = () => { clearTimeout(to); rej(new Error("media error")); };
  });
}
async function waitFor(cond, ms, label) {
  const start = Date.now();
  while (Date.now() - start < ms) {
    if (cond()) return;
    await sleep(50);
  }
  throw new Error("wait timeout: " + label);
}
window.__boot = async () => {
  const out = { cycles: [], shots: {} };
  const theater = document.querySelector("#theater");
  const manifestBytes = await load("/fx/manifest.json", 1024 * 1024);
  const manifestDigest = await sha256(manifestBytes);
  const read = async (path) => load("/fx/" + path, 64 * 1024 * 1024);
  const policy = { player_sha256: "0".repeat(64), checker_sha256: "0".repeat(64), recipe_sha256: "0".repeat(64), allow_diagnostic: false };
  const timeline = JSON.parse(text(await load("/fx/timeline.json", 1024 * 1024)));
  const actorsDoc = JSON.parse(text(await load("/fx/actors.json", 1024 * 1024)));
  const figures = actorsDoc.map((card) => ({
    asset_id: card.id, url: "/fx/" + card.image.path, pixel_size: card.pixel_size,
    pivot_top_left: card.pivot_top_left, stage_size: card.stage_size, allow_mirror: card.allow_mirror,
  }));
  const track = JSON.parse(text(await load("/fx/read-along.json", 4 * 1024 * 1024)));
  const OPENING = 36000;
  const BLACKOUT = 1752000;
  const SCENE = 500000;
  const REWIND = 4800;
  out.sampleCount = timeline.sample_count;
  const poseForRoi = api.baseCameraPose();
  const vp = { width: ${W}, height: ${H} };
  const [apX, apY] = api.projectToCss([0, 2, 0], poseForRoi, vp);
  const [hoX, hoY] = api.projectToCss([-9.5, 4, 1.8], poseForRoi, vp);
  const roiMax = (pixels, cx, cy, half) => {
    let m = 0;
    const x0 = Math.max(0, Math.floor(cx - half));
    const x1 = Math.min(${W} - 1, Math.ceil(cx + half));
    const y0 = Math.max(0, Math.floor(cy - half));
    const y1 = Math.min(${H} - 1, Math.ceil(cy + half));
    for (let y = y0; y <= y1; y++) {
      for (let x = x0; x <= x1; x++) {
        const i = ((${H} - 1 - y) * ${W} + x) * 4;
        m = Math.max(m, pixels[i] / 255, pixels[i + 1] / 255, pixels[i + 2] / 255);
      }
    }
    return m;
  };
  async function buildMount(tag) {
    const cell = { gate: null };
    const handle = await api.loadDiagnostic(manifestBytes, manifestDigest, read, policy);
    const canvas = document.createElement("canvas");
    const sink = await api.createThreeSink(${W}, ${H}, { figures, canvas });
    sink.canvas.style.width = ${W} + "px";
    sink.canvas.style.height = ${H} + "px";
    theater.appendChild(sink.canvas);
    const audio = document.createElement("audio");
    audio.preload = "auto";
    audio.src = "/fx/audio/master.wav";
    theater.appendChild(audio);
    const positioner = document.createElement("div");
    const plaqueEl = document.createElement("div");
    positioner.appendChild(plaqueEl);
    theater.appendChild(positioner);
    const ev = { samples: [], reasons: [], signs: [], errors: [], states: 0 };
    const sinks = [sink];
    const mount = api.mountPlayer(theater, { handle, timeline }, {
      clock: audio, sink, candidate_id: tag,
      readAlong: { words: track.words, groups: track.groups, container: plaqueEl },
      recreateSink: async () => {
        if (cell.gate) await cell.gate;
        const fresh = await api.createThreeSink(${W}, ${H}, { figures, canvas });
        sinks.push(fresh);
        return fresh;
      },
      events: {
        onSample: (s) => ev.samples.push(s),
        onReason: (r) => ev.reasons.push(r),
        onSign: (p, q) => ev.signs.push([p, +q.toFixed(3)]),
        onError: (e) => ev.errors.push(String((e && e.message) || e)),
        onState: () => ev.states++,
      },
    });
    await waitMedia(audio, "canplay", 30000);
    return { mount, sink, canvas, audio, plaqueEl, positioner, ev, sinks, cell };
  }
  function teardownDom(m) {
    m.sink.canvas.remove();
    m.audio.remove();
    m.positioner.remove();
  }
  // Screenshots frame one canvas: hide the others so the shot is readable.
  function isolateCanvas(keep) {
    const hidden = [];
    for (const canvas of theater.querySelectorAll("canvas")) {
      if (canvas !== keep && canvas.style.display !== "none") {
        canvas.style.display = "none";
        hidden.push(canvas);
      }
    }
    return hidden;
  }
  const snapCanvas = async (name, keep) => {
    const hidden = isolateCanvas(keep);
    await snap(name);
    for (const canvas of hidden) canvas.style.display = "";
  };
  const counts = (m) => m.ev.samples.length + m.ev.reasons.length + m.ev.signs.length + m.ev.states + m.ev.errors.length;
  const lastPose = (m) => (m.ev.signs.length ? m.ev.signs[m.ev.signs.length - 1][0] : null);
  const plaqueText = (m) => {
    const chunk = m.positioner.querySelector(".pl-chunk");
    return chunk ? (chunk.textContent || "").replace(/\\s+/g, " ").slice(0, 160) : "";
  };
  async function captureOf(m) {
    const f = await m.mount.captureFrame(${W}, ${H});
    return {
      sample: f.sample, phase: f.phase, accepted: f.accepted,
      stateDigest: f.state_digest, contributors: f.contributor_ids,
      pixelsSha: await sha256(f.pixels),
      openingMax: roiMax(f.pixels, apX, apY, 20),
      houseMax: roiMax(f.pixels, hoX, hoY, 20),
      plaque: plaqueText(m),
    };
  }
  const snap = async (name) => {
    window.__wantShot = name;
    window.__shotAck = false;
    while (!window.__shotAck) await sleep(100);
    window.__wantShot = null;
  };
  function loseExt(canvas) {
    const gl = canvas.getContext("webgl2");
    if (!gl) return null;
    return gl.getExtension("WEBGL_lose_context");
  }
  // ---- Control mount: never loses context, equality oracle.
  const b = await buildMount("recovery-control");
  // ---- C1: loss while playing at blackout.
  const a = await buildMount("recovery-main");
  const extA = loseExt(a.canvas);
  out.loseContextSupported = !!extA;
  if (!extA) throw new Error("WEBGL_lose_context unavailable: recovery branch unverified");
  a.mount.seek(BLACKOUT);
  a.mount.tick();
  await a.mount.play();
  await sleep(400);
  const preSample = a.mount.sample;
  const preSigns = a.ev.signs.length;
  const prePlaque = plaqueText(a);
  extA.loseContext();
  await waitFor(() => a.audio.paused, 3000, "media did not pause on loss");
  let capErr = "";
  try {
    await a.mount.captureFrame(${W}, ${H});
  } catch (e) { capErr = String((e && e.message) || e); }
  out.c1Loss = {
    status: a.mount.graphicsStatus(), sampleRetained: a.mount.sample === preSample,
    tickNull: a.mount.tick() === null, capErr,
    signFrozen: a.ev.signs.length === preSigns, plaqueFrozen: plaqueText(a) === prePlaque,
  };
  extA.restoreContext();
  await waitFor(() => a.mount.graphicsStatus().state === "ready", 30000, "auto-recovery did not complete");
  const afterBlackout = await captureOf(a);
  // Control posed at the retained sample after the fact: exact equality.
  b.mount.seek(afterBlackout.sample);
  b.mount.tick();
  const controlBlackout = await captureOf(b);
  out.c1Restored = {
    status: a.mount.graphicsStatus(), sample: a.mount.sample,
    stillPaused: a.audio.paused, reason: a.mount.reason,
    capture: afterBlackout, control: controlBlackout,
  };
  // Re-pose to the exact blackout sample for a comparable shot.
  a.mount.seek(BLACKOUT);
  a.mount.tick();
  await snapCanvas("restored-blackout", a.canvas);
  await a.mount.play();
  out.c1Play = { reason: a.mount.reason };
  // ---- C2: loss while user-paused at opening + rewind mid-rebuild.
  a.mount.pause();
  await sleep(500);
  a.mount.seek(OPENING);
  a.mount.tick();
  const c2SignsBefore = a.ev.signs.length;
  extA.loseContext();
  await waitFor(() => a.mount.graphicsStatus().state === "lost", 3000, "status did not flip");
  out.c2Loss = {
    status: a.mount.graphicsStatus(),
    signTail: a.ev.signs.slice(c2SignsBefore).map((s) => s[0]),
  };
  a.cell.gate = sleep(800);
  extA.restoreContext();
  await sleep(150);
  a.mount.seek(REWIND);
  let staleErr = "";
  try {
    await a.mount.retryGraphics();
  } catch (e) { staleErr = String((e && e.message) || e); }
  out.c2Stale = { staleErr, status: a.mount.graphicsStatus() };
  a.cell.gate = null;
  await a.mount.retryGraphics();
  b.mount.seek(REWIND);
  b.mount.tick();
  out.c2Retried = {
    status: a.mount.graphicsStatus(), sample: a.mount.sample,
    eqA: await captureOf(a), eqB: await captureOf(b),
  };
  // ---- C3: loss at revealed scene + restart + rejected play + replace.
  a.mount.seek(SCENE);
  a.mount.tick();
  out.c3SceneContributors = (await captureOf(a)).contributors;
  await a.mount.play();
  await sleep(300);
  extA.loseContext();
  await waitFor(() => a.audio.paused, 3000, "media did not pause on loss");
  a.mount.restart();
  out.c3Restart = {
    sample: a.mount.sample, status: a.mount.graphicsStatus(),
    tickNull: a.mount.tick() === null,
  };
  extA.restoreContext();
  await waitFor(() => a.mount.graphicsStatus().state === "ready", 30000, "recovery after restart did not complete");
  await a.mount.play();
  a.audio.src = "/fx/missing.wav";
  a.audio.load();
  const missAttempt = a.mount.play();
  const missOutcome = await Promise.race([
    missAttempt.then(() => "resolved", (e) => "rejected:" + String((e && e.message) || e)),
    sleep(15000).then(() => "pended"),
  ]);
  missAttempt.catch(() => {});
  a.audio.src = "/fx/audio/master.wav";
  a.audio.load();
  await waitMedia(a.audio, "canplay", 30000);
  await a.mount.play();
  out.c3Play = { missOutcome, reason: a.mount.reason };
  // Re-pose for a deterministic scene shot: pause freezes the sample.
  a.mount.pause();
  a.mount.seek(SCENE);
  a.mount.tick();
  await snapCanvas("restored-scene", a.canvas);
  extA.loseContext();
  await waitFor(() => a.mount.graphicsStatus().state === "lost", 3000, "status did not flip");
  a.mount.dispose();
  teardownDom(a);
  await sleep(300);
  // Freeze snapshot goes after teardown emissions (sign cancel hides once).
  const aBefore = counts(a);
  await a.mount.play().catch(() => {});
  a.mount.pause();
  a.mount.seek(100);
  a.mount.restart();
  out.c3Dispose = { frozen: counts(a) === aBefore, tickNull: a.mount.tick() === null };
  const r = await buildMount("recovery-replacement");
  r.mount.seek(SCENE);
  r.mount.tick();
  await r.mount.play();
  out.replacement = {
    tick: r.mount.tick() !== null, reason: r.mount.reason,
    captureAccepted: (await r.mount.captureFrame(${W}, ${H})).accepted,
  };
  r.mount.dispose();
  teardownDom(r);
  b.mount.dispose();
  teardownDom(b);
  await sleep(300);
  const allSinks = [...a.sinks, ...b.sinks, ...r.sinks];
  out.teardown = {
    raf: rafLive(), listeners: res.listenerNet, ro: res.roLive,
    dom: theater.childElementCount, urls: [res.urlsCreated, res.urlsRevoked],
    targets: res.targetDisposals, sinks: allSinks.length,
    mems: allSinks.map((s) => s.resourceInfo()),
  };
  out.errors = window.__errors;
  return out;
};
window.__start = () => {
  window.__done = false;
  window.__boot().then(
    (result) => {
      window.__result = result;
      window.__done = true;
    },
    (error) => {
      window.__bootError = String((error && error.message) || error);
      window.__done = true;
    },
  );
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

// Media clocks seek: honor Range requests (206) or the element stalls
// mid-harness with currentTime frozen at 0.
async function serveRanged(req, res, path, type) {
  try {
    const data = await readFile(path);
    const total = data.length;
    const match = req.headers.range && /^bytes=(\d*)-(\d*)$/.exec(req.headers.range);
    if (!match) {
      res.writeHead(200, { "content-type": type, "content-length": total, "accept-ranges": "bytes" });
      res.end(data);
      return;
    }
    let start = match[1] === "" ? total - Number(match[2]) : Number(match[1]);
    let end = match[2] === "" ? total - 1 : Number(match[2]);
    if (!(start >= 0 && end >= start && start < total)) {
      res.writeHead(416, { "content-range": `bytes */${total}` });
      res.end();
      return;
    }
    end = Math.min(end, total - 1);
    const slice = data.subarray(start, end + 1);
    res.writeHead(206, {
      "content-type": type,
      "content-length": slice.length,
      "content-range": `bytes ${start}-${end}/${total}`,
      "accept-ranges": "bytes",
    });
    res.end(slice);
  } catch {
    res.writeHead(404);
    res.end("missing");
  }
}

const failures = [];
const check = (cond, message) => {
  if (!cond) failures.push(message);
};
const eqCap = (a, b, label) => {
  check(a.sample === b.sample, `${label}: sample ${a.sample} vs ${b.sample}`);
  check(a.phase === b.phase, `${label}: phase ${a.phase} vs ${b.phase}`);
  check(a.stateDigest === b.stateDigest, `${label}: state digest differs`);
  check(JSON.stringify(a.contributors) === JSON.stringify(b.contributors), `${label}: contributors differ`);
  check(a.pixelsSha === b.pixelsSha, `${label}: pixels differ`);
  check(a.plaque === b.plaque, `${label}: plaque differs`);
  check(a.accepted === false && b.accepted === false, `${label}: acceptance leaked`);
};

const temp = await mkdtemp(join(tmpdir(), "pt-player-recovery-"));
const children = [];
let ws;
let server;
try {
  const requests = [];
  server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://127.0.0.1");
    requests.push(url.pathname);
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
    if (url.pathname.startsWith("/fx/")) {
      const name = url.pathname.slice("/fx/".length);
      if (name.includes("..")) {
        res.writeHead(404);
        res.end("missing");
        return;
      }
      const type = name.endsWith(".wav") ? "audio/wav" : "application/octet-stream";
      return serveRanged(req, res, join(fixtureRoot, name), type);
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
  const page = await (await fetch(`http://127.0.0.1:${debugPort}/json/new?${encodeURIComponent(`http://127.0.0.1:${port}/`)}`, { method: "PUT" })).json();
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
  const evaluate = async (expression, awaitPromise = true) => {
    const result = await call("Runtime.evaluate", { expression, awaitPromise, returnByValue: true });
    if (result.exceptionDetails) {
      throw new Error(`page exception: ${JSON.stringify(result.exceptionDetails).slice(0, 500)}`);
    }
    return result.result.value;
  };
  const shaFile = async (path) => (await import("node:crypto")).createHash("sha256").update(await readFile(path)).digest("hex");
  await call("Page.enable");
  // Keep the page foreground: a frozen background page stops timers and
  // media events while still answering CDP, stalling every harness wait.
  await call("Page.bringToFront");
  await call("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 60000) {
      try {
        if (await evaluate(`typeof window.__start === "function"`)) return;
      } catch { /* retry */ }
      await delay(200);
    }
    throw new Error("recovery harness did not boot");
  })();
  await evaluate(`window.__start()`, false);
  const shots = {};
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 300000) {
      const st = await evaluate(`({ done: window.__done === true, want: window.__wantShot ?? null, error: window.__bootError ?? null })`);
      if (st.error) throw new Error(`in-page failure: ${st.error}`);
      if (st.done) return;
      if (st.want) {
        const shot = await call("Page.captureScreenshot", { format: "png" });
        await writeFile(shotName(st.want), Buffer.from(shot.data, "base64"));
        shots[st.want] = await shaFile(shotName(st.want));
        await evaluate(`window.__shotAck = true`);
      }
      await delay(500);
    }
    throw new Error("recovery run timed out");
  })();
  const out = await evaluate(`window.__result`);

  check(out.loseContextSupported === true, "WEBGL_lose_context unavailable: real loss unproven");
  // C1: loss while playing at blackout.
  check(out.c1Loss.status.state === "lost", `C1 status ${JSON.stringify(out.c1Loss.status)}`);
  check(out.c1Loss.status.suspendedWhilePlaying === true, "C1 did not retain play intent");
  check((out.c1Loss.status.error ?? "").length > 0, "C1 retry state not actionable");
  check(out.c1Loss.sampleRetained === true, "C1 sample not retained");
  check(out.c1Loss.tickNull === true, "C1 tick delivered while lost");
  check(/graphics_lost/.test(out.c1Loss.capErr ?? ""), `C1 capture not blocked: ${out.c1Loss.capErr}`);
  check(out.c1Loss.signFrozen === true, "C1 sign moved on loss");
  check(out.c1Restored.status.state === "ready", "C1 not ready after restore");
  check(out.c1Restored.status.error === null, "C1 error not cleared");
  check(out.c1Restored.stillPaused === true, "C1 autoplayed after recovery");
  check(out.c1Restored.reason !== "playing", `C1 reason ${out.c1Restored.reason}`);
  eqCap(out.c1Restored.capture, out.c1Restored.control, "C1 blackout restore");
  check(out.c1Restored.capture.openingMax < 0.004, `C1 aperture not dark (${out.c1Restored.capture.openingMax})`);
  check(out.c1Restored.capture.houseMax >= 0.008, `C1 house not lit (${out.c1Restored.capture.houseMax})`);
  check(out.c1Play.reason === "playing", `C1 explicit play reason ${out.c1Play.reason}`);
  // C2: loss while paused + rewind mid-rebuild.
  check(out.c2Loss.status.state === "lost", "C2 status not lost");
  check(out.c2Loss.status.suspendedWhilePlaying === false, "C2 misreported play intent");
  check(!(out.c2Loss.signTail ?? []).includes("hidden"), `C2 sign hidden on loss: ${(out.c2Loss.signTail ?? []).join(",")}`);
  check(/stale/.test(out.c2Stale.staleErr ?? ""), `C2 recovery not stale: ${out.c2Stale.staleErr}`);
  check(out.c2Stale.status.state === "lost", "C2 left ready after stale abort");
  check(out.c2Retried.status.state === "ready", "C2 retry did not recover");
  check(out.c2Retried.sample === 4800, `C2 retry sample ${out.c2Retried.sample}`);
  eqCap(out.c2Retried.eqA, out.c2Retried.eqB, "C2 rewind restore");
  // C3: restart during outage, rejected play, replace.
  check((out.c3SceneContributors ?? []).length > 0, "C3 scene has no contributors");
  check(out.c3Restart.sample === 0, `C3 restart sample ${out.c3Restart.sample}`);
  check(out.c3Restart.status.state === "lost", "C3 left ready by restart");
  check(out.c3Restart.tickNull === true, "C3 tick delivered while lost");
  check((out.c3Play.missOutcome ?? "").startsWith("rejected:"), `C3 play not rejected: ${out.c3Play.missOutcome}`);
  check(out.c3Play.reason === "playing", `C3 retryable play reason ${out.c3Play.reason}`);
  check(out.c3Dispose.frozen === true, "C3 disposed mount emitted");
  check(out.c3Dispose.tickNull === true, "C3 disposed mount ticked");
  check(out.replacement.tick === true, "replacement mount dead");
  check(out.replacement.reason === "playing", `replacement reason ${out.replacement.reason}`);
  check(out.replacement.captureAccepted === false, "replacement granted acceptance");
  // Final teardown accounting.
  check(out.teardown.sinks === 7, `sinks created: ${out.teardown.sinks}`);
  check((out.teardown.mems ?? []).every((m) => m.geometries === 0 && m.textures === 0), `GL live: ${JSON.stringify(out.teardown.mems)}`);
  check(out.teardown.targets === 14, `targets disposed: ${out.teardown.targets}`);
  check(out.teardown.raf === 0, `RAF live ${out.teardown.raf}`);
  check(out.teardown.listeners === 0, `listeners net ${out.teardown.listeners}`);
  check(out.teardown.ro === 0, `observers live ${out.teardown.ro}`);
  check(out.teardown.dom === 0, `DOM children ${out.teardown.dom}`);
  check((out.teardown.urls ?? []).join("/") === "0/0", `object URLs ${out.teardown.urls}`);
  check(Object.keys(shots).length === 2, `screenshots: ${Object.keys(shots).join(",")}`);
  check((out.errors ?? []).length === 0, `console errors: ${(out.errors ?? []).join("; ").slice(0, 200)}`);
  check(requests.length > 0 && requests.every((r) => r.startsWith("/")), `non-local request: ${requests.join(",")}`);
  const report = {
    tool: "check:recovery",
    fixture: "fixtures/generation/two-scene-readiness-v2",
    viewport: `${W}x${H}`,
    sampleCount: out.sampleCount,
    loseContextSupported: out.loseContextSupported,
    c1Loss: out.c1Loss,
    c1Restored: out.c1Restored,
    c1Play: out.c1Play,
    c2Loss: out.c2Loss,
    c2Stale: out.c2Stale,
    c2Retried: out.c2Retried,
    c3SceneContributors: out.c3SceneContributors,
    c3Restart: out.c3Restart,
    c3Play: out.c3Play,
    c3Dispose: out.c3Dispose,
    replacement: out.replacement,
    teardown: out.teardown,
    shotSha256: shots,
    requests: { count: requests.length, remote: 0 },
    provider_calls: 0,
    failures,
  };
  await writeFile(evidenceJson, JSON.stringify(report, null, 2));
  if (failures.length) throw new Error(`check:recovery failed:\n- ${failures.join("\n- ")}`);
  console.log("check:recovery pass (loss/restore/rewind/stale/dispose proven in real Chrome)");
} catch (error) {
  console.error(`check:recovery failed: ${error instanceof Error ? error.message : String(error)}`);
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
