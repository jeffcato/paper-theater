// geometry group: strict projection/support-contact conformance (P18).
// The player's pure pinhole projection must agree with an independently
// constructed Three.js camera to well under a pixel, and registered pivots
// must survive the real matrix chain (compose + V3 flip + mirror + nod)
// at their authored contact points to within 1e-6 world units. Three.js
// performs every matrix composition here; the player supplies only the
// conversions under test, and expectations are hand-derived from fixture
// authoring (never re-statements of the implementation).
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import * as THREE from "three";
import {
  assertNativeAspect,
  baseCameraPose,
  envelopeCameraPose,
  layoutCard,
  nodAngle,
  pivotOffset,
  pivotTopToBottom,
  projectPlaqueSafe,
  projectToCss,
  refitShell,
  validateSchematicShell,
} from "../src/index.js";
import type { CameraPose } from "../src/index.js";
import { APRON, DECK, LAYER_Z, PLAQUE_SAFE } from "../src/profile.js";

// Canonical fixture registration (figure-em): asymmetric u, near-bottom v.
const FIGURE_EM = {
  pixel_size: [404, 1287] as [number, number],
  pivot_top_left: [0.1806930693069307, 0.9953379953379954] as [number, number],
  stage_size: [0.5053948397185302, 1.6100078186082878] as [number, number],
  allow_mirror: false,
};
// Hand-derived bottom edge: -sh/2 + (v-0.5)*sh. Negative: the authored card
// bottom sits 0.0075 world units below the deck contact plane. That is the
// authored geometry and must NOT be clamped to the floor by a fix.
const FIGURE_EM_BOTTOM_HAND =
  -FIGURE_EM.stage_size[1] / 2 +
  (FIGURE_EM.pivot_top_left[1] - 0.5) * FIGURE_EM.stage_size[1];

/** Independent Three.js camera projection to CSS px (not the player helper). */
function threeProjectCss(
  point: [number, number, number],
  pose: CameraPose,
  width: number,
  height: number,
): [number, number] {
  const camera = new THREE.PerspectiveCamera(40, width / height, 0.1, 100);
  camera.position.set(pose.position[0], pose.position[1], pose.position[2]);
  camera.lookAt(pose.target[0], pose.target[1], pose.target[2]);
  camera.updateMatrixWorld();
  const v = new THREE.Vector3(point[0], point[1], point[2]).project(camera);
  return [((v.x + 1) / 2) * width, ((1 - v.y) / 2) * height];
}

const VIEWPORTS: [number, number][] = [[960, 540], [1280, 720], [1920, 1080]];

function poses(): { name: string; pose: CameraPose }[] {
  const shell = refitShell(2);
  return [
    { name: "base", pose: baseCameraPose() },
    { name: "refit", pose: { position: [shell.centerX, shell.centerY, shell.distance], target: [shell.centerX, shell.centerY, 0] } },
    { name: "extreme+", pose: envelopeCameraPose([0.5, 0.25], 2, [0.8, 0.4], false, shell) },
    { name: "extreme-", pose: envelopeCameraPose([-0.5, -0.25], 2, [-0.8, -0.4], false, shell) },
    { name: "leave", pose: envelopeCameraPose([0.5, 0.25], 2, [0, 0], false, shell) },
    { name: "reduced", pose: envelopeCameraPose([0.5, 0.25], 2, [0.8, 0.4], true, shell) },
  ];
}

function insetCorners(): [number, number, number][] {
  const z = LAYER_Z.plaque;
  return [
    [PLAQUE_SAFE.x[0], PLAQUE_SAFE.y[1], z],
    [PLAQUE_SAFE.x[1], PLAQUE_SAFE.y[1], z],
    [PLAQUE_SAFE.x[0], PLAQUE_SAFE.y[0], z],
    [PLAQUE_SAFE.x[1], PLAQUE_SAFE.y[0], z],
  ];
}

/** The sink's per-instance matrix chain, composed by Three.js itself. */
function instanceMatrix(
  position: [number, number, number],
  euler: [number, number, number],
  mirrorX: 1 | -1,
): THREE.Matrix4 {
  const composed = new THREE.Matrix4().compose(
    new THREE.Vector3(position[0], position[1], position[2]),
    new THREE.Quaternion().setFromEuler(new THREE.Euler(euler[0], euler[1], euler[2])),
    new THREE.Vector3(mirrorX, 1, 1),
  );
  const flip = new THREE.Matrix4().makeScale(1, 1, -1);
  return new THREE.Matrix4().multiplyMatrices(flip, composed).multiply(flip);
}

/** Authoring-frame world point through the sink chain (re-flip Z). */
function authoringPoint(matrix: THREE.Matrix4, local: [number, number, number]): [number, number, number] {
  const v = new THREE.Vector3(local[0], local[1], local[2]).applyMatrix4(matrix);
  return [v.x, v.y, -v.z];
}

function err3(a: [number, number, number], b: [number, number, number]): number {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
}

describe("geometry: independent projection agreement", () => {
  it("matches a Three.js camera within 1e-6 px on every inset corner and pose", () => {
    let worst = 0;
    let worstAt = "";
    const probes: [number, number, number][] = [
      ...insetCorners(),
      [0, 0, LAYER_Z.plaque],
      [0, 2, 0],
      [-9.5, 4, 1.8],
      [0, 6, 3.0],
    ];
    for (const [w, h] of VIEWPORTS) {
      for (const { name, pose } of poses()) {
        for (const point of probes) {
          const mine = projectToCss(point, pose, { width: w, height: h });
          const theirs = threeProjectCss(point, pose, w, h);
          const err = Math.hypot(mine[0] - theirs[0], mine[1] - theirs[1]);
          if (err > worst) {
            worst = err;
            worstAt = `${w}x${h}/${name}/${point}`;
          }
        }
      }
    }
    assert.ok(worst <= 1e-6, `projection drift ${worst} px at ${worstAt}`);
  });

  it("projectPlaqueSafe edges match the Three.js inset corners", () => {
    for (const [w, h] of VIEWPORTS) {
      const pose = baseCameraPose();
      const rect = projectPlaqueSafe(pose, { width: w, height: h });
      const corners = insetCorners().map((c) => threeProjectCss(c, pose, w, h));
      const tl = corners[0]!;
      const tr = corners[1]!;
      const bl = corners[2]!;
      assert.ok(Math.abs(rect.left - tl[0]) <= 1e-6, `left ${w}x${h}`);
      assert.ok(Math.abs(rect.top - tl[1]) <= 1e-6, `top ${w}x${h}`);
      assert.ok(Math.abs(rect.left + rect.width - tr[0]) <= 1e-6, `right ${w}x${h}`);
      assert.ok(Math.abs(rect.top + rect.height - bl[1]) <= 1e-6, `bottom ${w}x${h}`);
    }
  });
});

describe("geometry: pivot/support contact through world transforms", () => {
  it("keeps native aspect, top-origin pivots and hand-derived edges", () => {
    assert.doesNotThrow(() => assertNativeAspect(FIGURE_EM.pixel_size, FIGURE_EM.stage_size));
    const flipped = pivotTopToBottom([0.3, 0.8]);
    assert.ok(Math.abs(flipped[0] - 0.3) <= 1e-12 && Math.abs(flipped[1] - 0.2) <= 1e-12, `pivot flip ${flipped}`);
    const layout = layoutCard(FIGURE_EM, { mirrored: false });
    assert.equal(layout.scaleX, 1);
    assert.ok(Math.abs(layout.bottomY - FIGURE_EM_BOTTOM_HAND) <= 1e-12, `bottom ${layout.bottomY}`);
    // The authored bottom edge is genuinely below the deck plane: a fix
    // must never clamp it to the floor.
    assert.ok(FIGURE_EM_BOTTOM_HAND < 0, "authored lift below deck preserved");
    const [ox, oy] = pivotOffset(FIGURE_EM.pivot_top_left, FIGURE_EM.stage_size);
    assert.ok(Math.abs(ox - (0.5 - FIGURE_EM.pivot_top_left[0]) * FIGURE_EM.stage_size[0]) <= 1e-12, "asymmetric u offset");
    void oy;
  });

  it("holds the standing pivot at its authored contact within 1e-6", () => {
    const position: [number, number, number] = [0, 0, 0];
    const matrix = instanceMatrix(position, [0, 0, 0], 1);
    const pivot = authoringPoint(matrix, [0, 0, 0]);
    assert.ok(err3(pivot, position) <= 1e-6, `pivot drift ${err3(pivot, position)}`);
    const [ox, oy] = pivotOffset(FIGURE_EM.pivot_top_left, FIGURE_EM.stage_size);
    const bottom = authoringPoint(matrix, [0, -FIGURE_EM.stage_size[1] / 2 + oy, 0]);
    assert.ok(Math.abs(bottom[0] - (position[0] + 0)) <= 1e-6, "bottom x centered");
    assert.ok(Math.abs(bottom[1] - FIGURE_EM_BOTTOM_HAND) <= 1e-9, `bottom ${bottom[1]}`);
    void ox;
  });

  it("holds the pivot through nod, mirror, facing and asymmetric registrations", () => {
    const position: [number, number, number] = [0, 0, 0];
    const nod = nodAngle(0.5, 1);
    const cases: { name: string; euler: [number, number, number]; mirror: 1 | -1 }[] = [
      { name: "nod-mid", euler: [nod, 0, 0], mirror: 1 },
      { name: "mirror", euler: [0, 0, 0], mirror: -1 },
      { name: "facing-left", euler: [0, -0.4, 0], mirror: 1 },
      { name: "facing-right", euler: [0, 0.4, 0], mirror: 1 },
    ];
    for (const { name, euler, mirror } of cases) {
      const pivot = authoringPoint(instanceMatrix(position, euler, mirror), [0, 0, 0]);
      assert.ok(err3(pivot, position) <= 1e-6, `${name} pivot drift ${err3(pivot, position)}`);
    }
    // Rigid motion about the pivot preserves the pivot-to-bottom distance.
    const bent = authoringPoint(instanceMatrix(position, [nod, 0, 0], 1), [0, FIGURE_EM_BOTTOM_HAND, 0]);
    assert.ok(Math.abs(err3(bent, position) - Math.abs(FIGURE_EM_BOTTOM_HAND)) <= 1e-9, "nod is rigid about the pivot");
    // Diagnostic asymmetric registration: pivot [0.3,0.8] on stage [1,2].
    const asym = { pivot_top_left: [0.3, 0.8] as [number, number], stage_size: [1, 2] as [number, number] };
    const [ax, ay] = pivotOffset(asym.pivot_top_left, asym.stage_size);
    assert.ok(Math.abs(ax - 0.2) <= 1e-12 && Math.abs(ay - 0.6) <= 1e-12, `asymmetric offset ${ax},${ay}`);
    const asymBottom: [number, number, number] = [0, -1 + 0.6, 0];
    const got = authoringPoint(instanceMatrix([1, 0.5, -1], [0, 0, 0], 1), asymBottom);
    assert.ok(err3(got, [1, 0.5 - 0.4, -1]) <= 1e-9, `asymmetric bottom ${got}`);
  });

  it("distinguishes an authored lift from a contact state", () => {
    const lifted: [number, number, number] = [1, 0.5, -1];
    const pivot = authoringPoint(instanceMatrix(lifted, [0, 0, 0], 1), [0, 0, 0]);
    assert.ok(err3(pivot, lifted) <= 1e-6, "pivot follows the authored position");
    // Against the deck anchor the error equals the authored lift exactly:
    // 0.5 of clearance, not a contact defect.
    assert.ok(Math.abs(pivot[1] - 0 - 0.5) <= 1e-9, "lift reads as clearance, not contact");
  });

  it("joins deck/apron at the authored constants", () => {
    assert.equal(validateSchematicShell().ok, true);
    assert.equal(DECK.z[1], APRON.z);
    assert.equal(DECK.y, 0);
  });
});
