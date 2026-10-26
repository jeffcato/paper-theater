// shell group: schematic joins, camera envelope, plaque growth,
// homography projection, blackout compositing, pinned shadow profile.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import * as THREE from "three";
import {
  baseCameraPose,
  baseDistance,
  baseShell,
  blackoutHolds,
  compositeStagePixel,
  curtainPanelX,
  envelopeCameraPose,
  plaqueGrowthSteps,
  projectPlaqueSafe,
  projectToCss,
  refitShell,
  signBoardY,
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

describe("shell: staging motion math", () => {
  it("slides curtain panels off the opening with the open ratio", () => {
    assert.equal(curtainPanelX("left", 0), -4);
    assert.equal(curtainPanelX("right", 0), 4);
    assert.equal(curtainPanelX("left", 1), -12);
    assert.equal(curtainPanelX("right", 1), 12);
    assert.ok(Math.abs(curtainPanelX("left", 0.5) - -8) < 1e-9);
    assert.ok(Math.abs(curtainPanelX("right", 0.5) - 8) < 1e-9);
  });

  it("rejects curtain ratios outside 0..1", () => {
    assert.throws(() => curtainPanelX("left", -0.1), /curtain/);
    assert.throws(() => curtainPanelX("right", 1.1), /curtain/);
    assert.throws(() => curtainPanelX("left", Number.NaN), /curtain/);
  });

  it("lowers the sign board into its bounds, hiding it above the frame", () => {
    // Visible center y=6 (SIGN_BOUNDS y [5,7]); hidden clears the frame top.
    assert.equal(signBoardY(1), 6);
    assert.ok(signBoardY(0) - 1 > 11, "hidden board clears the shell top");
    assert.ok(Math.abs(signBoardY(0.5) - 10) < 1e-9);
  });

  it("rejects sign progress outside 0..1", () => {
    assert.throws(() => signBoardY(-0.1), /sign/);
    assert.throws(() => signBoardY(1.1), /sign/);
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

describe("shell: camera refit for plaque growth", () => {
  const viewports = [
    { width: 960, height: 540 },
    { width: 1280, height: 720 },
    { width: 1920, height: 1080 },
  ];

  it("frames the base shell unchanged by default", () => {
    const shell = baseShell();
    assert.equal(shell.centerX, 0);
    assert.equal(shell.centerY, 4);
    assert.equal(shell.distance, baseDistance());
    const pose = envelopeCameraPose([0, 0], 0, [0, 0], false);
    assert.deepEqual(pose, baseCameraPose());
  });

  it("expands the framed shell downward with growth, keeping the envelope margin", () => {
    const shell = refitShell(13);
    // 13 steps extend the shell 3.25 world units down plus the 0.65
    // target/pointer margin: bottom -6.9, center 2.05.
    assert.equal(shell.centerX, 0);
    assert.equal(shell.centerY, 2.05);
    assert.ok(shell.distance > baseDistance(), "refit pulls back");
    const flat = refitShell(0);
    assert.equal(flat.centerX, 0);
    assert.ok(Math.abs(flat.centerY - 3.675) < 1e-9, `margin-only center (${flat.centerY})`);
    assert.ok(flat.distance > baseDistance(), "margin alone pulls back");
  });

  it("rejects negative or non-integer growth steps", () => {
    assert.throws(() => refitShell(-1), /growth/);
    assert.throws(() => refitShell(1.5), /growth/);
  });

  it("keeps the grown safe inset inside every frame at the refit pose", () => {
    for (const steps of [0, 3, 8, 13]) {
      const shell = refitShell(steps);
      const pose = envelopeCameraPose([0, 0], 0, [0, 0], false, shell);
      for (const viewport of viewports) {
        const rect = projectPlaqueSafe(pose, viewport);
        assert.ok(rect.top >= 0, `steps ${steps}: top on screen`);
        assert.ok(rect.top + rect.height <= viewport.height, `steps ${steps}: grown inset in frame`);
        assert.ok(rect.floorY < rect.top, `steps ${steps}: floor above plaque`);
      }
    }
  });

  it("holds the grown inset through camera/pointer extremes", () => {
    const shell = refitShell(13);
    const extremes: Array<[[number, number], number, [number, number], boolean]> = [
      [[0.5, 0.25], 2, [0.8, 0.4], false],
      [[-0.5, -0.25], 2, [-0.8, -0.4], false],
      [[0.5, -0.25], 0, [0.8, -0.4], false],
      [[0, 0], 0, [0.8, 0.4], true],
    ];
    for (const [offset, dolly, pointer, reduced] of extremes) {
      const pose = envelopeCameraPose(offset, dolly, pointer, reduced, shell);
      for (const viewport of viewports) {
        const rect = projectPlaqueSafe(pose, viewport);
        assert.ok(rect.top + rect.height <= viewport.height, `extreme ${offset}: inset in frame`);
        assert.ok(rect.top >= 0, `extreme ${offset}: top on screen`);
      }
    }
  });
});
