#!/usr/bin/env node
// check:lifecycle — prove transport control races and repeated teardown in
// headless Chrome with the real player build. Phase A drives one mounted
// release (readiness-v2 fixture, real <audio> clock, three.js sink,
// read-along plaque) through rapid pause/play/seek/restart bursts at
// opening/blackout/closing samples, a rejected resume, a stale play
// completion and pause-sign retargeting, asserting latest-intent state.
// Phase B runs ten complete load/mount/play/seek/dispose cycles through the
// representative consumer path, asserting disposed instances emit no later
// state and owned resources (RAF handles, listeners, ResizeObservers, media
// object URLs, DOM children, Three.js geometries/textures/targets) return
// to baseline every cycle. Zero provider calls: every request is
// same-origin localhost (request log asserted).
// Evidence: docs/implementation/player/lifecycle.json. Headless
// functional/lifecycle evidence only — not demo-device performance.
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
const evidenceJson = join(root, "docs", "implementation", "player", "lifecycle.json");
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
window.__boot = async () => {
  const out = { races: {}, rejected: {}, retarget: {}, burst: {}, baseline: null, cycles: [] };
  window.__progress = [];
  const prog = (name) => window.__progress.push([name, Math.round(performance.now())]);
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
  const CLOSING = 3227000;
  out.sampleCount = timeline.sample_count;
  async function buildMount(tag) {
    const handle = await api.loadDiagnostic(manifestBytes, manifestDigest, read, policy);
    const sink = await api.createThreeSink(${W}, ${H}, { figures });
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
    const mount = api.mountPlayer(theater, { handle, timeline }, {
      clock: audio, sink, candidate_id: tag,
      readAlong: { words: track.words, groups: track.groups, container: plaqueEl },
      events: {
        onSample: (s) => ev.samples.push(s),
        onReason: (r) => ev.reasons.push(r),
        onSign: (p, q) => ev.signs.push([p, +q.toFixed(3)]),
        onError: (e) => ev.errors.push(String((e && e.message) || e)),
        onState: () => ev.states++,
      },
    });
    await waitMedia(audio, "canplay", 30000);
    return { mount, sink, audio, plaqueEl, positioner, ev };
  }
  function teardown(m) {
    m.mount.dispose();
    m.sink.canvas.remove();
    m.audio.remove();
    m.positioner.remove();
  }
  const signTail = (ev, n) => ev.signs.slice(-n).map((s) => s[0]);
  const lastPose = (ev) => (ev.signs.length ? ev.signs[ev.signs.length - 1][0] : null);
  // ---- Phase A: control races on one mounted release.
  prog("phase-a-start");
  const a = await buildMount("lifecycle-races");
  prog("phase-a-mounted");
  const A = out.races;
  for (const [name, S] of [["opening", OPENING], ["blackout", BLACKOUT], ["closing", CLOSING]]) {
    prog("race-" + name + "-seek");
    a.mount.seek(S);
    prog("race-" + name + "-seek-done");
    const posed = a.mount.tick();
    prog("race-" + name + "-ticked");
    const p = a.mount.play();
    prog("race-" + name + "-play-called");
    a.mount.pause();
    a.mount.pause();
    a.mount.seek(S + 4800);
    a.mount.pause();
    prog("race-" + name + "-paused");
    await p.catch(() => {});
    prog("race-" + name + "-play-settled");
    const end = a.mount.tick();
    prog("race-" + name + "-done");
    A[name] = {
      posePhase: posed.phase, sample: end.sample, phase: end.phase,
      reason: a.mount.reason, pose: lastPose(a.ev),
    };
  }
  a.mount.seek(OPENING);
  prog("a2-seek-done");
  a.mount.tick();
  prog("a2-tick-done");
  a.mount.pause();
  prog("a2-pause-done");
  window.__audioEl = a.audio;
  const playState = () => ({
    ready: a.audio.readyState, seeking: a.audio.seeking, net: a.audio.networkState,
    paused: a.audio.paused, time: +a.audio.currentTime.toFixed(3),
    err: a.audio.error ? a.audio.error.code : 0,
  });
  const racePlay = async (tag) => {
    let timer;
    const timeout = new Promise((_, rej) => {
      timer = setTimeout(() => rej(new Error(tag + " play stalled: " + JSON.stringify(playState()))), 15000);
    });
    try {
      await Promise.race([a.mount.play().finally(() => clearTimeout(timer)), timeout]);
    } catch (e) {
      clearTimeout(timer);
      throw e;
    }
  };
  prog("a2-play-called");
  await racePlay("A2");
  prog("a2-play-settled");
  a.mount.tick();
  A.playWins = { reason: a.mount.reason, tail: signTail(a.ev, 3) };
  await sleep(1200);
  a.mount.tick();
  A.signSettled = { tail: signTail(a.ev, 2) };
  prog("a3-start");
  a.mount.seek(BLACKOUT);
  a.mount.tick();
  await racePlay("A3");
  a.mount.restart();
  prog("a3-done");
  A.restart = { sample: a.mount.sample, reason: a.mount.reason, tail: signTail(a.ev, 1) };
  a.audio.currentTime = 0;
  a.mount.tick();
  a.mount.seek(OPENING);
  a.mount.tick();
  a.mount.pause();
  prog("a4-start");
  const rMark = a.ev.reasons.length;
  const sMark = a.ev.signs.length;
  const sp = a.mount.play();
  a.mount.pause();
  await sp;
  a.mount.tick();
  // Let the UI clock run: pose assertions need at least one RAF step, and
  // the race window above is microtasks only (a step may legitimately not
  // have fired yet). A genuinely broken loop still reads stale here.
  await sleep(100);
  prog("a4-done");
  A.stalePlay = {
    reason: a.mount.reason, pose: lastPose(a.ev),
    laterPlaying: a.ev.reasons.slice(rMark).includes("playing"),
    signAdded: a.ev.signs.length - sMark,
    tail: signTail(a.ev, 5),
  };
  prog("a5-start");
  a.mount.pause();
  await sleep(400);
  const eMark = a.ev.errors.length;
  // Fast-failing broken media (404): a sourceless element pends play()
  // instead of rejecting, so bare-awaiting it would stall the harness.
  a.audio.src = "/fx/missing.wav";
  a.audio.load();
  prog("a5-broken");
  const brokenAttempt = a.mount.play();
  const brokenOutcome = await Promise.race([
    brokenAttempt.then(() => "resolved", (e) => "rejected:" + String((e && e.message) || e)),
    sleep(15000).then(() => "pended"),
  ]);
  brokenAttempt.catch(() => {});
  const rejMsg = brokenOutcome.startsWith("rejected:") ? brokenOutcome.slice("rejected:".length) : "";
  if (!brokenOutcome.startsWith("rejected:")) {
    throw new Error("rejected resume did not reject: " + brokenOutcome);
  }
  out.rejected = {
    message: rejMsg, reason: a.mount.reason, pose: lastPose(a.ev),
    newErrors: a.ev.errors.length - eMark,
  };
  a.audio.src = "/fx/audio/master.wav";
  a.audio.load();
  await waitMedia(a.audio, "canplay", 30000);
  prog("a5-restored");
  a.mount.tick();
  a.mount.seek(OPENING);
  a.mount.tick();
  await a.mount.play();
  a.mount.pause();
  await sleep(400);
  const mid = a.ev.signs[a.ev.signs.length - 1];
  await a.mount.play();
  a.mount.pause();
  const retargetTail = signTail(a.ev, 3);
  prog("a6-repaused");
  await sleep(1600);
  a.mount.tick();
  prog("a6-done");
  out.retarget = {
    midPose: mid[0], midProgress: mid[1], retargetTail,
    finalPose: lastPose(a.ev),
  };
  a.mount.seek(CLOSING);
  a.mount.tick();
  const burst = [a.mount.play()];
  a.mount.pause();
  a.mount.seek(OPENING);
  a.mount.pause();
  a.mount.restart();
  const burstPlay = a.mount.play();
  burst.push(burstPlay);
  a.mount.pause();
  await Promise.all(burst.map((p) => p.catch(() => {})));
  await sleep(100);
  // No frame since restart: the transport still holds the reset destination.
  out.burst = { sample: a.mount.sample, reason: a.mount.reason, pose: lastPose(a.ev) };
  a.mount.tick();
  teardown(a);
  await sleep(400);
  prog("phase-a-done");
  // ---- Phase B: ten load/mount/play/seek/dispose cycles.
  out.baseline = {
    raf: rafLive(), listeners: res.listenerNet, ro: res.roLive,
    dom: theater.childElementCount, urls: [res.urlsCreated, res.urlsRevoked],
    targets: res.targetDisposals,
  };
  for (let i = 0; i < 10; i++) {
    prog("cycle-" + (i + 1) + "-start");
    const c = await buildMount("lifecycle-cycle-" + (i + 1));
    prog("cycle-" + (i + 1) + "-mounted");
    await c.mount.play();
    prog("cycle-" + (i + 1) + "-playing");
    await sleep(400);
    c.mount.seek(OPENING);
    c.mount.tick();
    c.mount.seek(BLACKOUT);
    c.mount.tick();
    c.mount.seek(CLOSING);
    c.mount.tick();
    c.mount.pause();
    const counts = () => c.ev.samples.length + c.ev.reasons.length + c.ev.signs.length + c.ev.states + c.ev.errors.length;
    const worked = counts();
    const workedMem = c.sink.resourceInfo();
    const t0 = res.targetDisposals;
    teardown(c);
    await sleep(300);
    const postMem = c.sink.resourceInfo();
    // Freeze snapshot goes after teardown: sign.cancel()'s single hidden
    // emission is teardown itself, not later state.
    const before = counts();
    // Post-dispose traffic must be silent, and the tick must stay null.
    await c.mount.play();
    c.mount.pause();
    c.mount.seek(100);
    c.mount.restart();
    const tickNull = c.mount.tick() === null;
    let captureThrows = false;
    try {
      await c.mount.captureFrame(${W}, ${H});
    } catch (e) { captureThrows = /after dispose/.test(String((e && e.message) || e)); }
    out.cycles.push({
      index: i + 1, worked, workedMem, postMem, frozen: counts() === before, tickNull, captureThrows,
      resources: {
        raf: rafLive(), listeners: res.listenerNet, ro: res.roLive,
        dom: theater.childElementCount, urls: [res.urlsCreated, res.urlsRevoked],
        targets: res.targetDisposals - t0,
      },
    });
  }
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
const pausedPose = (p) => p === "lowering" || p === "visible";

const temp = await mkdtemp(join(tmpdir(), "pt-player-lifecycle-"));
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
  await call("Page.enable");
  // Keep the page foreground: a frozen background page stops timers and
  // media events while still answering CDP, stalling every harness wait.
  await call("Page.bringToFront");
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 60000) {
      try {
        if (await evaluate(`typeof window.__start === "function"`)) return;
      } catch { /* retry */ }
      await delay(200);
    }
    throw new Error("lifecycle harness did not boot");
  })();
  await evaluate(`window.__start()`, false);
  await (async () => {
    const start = Date.now();
    let lastLogged = "";
    while (Date.now() - start < 240000) {
      const st = await evaluate(`({ done: window.__done === true, error: window.__bootError ?? null, progress: (window.__progress ?? []).map((p) => p.join("@")).join(" "), audio: window.__audioEl ? { ready: window.__audioEl.readyState, seeking: window.__audioEl.seeking, net: window.__audioEl.networkState, paused: window.__audioEl.paused, time: +window.__audioEl.currentTime.toFixed(2) } : null })`);
      if (st.error) throw new Error(`in-page failure: ${st.error}`);
      if (st.done) return;
      if (st.progress !== lastLogged) {
        lastLogged = st.progress;
        console.log(`lifecycle progress: ${st.progress} audio=${JSON.stringify(st.audio)}`);
      }
      await delay(5000);
    }
    const tail = await evaluate(`(window.__progress ?? []).map((p) => p.join("@")).join(" ")`);
    const audio = await evaluate(`window.__audioEl ? { ready: window.__audioEl.readyState, seeking: window.__audioEl.seeking, net: window.__audioEl.networkState, paused: window.__audioEl.paused, time: +window.__audioEl.currentTime.toFixed(2) } : null`);
    throw new Error(`lifecycle run timed out at: ${tail} audio=${JSON.stringify(audio)}`);
  })();
  const out = await evaluate(`window.__result`);

  // Phase A: latest intent wins every burst, at every pose. Reason accepts
  // "seeking": a tick during an in-flight media seek truthfully reports the
  // seek over a standing user pause (sample and sign still follow the pause;
  // the seek readout is the unit-pinned reason model, not a control race).
  const pausedReason = (r) => r === "user" || r === "seeking";
  for (const name of ["opening", "blackout", "closing"]) {
    const r = out.races[name];
    check(r.sample === 36000 + 4800 || r.sample === 1752000 + 4800 || r.sample === 3227000 + 4800, `${name}: burst sample ${r.sample}`);
    check(pausedReason(r.reason), `${name}: burst reason ${r.reason}`);
    check(pausedPose(r.pose), `${name}: burst pose ${r.pose}`);
  }
  check(out.races.opening.posePhase === "opening", `opening pose phase ${out.races.opening.posePhase}`);
  check(out.races.closing.posePhase === "closing", `closing pose phase ${out.races.closing.posePhase}`);
  check(out.races.playWins.reason === "playing", `play-wins reason ${out.races.playWins.reason}`);
  check(out.races.signSettled.tail.at(-1) === "hidden", `sign settles hidden: ${out.races.signSettled.tail.join(",")}`);
  check(out.races.restart.sample === 0, `restart sample ${out.races.restart.sample}`);
  check(out.races.restart.reason === "readiness", `restart reason ${out.races.restart.reason}`);
  check(out.races.restart.tail.at(-1) === "hidden", `restart hides sign: ${out.races.restart.tail.join(",")}`);
  check(pausedReason(out.races.stalePlay.reason), `stale play reason ${out.races.stalePlay.reason}`);
  check(out.races.stalePlay.laterPlaying === false, "stale play flipped reason to playing");
  check(pausedPose(out.races.stalePlay.pose), `stale play pose ${out.races.stalePlay.pose}`);
  check((out.rejected.message ?? "").length > 0, "rejected resume did not throw");
  check(out.rejected.reason === "user", `rejected resume reason ${out.rejected.reason}`);
  check(pausedPose(out.rejected.pose), `rejected resume pose ${out.rejected.pose}`);
  check(out.rejected.newErrors === 1, `rejected resume errors ${out.rejected.newErrors}`);
  check(out.retarget.midPose === "lowering", `retarget mid pose ${out.retarget.midPose}`);
  check(out.retarget.midProgress > 0 && out.retarget.midProgress < 1, `retarget mid progress ${out.retarget.midProgress}`);
  check(!out.retarget.retargetTail.includes("visible"), `retarget settled early: ${out.retarget.retargetTail.join(",")}`);
  check(out.retarget.finalPose === "visible", `retarget final pose ${out.retarget.finalPose}`);
  check(out.burst.sample === 0, `burst sample ${out.burst.sample}`);
  check(pausedReason(out.burst.reason), `burst reason ${out.burst.reason}`);
  check(pausedPose(out.burst.pose), `burst pose ${out.burst.pose}`);
  // Phase B: every cycle works, then returns to baseline with no later state.
  check(out.cycles.length === 10, `cycles ran: ${out.cycles.length}`);
  for (const c of out.cycles) {
    const r = c.resources;
    check(c.worked > 0, `cycle ${c.index} emitted nothing`);
    check(c.frozen === true, `cycle ${c.index} emitted after dispose`);
    check(c.tickNull === true, `cycle ${c.index} ticked after dispose`);
    check(c.captureThrows === true, `cycle ${c.index} captured after dispose`);
    check(r.raf === out.baseline.raf, `cycle ${c.index} RAF live ${r.raf}`);
    check(r.listeners === out.baseline.listeners, `cycle ${c.index} listeners net ${r.listeners}`);
    check(r.ro === out.baseline.ro, `cycle ${c.index} observers live ${r.ro}`);
    check(r.dom === out.baseline.dom, `cycle ${c.index} DOM children ${r.dom}`);
    check(r.urls[0] === 0 && r.urls[1] === 0, `cycle ${c.index} object URLs ${r.urls.join("/")}`);
    check(c.workedMem.geometries + c.workedMem.textures > 0, `cycle ${c.index} created no GL resources`);
    check(c.postMem.geometries === 0 && c.postMem.textures === 0, `cycle ${c.index} GL live ${JSON.stringify(c.postMem)}`);
    check(r.targets === 2, `cycle ${c.index} targets disposed ${r.targets}`);
  }
  check((out.errors ?? []).length === 0, `console errors: ${(out.errors ?? []).join("; ").slice(0, 200)}`);
  check(requests.length > 0 && requests.every((r) => r.startsWith("/")), `non-local request: ${requests.join(",")}`);
  const report = {
    tool: "check:lifecycle",
    fixture: "fixtures/generation/two-scene-readiness-v2",
    viewport: `${W}x${H}`,
    sampleCount: out.sampleCount,
    races: out.races,
    rejected: out.rejected,
    retarget: out.retarget,
    burst: out.burst,
    baseline: out.baseline,
    cycles: out.cycles,
    requests: { count: requests.length, remote: 0 },
    provider_calls: 0,
    caller_cleanup: [
      "Media element state (currentTime/src/playback) stays caller-owned: restart() resets transport state only, matching the staging pattern of resetting currentTime at the call site.",
      "DOM appended by the caller (sink canvas, audio element, plaque positioner) must be removed by the caller; dispose() removes only mount-owned plaque nodes and disconnects its ResizeObserver.",
      "No object URLs are created by the player; WebGL context slots release with canvas removal/GC after renderer.dispose().",
    ],
    failures,
  };
  await writeFile(evidenceJson, JSON.stringify(report, null, 2));
  if (failures.length) throw new Error(`check:lifecycle failed:\n- ${failures.join("\n- ")}`);
  console.log("check:lifecycle pass (races/rejected/stale latest-intent; 10 cycles dispose-clean)");
} catch (error) {
  console.error(`check:lifecycle failed: ${error instanceof Error ? error.message : String(error)}`);
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
