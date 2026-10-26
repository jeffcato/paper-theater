// Mounted read-along plaque: the active two-to-three-line chunk in the
// below-stage plaque, with measured highlight groups driven by the media
// sample. Pure w.r.t. sample: seeks and rewinds resolve through the same
// path as playback. Only the active chunk enters the DOM/accessibility
// tree; reflow regroups chunk boundaries without changing word/group
// samples; viewer text scaling stays within 100%-200%.
import { FONT } from "./profile.js";
import { groupWords, layoutChunks, layoutMeasured } from "./readalong.js";
import type { Chunk, MeasureFn } from "./readalong.js";
import type { ReadAlongGroup, ReadAlongWord } from "./types.js";

/** Viewer text scaling, clamped to the supported 100%-200% range. */
export function clampScale(scale: number): number {
  if (!Number.isFinite(scale)) return FONT.min_scale;
  return Math.max(FONT.min_scale, Math.min(FONT.max_scale, scale));
}

/**
 * Line budget from the measured inset width and current font size.
 * Noto Serif averages ~0.52em per character; at least 10 characters so a
 * narrow plaque still shows a whole group.
 */
export function charsPerLine(insetWidthPx: number, fontPx: number): number {
  if (!(insetWidthPx > 0) || !(fontPx > 0)) return 10;
  return Math.max(10, Math.floor(insetWidthPx / (fontPx * 0.52)));
}

/** Regroup words into chunks at a new line budget. Group membership and
 * every word/group sample are preserved; only chunk boundaries move. */
export function reflowChunks(
  words: ReadAlongWord[],
  groups: ReadAlongGroup[],
  maxCharsPerLine: number,
  maxLines = 3,
): Chunk[] {
  return layoutChunks(words, groups, maxCharsPerLine, maxLines);
}

export interface PlaqueOptions {
  /** Base font size in CSS px (default 22, the profile size). */
  basePx?: number;
  /** Viewer text scale 1..2 (default 1). */
  scale?: number;
  /**
   * Width measurer for certifying line fits. Omitted: canvas 2D measurement
   * against the rendered font string. Explicit null is impossible — pass a
   * stub only in non-browser checks, which then certify nothing (fontReady
   * resolves false there).
   */
  measure?: MeasureFn;
}

/**
 * DOM plaque bound to a read-along word/group track. Call setSample on every
 * transport tick; the view diffs by chunk identity and highlight identity so
 * seeks, rewinds and steady playback share one path. Construction waits for
 * the profile font before first layout; text renders in the fallback serif
 * only if the font fails to load.
 */
export class PlaqueView {
  private chunks: Chunk[];
  private currentChunkId: string | null = null;
  private currentGroupId: string | null = null;
  private lastSample = 0;
  private enabled = true;
  private scale: number;
  private readonly basePx: number;
  private readonly chunkEl: HTMLElement;
  private readonly canvasMeasure: MeasureFn | null;
  private readonly injectedMeasure: MeasureFn | null;
  private observer: ResizeObserver | null = null;
  readonly ready: Promise<void>;
  /**
   * True once the bundled Noto Serif face is loaded and covers the track
   * text. False means every line fit below is provisional (fallback serif
   * or stub metrics) and must not certify layout. Injected stub measurers
   * always resolve false: non-browser checks certify nothing.
   */
  readonly fontReady: Promise<boolean>;

  constructor(
    private readonly container: HTMLElement,
    private readonly words: ReadAlongWord[],
    private readonly groups: ReadAlongGroup[],
    options: PlaqueOptions = {},
  ) {
    groupWords(words, groups);
    this.basePx = options.basePx ?? 22;
    this.scale = clampScale(options.scale ?? 1);
    this.injectedMeasure = options.measure ?? null;
    this.canvasMeasure = options.measure ? null : this.defaultMeasure();
    this.container.setAttribute("aria-label", "Read along");
    this.chunkEl = container.ownerDocument.createElement("div");
    this.chunkEl.className = "pl-chunk";
    this.container.appendChild(this.chunkEl);
    // Provisional pre-font layout; ready reflows with real metrics.
    this.chunks = reflowChunks(words, groups, this.budget());
    this.applyScale();
    this.fontReady = this.awaitFont();
    this.ready = this.fontReady.then(() => {
      this.reflow();
    });
    if (typeof ResizeObserver !== "undefined") {
      this.observer = new ResizeObserver(() => this.reflow());
      this.observer.observe(this.container);
    }
  }

  private fontPx(): number {
    return this.basePx * this.scale;
  }

  private budget(): number {
    const inset = this.container.clientWidth;
    return charsPerLine(inset > 0 ? inset : 520, this.fontPx());
  }

  private fontCss(): string {
    return `${this.fontPx()}px "Noto Serif", Georgia, serif`;
  }

  private defaultMeasure(): MeasureFn | null {
    try {
      const doc = this.container.ownerDocument;
      const canvas = doc.createElement("canvas");
      const ctx = (canvas as HTMLCanvasElement).getContext("2d");
      if (!ctx) return null;
      return (text: string): number => {
        ctx.font = this.fontCss();
        return ctx.measureText(text).width;
      };
    } catch {
      return null;
    }
  }

  /**
   * Wait for the bundled face, then report whether it truly covers the
   * track. A failed load or missing coverage resolves false: callers must
   * fail missing font readiness rather than certify fallback metrics.
   */
  private async awaitFont(): Promise<boolean> {
    try {
      if (this.injectedMeasure) return false;
      const doc = this.container.ownerDocument;
      const fonts = (doc as Document & { fonts?: FontFaceSet }).fonts;
      if (!fonts) return false;
      const probe = this.words
        .slice(0, 32)
        .map((w) => w.text + w.following_text)
        .join("");
      await fonts.load(this.fontCss(), probe);
      return fonts.check(this.fontCss(), probe);
    } catch {
      return false;
    }
  }

  private applyScale(): void {
    this.chunkEl.style.fontFamily = '"Noto Serif", Georgia, serif';
    this.chunkEl.style.fontSize = `${this.fontPx()}px`;
    this.chunkEl.style.lineHeight = String(FONT.line_height);
  }

  /** Active measurer: injected stub, else canvas; null while provisional. */
  private measurer(): MeasureFn | null {
    return this.injectedMeasure ?? this.canvasMeasure;
  }

  /**
   * Per-group rendered overhead beyond the text itself (host padding /
   * borders on .pl-group), read off one probe span wearing the same
   * classes inside the chunk. Zero when unreadable: packing then matches
   * the text exactly, as the unit checks assume.
   */
  private groupOverheadPx(base: MeasureFn): number {
    try {
      const doc = this.container.ownerDocument;
      const probe = doc.createElement("span") as HTMLElement;
      probe.className = "pl-group";
      probe.textContent = "x";
      probe.style.position = "absolute";
      probe.style.visibility = "hidden";
      this.chunkEl.appendChild(probe);
      const rendered = (probe as HTMLElement & { offsetWidth?: unknown }).offsetWidth;
      probe.remove();
      const over = (typeof rendered === "number" ? rendered : Number.NaN) - base("x");
      return over > 0 && Number.isFinite(over) ? over : 0;
    } catch {
      return 0;
    }
  }

  /** Inset content width in CSS px for certified line fits (padding out). */
  private insetWidth(): number {
    const total = this.container.clientWidth;
    let pad = 0;
    try {
      const view = this.container.ownerDocument.defaultView;
      if (view) {
        const cs = view.getComputedStyle(this.container);
        const parsed = parseFloat(cs.paddingLeft) + parseFloat(cs.paddingRight);
        if (Number.isFinite(parsed)) pad = parsed;
      }
    } catch {
      // No view (non-browser check): content box is the whole box.
    }
    const inset = total - pad;
    return inset > 0 ? inset : 520;
  }

  /** Viewer text scaling (100%-200%); reflows around the active group. */
  setScale(scale: number): void {
    this.scale = clampScale(scale);
    this.applyScale();
    this.reflow();
  }

  /** Show or hide the plaque. Hidden plaques keep no text in the tree. */
  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    this.container.style.display = enabled ? "" : "none";
    if (!enabled) {
      this.chunkEl.replaceChildren();
      this.currentChunkId = null;
      this.currentGroupId = null;
    } else {
      this.reflow();
      this.setSample(this.lastSample);
    }
  }

  get isEnabled(): boolean {
    return this.enabled;
  }

  /**
   * Re-measure the inset and regroup chunks; word/group samples untouched.
   * With a measurer this certifies line fits via layoutMeasured; without
   * one it keeps the provisional character-count layout (fontReady stays
   * false, so callers must not treat it as certified).
   */
  reflow(): void {
    if (!this.enabled) return;
    const activeGroup = this.currentGroupId;
    const measure = this.measurer();
    if (measure) {
      // layoutMeasured calls once per group text: fold the rendered
      // per-group overhead in so CSS wraps exactly the packed lines.
      const overhead = this.groupOverheadPx(measure);
      const fitted: MeasureFn = (text) => measure(text) + overhead;
      this.chunks = layoutMeasured(this.words, this.groups, fitted, this.insetWidth());
    } else {
      this.chunks = reflowChunks(this.words, this.groups, this.budget());
    }
    this.currentChunkId = null;
    this.currentGroupId = null;
    if (activeGroup) {
      const chunk = this.chunks.find((c) => c.group_ids.includes(activeGroup));
      if (chunk) this.renderChunk(chunk, activeGroup);
    }
  }

  /** Advance the plaque to a media sample (integer, 48 kHz domain). */
  setSample(sample: number): string | null {
    if (!this.enabled || !Number.isSafeInteger(sample) || sample < 0) return null;
    this.lastSample = sample;
    let low = 0;
    let high = this.chunks.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      if (this.chunks[mid]!.start_sample <= sample) low = mid + 1;
      else high = mid;
    }
    const chunk = this.chunks[low - 1];
    if (!chunk || sample >= chunk.end_sample) {
      // Silence: no highlight, keep the last chunk visible per profile.
      this.highlight(null);
      return null;
    }
    let gLow = 0;
    let gHigh = this.groups.length;
    while (gLow < gHigh) {
      const mid = (gLow + gHigh) >>> 1;
      if (this.groups[mid]!.start_sample <= sample) gLow = mid + 1;
      else gHigh = mid;
    }
    const group = this.groups[gLow - 1];
    const groupId = group && sample < group.end_sample ? group.id : null;
    this.renderChunk(chunk, groupId);
    return groupId;
  }

  private chunkKey(chunk: Chunk): string {
    return chunk.group_ids.join("+");
  }

  private renderChunk(chunk: Chunk, groupId: string | null): void {
    const key = this.chunkKey(chunk);
    if (key !== this.currentChunkId) {
      const doc = this.container.ownerDocument;
      const byId = new Map(this.words.map((w) => [w.id, w]));
      this.chunkEl.replaceChildren();
      for (const gid of chunk.group_ids) {
        const span = doc.createElement("span");
        span.className = "pl-group";
        span.dataset.groupId = gid;
        const group = this.groups.find((g) => g.id === gid)!;
        span.textContent = group.word_ids
          .map((id) => {
            const w = byId.get(id)!;
            return w.text + w.following_text;
          })
          .join("");
        this.chunkEl.appendChild(span);
      }
      this.currentChunkId = key;
      this.currentGroupId = null;
    }
    this.highlight(groupId);
  }

  private highlight(groupId: string | null): void {
    if (groupId === this.currentGroupId) return;
    this.currentGroupId = groupId;
    for (const child of this.chunkEl.children) {
      const el = child as HTMLElement;
      const active = el.dataset.groupId === groupId;
      el.classList.toggle("pl-active", active);
      if (active) el.setAttribute("aria-current", "true");
      else el.removeAttribute("aria-current");
    }
  }

  dispose(): void {
    this.observer?.disconnect();
    this.observer = null;
    this.chunkEl.replaceChildren();
    this.chunkEl.remove();
  }
}
