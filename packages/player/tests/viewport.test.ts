// Audience theater viewport: the picture scales with the window, and its
// height stops at 700px. A shorter window holds that picture. Growing past
// 700px scales again. Width is never floored.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { THEATER_MIN_HEIGHT, theaterCanvasBox, theaterViewport } from "../src/immersive.js";

describe("theater viewport floor", () => {
  it("scales with the window at and above 700px", () => {
    assert.equal(THEATER_MIN_HEIGHT, 700);
    assert.deepEqual(theaterViewport(1280, 800), { width: 1280, height: 800 });
    assert.deepEqual(theaterViewport(1280, 700), { width: 1280, height: 700 });
    assert.deepEqual(theaterViewport(960, 1200), { width: 960, height: 1200 });
  });

  it("holds the picture at 700px below that height and lets width follow", () => {
    assert.deepEqual(theaterViewport(1280, 699), { width: 1280, height: 700 });
    assert.deepEqual(theaterViewport(400, 320), { width: 400, height: 700 });
    assert.deepEqual(theaterViewport(2000, 100), { width: 2000, height: 700 });
  });

  it("scales again as soon as the window is taller than the floor", () => {
    assert.deepEqual(theaterViewport(900, 699), { width: 900, height: 700 });
    assert.deepEqual(theaterViewport(900, 701), { width: 900, height: 701 });
  });

  it("keeps a short window from scaling the bitmap down", () => {
    assert.deepEqual(theaterCanvasBox(800, 800), { width: "100%", height: "100%", bottom: "0" });
    assert.deepEqual(theaterCanvasBox(700, 700), { width: "100%", height: "100%", bottom: "0" });
    assert.deepEqual(theaterCanvasBox(640, 700), { width: "100%", height: "700px", bottom: "auto" });
  });

  it("rejects an empty viewport", () => {
    assert.throws(() => theaterViewport(0, 800), /theater viewport/);
    assert.throws(() => theaterViewport(800, 0), /theater viewport/);
  });
});
