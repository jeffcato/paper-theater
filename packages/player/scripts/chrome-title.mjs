#!/usr/bin/env node
// check:title — prove the authored title renders in front of closed
// curtains on screen and in captures, at all three viewports. Mounts the
// readiness-v2 package at sample 0 (title_visible, curtains closed) with
// the real player build and counts bright title-text pixels inside the
// projected card rect on both paths: the composited canvas readback
// (screen, same bar as the studio strict check: R,G,B > 150, more than 20
// pixels) and captureFrame pixels. A blackout stop guards the gain gating.
// Zero provider calls: every request is same-origin localhost.
// Evidence: docs/implementation/player/title.json + title-<WxH>.png.
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
const evidenceJson = join(root, "docs", "implementation", "player", "title.json");
const shotName = (w, h) => join(root, "docs", "implementation", "player", `title-${w}x${h}.png`);
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const VIEWPORTS = [[960, 540], [1280, 720], [1920, 1080]];
const BLACKOUT = 1752000;

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script>
<style>html, body { margin: 0; padding: 0; background: #000; }</style></head>
<body><div id="theater"></div>
<script type="module">
import * as api from "/dist/src/index.js";
window.__errors = [];
addEventListener("error", (e) => window.__errors.push(String(e.message)));
addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason && e.reason.message ? e.reason.message : e.reason)));
async function load(path, maxBytes) {
  const res = await fetch(path);
  if (!res.ok) throw new Error("fetch failed: " + path);
  const buf = new Uint8Array(await res.arrayBuffer());
  if (buf.length > maxBytes) throw new Error("over bound: " + path);
  return buf;
}
async function sha256(bytes) {
  const d = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(d)].map((n) => n.toString(16).padStart(2, "0")).join("");
}
const text = (v) => new TextDecoder().decode(v);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
window.__boot = async () => {
  const out = {};
  const params = new URLSearchParams(location.search);
  const W = Number(params.get("w"));
  const H = Number(params.get("h"));
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
  const handle = await api.loadDiagnostic(manifestBytes, manifestDigest, read, policy);
  const sink = await api.createThreeSink(W, H, { figures });
  sink.canvas.style.width = W + "px";
  sink.canvas.style.height = H + "px";
  theater.appendChild(sink.canvas);
  const mount = api.mountPlayer(theater, { handle, timeline }, { sink, candidate_id: "title-proof" });
  // Title card corners from the studio strict check (inset card edges).
  const pose = api.baseCameraPose();
  const vp = { width: W, height: H };
  const cornerA = api.projectToCss([-4.9, 6, 2.5], pose, vp);
  const cornerB = api.projectToCss([4.9, 3, 2.5], pose, vp);
  const countBright = (pixels, pw, ph) => {
    let n = 0;
    let maxR = 0;
    let maxG = 0;
    let maxB = 0;
    const x0 = Math.max(0, Math.floor(cornerA[0] * pw / W));
    const x1 = Math.min(pw - 1, Math.ceil(cornerB[0] * pw / W));
    // CSS y grows downward: cornerA (world top) maps above cornerB.
    const y0 = Math.max(0, Math.floor(cornerA[1] * ph / H));
    const y1 = Math.min(ph - 1, Math.ceil(cornerB[1] * ph / H));
    for (let y = y0; y <= y1; y++) {
      for (let x = x0; x <= x1; x++) {
        const i = ((ph - 1 - y) * pw + x) * 4;
        maxR = Math.max(maxR, pixels[i]);
        maxG = Math.max(maxG, pixels[i + 1]);
        maxB = Math.max(maxB, pixels[i + 2]);
        if (pixels[i] > 150 && pixels[i + 1] > 150 && pixels[i + 2] > 150) n++;
      }
    }
    return { n, rect: [x0, y0, x1, y1], max: [maxR, maxG, maxB] };
  };
  // Screen path: read the presented canvas in the same task as the tick,
  // exactly like the studio strict check.
  const screenTitlePixels = () => {
    const gl = sink.canvas.getContext("webgl2");
    const pixels = new Uint8Array(sink.canvas.width * sink.canvas.height * 4);
    gl.readPixels(0, 0, sink.canvas.width, sink.canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    return countBright(pixels, sink.canvas.width, sink.canvas.height);
  };
  mount.seek(0);
  const opened = mount.tick();
  const capture = await mount.captureFrame(W, H);
  // Capture targets hold linear light; decode to sRGB so the capture path
  // asserts the identical presented-image bar as the screen path.
  const toSrgb = (l) => (l <= 0.0031308 ? 12.92 * l : 1.055 * Math.pow(l, 1 / 2.4) - 0.055);
  const decoded = new Uint8Array(capture.pixels.length);
  for (let i = 0; i < capture.pixels.length; i += 4) {
    decoded[i] = Math.round(toSrgb(capture.pixels[i] / 255) * 255);
    decoded[i + 1] = Math.round(toSrgb(capture.pixels[i + 1] / 255) * 255);
    decoded[i + 2] = Math.round(toSrgb(capture.pixels[i + 2] / 255) * 255);
    decoded[i + 3] = capture.pixels[i + 3];
  }
  const capCount = countBright(decoded, W, H);
  const screenCount = screenTitlePixels();
  // Screenshot the title stop itself (before the blackout re-pose below).
  window.__wantShot = W + "x" + H + "-title";
  window.__shotAck = false;
  while (!window.__shotAck) await sleep(100);
  window.__wantShot = null;
  out.title = {
    sample: opened.sample, phase: opened.phase, titleVisible: opened.title_visible,
    curtain: opened.curtain_open_ratio, gain: opened.stage_gain,
    capturePixels: capCount, screenPixels: screenCount,
    captureSha: await sha256(capture.pixels),
  };
  // Blackout guard: the gain gate must keep the aperture dark with house lit.
  mount.seek(1752000);
  mount.tick();
  const dark = await mount.captureFrame(W, H);
  const ap = api.projectToCss([0, 2, 0], pose, vp);
  const ho = api.projectToCss([-9.5, 4, 1.8], pose, vp);
  const box = (c, half) => {
    let m = 0;
    for (let y = Math.max(0, Math.floor(c[1] - half)); y <= Math.min(H - 1, Math.ceil(c[1] + half)); y++) {
      for (let x = Math.max(0, Math.floor(c[0] - half)); x <= Math.min(W - 1, Math.ceil(c[0] + half)); x++) {
        const i = ((H - 1 - y) * W + x) * 4;
        m = Math.max(m, dark.pixels[i] / 255, dark.pixels[i + 1] / 255, dark.pixels[i + 2] / 255);
      }
    }
    return m;
  };
  out.blackout = { sample: 1752000, openingMax: box(ap, 20), houseMax: box(ho, 20) };
  mount.dispose();
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

const failures = [];
const check = (cond, message) => {
  if (!cond) failures.push(message);
};

const temp = await mkdtemp(join(tmpdir(), "pt-player-title-"));
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
  const viewports = [];
  for (const [W, H] of VIEWPORTS) {
    const page = await (await fetch(`http://127.0.0.1:${debugPort}/json/new?${encodeURIComponent(`http://127.0.0.1:${port}/?w=${W}&h=${H}`)}`, { method: "PUT" })).json();
    if (ws) {
      try {
        ws.close();
      } catch { /* ignore */ }
    }
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
      throw new Error("title harness did not boot");
    })();
    await evaluate(`window.__start()`, false);
    await (async () => {
      const start = Date.now();
      while (Date.now() - start < 120000) {
        const st = await evaluate(`({ done: window.__done === true, want: window.__wantShot ?? null, error: window.__bootError ?? null })`);
        if (st.error) throw new Error(`in-page failure: ${st.error}`);
        if (st.done) return;
        if (st.want) {
          const shot = await call("Page.captureScreenshot", { format: "png" });
          await writeFile(shotName(W, H), Buffer.from(shot.data, "base64"));
          await evaluate(`window.__shotAck = true`);
        }
        await delay(500);
      }
      throw new Error(`title run timed out at ${W}x${H}`);
    })();
    const out = await evaluate(`window.__result`);
    viewports.push({ width: W, height: H, ...out });
    await call("Page.close");
  }

  for (const v of viewports) {
    const label = `${v.width}x${v.height}`;
    check(v.title.sample === 0, `${label}: title sample ${v.title.sample}`);
    check(v.title.titleVisible === true, `${label}: title_visible false`);
    check(v.title.curtain === 0, `${label}: curtains not closed (${v.title.curtain})`);
    check(v.title.capturePixels.n > 20, `${label}: capture title pixels ${v.title.capturePixels.n}`);
    check(v.title.screenPixels.n > 20, `${label}: screen title pixels ${v.title.screenPixels.n}`);
    check(v.blackout.openingMax < 0.004, `${label}: aperture not dark (${v.blackout.openingMax})`);
    check(v.blackout.houseMax >= 0.008, `${label}: house not lit (${v.blackout.houseMax})`);
    check((v.errors ?? []).length === 0, `${label}: console errors`);
  }
  check(requests.length > 0 && requests.every((r) => r.startsWith("/")), `non-local request: ${requests.join(",")}`);
  const report = {
    tool: "check:title",
    fixture: "fixtures/generation/two-scene-readiness-v2",
    opening: { title: "Paper Theater diagnostic", sample: 0, curtains: "closed" },
    viewports: viewports.map((v) => ({
      viewport: `${v.width}x${v.height}`,
      sample: v.title.sample, titleVisible: v.title.titleVisible, curtain: v.title.curtain, gain: v.title.gain,
      captureTitlePixels: v.title.capturePixels.n, captureRect: v.title.capturePixels.rect,
      captureMax: v.title.capturePixels.max,
      screenTitlePixels: v.title.screenPixels.n, screenRect: v.title.screenPixels.rect,
      screenMax: v.title.screenPixels.max,
      captureSha: v.title.captureSha,
      blackout: v.blackout,
    })),
    requests: { count: requests.length, remote: 0 },
    provider_calls: 0,
    failures,
  };
  await writeFile(evidenceJson, JSON.stringify(report, null, 2));
  if (failures.length) throw new Error(`check:title failed:\n- ${failures.join("\n- ")}`);
  console.log("check:title pass (authored title in front of closed curtains, 3 viewports)");
} catch (error) {
  console.error(`check:title failed: ${error instanceof Error ? error.message : String(error)}`);
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