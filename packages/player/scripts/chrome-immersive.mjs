#!/usr/bin/env node
// check:immersive — full-viewport multiplane theater world (audience
// immersion) in headless Chrome. At 960x540, 1280x720 and 1920x1080 (DPR 1)
// it drives real mouse input through the box office (hover, ticket
// inspection, Take Your Seat), captures the aisle dolly, mounts the
// readiness-v2 diagnostic performance at the seat through the explicit
// diagnostic loader, and operates the lithographed playback cards hung on
// the proscenium banner by raycast clicks (play, gauge seek, pause/
// INTERMISSION). It asserts pick/projection agreement at pointer extremes,
// the pinned proscenium plane, reduced motion, the lectern plaque (2-3
// lines, inside the inset, clear of floor, banner clear of the stage
// opening), frame intervals and the accounted graphics memory, then ten
// create/dispose cycles with no growing listener/frame/GPU counts.
// Evidence: immersive.json + immersive-{phase}-{W}x{H}.png.
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
const outDir = join(root, "docs", "implementation", "player");
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const VIEWPORTS = [[960, 540], [1280, 720], [1920, 1080]];

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<link rel="stylesheet" href="/fonts/noto-serif.css">
<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script>
<style>
html, body { margin: 0; height: 100%; overflow: hidden; background: #000; }
#world { position: fixed; inset: 0; overflow: hidden; }
#plaque { color: #f1e6c8; padding: 6px 14px; overflow: hidden; }
.pl-group { border-radius: 3px; }
.pl-group.pl-active { background: #c9a24a; color: #1d1712; }
</style></head>
<body><div id="world"><div id="plaque"></div></div>
<audio id="audio" muted preload="auto" src="/fx/audio/master.wav"></audio>
<script type="module">
import * as api from "/dist/src/index.js";
window.__errors = [];
addEventListener("error", (e) => {
  if (!String(e.message).includes("ResizeObserver loop")) window.__errors.push(String(e.message));
});
addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason?.message ?? e.reason)));
// Listener census for the teardown proof.
let listeners = 0;
const add = EventTarget.prototype.addEventListener;
const remove = EventTarget.prototype.removeEventListener;
EventTarget.prototype.addEventListener = function (...a) { listeners++; return add.apply(this, a); };
EventTarget.prototype.removeEventListener = function (...a) { listeners--; return remove.apply(this, a); };
const frames = (n) => new Promise((r) => { const f = () => (--n <= 0 ? r() : requestAnimationFrame(f)); requestAnimationFrame(f); });
const text = (v) => new TextDecoder().decode(v);
async function load(path, max) {
  const res = await fetch(path);
  if (!res.ok) throw new Error("fetch failed: " + path);
  const buf = new Uint8Array(await res.arrayBuffer());
  if (buf.length > max) throw new Error("over bound: " + path);
  return buf;
}
async function sha256(bytes) {
  const d = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(d)].map((n) => n.toString(16).padStart(2, "0")).join("");
}
const container = document.querySelector("#world");
const plaqueEl = document.querySelector("#plaque");
const audio = document.querySelector("#audio");
const S = { picks: [], hovers: [], sample: 0, state: null, readAlong: true };
let world, mount, fx, growth = 0;
const vp = () => ({ width: innerWidth, height: innerHeight });
const center = (r) => api.projectWorld(world.sink.camera(), [(r.x[0] + r.x[1]) / 2, (r.y[0] + r.y[1]) / 2, r.z], vp());
const controls = () => {
  if (!mount) return;
  world.sink.setControls({ enabled: true, playing: !audio.paused, readAlong: S.readAlong, progress: S.sample / fx.timeline.sample_count, fullscreen: false });
};
function onPick(p) {
  S.picks.push(p.id);
  if (p.id.startsWith("ticket:")) {
    world.sink.setInspect({ id: p.id, title: "Diagnostic Ticket B", subtitle: "Neutral fixture performance",
      synopsis: "A neutral two-scene diagnostic performance used for player checks.",
      excerpt: "Neutral fixture text stands in for the opening lines of an accepted chapter.",
      readiness: "Diagnostic fixture — never shown as a published release", seatLabel: "Take Your Seat" });
  } else if (p.id === "inspect:back") {
    world.sink.setInspect(null);
  } else if (p.id === "inspect:seat") {
    world.sink.setInspect(null);
    world.sink.setTickets(TICKETS.map((t) => ({ ...t, punched: t.id === "b" })), null);
    world.travel(1);
  } else if (p.id === "rail:play" && mount) {
    if (audio.paused) void mount.play().then(controls); else { mount.pause(true); controls(); }
  } else if (p.id === "rail:seek" && mount) {
    S.seekU = p.u;
    mount.seek(Math.round(p.u * fx.timeline.sample_count));
    mount.tick();
  } else if (p.id === "rail:readalong" && mount) {
    S.readAlong = !S.readAlong;
    mount.setReadAlong(S.readAlong);
    controls();
  }
}
const TICKETS = [
  { id: "a", title: "Diagnostic Ticket A", subtitle: "Neutral fixture", ready: false },
  { id: "b", title: "Diagnostic Ticket B", subtitle: "Neutral fixture", ready: true },
];
window.__lobby = async () => {
  await document.fonts.load('22px "Noto Serif"');
  const manifestBytes = await load("/fx/manifest.json", 1 << 20);
  const read = async (path) => load("/fx/" + path, 64 << 20);
  const policy = { player_sha256: "0".repeat(64), checker_sha256: "0".repeat(64), recipe_sha256: "0".repeat(64), allow_diagnostic: false };
  const handle = await api.loadDiagnostic(manifestBytes, await sha256(manifestBytes), read, policy);
  const timeline = JSON.parse(text(await load("/fx/timeline.json", 1 << 20)));
  const actors = JSON.parse(text(await load("/fx/actors.json", 1 << 20)));
  const track = JSON.parse(text(await load("/fx/read-along.json", 4 << 20)));
  const figures = actors.map((c) => ({ asset_id: c.id, url: "/fx/" + c.image.path, pixel_size: c.pixel_size,
    pivot_top_left: c.pivot_top_left, stage_size: c.stage_size, allow_mirror: c.allow_mirror }));
  fx = { handle, timeline, track, accepted: handle.accepted ?? false };
  world = await api.createTheaterWorld({ container, plaque: plaqueEl, journey: 0, travelMs: 2400, figures,
    title: { heading: "Diagnostic Fixture", sub: "Player check" },
    onPick, onHover: (id) => S.hovers.push(id), onPlaqueLayout: () => mount?.plaque?.reflow() });
  world.sink.setTickets(TICKETS, null);
  await frames(4);
  return { tickets: api.TICKET_SLOTS.map(center), seat: center(api.INSPECT_SEAT), back: center(api.INSPECT_BACK),
    canvas: [world.sink.canvas.width, world.sink.canvas.height], dpr: devicePixelRatio };
};
window.__state = () => ({ picks: S.picks, hovers: S.hovers, journey: world.journey(), sample: S.sample,
  curtain: S.state?.curtain_open_ratio ?? null, phase: S.state?.phase ?? null, paused: audio.paused,
  time: audio.currentTime, errors: window.__errors });
// The banner cards are world cards: project them with the live camera.
const railCenters = () => Object.fromEntries(api.bannerScreen(world.sink.camera(), vp()).map((sign) => [
  sign.id,
  [sign.rect.left + sign.rect.width / 2, sign.rect.top + sign.rect.height / 2],
]));
window.__seat = async () => {
  mount = api.mountPlayer(container, { handle: fx.handle, timeline: fx.timeline }, {
    clock: audio, sink: world.sink, candidate_id: "immersive-run-1",
    readAlong: { words: fx.track.words, groups: fx.track.groups, container: plaqueEl },
    events: { onState: (st) => { S.state = st; S.sample = st.sample; world.sink.setControls({ enabled: true, playing: !audio.paused, readAlong: S.readAlong, progress: st.sample / fx.timeline.sample_count, fullscreen: false }); } },
  });
  world.attach(mount);
  await mount.plaque.ready;
  const fontReady = await mount.plaque.fontReady;
  mount.setReadAlong(true);
  // Mid-performance group: curtains open, story underway.
  const first = fx.track.groups[Math.floor(fx.track.groups.length / 2)];
  mount.seek(first.start_sample);
  mount.tick();
  growth = world.fitPlaque();
  controls();
  await frames(3);
  return { fontReady, growth, rail: railCenters(), diagnostic: mount.diagnostic, accepted: mount.accepted };
};
function lineRows() {
  const rows = [];
  for (const span of document.querySelectorAll("#plaque .pl-group")) {
    const range = document.createRange();
    range.selectNodeContents(span);
    for (const r of range.getClientRects()) {
      const row = rows.find((x) => Math.abs(x.y + x.h / 2 - (r.top + r.height / 2)) < 3);
      if (row) { row.x0 = Math.min(row.x0, r.left); row.x1 = Math.max(row.x1, r.right); }
      else rows.push({ y: r.top, h: r.height, x0: r.left, x1: r.right });
    }
  }
  return rows;
}
window.__plaque = () => {
  const box = plaqueEl.getBoundingClientRect();
  const rows = lineRows();
  const cam = world.sink.camera();
  const floor = Math.max(...api.STAGE_OPENING.x.map((x) => api.projectWorld(cam, [x, 0, api.PROSCENIUM_Z], vp())[1]));
  const rail = world.sink.railRect();
  const openingTop = api.projectWorld(cam, [0, api.STAGE_OPENING.y[1], api.PROSCENIUM_Z], vp())[1];
  return { box: { left: box.left, top: box.top, right: box.right, bottom: box.bottom }, lines: rows.length,
    rowsInside: rows.every((r) => r.y >= box.top - 0.5 && r.y + r.h <= box.bottom + 0.5 && r.x0 >= box.left - 0.5 && r.x1 <= box.right + 0.5),
    floorGap: box.top - floor, bannerGap: openingTop - (rail.top + rail.height), bannerInView: rail.left >= 0 && rail.left + rail.width <= innerWidth, text: plaqueEl.textContent.slice(0, 160),
    fontSize: getComputedStyle(plaqueEl.querySelector(".pl-chunk") ?? plaqueEl).fontSize };
};
window.__parallax = async () => {
  const out = {};
  world.setPointer(0, 0);
  await frames(2);
  const cam0 = world.sink.camera();
  world.setPointer(1, 1);
  await frames(2);
  const cam1 = world.sink.camera();
  const shift = (z) => api.projectWorld(cam1, [3, 2, z], vp())[0] - api.projectWorld(cam0, [3, 2, z], vp())[0];
  out.prosceniumShift = shift(api.PROSCENIUM_Z);
  out.layerShifts = api.MULTIPLANE_LAYERS.map((l) => [l.id, Number(shift(api.PROSCENIUM_Z + l.z).toFixed(3))]);
  // Raycast picks at the pure projection of every rail control (pointer extreme).
  const rail = railCenters();
  out.extremePicks = Object.entries(rail).map(([id, [x, y]]) => [id, world.sink.pick(x, y)?.id ?? null]);
  world.setReducedMotion(true);
  await frames(2);
  out.reducedShift = world.sink.camera().shift;
  out.reducedPosition = world.sink.camera().position;
  out.basePosition = cam0.position;
  world.setReducedMotion(false);
  world.setPointer(0, 0);
  await frames(2);
  return out;
};
window.__perf = async (ms) => {
  world.takeFrameIntervals();
  await new Promise((r) => setTimeout(r, ms));
  const v = world.takeFrameIntervals().sort((a, b) => a - b);
  const q = (p) => v[Math.min(v.length - 1, Math.floor(p * v.length))];
  return { frames: v.length, p50: q(0.5), p95: q(0.95), max: v[v.length - 1], memory: world.sink.memory(), gl: world.sink.resourceInfo() };
};
window.__teardown = async () => {
  mount?.dispose();
  world.dispose();
  mount = null;
  await frames(2);
  const base = listeners;
  const cycles = [];
  for (let i = 0; i < 10; i++) {
    const w = await api.createTheaterWorld({ container, plaque: plaqueEl, journey: i % 2 });
    w.sink.setTickets(TICKETS, null);
    await frames(3);
    const live = w.sink.resourceInfo();
    w.dispose();
    const after = w.sink.resourceInfo();
    const before = w.takeFrameIntervals().length;
    await frames(3);
    cycles.push({ live, after, framesAfterDispose: w.takeFrameIntervals().length, listenerDelta: listeners - base, canvases: container.querySelectorAll("canvas").length, before });
  }
  return cycles;
};
window.__seekU = () => S.seekU ?? null;
window.__resized = () => ({ canvas: [world.sink.canvas.width, world.sink.canvas.height],
  picks: Object.entries(railCenters()).map(([id, [x, y]]) => [id, world.sink.pick(x, y)?.id ?? null]),
  lectern: world.sink.lecternRect(), rail: world.sink.railRect() });
window.__count = () => fx.timeline.sample_count;
window.__ready = true;
</script></body></html>`;

// Media clocks seek only when Range requests are honored (206).
async function serveFile(res, path, type, range) {
  try {
    const data = await readFile(path);
    const match = range && /^bytes=(\d*)-(\d*)$/.exec(range);
    if (!match) {
      res.writeHead(200, { "content-type": type, "content-length": data.length, "accept-ranges": "bytes" });
      return res.end(data);
    }
    const total = data.length;
    const start = match[1] === "" ? total - Number(match[2]) : Number(match[1]);
    const end = Math.min(match[2] === "" || match[1] === "" ? total - 1 : Number(match[2]), total - 1);
    if (!(start >= 0 && end >= start)) {
      res.writeHead(416, { "content-range": `bytes */${total}` });
      return res.end();
    }
    res.writeHead(206, { "content-type": type, "content-length": end - start + 1, "content-range": `bytes ${start}-${end}/${total}`, "accept-ranges": "bytes" });
    res.end(data.subarray(start, end + 1));
  } catch {
    res.writeHead(404);
    res.end("missing");
  }
}

const failures = [];
const check = (cond, message) => {
  if (!cond) failures.push(message);
};
const temp = await mkdtemp(join(tmpdir(), "pt-player-immersive-"));
let chrome;
let server;
let ws;
const evidence = { harness: "chrome-immersive", viewports: [] };
try {
  server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://127.0.0.1");
    const name = url.pathname;
    if (name === "/") {
      const data = Buffer.from(PAGE);
      res.writeHead(200, { "content-type": "text/html", "content-length": data.length });
      return res.end(data);
    }
    if (name.includes("..")) {
      res.writeHead(404);
      return res.end("missing");
    }
    if (name.startsWith("/vendor/")) return serveFile(res, join(pkgDir, "node_modules", "three", "build", name.slice(8)), "text/javascript");
    if (name.startsWith("/dist/")) return serveFile(res, join(pkgDir, name), "text/javascript");
    if (name.startsWith("/fonts/")) return serveFile(res, join(pkgDir, name), name.endsWith(".css") ? "text/css" : "font/woff2");
    if (name.startsWith("/fx/")) return serveFile(res, join(fixtureRoot, name.slice(4)), name.endsWith(".wav") ? "audio/wav" : "application/octet-stream", req.headers.range);
    res.writeHead(404);
    res.end("missing");
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const port = server.address().port;
  chrome = spawn(CHROME, [
    "--headless=new", "--no-first-run", "--no-default-browser-check", "--disable-background-networking",
    "--remote-debugging-port=0", "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
    "--autoplay-policy=no-user-gesture-required", `--user-data-dir=${join(temp, "chrome")}`, "about:blank",
  ], { stdio: "ignore" });
  const debugPort = await (async () => {
    const start = Date.now();
    while (Date.now() - start < 30000) {
      try {
        return String(await readFile(join(temp, "chrome", "DevToolsActivePort"))).split("\n")[0];
      } catch { await delay(100); }
    }
    throw new Error("Chrome did not start");
  })();
  const version = await (await fetch(`http://127.0.0.1:${debugPort}/json/version`)).json();
  evidence.browser = version.Browser;
  for (const [W, H] of VIEWPORTS) {
    const page = await (await fetch(`http://127.0.0.1:${debugPort}/json/new?${encodeURIComponent(`http://127.0.0.1:${port}/`)}`, { method: "PUT" })).json();
    try { ws?.close(); } catch { /* ignore */ }
    ws = new WebSocket(page.webSocketDebuggerUrl);
    await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
    let seq = 0;
    const pending = new Map();
    ws.onmessage = ({ data }) => {
      const m = JSON.parse(data);
      if (m.id && pending.has(m.id)) {
        const p = pending.get(m.id);
        pending.delete(m.id);
        if (m.error) p.reject(new Error(m.error.message)); else p.resolve(m.result);
      }
    };
    const call = (method, params = {}) => new Promise((r, j) => {
      const id = ++seq;
      pending.set(id, { resolve: r, reject: j });
      ws.send(JSON.stringify({ id, method, params }));
    });
    const evaluate = async (expression) => {
      const result = await call("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
      if (result.exceptionDetails) throw new Error(`page exception: ${JSON.stringify(result.exceptionDetails).slice(0, 600)}`);
      return result.result.value;
    };
    const shot = async (phase) => {
      const s = await call("Page.captureScreenshot", { format: "png" });
      await writeFile(join(outDir, `immersive-${phase}-${W}x${H}.png`), Buffer.from(s.data, "base64"));
    };
    const mouse = async (type, [x, y]) => call("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" ? "none" : "left", clickCount: type === "mouseMoved" ? 0 : 1 });
    const click = async (pt) => {
      await mouse("mouseMoved", pt);
      await mouse("mousePressed", pt);
      await mouse("mouseReleased", pt);
    };
    // A ticket zoom lingers after the click and eases out slowly on a busy machine, so the
    // button is not at its rest position yet: press only once the pointer sits on a pickable.
    const clickWhenPointer = async (pt) => {
      const start = Date.now();
      for (;;) {
        await mouse("mouseMoved", pt);
        await delay(120);
        if (await evaluate(`getComputedStyle(document.querySelector("#world")).cursor === "pointer"`)) break;
        if (Date.now() - start > 20000) throw new Error(`timeout: nothing pickable under ${pt}`);
      }
      await mouse("mousePressed", pt);
      await mouse("mouseReleased", pt);
    };
    const until = async (expr, ms, label) => {
      const start = Date.now();
      while (Date.now() - start < ms) {
        if (await evaluate(expr)) return;
        await delay(50);
      }
      throw new Error(`timeout: ${label}`);
    };
    await call("Page.enable");
    await call("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
    await until("window.__ready === true", 60000, "boot");
    const v = { width: W, height: H };
    const lobby = await evaluate("window.__lobby()");
    v.canvas = lobby.canvas;
    check(lobby.canvas[0] === W && lobby.canvas[1] === H, `${W}x${H}: canvas ${lobby.canvas} is not full-viewport`);
    await shot("lobby");
    await mouse("mouseMoved", lobby.tickets[1]);
    await delay(150);
    let st = await evaluate("window.__state()");
    check(st.hovers.at(-1) === "ticket:b", `${W}x${H}: hover ${st.hovers.at(-1)}`);
    await shot("hover");
    await click(lobby.tickets[1]);
    await delay(150);
    await shot("inspect");
    await clickWhenPointer(lobby.seat);
    await until("window.__state().journey >= 0.5", 10000, "aisle");
    await shot("aisle");
    await until("window.__state().journey === 1", 10000, "arrival");
    st = await evaluate("window.__state()");
    check(JSON.stringify(st.picks) === JSON.stringify(["ticket:b", "inspect:seat"]), `${W}x${H}: picks ${st.picks}`);
    const seat = await evaluate("window.__seat()");
    v.seat = seat;
    check(seat.fontReady === true, `${W}x${H}: Noto Serif not certified`);
    check(seat.diagnostic === true && seat.accepted === false, `${W}x${H}: diagnostic fixture reported as accepted`);
    await click(seat.rail.play);
    await until("!window.__state().paused && window.__state().time > 0.8", 20000, "playback");
    await delay(300);
    v.playing = await evaluate("window.__state()");
    v.plaque = await evaluate("window.__plaque()");
    check(v.plaque.lines >= 2 && v.plaque.lines <= 3, `${W}x${H}: plaque lines ${v.plaque.lines}`);
    check(v.plaque.rowsInside, `${W}x${H}: plaque text escapes the lectern inset`);
    check(v.plaque.floorGap >= 8, `${W}x${H}: plaque floor gap ${v.plaque.floorGap}`);
    check(v.plaque.bannerGap >= 0, `${W}x${H}: banner cards cover the stage opening by ${-v.plaque.bannerGap}px`);
    check(v.plaque.bannerInView, `${W}x${H}: banner cards leave the window`);
    await shot("stage");
    const [sx, sy] = seat.rail.seek;
    await click([sx, sy]);
    await delay(150);
    v.seek = await evaluate("({ u: window.__seekU(), sample: window.__state().sample, count: window.__count() })");
    check(v.seek.u !== null && Math.abs(v.seek.sample / v.seek.count - v.seek.u) < 0.02, `${W}x${H}: gauge seek ${JSON.stringify(v.seek)}`);
    check(Math.abs(v.seek.u - 0.5) < 0.05, `${W}x${H}: gauge centre pick u=${v.seek.u}`);
    v.parallax = await evaluate("window.__parallax()");
    check(Math.abs(v.parallax.prosceniumShift) < 1, `${W}x${H}: proscenium moved ${v.parallax.prosceniumShift}`);
    const shifts = v.parallax.layerShifts.map(([, s]) => s);
    check(shifts.every((s, i) => i === 0 || shifts[i - 1] < s), `${W}x${H}: layer shifts not ordered ${shifts}`);
    for (const [id, got] of v.parallax.extremePicks) check(got === `rail:${id}`, `${W}x${H}: pick at ${id} -> ${got}`);
    check(v.parallax.reducedShift[0] === 0 && v.parallax.reducedShift[1] === 0, `${W}x${H}: reduced motion shift`);
    await click(seat.rail.play);
    await delay(1500);
    v.paused = await evaluate("window.__state()");
    check(v.paused.paused === true, `${W}x${H}: rail pause failed`);
    await shot("intermission");
    if (W === 1280) {
      // Live resize to a 4:3 window: full-viewport buffer, refit, picks.
      await call("Emulation.setDeviceMetricsOverride", { width: 1600, height: 1200, deviceScaleFactor: 1, mobile: false });
      await delay(400);
      v.resized = await evaluate("window.__resized()");
      check(v.resized.canvas[0] === 1600 && v.resized.canvas[1] === 1200, `resize: canvas ${v.resized.canvas}`);
      for (const [id, got] of v.resized.picks) check(got === `rail:${id}`, `resize: pick at ${id} -> ${got}`);
      check(v.resized.lectern.top > 0 && v.resized.rail.top + v.resized.rail.height <= 1200, `resize: layout ${JSON.stringify(v.resized)}`);
      await call("Page.captureScreenshot", { format: "png" }).then((s) => writeFile(join(outDir, "immersive-stage-resized-1600x1200.png"), Buffer.from(s.data, "base64")));
      await call("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
      await delay(400);
    }
    if (W === 1280) {
      await click(seat.rail.play);
      await delay(400);
      v.perf = await evaluate("window.__perf(10000)");
      evidence.teardown = await evaluate("window.__teardown()");
      for (const [i, c] of evidence.teardown.entries()) {
        check(c.listenerDelta === 0, `cycle ${i}: listener delta ${c.listenerDelta}`);
        check(c.framesAfterDispose === 0, `cycle ${i}: frames after dispose ${c.framesAfterDispose}`);
        check(c.after.geometries === 0 && c.after.textures === 0, `cycle ${i}: GPU resources after dispose ${JSON.stringify(c.after)}`);
        check(c.canvases === 0, `cycle ${i}: ${c.canvases} canvases left`);
      }
    }
    st = await evaluate("window.__state()");
    check(st.errors.length === 0, `${W}x${H}: page errors ${st.errors.join("; ")}`);
    evidence.viewports.push(v);
    await call("Page.close").catch(() => {});
  }
} catch (error) {
  failures.push(`harness: ${error instanceof Error ? error.stack ?? error.message : String(error)}`);
} finally {
  try { ws?.close(); } catch { /* ignore */ }
  chrome?.kill("SIGKILL");
  server?.close();
}
evidence.failures = failures;
await writeFile(join(outDir, "immersive.json"), JSON.stringify(evidence, null, 2) + "\n");
if (failures.length) {
  console.error(`check:immersive FAILED\n- ${failures.join("\n- ")}`);
  process.exit(1);
}
console.log("check:immersive passed");
