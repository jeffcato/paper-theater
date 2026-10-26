// plaque group (runs under check:player -- integration): pure reflow and
// scale helpers behind the mounted read-along plaque. DOM behavior itself
// is verified in Chrome (check:plaque); these lock the sampler-safe rules.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  charsPerLine,
  clampScale,
  reflowChunks,
} from "../src/index.js";
import type { ReadAlongGroup, ReadAlongWord } from "../src/index.js";

function words(): ReadAlongWord[] {
  const long0 = "hush" + "h".repeat(41);
  const long1 = "now" + "n".repeat(42);
  const long3 = "listen" + "l".repeat(39);
  return [
    { id: "w0", source_span: [0, 45], text: long0, following_text: " ", start_sample: 0, end_sample: 24000 },
    { id: "w1", source_span: [45, 91], text: long1, following_text: ".", start_sample: 24000, end_sample: 48000 },
    { id: "w2", source_span: [91, 92], text: "—", following_text: " ", start_sample: 48000, end_sample: 48000 },
    { id: "w3", source_span: [92, 140], text: long3, following_text: ".", start_sample: 48000, end_sample: 96000 },
  ];
}
function groups(): ReadAlongGroup[] {
  return [
    { id: "g0", word_ids: ["w0"], timing_owner_word_id: "w0", start_sample: 0, end_sample: 24000 },
    { id: "g1", word_ids: ["w1"], timing_owner_word_id: "w1", start_sample: 24000, end_sample: 48000 },
    { id: "g2", word_ids: ["w2", "w3"], timing_owner_word_id: "w3", start_sample: 48000, end_sample: 96000 },
  ];
}

describe("plaque: scale and reflow rules", () => {
  it("clamps viewer text scaling to 100%-200%", () => {
    assert.equal(clampScale(1), 1);
    assert.equal(clampScale(2), 2);
    assert.equal(clampScale(0.5), 1);
    assert.equal(clampScale(3), 2);
    assert.equal(clampScale(Number.NaN), 1);
  });

  it("derives a positive integer line budget from the measured inset", () => {
    assert.ok(charsPerLine(520, 22) >= 20);
    assert.ok(charsPerLine(520, 44) < charsPerLine(520, 22));
    assert.equal(charsPerLine(0, 22), 10);
    assert.equal(charsPerLine(-5, 22), 10);
  });

  it("reflows chunk boundaries without changing word/group samples", () => {
    const narrow = reflowChunks(words(), groups(), 20);
    const wide = reflowChunks(words(), groups(), 200);
    assert.ok(narrow.length >= wide.length);
    const collect = (chunks: ReturnType<typeof reflowChunks>) =>
      chunks.flatMap((c) => c.group_ids);
    assert.deepEqual(collect(narrow).sort(), ["g0", "g1", "g2"]);
    assert.deepEqual(collect(wide).sort(), ["g0", "g1", "g2"]);
    for (const chunks of [narrow, wide]) {
      for (const chunk of chunks) {
        const group = groups().find((g) => g.id === chunk.group_ids[0])!;
        assert.equal(chunk.start_sample, group.start_sample);
        assert.equal(chunk.end_sample, chunks[chunks.indexOf(chunk)]!.end_sample);
      }
    }
    // Word sample times are untouched by layout.
    assert.deepEqual(words().map((w) => [w.start_sample, w.end_sample]), [
      [0, 24000], [24000, 48000], [48000, 48000], [48000, 96000],
    ]);
  });

  it("keeps the active group visible across a reflow", () => {
    const before = reflowChunks(words(), groups(), 20);
    const after = reflowChunks(words(), groups(), 200);
    const activeAt = (chunks: ReturnType<typeof reflowChunks>, sample: number) =>
      chunks.find((c) => sample >= c.start_sample && sample < c.end_sample)?.group_ids ?? [];
    // Sample 30000 sits in g1 under every budget.
    assert.ok(activeAt(before, 30000).includes("g1"));
    assert.ok(activeAt(after, 30000).includes("g1"));
  });
});
