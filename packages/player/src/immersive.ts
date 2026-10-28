// Full-viewport theater world for the audience: one canvas filling its
// container, one render scheduler (requestAnimationFrame) that either ticks
// an attached mount or renders the idle house, the box-office-to-seat
// journey on UI time, pointer parallax/hover/picking, resize refit, the
// read-along plaque projected onto the lectern with the growth/refit rule,
// and strict teardown of every listener, frame, observer and GPU resource.
import { createImmersiveSink } from "./shell.js";
import type { FigureSource, HousePick, ImmersiveSink, TitleCard } from "./shell.js";
import { boothSeatCrossfade, rehearsalCurtain } from "./multiplane.js";
import type { CssRect } from "./multiplane.js";
import type { LoadedHouseArt } from "./house-art.js";

/** Shortest height the theater will scale to. Below this the picture holds. */
export const THEATER_MIN_HEIGHT = 700;

/**
 * Drawing size for the audience theater. Width follows the viewport. Height
 * follows it too, and stops at 700px: a shorter window keeps the 700px picture
 * and crops it, and growing past 700px scales with the window again.
 */
export function theaterViewport(width: number, height: number): { width: number; height: number } {
  if (!(width > 0) || !(height > 0) || !Number.isFinite(width) || !Number.isFinite(height)) {
    throw new Error(`theater viewport: ${width}x${height}`);
  }
  const roundedWidth = Math.max(1, Math.round(width));
  const roundedHeight = Math.max(1, Math.round(height));
  return {
    width: roundedWidth,
    height: roundedHeight < THEATER_MIN_HEIGHT ? THEATER_MIN_HEIGHT : roundedHeight,
  };
}

/** Canvas box. A held picture is 700px tall so the bitmap is not scaled down. */
export function theaterCanvasBox(viewportHeight: number, renderHeight: number): { width: string; height: string; bottom: string } {
  if (!(viewportHeight > 0) || !Number.isFinite(viewportHeight)) throw new Error(`theater canvas: ${viewportHeight}`);
  const held = viewportHeight < THEATER_MIN_HEIGHT;
  return {
    width: "100%",
    height: held ? `${renderHeight}px` : "100%",
    bottom: held ? "auto" : "0",
  };
}

export interface TheaterWorldOptions {
  container: HTMLElement;
  /** Read-along plaque element; positioned over the lectern each frame. */
  plaque?: HTMLElement | null;
  reducedMotion?: boolean;
  /** Starting journey: 0 box office, 1 orchestra seat. */
  journey?: 0 | 1;
  /** Retained for callers. The booth and seat change is a one-second fade each way. */
  travelMs?: number;
  title?: TitleCard;
  /** Registered performer art for a loaded performance (none in the lobby). */
  figures?: FigureSource[];
  /** Registered-style house art manifest for the multiplane house. */
  houseArt?: LoadedHouseArt | null;
  onPick?: (pick: HousePick) => void;
  onHover?: (id: string | null) => void;
  onArrive?: (journey: 0 | 1) => void;
  /** Journey progress while travelling (UI time). */
  onJourney?: (journey: number) => void;
  onPlaqueLayout?: (rect: CssRect) => void;
}

export interface TheaterWorld {
  readonly sink: ImmersiveSink;
  /** Mount whose tick() renders each frame; null renders the idle house. */
  attach(mount: { tick(): unknown } | null): void;
  travel(target: 0 | 1): Promise<void>;
  journey(): number;
  setReducedMotion(on: boolean): void;
  setIdleTitle(visible: boolean): void;
  /** Open or close the empty-stage curtain when no performance is mounted. */
  setRehearsalCurtain(open: boolean): void;
  /** Pointer in NDC (+Y up), as pointer events would set it. */
  setPointer(x: number, y: number): void;
  /** Grow the lectern until the plaque content fits; returns the steps. */
  fitPlaque(): number;
  /** Frame intervals (ms) since the last call. */
  takeFrameIntervals(): number[];
  dispose(): void;
}

const MAX_GROWTH = 40;

export async function createTheaterWorld(options: TheaterWorldOptions): Promise<TheaterWorld> {
  const { container } = options;
  const sizeOf = (): { width: number; height: number } =>
    theaterViewport(Math.max(1, container.clientWidth), Math.max(1, container.clientHeight));
  let { width, height } = sizeOf();
  // Never the diagnostic calibration fixture: only authored figures enter.
  const sink = await createImmersiveSink(width, height, {
    calibration: false,
    ...(options.title ? { title: options.title } : {}),
    ...(options.figures ? { figures: options.figures } : {}),
    ...(options.houseArt ? { houseArt: options.houseArt } : {}),
  });
  const canvas = sink.canvas;
  const placeCanvas = (): void => {
    const box = theaterCanvasBox(Math.max(container.clientHeight, 1), height);
    canvas.style.position = "absolute";
    canvas.style.left = "0";
    canvas.style.top = "0";
    canvas.style.right = "0";
    canvas.style.bottom = box.bottom;
    canvas.style.width = box.width;
    canvas.style.height = box.height;
    canvas.style.display = "block";
    canvas.style.touchAction = "none";
  };
  placeCanvas();
  container.prepend(canvas);

  let journey: number = options.journey ?? 0;
  let target: 0 | 1 = options.journey ?? 0;
  let reducedMotion = options.reducedMotion ?? false;
  let pointer: [number, number] = [0, 0];
  let growth = 0;
  let idleTitle = false;
  let curtainOpen = false;
  let curtainFrom = 0;
  let curtainAt = 0;
  let mount: { tick(): unknown } | null = null;
  let arrivals: (() => void)[] = [];
  let lastTime: number | null = null;
  let intervals: number[] = [];
  let lastRect = "";
  let disposed = false;
  let veil = 0;
  let fade: { to: 0 | 1; started: number } | null = null;
  const plaque = options.plaque ?? null;

  const view = () => ({ journey, pointer, reducedMotion, width, height, growthSteps: growth, veil });
  const layoutPlaque = (): void => {
    if (!plaque) return;
    const r = sink.lecternRect();
    // Inner dark panel of the brass-framed lectern face.
    const inset: CssRect = {
      left: r.left + r.width * 0.035,
      top: r.top + r.height * 0.085,
      width: r.width * 0.93,
      height: r.height * 0.83,
    };
    plaque.style.position = "absolute";
    plaque.style.boxSizing = "border-box";
    plaque.style.left = `${inset.left.toFixed(2)}px`;
    plaque.style.top = `${inset.top.toFixed(2)}px`;
    plaque.style.width = `${inset.width.toFixed(2)}px`;
    plaque.style.height = `${inset.height.toFixed(2)}px`;
    plaque.style.visibility = journey === 1 && veil < 0.02 ? "visible" : "hidden";
    const key = `${Math.round(inset.width)}x${Math.round(inset.height)}`;
    if (key !== lastRect) {
      lastRect = key;
      options.onPlaqueLayout?.(inset);
    }
  };
  const settle = (): void => {
    if (journey !== target) return;
    const done = arrivals;
    arrivals = [];
    for (const resolve of done) resolve();
    options.onArrive?.(target);
  };

  let frameId = 0;
  const frame = (now: number): void => {
    if (disposed) return;
    frameId = requestAnimationFrame(frame);
    if (lastTime !== null) {
      const dt = Math.max(0, Math.min(now - lastTime, 50));
      if (intervals.length < 20000) intervals.push(dt);
    }
    if (fade) {
      const phase = boothSeatCrossfade(now - fade.started);
      if (phase.switched && journey !== fade.to) {
        journey = fade.to;
        options.onJourney?.(journey);
      }
      veil = phase.done ? 0 : phase.veil;
      if (phase.done) {
        fade = null;
        settle();
      }
    }
    lastTime = now;
    sink.setView(view());
    const curtain = rehearsalCurtain(Math.max(0, now - curtainAt), curtainFrom, curtainOpen ? 1 : 0);
    if (!curtain.moving) curtainFrom = curtain.ratio;
    if (mount) mount.tick();
    else sink.renderIdle(idleTitle && curtain.titleVisible, curtain.ratio);
    layoutPlaque();
  };

  const toNdc = (event: PointerEvent): [number, number] => {
    const rect = container.getBoundingClientRect();
    return [
      ((event.clientX - rect.left) / rect.width) * 2 - 1,
      1 - ((event.clientY - rect.top) / rect.height) * 2,
    ];
  };
  const localXY = (event: PointerEvent | MouseEvent): [number, number] => {
    const rect = container.getBoundingClientRect();
    return [event.clientX - rect.left, event.clientY - rect.top];
  };
  let hovered: string | null = null;
  const onMove = (event: PointerEvent): void => {
    pointer = toNdc(event);
    const hit = sink.pick(...localXY(event));
    const id = hit?.id ?? null;
    if (id !== hovered) {
      hovered = id;
      sink.setHighlight(id);
      container.style.cursor = id ? "pointer" : "";
      options.onHover?.(id);
    }
  };
  const onLeave = (): void => {
    pointer = [0, 0];
    if (hovered !== null) {
      hovered = null;
      sink.setHighlight(null);
      container.style.cursor = "";
      options.onHover?.(null);
    }
  };
  const onClick = (event: MouseEvent): void => {
    const hit = sink.pick(...localXY(event));
    if (hit) options.onPick?.(hit);
  };
  const onResize = (): void => {
    const next = sizeOf();
    if (next.width === width && next.height === height) {
      placeCanvas();
      return;
    }
    width = next.width;
    height = next.height;
    sink.resize(width, height);
    placeCanvas();
  };
  container.addEventListener("pointermove", onMove);
  container.addEventListener("pointerleave", onLeave);
  container.addEventListener("click", onClick);
  const observer = typeof ResizeObserver === "function" ? new ResizeObserver(onResize) : null;
  observer?.observe(container);
  window.addEventListener("resize", onResize);
  sink.setView(view());
  frameId = requestAnimationFrame(frame);

  const live = (): void => {
    if (disposed) throw new Error("theater world disposed");
  };
  return {
    sink,
    attach(next) {
      live();
      mount = next;
    },
    travel(next) {
      live();
      target = next;
      if (fade && fade.to === next) {
        return new Promise<void>((resolve) => arrivals.push(resolve));
      }
      if (reducedMotion || journey === next) {
        fade = null;
        veil = 0;
        if (journey !== next) {
          journey = next;
          options.onJourney?.(journey);
        }
        settle();
        return Promise.resolve();
      }
      fade = { to: next, started: performance.now() };
      return new Promise<void>((resolve) => arrivals.push(resolve));
    },
    journey: () => journey,
    setReducedMotion(on) {
      reducedMotion = on;
    },
    setIdleTitle(visible) {
      idleTitle = visible;
    },
    setRehearsalCurtain(open) {
      live();
      const now = performance.now();
      const current = rehearsalCurtain(Math.max(0, now - curtainAt), curtainFrom, curtainOpen ? 1 : 0);
      if (curtainOpen === open) return;
      curtainFrom = current.ratio;
      curtainOpen = open;
      curtainAt = now;
    },
    setPointer(x, y) {
      pointer = [x, y];
    },
    fitPlaque() {
      live();
      if (!plaque) return growth;
      growth = 0;
      sink.setView(view());
      layoutPlaque();
      while (plaque.scrollHeight > plaque.clientHeight + 1 && growth < MAX_GROWTH) {
        growth++;
        sink.setView(view());
        layoutPlaque();
      }
      return growth;
    },
    takeFrameIntervals() {
      const out = intervals;
      intervals = [];
      return out;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      cancelAnimationFrame(frameId);
      container.removeEventListener("pointermove", onMove);
      container.removeEventListener("pointerleave", onLeave);
      container.removeEventListener("click", onClick);
      window.removeEventListener("resize", onResize);
      observer?.disconnect();
      const done = arrivals;
      arrivals = [];
      for (const resolve of done) resolve();
      mount = null;
      sink.dispose?.();
      // Release the context now rather than at garbage collection.
      const gl = (canvas.getContext("webgl2") ?? canvas.getContext("webgl")) as WebGLRenderingContext | null;
      gl?.getExtension("WEBGL_lose_context")?.loseContext();
      canvas.remove();
      container.style.cursor = "";
    },
  };
}
