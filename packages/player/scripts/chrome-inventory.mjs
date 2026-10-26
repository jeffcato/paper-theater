#!/usr/bin/env node
// check:inventory — execute the 144 authored capture requests from
// fixtures/generation/capture/inventory.json in headless Chrome, one mount
// per viewport (960x540, 1280x720, 1920x1080 at DPR 1). Hashes are refreshed
// live at run time (inventory bytes, manifest bytes, canonical fixture
// digest) and recorded alongside every result; mismatches against the frozen
// records fail loudly instead of pinning stale bytes.
// Evidence: docs/implementation/player/inventory-capture.json + 3 PNGs.
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
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

function arg(name, fallback) {
  const i = process.argv.indexOf(name);
  return i >= 0 && i + 1 < process.argv.length ? process.argv[i + 1] : fallback;
}
// Versioned fixture/inventory inputs. Legacy defaults reproduce the earlier
// shared-fixture run; the published v2 run passes explicit v2 paths.
const fixtureRoot = resolve(root, arg("--fixture", "fixtures/shared/two-scene-v1"));
const inventoryPath = resolve(root, arg("--inventory", "fixtures/generation/capture/inventory.json"));
const tag = arg("--tag", "");
const evidenceJson = join(
  root, "docs", "implementation", "player",
  tag ? `inventory-capture-${tag}.json` : "inventory-capture.json",
);
const shotName = (w, h) => join(
  root, "docs", "implementation", "player",
  tag ? `inventory-${tag}-${w}x${h}.png` : `inventory-${w}x${h}.png`,
);

const shaFile = async (path) => createHash("sha256").update(await readFile(path)).digest("hex");

// Mirror of the generator's binding digest: SHA-256 of sorted-path player
// file records as UTF-8 sorted-key compact JSON plus LF.
async function playerFilesDigest() {
  const { readdir } = await import("node:fs/promises");
  const dir = join(pkgDir, "dist", "src");
  const names = (await readdir(dir)).filter((n) => n.endsWith(".js")).sort();
  if (!names.length) throw new Error("player build required before pinning capture code");
  const records = [];
  for (const name of names) {
    const data = await readFile(join(dir, name));
    // Sorted-key order (bytes, path, sha256) to match the generator's
    // canonical encoding exactly; JSON.stringify emits insertion order.
    records.push({ bytes: data.length, path: `packages/player/dist/src/${name}`, sha256: createHash("sha256").update(data).digest("hex") });
  }
  records.sort((a, b) => (a.path < b.path ? -1 : 1));
  return createHash("sha256").update(JSON.stringify(records) + "\n").digest("hex");
}

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script></head>
<body><div id="stage"></div>
<script type="module">
import * as api from "/dist/src/index.js";
window.__errors = [];
addEventListener("error", (e) => window.__errors.push(String(e.message)));
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
function stats(pixels) {
  let nonblack = 0, max = 0;
  for (let i = 0; i < pixels.length; i += 4) {
    const r = pixels[i] / 255, g = pixels[i + 1] / 255, b = pixels[i + 2] / 255;
    max = Math.max(max, r, g, b);
    if (r > 8 / 255 || g > 8 / 255 || b > 8 / 255) nonblack++;
  }
  return { nonblackFraction: nonblack / (pixels.length / 4), maxChannel: max };
}
window.__run = async () => {
  const inventory = JSON.parse(new TextDecoder().decode(await load("/inventory.json", 4 * 1024 * 1024)));
  const manifestBytes = await load("/fx/manifest.json", 1024 * 1024);
  const manifestDigest = await sha256(manifestBytes);
  const read = async (path) => load("/fx/" + path, 64 * 1024 * 1024);
  const policy = { player_sha256: "0".repeat(64), checker_sha256: "0".repeat(64), recipe_sha256: "0".repeat(64), allow_diagnostic: false };
  const handle = await api.loadDiagnostic(manifestBytes, manifestDigest, read, policy);
  const timeline = JSON.parse(new TextDecoder().decode(await load("/fx/timeline.json", 1024 * 1024)));
  const actorsDoc = JSON.parse(new TextDecoder().decode(await load("/fx/actors.json", 1024 * 1024)));
  const figures = actorsDoc.map((card) => ({
    asset_id: card.id,
    url: "/fx/" + card.image.path,
    pixel_size: card.pixel_size,
    pivot_top_left: card.pivot_top_left,
    stage_size: card.stage_size,
    allow_mirror: card.allow_mirror,
  }));
  const nodCue = timeline.cues.find((c) => c.id === "nod-em");
  const nodSamples = nodCue
    ? [6000, 12000, 18000].map((d) => nodCue.start_sample + d)
    : [];
  const stage = document.querySelector("#stage");
  const results = [];
  for (const [width, height] of [[960, 540], [1280, 720], [1920, 1080]]) {
    const sink = await api.createThreeSink(width, height, { figures });
    stage.replaceChildren(sink.canvas);
    const mount = api.mountPlayer(document.body, { handle, timeline }, { sink, candidate_id: "inventory-run-1" });
    for (const item of inventory.requests.filter((r) => r.request.width === width && r.request.height === height)) {
      const req = item.request;
      if (req.manifest_digest !== manifestDigest) throw new Error("manifest drift under run: " + item.id);
      mount.seek(req.sample);
      mount.tick();
      const frame = await mount.captureFrame(req.width, req.height);
      const s = stats(frame.pixels);
      results.push({
        id: item.id, sample: frame.sample, phase: frame.phase,
        state_digest: frame.state_digest, contributor_ids: frame.contributor_ids,
        accepted: frame.accepted, provenance: frame.provenance,
        pixels_sha256: await sha256(frame.pixels),
        nonblackFraction: s.nonblackFraction, maxChannel: s.maxChannel,
      });
    }
    // nod-em gesture evidence at cue-interior samples (instance em-2).
    const gestures = {};
    for (const sample of nodSamples) {
      mount.seek(sample);
      const state = mount.tick();
      gestures[sample] = (state.actors.find((a) => a.instance_id === "em-2") ?? {}).gesture ?? null;
    }
    results.push({ id: "nod-em:" + width + "x" + height, gestures });
    // Evidence screenshot handshake: re-render the nod interior, publish the
    // shot request, and wait for the driver to capture before disposing.
    mount.seek(nodSamples[1] ?? 0);
    mount.tick();
    window.__wantShot = { width, height };
    window.__shotAck = false;
    await new Promise((resolve) => {
      const timer = setInterval(() => {
        if (window.__shotAck) {
          clearInterval(timer);
          resolve();
        }
      }, 100);
    });
    window.__wantShot = null;
    mount.dispose();
    sink.dispose();
  }
  return { results, errors: window.__errors };
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

const temp = await mkdtemp(join(tmpdir(), "pt-player-inventory-"));
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
      return serveFile(res, join(fixtureRoot, name), "application/octet-stream");
    }
    if (url.pathname === "/inventory.json") {
      return serveFile(res, inventoryPath, "application/octet-stream");
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
    "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
    "--remote-debugging-port=0", `--user-data-dir=${join(temp, "chrome")}`,
    "--window-size=1920,1080", "about:blank",
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
  const runPromise = evaluate(`window.__run()`);
  // Screenshot handshake: three shots arrive mid-run, one per viewport.
  const shotsTaken = [];
  {
    const start = Date.now();
    while (shotsTaken.length < 3 && Date.now() - start < 420000) {
      const want = await evaluate(`window.__wantShot ?? null`);
      if (want && !shotsTaken.includes(want.width + "x" + want.height)) {
        await call("Emulation.setDeviceMetricsOverride", { width: want.width + 40, height: want.height + 40, deviceScaleFactor: 1, mobile: false });
        await delay(400);
        const shot = await call("Page.captureScreenshot", { format: "png" });
        await writeFile(shotName(want.width, want.height), Buffer.from(shot.data, "base64"));
        shotsTaken.push(want.width + "x" + want.height);
        await evaluate(`window.__shotAck = true`);
      } else {
        await delay(250);
      }
    }
  }
  const out = await runPromise;
  if (shotsTaken.length !== 3) throw new Error(`only ${shotsTaken.length}/3 evidence shots taken`);
  if (out.errors?.length) throw new Error(`console errors: ${out.errors.join("; ").slice(0, 400)}`);

  // Refreshed hashes, computed live from the bytes actually executed.
  const manifestPath = join(fixtureRoot, "manifest.json");
  const inventoryDoc = JSON.parse(await readFile(inventoryPath, "utf8"));
  const refreshed = {
    inventory_sha256: await shaFile(inventoryPath),
    manifest_sha256: await shaFile(manifestPath),
  };
  const hashNotes = [];
  if (refreshed.manifest_sha256 !== inventoryDoc.bindings.manifest_sha256) {
    hashNotes.push(`manifest drift: live ${refreshed.manifest_sha256} vs inventory binding ${inventoryDoc.bindings.manifest_sha256}`);
  }
  // Exact executed player/checker identities: recompute the pinned digests
  // from the live build and generator, and report (never silently switch).
  const playerDigest = await playerFilesDigest();
  const checkerDigest = await shaFile(join(root, "tools", "generation", "capture_inventory.py"));
  const identityNotes = [];
  const pinnedPlayer = inventoryDoc.bindings.player_sha256;
  const pinnedChecker = inventoryDoc.bindings.checker_sha256;
  if (playerDigest !== pinnedPlayer) {
    identityNotes.push(`player binding: inventory pins earlier build ${pinnedPlayer.slice(0, 16)}…, executed ${playerDigest.slice(0, 16)}… (preserved, reported)`);
  }
  if (checkerDigest !== pinnedChecker) {
    identityNotes.push(`checker binding: inventory pins ${pinnedChecker.slice(0, 16)}…, executed ${checkerDigest.slice(0, 16)}… (preserved, reported)`);
  }
  const captures = out.results.filter((r) => r.state_digest);
  const expected = inventoryDoc.requests.length;
  if (captures.length !== expected) throw new Error(`expected ${expected} captures, got ${captures.length}`);
  const failures = [...hashNotes];
  for (const c of captures) {
    if (c.accepted !== false) failures.push(`${c.id}: not stamped diagnostic`);
    if (c.provenance !== "diagnostic") failures.push(`${c.id}: wrong provenance`);
    if (!/^[a-f0-9]{64}$/.test(c.state_digest)) failures.push(`${c.id}: bad state digest`);
    if (!/^[a-f0-9]{64}$/.test(c.pixels_sha256)) failures.push(`${c.id}: bad image hash`);
    if (!Array.isArray(c.contributor_ids)) failures.push(`${c.id}: missing contributor ids`);
  }
  const nodRows = out.results.filter((r) => r.gestures);
  for (const row of nodRows) {
    for (const [sample, gesture] of Object.entries(row.gestures)) {
      if (gesture !== "nod") failures.push(`${row.id} @${sample}: em-2 gesture ${gesture} (want nod)`);
    }
  }
  await writeFile(evidenceJson, JSON.stringify({
    tool: "check:inventory", profile: "paper-player-r186-v1",
    fixture: fixtureRoot, inventory: inventoryPath,
    refreshed_hashes: refreshed,
    executed_identities: { player_sha256: playerDigest, checker_sha256: checkerDigest },
    pinned_identities: { player_sha256: pinnedPlayer, checker_sha256: pinnedChecker },
    identity_notes: identityNotes,
    hash_notes: hashNotes, captures: captures.length,
    capture_rows: captures.map((c) => ({
      id: c.id, sample: c.sample, phase: c.phase, state_digest: c.state_digest,
      contributor_ids: c.contributor_ids, accepted: c.accepted, provenance: c.provenance,
      pixels_sha256: c.pixels_sha256,
      nonblackFraction: c.nonblackFraction, maxChannel: c.maxChannel,
    })),
    nod_em_gestures: nodRows, failures,
    browser: "headless Chrome (SwiftShader)",
  }, null, 2));
  if (failures.length) throw new Error(`inventory run failed: ${failures.slice(0, 5).join("; ")}${failures.length > 5 ? ` (+${failures.length - 5} more)` : ""}`);
  console.log(`check:inventory pass (${captures.length} captures, nod-em gestures nod x${nodRows.length * 3}, hashes refreshed)`);
  for (const note of identityNotes) console.log(`identity note: ${note}`);
} catch (error) {
  console.error(`check:inventory failed: ${error instanceof Error ? error.message : String(error)}`);
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
