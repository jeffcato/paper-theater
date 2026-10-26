// assets group (runs under check:player -- integration): registered-figure
// binding math. A book-local asset may be reused by many instances; each
// instance keeps its own state. Numbers pinned from the v2 fixture's
// figure-em card (404x1287 PNG, pivot [0.1807,0.9953], stage [0.505,1.61]).
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  facingAngle,
  layoutCard,
  nodAngle,
} from "../src/index.js";
import { planeOffsetDirect } from "../src/index.js";

const FIGURE_EM = {
  pixel_size: [404, 1287] as [number, number],
  pivot_top_left: [0.1806930693069307, 0.9953379953379954] as [number, number],
  stage_size: [0.5053948397185302, 1.6100078186082878] as [number, number],
  allow_mirror: false,
};

describe("assets: registered card layout", () => {
  it("preserves native aspect in stage units", () => {
    const layout = layoutCard(FIGURE_EM, { mirrored: false });
    assert.ok(Math.abs(layout.width / layout.height - 404 / 1287) < 1e-9);
    assert.equal(layout.width, FIGURE_EM.stage_size[0]);
    assert.equal(layout.height, FIGURE_EM.stage_size[1]);
  });

  it("places the registered pivot at the mesh origin", () => {
    const layout = layoutCard(FIGURE_EM, { mirrored: false });
    // Pivot (0.1807 right, 0.9953 down from top-left) lands on the origin;
    // the figure therefore stands on the support instead of sinking.
    assert.ok(Math.abs(layout.offsetX - 0.1613) < 0.001, `offsetX ${layout.offsetX}`);
    assert.ok(Math.abs(layout.offsetY - 0.7974) < 0.001, `offsetY ${layout.offsetY}`);
    assert.ok(layout.bottomY <= 0.001 && layout.bottomY > -0.05);
    assert.ok(layout.topY > 1.6);
  });

  it("matches the handoff worked example through the same rule", () => {
    const [x, y] = planeOffsetDirect([0.25, 0.9], [1, 2]);
    assert.ok(Math.abs(x - 0.25) < 1e-12 && Math.abs(y - 0.8) < 1e-12);
  });

  it("never mirrors a card that disallows it", () => {
    assert.equal(layoutCard(FIGURE_EM, { mirrored: true }).scaleX, 1);
    assert.equal(
      layoutCard({ ...FIGURE_EM, allow_mirror: true }, { mirrored: true }).scaleX,
      -1,
    );
    assert.equal(layoutCard({ ...FIGURE_EM, allow_mirror: true }, { mirrored: false }).scaleX, 1);
  });

  it("renders the nod as motion around the pivot that returns to rest", () => {
    assert.equal(nodAngle(0, 0.25), 0);
    assert.equal(nodAngle(1, 0.25), 0);
    const mid = nodAngle(0.5, 0.25);
    assert.ok(mid < 0 && Math.abs(mid - -0.25 * (Math.PI / 6)) < 1e-12, `mid ${mid}`);
    assert.equal(nodAngle(0.5, 0), 0);
  });

  it("faces left/right symmetrically and rejects anything else", () => {
    assert.equal(facingAngle("left"), -facingAngle("right"));
    assert.ok(Math.abs(facingAngle("left")) > 0);
    assert.throws(() => facingAngle("up"), /facing/);
  });
});
