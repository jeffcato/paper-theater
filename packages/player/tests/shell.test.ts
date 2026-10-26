// shell group: schematic joins, camera envelope, plaque growth,
// homography projection, blackout compositing, pinned shadow profile.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import * as THREE from "three";
import {
  baseCameraPose,
  baseDistance,
  blackoutHolds,
  compositeStagePixel,
  envelopeCameraPose,
  plaqueGrowthSteps,
  projectPlaqueSafe,
  projectToCss,
  solveHomography,
  validateSchematicShell,
  RENDER_DEFAULTS,
} from "../src/index.js";
import type { CameraPose } from "../src/index.js";

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

describe("shell: projected plaque placement", () => {
  const viewports = [
    { width: 960, height: 540 },
    { width: 1280, height: 720 },
    { width: 1920, height: 1080 },
  ];

  function threeProject(
    point: [number, number, number],
    pose: CameraPose,
    viewport: { width: number; height: number },
  ): [number, number] {
    const camera = new THREE.PerspectiveCamera(40, viewport.width / viewport.height, 0.1, 100);
    camera.position.set(pose.position[0], pose.position[1], pose.position[2]);
    camera.lookAt(pose.target[0], pose.target[1], pose.target[2]);
    const v = new THREE.Vector3(point[0], point[1], point[2]).project(camera);
    return [((v.x + 1) / 2) * viewport.width, ((1 - v.y) / 2) * viewport.height];
  }

  it("centers the shell target and matches the three.js oracle", () => {
    const pose = baseCameraPose();
    const points: Array<[number, number, number]> = [
      [0, 4, 0], [-7, -2.8, 2.8], [7, -0.4, 2.8], [-8, 0, -1.6], [2.2, 1.6, 0.4],
    ];
    for (const viewport of viewports) {
      const center = projectToCss([pose.target[0], pose.target[1], 0], pose, viewport);
      assert.ok(Math.abs(center[0] - viewport.width / 2) < 1e-6, "target x centered");
      assert.ok(Math.abs(center[1] - viewport.height / 2) < 1e-6, "target y centered");
      for (const point of points) {
        const [x, y] = projectToCss(point, pose, viewport);
        const [ex, ey] = threeProject(point, pose, viewport);
        assert.ok(Math.abs(x - ex) < 1e-6 && Math.abs(y - ey) < 1e-6, `oracle ${point}`);
      }
    }
  });

  it("rejects points behind the camera", () => {
    const pose = baseCameraPose();
    assert.throws(
      () => projectToCss([pose.position[0], pose.position[1], pose.position[2] + 1], pose, { width: 960, height: 540 }),
      /behind camera/,
    );
  });

  it("projects the safe plaque rect inside the frame with the floor above it", () => {
    const pose = baseCameraPose();
    for (const viewport of viewports) {
      const rect = projectPlaqueSafe(pose, viewport);
      const right = rect.left + rect.width;
      const bottom = rect.top + rect.height;
      assert.ok(rect.width > 0 && rect.height > 0, "rect ordered");
      assert.ok(rect.left >= 0 && rect.top >= 0, "rect on screen");
      assert.ok(right <= viewport.width && bottom <= viewport.height, "rect in frame");
      assert.ok(rect.floorY < rect.top, "projected floor sits above the plaque");
      assert.ok(rect.top - rect.floorY >= 1, "visible gap between floor and plaque");
    }
  });
});
