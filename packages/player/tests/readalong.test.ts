// readalong group: point-following-same-boundary-v1, atomic groups, active
// chunk only in the visible tree, reflow without changing samples.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ReadAlongModel } from "../src/index.js";
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
