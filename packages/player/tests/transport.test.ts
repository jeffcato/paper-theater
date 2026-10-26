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

describe("transport: control races and stale completions", () => {
  /** Clock whose play() only settles when the test says so. */
  function gatedClock() {
    let settlePlay!: () => void;
    let failPlay!: (error: unknown) => void;
    return {
      currentTime: 0,
      paused: true,
      seeking: false,
      ended: false,
      plays: 0,
      async play(): Promise<void> {
        this.plays++;
        this.paused = false;
        await new Promise<void>((resolve, reject) => {
          settlePlay = resolve;
          failPlay = reject;
        });
      },
      pause(): void {
        this.paused = true;
      },
      settle(): void {
        settlePlay();
      },
      fail(error: unknown): void {
        failPlay(error);
      },
    };
  }

  function raceHarness() {
    const clock = gatedClock();
    const reasons: string[] = [];
    const poses: string[] = [];
    const errors: unknown[] = [];
    const samples: number[] = [];
    const t = createTransport(
      96000,
      clock,
      {
        onSample: (s) => samples.push(s),
        onReason: (r) => reasons.push(r),
        onSign: (pose) => poses.push(pose),
        onError: (e) => errors.push(e),
      },
      { reducedMotion: true },
    );
    return { clock, reasons, poses, errors, samples, t };
  }

  it("stale play completion after a synchronous pause changes nothing", async () => {
    const { clock, reasons, poses, errors, t } = raceHarness();
    t.pause(true);
    const pending = t.play();
    t.pause(true);
    clock.settle();
    await pending;
    assert.equal(t.reason, "user");
    assert.equal(t.sign.state.pose, "visible");
    assert.ok(!reasons.includes("playing"), `stale completion flipped reason: ${reasons.join(",")}`);
    assert.equal(poses[poses.length - 1], "visible");
    assert.equal(errors.length, 0);
    t.dispose();
  });

  it("seek during pending play keeps playback truthful without lowering the sign", async () => {
    const { clock, reasons, poses, t } = raceHarness();
    const pending = t.play();
    t.seek(48000);
    clock.settle();
    await pending;
    // A seek does not cancel playback: the destination holds and the live
    // media state (playing) is reported. The sign was never lowered, so the
    // completion's raise is a hidden-repeat, never a visible pose.
    assert.equal(t.sample, 48000);
    assert.equal(t.reason, "playing");
    assert.deepEqual(poses, ["hidden"]);
    assert.deepEqual(reasons, ["seeking", "playing"]);
    t.dispose();
  });

  it("restart during pending play drops the stale completion", async () => {
    const { clock, reasons, poses, t } = raceHarness();
    t.seek(48000);
    const pending = t.play();
    t.restart();
    clock.settle();
    await pending;
    assert.equal(t.sample, 0);
    assert.equal(t.reason, "readiness");
    assert.equal(t.sign.state.pose, "hidden");
    assert.ok(!reasons.includes("playing"), `stale completion flipped reason: ${reasons.join(",")}`);
    assert.equal(poses[poses.length - 1], "hidden");
    t.dispose();
  });

  it("dispose during pending play emits nothing after", async () => {
    const { clock, reasons, poses, errors, samples, t } = raceHarness();
    const pending = t.play();
    t.dispose();
    // Teardown itself hides the sign exactly once; everything after is silent.
    assert.deepEqual(poses, ["hidden"]);
    const frozen = [reasons.length, poses.length, errors.length, samples.length];
    clock.settle();
    await pending;
    t.frame();
    t.pause();
    t.seek(100);
    t.restart();
    assert.deepEqual([reasons.length, poses.length, errors.length, samples.length], frozen);
    assert.deepEqual(reasons, []);
    assert.deepEqual(errors, []);
    assert.deepEqual(samples, []);
    assert.equal(t.sample, 0);
  });

  it("a superseded play rejection stays silent", async () => {
    const { clock, reasons, errors, t } = raceHarness();
    t.pause(true);
    const pending = t.play();
    t.pause(true);
    clock.fail(new Error("stale rejection"));
    await pending;
    assert.equal(t.reason, "user");
    assert.equal(errors.length, 0);
    assert.ok(!reasons.includes("readiness"), `stale failure reset reason: ${reasons.join(",")}`);
    t.dispose();
  });

  it("rapid pause/play/pause ends with the latest intent and rising epochs", async () => {
    const { clock, t } = raceHarness();
    const e0 = t.epoch;
    t.pause(true);
    const e1 = t.epoch;
    const pending = t.play();
    t.pause(true);
    const e3 = t.epoch;
    clock.settle();
    await pending;
    assert.ok(e1 > e0 && e3 > e1);
    assert.equal(t.reason, "user");
    assert.equal(t.sign.state.pose, "visible");
    t.dispose();
  });

  it("back-to-back seeks keep only the latest destination", async () => {
    const { reasons, t } = raceHarness();
    t.seek(100);
    t.seek(200);
    await null;
    await null;
    assert.equal(t.sample, 200);
    assert.equal(t.reason, "seeking");
    assert.ok(reasons.filter((r) => r === "seeking").length <= 2, reasons.join(","));
    t.dispose();
  });
});
