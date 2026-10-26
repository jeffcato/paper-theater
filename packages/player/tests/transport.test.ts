// transport group: one media element drives the master; seek/load epochs
// reject stale completions; user-pause INTERMISSION owns the sign.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { createTransport } from "../src/index.js";

function fakeClock() {
  return {
    currentTime: 0,
    paused: true,
    seeking: false,
    ended: false,
    plays: 0,
    async play() {
      this.plays++;
      this.paused = false;
    },
    pause() {
      this.paused = true;
    },
  };
}

describe("transport: single sampled clock", () => {
  it("reads currentTime once per frame into a clamped integer sample", () => {
    const clock = fakeClock();
    const seen: number[] = [];
    const t = createTransport(96000, clock, { onSample: (s) => seen.push(s) });
    clock.currentTime = 0.5;
    assert.equal(t.frame(), 24000);
    clock.currentTime = 99;
    assert.equal(t.frame(), 96000);
    assert.deepEqual(seen, [24000, 96000]);
    t.dispose();
  });

  it("evaluates sample_count on ended", () => {
    const clock = fakeClock();
    const t = createTransport(96000, clock, undefined, { reducedMotion: true });
    clock.currentTime = 1;
    clock.ended = true;
    assert.equal(t.frame(), 96000);
    assert.equal(t.reason, "ended");
    t.dispose();
  });

  it("seek publishes the destination synchronously and bumps the epoch", () => {
    const clock = fakeClock();
    const seen: number[] = [];
    const t = createTransport(96000, clock, { onSample: (s) => seen.push(s) });
    const before = t.epoch;
    t.seek(48000);
    assert.equal(t.sample, 48000);
    assert.ok(t.epoch > before);
    assert.ok(seen.includes(48000));
    t.dispose();
  });

  it("user pause lowers the sign; restart hides it again", () => {
    const clock = fakeClock();
    const poses: string[] = [];
    const t = createTransport(
      96000,
      clock,
      { onSign: (pose) => poses.push(pose) },
      { reducedMotion: true },
    );
    t.pause(true);
    assert.equal(t.reason, "user");
    assert.equal(t.sign.state.pose, "visible");
    t.restart();
    assert.equal(t.sample, 0);
    assert.equal(t.sign.state.pose, "hidden");
    assert.ok(poses.includes("visible"));
    t.dispose();
  });

  it("failed resume preserves the sign and reports the error", async () => {
    const clock = fakeClock();
    const errors: unknown[] = [];
    const t = createTransport(
      96000,
      clock,
      { onError: (e) => errors.push(e) },
      { reducedMotion: true },
    );
    t.pause(true);
    const failing = {
      ...clock,
      paused: true,
      seeking: false,
      ended: false,
      async play(): Promise<void> {
        throw new Error("play rejected");
      },
      pause() {},
    };
    const t2 = createTransport(
      96000,
      failing,
      { onError: (e) => errors.push(e) },
      { reducedMotion: true },
    );
    t2.pause(true);
    await assert.rejects(t2.play(), /play rejected/);
    assert.equal(t2.sign.state.pose, "visible");
    assert.equal(errors.length, 1);
    t.dispose();
    t2.dispose();
  });

  it("seek out of bounds throws instead of clamping silently", () => {
    const t = createTransport(96000, fakeClock());
    assert.throws(() => t.seek(96001), /bounds/);
    assert.throws(() => t.seek(-1 as unknown as number), /bounds/);
    t.dispose();
  });
});
