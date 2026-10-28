#!/usr/bin/env node
// check:browser — the built audience app in headless Chrome at 960x540,
// 1280x720 and 1920x1080 (DPR 1) with an empty accepted catalog (the honest
// current state). Real mouse input walks the 2.5D box office: inspect a
// ticket, Take Your Seat, the aisle dolly, the closed-curtain seat, and the
// banner's Box Office medallion back; the keyboard path drives the same
// flow through the visually hidden control mirror. Asserts a single
// full-viewport canvas with no visible DOM overlay, no diagnostic/fixture
// requests and no page errors. Evidence: docs/implementation/audience/.
import { spawn, spawnSync } from "node:child_process";
import { createServer } from "node:http";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(appDir, "..", "..");
const outDir = join(root, "docs", "implementation", "audience");
const CHROME = process.env.PT_CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const VIEWPORTS = [[960, 540], [1280, 720], [1920, 1080]];
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2", ".json": "application/json" };

for (const [dir, args] of [[join(root, "packages", "player"), ["run", "build"]], [appDir, ["run", "build"]]]) {
  const r = spawnSync("npm", args, { cwd: dir, stdio: "inherit" });
  if (r.status !== 0) process.exit(r.status ?? 1);
}
const plan = await import(pathToFileURL(join(root, "packages", "player", "dist", "src", "multiplane.js")).href);
await mkdir(outDir, { recursive: true });

const failures = [];
const check = (cond, message) => {
  if (!cond) failures.push(message);
};
const requests = [];
const temp = await mkdtemp(join(tmpdir(), "pt-audience-"));
let chrome;
let server;
let ws;
const evidence = { harness: "chrome-audience", catalog: { packages: [] }, viewports: [] };
try {
  server = createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", "http://127.0.0.1");
    requests.push(url.pathname);
    if (url.pathname === "/tickets/catalog.json") {
      const body = Buffer.from(JSON.stringify(evidence.catalog));
      res.writeHead(200, { "content-type": "application/json", "content-length": body.length });
      return res.end(body);
    }
    const name = url.pathname === "/" ? "/index.html" : url.pathname;
    if (name.includes("..")) {
      res.writeHead(404);
      return res.end();
    }
    try {
      const data = await readFile(join(appDir, "dist", name));
      res.writeHead(200, { "content-type": TYPES[extname(name)] ?? "application/octet-stream", "content-length": data.length });
      res.end(data);
    } catch {
      res.writeHead(404);
      res.end();
    }
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const port = server.address().port;
  chrome = spawn(CHROME, [
    "--headless=new", "--no-first-run", "--no-default-browser-check", "--disable-background-networking",
    "--remote-debugging-port=0", "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
    `--user-data-dir=${join(temp, "chrome")}`, "about:blank",
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
  evidence.browser = (await (await fetch(`http://127.0.0.1:${debugPort}/json/version`)).json()).Browser;
  for (const [W, H] of VIEWPORTS) {
    const page = await (await fetch(`http://127.0.0.1:${debugPort}/json/new?about:blank`, { method: "PUT" })).json();
    try { ws?.close(); } catch { /* ignore */ }
    ws = new WebSocket(page.webSocketDebuggerUrl);
    await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
    let seq = 0;
    const pending = new Map();
    const errors = [];
    ws.onmessage = ({ data }) => {
      const m = JSON.parse(data);
      if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails?.exception?.description ?? "exception");
      if (m.method === "Runtime.consoleAPICalled" && m.params.type === "error") errors.push(JSON.stringify(m.params.args?.[0]?.value ?? "console.error"));
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
      if (result.exceptionDetails) throw new Error(`page exception: ${JSON.stringify(result.exceptionDetails).slice(0, 400)}`);
      return result.result.value;
    };
    const until = async (expr, ms, label) => {
      const start = Date.now();
      while (Date.now() - start < ms) {
        if (await evaluate(expr)) return;
        await delay(50);
      }
      throw new Error(`timeout: ${label} (${W}x${H}) active=${await evaluate("document.activeElement.tagName + ':' + document.activeElement.textContent.slice(0, 80)")}`);
    };
    const shot = async (phase) => {
      const s = await call("Page.captureScreenshot", { format: "png" });
      await writeFile(join(outDir, `audience-${phase}-${W}x${H}.png`), Buffer.from(s.data, "base64"));
    };
    const mouse = (type, [x, y]) => call("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" ? "none" : "left", clickCount: type === "mouseMoved" ? 0 : 1 });
    const click = async (pt) => {
      await mouse("mouseMoved", pt);
      await delay(60);
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
        if (await evaluate(`getComputedStyle(document.querySelector("canvas").parentElement).cursor === "pointer"`)) break;
        if (Date.now() - start > 20000) throw new Error(`timeout: nothing pickable under ${pt} (${W}x${H})`);
      }
      await mouse("mousePressed", pt);
      await mouse("mouseReleased", pt);
    };
    const key = async (k) => {
      const code = k === "Tab" ? 9 : 13;
      const text = k === "Enter" ? "\r" : "";
      await call("Input.dispatchKeyEvent", { type: "keyDown", key: k, code: k, windowsVirtualKeyCode: code, ...(text ? { text } : {}) });
      await call("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k, windowsVirtualKeyCode: code });
    };
    const at = (journey, rect) => {
      const cam = plan.worldCamera({ journey, pointer: [0, 0], reducedMotion: false, width: W, height: H, growthSteps: 0 });
      return plan.projectWorld(cam, [(rect.x[0] + rect.x[1]) / 2, (rect.y[0] + rect.y[1]) / 2, rect.z], { width: W, height: H });
    };
    // The playback cards hang on the proscenium banner: project them with the seat camera.
    const railAt = (id) => {
      const cam = plan.worldCamera({ journey: 1, pointer: [0, 0], reducedMotion: false, width: W, height: H, growthSteps: 0 });
      const sign = plan.bannerScreen(cam, { width: W, height: H }).find((item) => item.id === id);
      return [sign.rect.left + sign.rect.width / 2, sign.rect.top + sign.rect.height / 2];
    };
    const surface = () => evaluate(`(() => {
      const canvas = document.querySelector("canvas");
      const r = canvas?.getBoundingClientRect();
      const visible = [...document.body.querySelectorAll("*")].filter((el) => {
        if (el === canvas || el.matches(".world, #root, script")) return false;
        const b = el.getBoundingClientRect();
        const style = getComputedStyle(el);
        return b.width * b.height > 1 && style.visibility !== "hidden" && style.display !== "none" && style.clipPath !== "inset(50%)";
      }).map((el) => el.tagName + "." + el.className);
      return { canvases: document.querySelectorAll("canvas").length, rect: r && [r.left, r.top, r.width, r.height],
        buffer: canvas && [canvas.width, canvas.height], visible, journey: document.documentElement.dataset.journey ?? null,
        hash: location.hash, headings: [...document.querySelectorAll("h1, h2")].map((h) => h.textContent) };
    })()`);
    await call("Page.enable");
    await call("Runtime.enable");
    await call("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
    await call("Page.navigate", { url: `http://127.0.0.1:${port}/#/` });
    await until(`document.documentElement.dataset.journey === "0" && document.querySelector("canvas") !== null`, 60000, "booth");
    await delay(400);
    const v = { width: W, height: H, booth: await surface() };
    check(v.booth.canvases === 1, `${W}x${H}: ${v.booth.canvases} canvases`);
    check(JSON.stringify(v.booth.rect) === JSON.stringify([0, 0, W, H]), `${W}x${H}: canvas rect ${v.booth.rect}`);
    check(JSON.stringify(v.booth.buffer) === JSON.stringify([W, H]), `${W}x${H}: drawing buffer ${v.booth.buffer}`);
    check(v.booth.visible.length === 0, `${W}x${H}: visible DOM overlay ${v.booth.visible.join(", ")}`);
    await shot("booth");
    await click(at(0, plan.TICKET_SLOTS[0]));
    await until(`[...document.querySelectorAll("h2")].some((h) => h.textContent === "The Wonderful Wizard of Oz")`, 5000, "inspect");
    await delay(200);
    await shot("inspect");
    await clickWhenPointer(at(0, plan.INSPECT_SEAT));
    await until(`Number(document.documentElement.dataset.journey) >= 0.5`, 10000, "aisle");
    await shot("aisle");
    await until(`document.documentElement.dataset.journey === "1"`, 15000, "seat");
    await delay(300);
    v.seat = await surface();
    check(v.seat.hash === "#/theater/oz-chapter-1", `${W}x${H}: seat hash ${v.seat.hash}`);
    check(v.seat.headings.includes("The Wonderful Wizard of Oz — stalls"), `${W}x${H}: seat heading ${v.seat.headings}`);
    check(v.seat.visible.length === 0, `${W}x${H}: visible DOM overlay at seat ${v.seat.visible.join(", ")}`);
    await shot("seat");
    await click(railAt("return"));
    await until(`document.documentElement.dataset.journey === "0"`, 15000, "return");
    v.returned = (await surface()).hash;
    check(v.returned === "#/", `${W}x${H}: return hash ${v.returned}`);
    // Keyboard: Tab to the first ticket proxy, Enter inspects, Tab+Enter seats.
    await key("Tab");
    v.firstTab = await evaluate(`document.activeElement.textContent`);
    await key("Enter");
    await until(`document.activeElement !== document.body && [...document.querySelectorAll("h2")].some((h) => h.textContent === "The Wonderful Wizard of Oz")`, 5000, "keyboard inspect");
    v.keyboardFocus = await evaluate(`document.activeElement.textContent`);
    await evaluate(`[...document.querySelectorAll("button")].find((b) => b.textContent === "Take your seat").focus()`);
    await key("Enter");
    await until(`document.documentElement.dataset.journey === "1"`, 15000, "keyboard seat");
    v.keyboardSeat = (await surface()).hash;
    check(v.keyboardSeat === "#/theater/oz-chapter-1", `${W}x${H}: keyboard seat ${v.keyboardSeat}`);
    // Deep link straight to a seat starts seated (no replayed walk).
    await call("Page.navigate", { url: `http://127.0.0.1:${port}/#/theater/wells-book-one-chapter-1` });
    await call("Page.reload");
    await until(`document.documentElement.dataset.journey === "1" && document.querySelector("canvas") !== null`, 60000, "deep link");
    await delay(400);
    await shot("seat-deeplink");
    v.errors = errors;
    check(errors.length === 0, `${W}x${H}: page errors ${errors.join("; ")}`);
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
evidence.requests = [...new Set(requests)].sort();
check(!evidence.requests.some((p) => /fixture|\/fx\/|diagnostic|manifest/.test(p)), `diagnostic/fixture requests: ${evidence.requests}`);
evidence.failures = failures;
await writeFile(join(outDir, "audience.json"), JSON.stringify(evidence, null, 2) + "\n");
if (failures.length) {
  console.error(`check:browser FAILED\n- ${failures.join("\n- ")}`);
  process.exit(1);
}
console.log("check:browser passed");
