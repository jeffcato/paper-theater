#!/usr/bin/env node
// check:staging — verify the rendered stage shell, authored curtains/title
// and pause-driven INTERMISSION sign in headless Chrome. Mounts readiness-v2
// with the real <audio> clock and the three.js sink (no plaque: this is the
// staging path; read-along/refit stay covered by check:theater). Captures
// actual frames for opening title, curtain travel, open scene, blackout,
// closing travel, end, sign mid-lowering, sign settled, resume-raised and
// restart, asserting evaluated state linkage and sign ROI hash relations.
// Evidence: staging.json + staging-*.png.
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(pkgDir, "..", "..");
const fixtureRoot = join(root, "fixtures", "generation", "two-scene-readiness-v2");
const evidenceJson = join(root, "docs", "implementation", "player", "staging.json");
const shotName = (name) => join(root, "docs", "implementation", "player", `staging-${name}.png`);
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const W = 1280;
const H = 720;

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script>
<style>html, body { margin: 0; padding: 0; background: #000; }</style></head>
<body><div id="theater"><audio id="audio" muted preload="auto" src="/fx/audio/master.wav"></audio></div>
<script type="module">
import * as api from "/dist/src/index.js";
window.__errors = [];
addEventListener("error", (e) => window.__errors.push(String(e.message)));
addEventListener("unhandledrejection", (e) => window.__errors.push(String(e.reason?.message ?? e.reason)));
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
window.__boot = async () => {
  const out = {};
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
  const sink = await api.createThreeSink(${W}, ${H}, {
    figures,
    title: { heading: timeline.opening.title, sub: timeline.opening.chapter_label },
  });
  const theater = document.querySelector("#theater");
  theater.appendChild(sink.canvas);
  const audio = document.querySelector("#audio");
  const mount = api.mountPlayer(theater, { handle, timeline }, { clock: audio, sink, candidate_id: "staging-run-1" });
  window.__mount = mount;
  window.__audio = audio;
  // Screenshot handshake: the driver captures each posed stop while the
  // run waits; audio-timed sign states cannot be re-posed afterwards.
  const snap = async (name) => {
    window.__wantShot = name;
    window.__shotAck = false;
    while (!window.__shotAck) await new Promise((r) => setTimeout(r, 100));
    window.__wantShot = null;
  };
  const audioState = () => ({
    t: +audio.currentTime.toFixed(3), paused: audio.paused,
    seeking: audio.seeking, ended: audio.ended, ready: audio.readyState,
  });
  const at = async (name, sample) => {
    mount.seek(sample);
    const state = mount.tick();
    const frame = await mount.captureFrame(${W}, ${H});
    let maxChannel = 0;
    for (let i = 0; i < frame.pixels.length; i += 4) {
      maxChannel = Math.max(maxChannel, frame.pixels[i] / 255, frame.pixels[i + 1] / 255, frame.pixels[i + 2] / 255);
    }
    const record = {
      sample: state.sample, phase: state.phase, curtain: state.curtain_open_ratio,
      title: state.title_visible, gain: state.stage_gain,
      pixels_sha256: await sha256(frame.pixels), maxChannel,
      audio: audioState(),
    };
    await snap(name);
    return record;
  };
  out.title = await at("opening-title", 36000);
  out.curtainMid = await at("curtain-mid", 204000);
  out.scene = await at("scene", 500000);
  out.blackout = await at("blackout", 1752000);
  out.closingMid = await at("closing-mid", 3227000);
  out.end = await at("end", 3262560);
  // Pause-driven sign through the transport (owns play intent, epochs and
  // the sign clock): baseline, mid-lowering, settled x2, resume, restart.
  const S = 600000;
  mount.seek(S);
  mount.tick();
  await mount.play();
  await new Promise((r) => setTimeout(r, 800));
  out.playing = await at("playing", mount.sample);
  mount.pause();
  await new Promise((r) => setTimeout(r, 400));
  mount.tick();
  out.signMid = await at("sign-mid", mount.sample);
  await new Promise((r) => setTimeout(r, 1500));
  mount.tick();
  out.signSettled = await at("sign-settled", mount.sample);
  mount.tick();
  out.signSettled2 = await at("sign-settled2", mount.sample);
  await mount.play();
  await new Promise((r) => setTimeout(r, 1300));
  mount.seek(S);
  out.resumed = await at("resumed", S);
  out.resumed2 = await at("resumed2", S);
  mount.restart();
  audio.currentTime = 0;
  mount.tick();
  out.restarted = await at("restarted", 0);
  mount.tick();
  out.restarted2 = await at("restarted2", 0);
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
      window.__bootError = String(error?.message ?? error);
      window.__done = true;
    },
  );
};
window.__finish = async () => {
  const out = { errors: window.__errors };
  window.__mount.dispose();
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

const temp = await mkdtemp(join(tmpdir(), "pt-player-staging-"));
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
  await call("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 60000) {
      try {
        if (await evaluate(`typeof window.__start === "function"`)) return;
      } catch { /* retry */ }
      await delay(200);
    }
    throw new Error("staging harness did not boot");
  })();
  await evaluate(`window.__start()`, false);
  const shots = {};
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 240000) {
      const st = await evaluate(`({ done: window.__done === true, want: window.__wantShot ?? null, error: window.__bootError ?? null })`);
      if (st.error) throw new Error(`in-page failure: ${st.error}`);
      if (st.done) return;
      if (st.want) {
        const shot = await call("Page.captureScreenshot", { format: "png" });
        await writeFile(shotName(st.want), Buffer.from(shot.data, "base64"));
        shots[st.want] = createHash("sha256").update(Buffer.from(shot.data, "base64")).digest("hex");
        await evaluate(`window.__shotAck = true`);
      }
      await delay(150);
    }
    throw new Error("staging run timed out");
  })();
  const out = await evaluate(`window.__result`);
  const tail = await evaluate(`window.__finish()`);
  Object.assign(out, tail);

  // State linkage per stop (evaluated values behind the rendered frames).
  check(out.title.phase === "opening" && out.title.title === true && out.title.curtain === 0, "opening title stop");
  check(out.curtainMid.phase === "scene" && out.curtainMid.curtain > 0.35 && out.curtainMid.curtain < 0.65, `curtain opens over scene-1 (${out.curtainMid.curtain})`);
  check(out.scene.phase === "scene" && out.scene.curtain === 1 && out.scene.title === false, "open scene stop");
  check(out.blackout.gain === 0 && out.blackout.maxChannel === 0, `blackout composites black (${out.blackout.maxChannel})`);
  check(out.closingMid.phase === "closing" && out.closingMid.curtain > 0.35 && out.closingMid.curtain < 0.65, `curtain closes (${out.closingMid.curtain})`);
  check(out.end.phase === "closing" && out.end.curtain === 0, "end stop");
  // Sign motion is visible across pause/resume/restart (screenshot hashes).
  check(shots.playing !== shots["sign-mid"], "sign lowering is visible");
  check(shots["sign-mid"] !== shots["sign-settled"], "sign travel is visible");
  check(shots["sign-settled"] === shots["sign-settled2"], "settled sign is deterministic");
  check(shots.resumed !== shots["sign-settled"], "resume raises the sign");
  check(shots.resumed === shots["resumed2"], "raised sign is deterministic");
  check(shots.restarted !== shots["sign-settled"], "restart hides the sign");
  check(shots.restarted === shots["restarted2"], "restarted frame is clean");
  check((out.errors ?? []).length === 0, `console errors: ${(out.errors ?? []).join("; ").slice(0, 200)}`);
  const report = {
    tool: "check:staging",
    fixture: "fixtures/generation/two-scene-readiness-v2",
    viewport: `${W}x${H}`,
    stops: Object.fromEntries(
      ["title", "curtainMid", "scene", "blackout", "closingMid", "end", "playing", "signMid", "signSettled", "resumed", "restarted"].map((k) => [
        k,
        {
          sample: out[k].sample, phase: out[k].phase, curtain: +out[k].curtain.toFixed(3),
          title: out[k].title, gain: out[k].gain, pixels_sha256: out[k].pixels_sha256,
          audio: out[k].audio,
        },
      ]),
    ),
    shotSha256: shots,
    failures,
  };
  await writeFile(evidenceJson, JSON.stringify(report, null, 2));
  if (failures.length) throw new Error(`check:staging failed:\n- ${failures.join("\n- ")}`);
  console.log("check:staging pass (shell/curtains/title/sign frames verified)");
} catch (error) {
  console.error(`check:staging failed: ${error instanceof Error ? error.message : String(error)}`);
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

