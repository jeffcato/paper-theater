#!/usr/bin/env node
// check:chapter — prove chapter-sized WAV loading in headless Chrome with the
// real player build. Synthesizes a real 180 s PCM16 stereo 48 kHz master
// (34,560,044 bytes, >32 MiB) into a temp dir — never into fixtures — serves
// it over localhost with Range support, then drives one page through: loader
// inspection + diagnostic load with exact bytes/hash, WebAudio decode with
// channel/rate/length/duration/content checks, real <audio> seeking
// (metadata, 90 s seek, 1 s of playback advance, 179 s seek), tampered/
// truncated/garbage rejection, and AudioContext disposal. Zero provider
// calls: every request is same-origin localhost (request log asserted).
// Evidence: docs/implementation/player/chapter.json.
// Note: the generation pipeline keeps its own 32 MiB production cap; this
// proof synthesizes its chapter master locally and makes no production
// claim about full-chapter generation output.
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { mkdtemp, readFile, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(pkgDir, "..", "..");
const evidenceJson = join(root, "docs", "implementation", "player", "chapter.json");
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const FRAMES_180S = 180 * 48000;
const EXPECTED_BYTES = 34560044;
const shaHex = (buf) => createHash("sha256").update(buf).digest("hex");

/** Real PCM16 stereo 48 kHz WAV: 44-byte header + int16 ramp samples. */
function chapterWav() {
  const channels = 2;
  const data = FRAMES_180S * channels * 2;
  const buf = Buffer.alloc(44 + data);
  buf.write("RIFF", 0);
  buf.writeUInt32LE(36 + data, 4);
  buf.write("WAVE", 8);
  buf.write("fmt ", 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(channels, 22);
  buf.writeUInt32LE(48000, 24);
  buf.writeUInt32LE(48000 * channels * 2, 28);
  buf.writeUInt16LE(channels * 2, 32);
  buf.writeUInt16LE(16, 34);
  buf.write("data", 36);
  buf.writeUInt32LE(data, 40);
  for (let i = 0; i < FRAMES_180S * channels; i++) {
    buf.writeInt16LE((i % 65536) - 32768, 44 + i * 2);
  }
  return buf;
}

const PAGE = `<!doctype html><html><head><meta charset="utf-8">
<style>html, body { margin: 0; padding: 0; background: #000; }</style></head>
<body>
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
window.__boot = async () => {
  const out = {};
  const manifestBytes = await load("/fx/manifest.json", 1024 * 1024);
  const manifestDigest = await sha256(manifestBytes);
  const manifest = JSON.parse(text(manifestBytes));
  const audioPath = manifest.entries.audio.path;
  const store = {};
  const read = async (path) => {
    const cap = path === audioPath ? 256 * 1024 * 1024 : path.endsWith(".json") ? 1024 * 1024 : 32 * 1024 * 1024;
    const buf = await load("/fx/" + path, cap);
    store[path] = buf;
    return buf;
  };
  const policy = { player_sha256: "0".repeat(64), checker_sha256: "0".repeat(64), recipe_sha256: "0".repeat(64), allow_diagnostic: true };
  const inspection = await api.inspectRelease(manifestBytes, manifestDigest, read, policy);
  out.inspectionIssues = inspection.issues.map((i) => i.code + ":" + i.target);
  const handle = await api.loadDiagnostic(manifestBytes, manifestDigest, read, policy);
  const got = store[audioPath];
  out.audioBytes = got.length;
  out.audioSha256 = await sha256(got);
  out.audioShaMatch = out.audioSha256 === manifest.entries.audio.sha256;
  out.handleFiles = handle.files.size;
  // WebAudio decode of the exact loaded bytes (copied: decode detaches).
  const AC = window.AudioContext || window.webkitAudioContext;
  const ctx = new AC({ sampleRate: 48000 });
  out.contextRate = ctx.sampleRate;
  const decoded = await ctx.decodeAudioData(got.slice().buffer);
  out.decodeChannels = decoded.numberOfChannels;
  out.decodedRate = decoded.sampleRate;
  out.decodedLength = decoded.length;
  out.decodeDuration = decoded.duration;
  if (decoded.sampleRate === 48000) {
    const left = decoded.getChannelData(0);
    out.rampHead = [left[0], left[1], left[2], left[3]];
  } else {
    out.rampHead = null;
  }
  // Real media-pipeline seeking against the Range-served master.
  const el = document.createElement("audio");
  el.preload = "auto";
  el.src = "/fx/" + audioPath;
  document.body.appendChild(el);
  await new Promise((res, rej) => {
    const to = setTimeout(() => rej(new Error("metadata timeout")), 60000);
    el.onloadedmetadata = () => { clearTimeout(to); res(null); };
    el.onerror = () => { clearTimeout(to); rej(new Error("media error")); };
  });
  out.mediaDuration = el.duration;
  const seekTo = (t) => new Promise((res, rej) => {
    const to = setTimeout(() => rej(new Error("seek timeout " + t)), 30000);
    el.onseeked = () => { clearTimeout(to); res(el.currentTime); };
    el.currentTime = t;
  });
  out.seek90 = await seekTo(90);
  await el.play();
  await new Promise((r) => setTimeout(r, 1000));
  out.playheadAfter1s = el.currentTime;
  out.playing = !el.paused;
  el.pause();
  out.seek179 = await seekTo(179);
  // Invalid media: tampered byte, truncated body, garbage labeled as WAV.
  const tampered = got.slice();
  tampered[60] = tampered[60] ^ 0xff;
  let r1 = "";
  try {
    await api.loadDiagnostic(manifestBytes, manifestDigest, async (p) => (p === audioPath ? tampered : read(p)), policy);
  } catch (e) { r1 = String((e && e.message) || e); }
  out.tamperRejection = r1;
  const short = got.subarray(0, 100);
  let r2 = "";
  try {
    await api.loadDiagnostic(manifestBytes, manifestDigest, async (p) => (p === audioPath ? short : read(p)), policy);
  } catch (e) { r2 = String((e && e.message) || e); }
  out.truncateRejection = r2;
  const garbage = new TextEncoder().encode("not a wav, just text bytes...................");
  const evil = JSON.parse(JSON.stringify(manifest));
  evil.entries.audio.bytes = garbage.length;
  evil.entries.audio.sha256 = await sha256(garbage);
  const evilBytes = new TextEncoder().encode(JSON.stringify(evil));
  const evilDigest = await sha256(evilBytes);
  const evilRead = async (p) => (p === audioPath ? garbage : read(p));
  const evilInspection = await api.inspectRelease(evilBytes, evilDigest, evilRead, policy);
  out.evilIssues = evilInspection.issues.map((i) => i.code + ":" + i.target);
  let r3 = "";
  try {
    await api.loadRelease(evilBytes, evilDigest, evilRead, policy);
  } catch (e) { r3 = String((e && e.message) || e); }
  out.evilRejection = r3;
  // Disposal: drop the element, close the context, release the buffer.
  el.removeAttribute("src");
  el.load();
  el.remove();
  await ctx.close();
  out.contextState = ctx.state;
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

const temp = await mkdtemp(join(tmpdir(), "pt-player-chapter-"));
const children = [];
let ws;
let server;
try {
  // Chapter media lives in temp only: never written into fixtures/.
  const fx = join(temp, "fx");
  await mkdir(join(fx, "audio"), { recursive: true });
  const wav = chapterWav();
  if (wav.length !== EXPECTED_BYTES) throw new Error(`synthetic master is ${wav.length} bytes, want ${EXPECTED_BYTES}`);
  await writeFile(join(fx, "audio", "master.wav"), wav);
  const metaBytes = Buffer.from(JSON.stringify({ chapter: 1 }));
  const acceptanceBytes = Buffer.from("acceptance-stub");
  const manifest = {
    schema_version: 1,
    capability_version: "paper-player-v1",
    provenance: "diagnostic",
    title: "Chapter master proof",
    author: "Agent 4",
    chapter_label: "chapter-180s",
    source_sha256: "0".repeat(64),
    sample_rate: 48000,
    sample_count: FRAMES_180S,
    entries: {
      audio: { path: "audio/master.wav", bytes: wav.length, media_type: "audio/wav", sha256: shaHex(wav) },
      meta: { path: "meta.json", bytes: metaBytes.length, media_type: "application/json", sha256: shaHex(metaBytes) },
    },
    assets: [],
    reveals: [],
    acceptance: { path: "acceptance.json", bytes: acceptanceBytes.length, media_type: "application/json", sha256: shaHex(acceptanceBytes) },
  };
  await writeFile(join(fx, "manifest.json"), JSON.stringify(manifest));
  await writeFile(join(fx, "meta.json"), metaBytes);
  await writeFile(join(fx, "acceptance.json"), acceptanceBytes);

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
    if (url.pathname.startsWith("/dist/")) return serveFile(res, join(pkgDir, url.pathname), "text/javascript");
    if (url.pathname.startsWith("/fx/")) {
      const name = url.pathname.slice("/fx/".length);
      if (name.includes("..")) {
        res.writeHead(404);
        res.end("missing");
        return;
      }
      const type = name.endsWith(".wav") ? "audio/wav" : "application/octet-stream";
      return serveRanged(req, res, join(fx, name), type);
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
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 60000) {
      try {
        if (await evaluate(`typeof window.__start === "function"`)) return;
      } catch { /* retry */ }
      await delay(200);
    }
    throw new Error("chapter harness did not boot");
  })();
  await evaluate(`window.__start()`, false);
  await (async () => {
    const start = Date.now();
    while (Date.now() - start < 240000) {
      const st = await evaluate(`({ done: window.__done === true, error: window.__bootError ?? null })`);
      if (st.error) throw new Error(`in-page failure: ${st.error}`);
      if (st.done) return;
      await delay(500);
    }
    throw new Error("chapter run timed out");
  })();
  const out = await evaluate(`window.__result`);

  check(out.audioBytes === EXPECTED_BYTES, `loader delivered ${out.audioBytes} bytes, want ${EXPECTED_BYTES}`);
  check(out.audioShaMatch === true, "loader bytes hash mismatch");
  check(
    !(out.inspectionIssues ?? []).some((s) => s.startsWith("bounds") || s.startsWith("invalid_wav")),
    `valid master flagged: ${(out.inspectionIssues ?? []).join(",")}`,
  );
  check(out.handleFiles === 0, "handle pins bytes");
  check(out.decodeChannels === 2, `decoded channels ${out.decodeChannels}`);
  check(Math.abs(out.decodeDuration - 180) < 0.05, `decoded duration ${out.decodeDuration}`);
  check(out.decodedRate === out.contextRate, `decoded rate ${out.decodedRate} vs context ${out.contextRate}`);
  if (out.decodedRate === 48000) {
    check(out.decodedLength === FRAMES_180S, `decoded length ${out.decodedLength}`);
    const expected = [-32768 / 32768, -32766 / 32768, -32764 / 32768, -32762 / 32768];
    const ramp = (out.rampHead ?? []).every((v, i) => Math.abs(v - expected[i]) < 1e-3);
    check(ramp, `ramp head mismatch: ${(out.rampHead ?? []).join(",")}`);
  }
  check(Math.abs(out.mediaDuration - 180) < 0.5, `media duration ${out.mediaDuration}`);
  check(Math.abs(out.seek90 - 90) < 0.5, `seek(90) landed at ${out.seek90}`);
  check(out.playing === true && out.playheadAfter1s > 90.3, `playhead stuck at ${out.playheadAfter1s}`);
  check(Math.abs(out.seek179 - 179) < 0.5, `seek(179) landed at ${out.seek179}`);
  check(/identity/i.test(out.tamperRejection ?? ""), `tamper accepted: ${out.tamperRejection}`);
  check(/identity/i.test(out.truncateRejection ?? ""), `truncate accepted: ${out.truncateRejection}`);
  check(
    (out.evilIssues ?? []).some((s) => s.startsWith("invalid_wav")),
    `garbage wav unflagged: ${(out.evilIssues ?? []).join(",")}`,
  );
  check(/invalid_wav/.test(out.evilRejection ?? ""), `garbage wav accepted: ${out.evilRejection}`);
  check(out.contextState === "closed", `audio context not disposed: ${out.contextState}`);
  check((out.errors ?? []).length === 0, `console errors: ${(out.errors ?? []).join("; ").slice(0, 200)}`);
  check(requests.length > 0 && requests.every((r) => r.startsWith("/")), `non-local request: ${requests.join(",")}`);
  const report = {
    tool: "check:chapter",
    fixture: "synthetic 180 s master in temp dir (not a fixture)",
    audio: { bytes: out.audioBytes, sha256: out.audioSha256, frames: FRAMES_180S, sample_rate: 48000, channels: 2 },
    load: { inspectionIssues: out.inspectionIssues, shaMatch: out.audioShaMatch, handleFiles: out.handleFiles },
    decode: {
      contextRate: out.contextRate, decodedRate: out.decodedRate, channels: out.decodeChannels,
      length: out.decodedLength, duration: out.decodeDuration, rampHead: out.rampHead,
    },
    seek: { mediaDuration: out.mediaDuration, seek90: out.seek90, playheadAfter1s: out.playheadAfter1s, seek179: out.seek179 },
    rejection: {
      tamper: out.tamperRejection, truncate: out.truncateRejection,
      evilIssues: out.evilIssues, evil: out.evilRejection,
    },
    disposal: { contextState: out.contextState },
    requests: { count: requests.length, remote: 0 },
    provider_calls: 0,
    generation_cap_note: "Generation keeps its own 32 MiB production cap; this proof synthesizes its chapter master locally and makes no full-chapter production claim.",
    failures,
  };
  await writeFile(evidenceJson, JSON.stringify(report, null, 2));
  if (failures.length) throw new Error(`check:chapter failed:\n- ${failures.join("\n- ")}`);
  console.log("check:chapter pass (34,560,044-byte master loads, decodes, seeks, rejects, disposes)");
} catch (error) {
  console.error(`check:chapter failed: ${error instanceof Error ? error.message : String(error)}`);
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
