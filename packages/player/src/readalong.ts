// Toggleable two-to-three-line exact read-along in the projected plaque.
// Fonts load before layout; measured groups stay atomic and the entire active
// chunk is visible. No future chunks enter the DOM/accessibility tree. Reflow
// around the active group at resize/text-size changes without changing
// samples. Preserves point-following-same-boundary-v1 grouping.
import type { ReadAlongGroup, ReadAlongWord } from "./types.js";
import { FONT } from "./profile.js";

export interface Chunk {
  group_ids: string[];
  word_ids: string[];
  text: string;
  start_sample: number;
  end_sample: number;
}

export function groupWords(
  words: ReadAlongWord[],
  groups: ReadAlongGroup[],
): void {
  // Validate point-following-same-boundary-v1: consecutive point words
  // (start==end) may join the next positive interval sharing its start.
  const byId = new Map(words.map((w) => [w.id, w]));
  const flattened = groups.flatMap((g) => g.word_ids);
  if (flattened.length !== words.length) {
    throw new Error("Each source word must occur exactly once, in order");
  }
  for (let i = 0; i < words.length; i++) {
    if (words[i]!.id !== flattened[i]) throw new Error("Word order mismatch");
  }
  for (const group of groups) {
    const members = group.word_ids.map((id) => {
      const w = byId.get(id);
      if (!w) throw new Error(`Unknown word ${id}`);
      return w;
    });
    const owner = members.at(-1);
    if (!owner || owner.end_sample <= owner.start_sample) {
      throw new Error(`Group ${group.id}: timing owner must have a positive interval`);
    }
    for (const m of members.slice(0, -1)) {
      // Point words only: zero-duration at the owner's start.
      if (!(m.start_sample === m.end_sample && m.start_sample === owner.start_sample)) {
        throw new Error(`Group ${group.id}: invented timing for ${m.id}`);
      }
    }
    if (group.timing_owner_word_id !== owner.id) {
      throw new Error(`Group ${group.id}: wrong timing owner`);
    }
    if (group.start_sample !== owner.start_sample || group.end_sample !== owner.end_sample) {
      throw new Error(`Group ${group.id}: must preserve measured following-word interval`);
    }
  }
}

/**
 * Lay out contiguous word ranges into chunks of at most three lines using
 * measured font metrics. Line breaking is approximate here (character count
 * proxy for the measured inset); the browser path reflows with real metrics
 * around the current group without changing samples. Groups never split.
 */
export function layoutChunks(
  words: ReadAlongWord[],
  groups: ReadAlongGroup[],
  maxCharsPerLine = 42,
  maxLines = 3,
): Chunk[] {
  groupWords(words, groups);
  const byId = new Map(words.map((w) => [w.id, w]));
  const chunks: Chunk[] = [];
  let current: Chunk = { group_ids: [], word_ids: [], text: "", start_sample: 0, end_sample: 0 };
  let lines = 1;
  let lineLen = 0;
  const flush = (): void => {
    if (current.group_ids.length > 0) chunks.push(current);
    current = { group_ids: [], word_ids: [], text: "", start_sample: 0, end_sample: 0 };
    lines = 1;
    lineLen = 0;
  };
  for (const group of groups) {
    const text = group.word_ids
      .map((id) => {
        const w = byId.get(id)!;
        return w.text + w.following_text;
      })
      .join("");
    const words_len = text.length;
    // Would this group overflow the 3-line budget?
    if (lineLen + words_len > lines * maxCharsPerLine) {
      if (lines < maxLines) {
        lines++;
        lineLen = (lines - 1) * maxCharsPerLine;
      } else {
        flush();
      }
    }
    if (current.group_ids.length === 0) current.start_sample = group.start_sample;
    current.group_ids.push(group.id);
    current.word_ids.push(...group.word_ids);
    current.text += text;
    current.end_sample = group.end_sample;
    lineLen += words_len;
  }
  flush();
  return chunks;
}

export class ReadAlongModel {
  readonly chunks: Chunk[];
  constructor(
    readonly words: ReadAlongWord[],
    readonly groups: ReadAlongGroup[],
    readonly sampleRate: number,
  ) {
    if (!Number.isSafeInteger(sampleRate) || sampleRate <= 0) {
      throw new Error("Invalid sample rate");
    }
    groupWords(words, groups);
    this.chunks = layoutChunks(words, groups);
  }
  /** Active chunk at a sample (half-open [start, end)). */
  chunkAt(sample: number): Chunk | undefined {
    let low = 0;
    let high = this.chunks.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      if (this.chunks[mid]!.start_sample <= sample) low = mid + 1;
      else high = mid;
    }
    const chunk = this.chunks[low - 1];
    return chunk && sample < chunk.end_sample ? chunk : undefined;
  }
  /** Active word/group highlight at a sample; silence has no highlight. */
  groupAt(sample: number): ReadAlongGroup | undefined {
    let low = 0;
    let high = this.groups.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      if (this.groups[mid]!.start_sample <= sample) low = mid + 1;
      else high = mid;
    }
    const group = this.groups[low - 1];
    return group && sample < group.end_sample ? group : undefined;
  }
  /** DOM/a11y rule: only the active chunk is rendered; future chunks stay out. */
  visibleTextAt(sample: number, enabled: boolean): string {
    if (!enabled) return "";
    return this.chunkAt(sample)?.text ?? "";
  }
  fontCss(scale = 1): string {
    const clamped = Math.max(FONT.min_scale, Math.min(FONT.max_scale, scale));
    const px = FONT.size_css_px * clamped;
    return `${px}px "${FONT.family}", serif`;
  }
  lineHeightCss(scale = 1): string {
    const clamped = Math.max(FONT.min_scale, Math.min(FONT.max_scale, scale));
    return `${FONT.size_css_px * clamped * FONT.line_height}px`;
  }
}
