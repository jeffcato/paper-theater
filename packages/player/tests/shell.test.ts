// shell group: schematic joins, camera envelope, plaque growth,
// homography projection, blackout compositing, pinned shadow profile.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  baseCameraPose,
  baseDistance,
  blackoutHolds,
  compositeStagePixel,
  envelopeCameraPose,
  plaqueGrowthSteps,
  solveHomography,
  validateSchematicShell,
  RENDER_DEFAULTS,
} from "../src/index.js";

describe("shell: schematic geometry and camera", () => {
  it("validates the layered proscenium/deck/apron/plaque joins", () => {
    const v = validateSchematicShell();
    assert.deepEqual(v.errors, []);
    assert.equal(v.ok, true);
    assert.ok(v.base_distance > 0);
  });

  it("frames the shell plus panel with a 10% reserve", () => {
    const base = baseCameraPose();
    assert.equal(base.position[0], base.target[0]);
    assert.equal(baseDistance(), base.position[2]);
  });

  it("rejects authored/pointer poses outside the envelope", () => {
    assert.throws(() => envelopeCameraPose([0.6, 0], 0, [0, 0], false), /envelope/);
    assert.throws(() => envelopeCameraPose([0, 0], 0, [0.9, 0], false), /envelope/);
    assert.throws(() => envelopeCameraPose([0, 0], 2.5, [0, 0], false), /Dolly/);
  });

  it("zeroes pointer parallax under reduced motion", () => {
    const a = envelopeCameraPose([0, 0], 0, [0.8, 0.4], false);
    const b = envelopeCameraPose([0, 0], 0, [0.8, 0.4], true);
    const base = baseCameraPose();
    assert.notDeepEqual(a.position, base.position);
    assert.deepEqual(b.position, base.position);
    assert.deepEqual(b.target, base.target);
  });

  it("grows the plaque in 0.25-unit steps instead of shrinking text", () => {
    assert.equal(plaqueGrowthSteps(1, 2), 0);
    assert.equal(plaqueGrowthSteps(2.5, 1.9), 3);
    assert.throws(() => plaqueGrowthSteps(0, 1), /height/);
  });

  it("solves the inset homography exactly on a rectangle", () => {
    const world: [[number, number], [number, number], [number, number], [number, number]] =
      [[-6.5, -2.55], [6.5, -2.55], [6.5, -0.65], [-6.5, -0.65]];
    const css: [[number, number], [number, number], [number, number], [number, number]] =
      [[100, 400], [500, 400], [500, 300], [100, 300]];
    const h = solveHomography(world, css);
    assert.equal(h.length, 8);
    for (let i = 0; i < 4; i++) {
      const [x, y] = world[i]!;
      const [u, v] = css[i]!;
      const w = h[6]! * x + h[7]! * y + 1;
      const pu = (h[0]! * x + h[1]! * y + h[2]!) / w;
      const pv = (h[3]! * x + h[4]! * y + h[5]!) / w;
      assert.ok(Math.abs(pu - u) < 1e-6 && Math.abs(pv - v) < 1e-6, `corner ${i}`);
    }
  });

  it("covers every stage contribution at full blackout", () => {
    const pixels: Array<[number, number, number]> = [
      [1, 1, 1], [0.5, 0.2, 0.9], [0, 0, 0], [3, 0.1, 0.1],
    ];
    const blacked = pixels.map((p) => compositeStagePixel(p, 0));
    assert.equal(blackoutHolds(blacked), true);
    const lit = pixels.map((p) => compositeStagePixel(p, 1));
    assert.deepEqual(lit, pixels);
  });

  it("pins PCFShadowMap (migration off PCFSoftShadowMap)", () => {
    assert.equal(RENDER_DEFAULTS.shadow_map, "PCFShadowMap");
  });
});
