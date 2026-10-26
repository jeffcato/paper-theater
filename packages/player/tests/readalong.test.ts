// readalong group: point-following-same-boundary-v1, atomic groups, active
// chunk only in the visible tree, reflow without changing samples.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { layoutMeasured, ReadAlongModel } from "../src/index.js";
import type { ReadAlongGroup, ReadAlongWord } from "../src/index.js";

function words(): ReadAlongWord[] {
  // Long timed words force a chunk boundary; point word "—" shares the
  // following word's start and joins its group only.
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

describe("readalong: measured grouping and chunking", () => {
  it("authorizes a point word sharing the following word's start", () => {
    const model = new ReadAlongModel(words(), groups(), 48000);
    assert.equal(model.groupAt(0)?.id, "g0");
    assert.equal(model.groupAt(47999)?.id, "g1");
    assert.equal(model.groupAt(48000)?.id, "g2");
  });

  it("rejects invented durations for point words", () => {
    const bad: ReadAlongGroup[] = [
      { id: "g0", word_ids: ["w0"], timing_owner_word_id: "w0", start_sample: 0, end_sample: 24000 },
      { id: "g1", word_ids: ["w1"], timing_owner_word_id: "w1", start_sample: 24000, end_sample: 48000 },
      { id: "g2", word_ids: ["w2", "w3"], timing_owner_word_id: "w3", start_sample: 47000, end_sample: 96000 },
    ];
    assert.throws(() => new ReadAlongModel(words(), bad, 48000), /timing|interval|invented/i);
  });

  it("keeps groups atomic and exposes only the active chunk", () => {
    const model = new ReadAlongModel(words(), groups(), 48000);
    assert.ok(model.chunks.length >= 2, "fixture must span at least two chunks");
    for (const chunk of model.chunks) {
      for (const gid of chunk.group_ids) {
        const group = groups().find((g) => g.id === gid)!;
        assert.deepEqual(
          group.word_ids.every((w) => chunk.word_ids.includes(w)),
          true,
        );
      }
    }
    assert.ok(model.visibleTextAt(0, true).includes("hush"));
    assert.ok(!model.visibleTextAt(0, true).includes("listen"));
    assert.equal(model.visibleTextAt(0, false), "");
  });

  it("silence has no highlight and reflow never changes samples", () => {
    const model = new ReadAlongModel(words(), groups(), 48000);
    assert.equal(model.groupAt(96000), undefined);
    const before = model.chunks.map((c) => [c.start_sample, c.end_sample]);
    const relaid = new ReadAlongModel(words(), groups(), 48000);
    assert.deepEqual(relaid.chunks.map((c) => [c.start_sample, c.end_sample]), before);
  });
});

describe("readalong: measured line layout", () => {
  const px = (text: string): number => text.length * 10;

  function sixTrack(): { words: ReadAlongWord[]; groups: ReadAlongGroup[] } {
    const letters = ["a", "b", "c", "d", "e", "f"];
    const words: ReadAlongWord[] = letters.map((letter, i) => ({
      id: `w${i}`,
      source_span: [i * 2, i * 2 + 1] as [number, number],
      text: letter,
      following_text: " ",
      start_sample: i * 1000,
      end_sample: (i + 1) * 1000,
    }));
    const groups: ReadAlongGroup[] = letters.map((_, i) => ({
      id: `g${i}`,
      word_ids: [`w${i}`],
      timing_owner_word_id: `w${i}`,
      start_sample: i * 1000,
      end_sample: (i + 1) * 1000,
    }));
    return { words, groups };
  }

  it("packs measured groups first-fit without splitting them", () => {
    const { words: w, groups: g } = sixTrack();
    // 20 px per group, 50 px lines: two groups per line, three lines.
    const chunks = layoutMeasured(w, g, px, 50, 3);
    assert.equal(chunks.length, 1);
    assert.deepEqual(chunks[0]!.group_ids, ["g0", "g1", "g2", "g3", "g4", "g5"]);
    assert.equal(chunks[0]!.start_sample, 0);
    assert.equal(chunks[0]!.end_sample, 6000);
    assert.ok(chunks[0]!.text.includes("a ") && chunks[0]!.text.includes("f "));
  });

  it("splits chunks past the line budget, keeping every group exactly once", () => {
    const { words: w, groups: g } = sixTrack();
    // 20 px per group, 30 px lines: one group per line, two chunks of 2.
    const chunks = layoutMeasured(w, g, px, 30, 2);
    assert.equal(chunks.length, 3);
    assert.deepEqual(chunks.flatMap((c) => c.group_ids), ["g0", "g1", "g2", "g3", "g4", "g5"]);
    for (const chunk of chunks) {
      const first = g.find((x) => x.id === chunk.group_ids[0])!;
      const last = g.find((x) => x.id === chunk.group_ids.at(-1))!;
      assert.equal(chunk.start_sample, first.start_sample);
      assert.equal(chunk.end_sample, last.end_sample);
    }
    // Every measured interval resolves to the chunk holding its group.
    for (const group of g) {
      const hit = chunks.find((c) => group.start_sample >= c.start_sample && group.start_sample < c.end_sample);
      assert.ok(hit?.group_ids.includes(group.id), `${group.id} must stay findable`);
    }
  });

  it("parks an oversize group alone instead of splitting or throwing", () => {
    const { words: w, groups: g } = sixTrack();
    // g0 overflows its line alone; the rest pack normally after it.
    const chunks = layoutMeasured(w, g, (t) => (t === "a " ? 500 : 20), 50, 3);
    assert.equal(chunks.length, 2);
    assert.deepEqual(chunks.flatMap((c) => c.group_ids), ["g0", "g1", "g2", "g3", "g4", "g5"]);
    assert.equal(chunks[0]!.group_ids[0], "g0");
  });

  it("rejects invalid tracks and bad budgets", () => {
    const { words: w, groups: g } = sixTrack();
    const bad = g.map((x) => ({ ...x }));
    bad[0]!.end_sample = -1;
    assert.throws(() => layoutMeasured(w, bad, px, 50, 3), /interval|timing|owner/i);
    assert.throws(() => layoutMeasured(w, g, px, 0, 3), /width/i);
    assert.throws(() => layoutMeasured(w, g, px, 50, 0), /line/i);
    assert.throws(() => layoutMeasured(w, g, () => Number.NaN, 50, 3), /measure/i);
  });
});
