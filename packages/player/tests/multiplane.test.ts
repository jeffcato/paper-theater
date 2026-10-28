// multiplane group (shell): full-viewport 2.5D world plan — discrete Z
// layers, contain-fit framing for any aspect, the lobby-to-seat journey,
// off-axis pointer parallax and the projected lectern/rail layout.
// Expectations are hand-derived from the immersive brief, not restated
// from the implementation.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  LOBBY_Z,
  BOOTH_FLIGHT,
  INSPECT,
  boothMap,
  placeInspectCard,
  LECTERN,
  LOBBY_FRAME,
  LOBBY_LAYERS,
  MULTIPLANE_LAYERS,
  PAGE_EDGE_X,
  PAGE_FACE_Z,
  PAGE_HINGE_X,
  PROSCENIUM_Z,
  PIT_FLOOR,
  GRILLE_SILL_Y,
  foyerWallBounds,
  BANNER_FIELD,
  BANNER_MEDALLION,
  BANNER_ROW_BOTTOM,
  BANNER_TITLE_WIDTH,
  BANNER_Z,
  RAIL_CONTROLS,
  bannerLayout,
  bannerScale,
  bannerScreen,
  PROSCENIUM_FACADE,
  TICKET_SLOTS,
  STALL_ROW_Z,
  STALL_FRAMES,
  FOV_Y_DEGREES,
  FOYER_WALL_Z,
  PATRONS,
  SIDE_VIEW_DEGREES,
  STALL_ASIDE,
  seatContainFitDepth,
  stallShift,
  pageTurn,
  LECTERN_RAISE,
  LECTERN_TRAVEL_MS,
  lecternRect,
  lecternTravel,
  lecternWorld,
  projectWorld,
  railRect,
  viewportPixelRatio,
  worldCamera,
  applyTicketZoom,
  ticketZoomIntent,
  ticketPanLock,
  boothSeatCrossfade,
  TICKET_ZOOM_HOLD_MS,
  BOOTH_SEAT_FADE_MS,
  CURTAIN_TRAVEL_MS,
  rehearsalCurtain,
  sceneLayout,
} from "../src/multiplane.js";
import type { Rect3, WorldCamera } from "../src/multiplane.js";
import { HOUSE_ART_BRIEF } from "../src/house-brief.js";
import { CAMERA, OPENING } from "../src/profile.js";

const VIEWPORTS = [[960, 540], [1280, 720], [1920, 1080], [2560, 1080], [1497, 1360], [800, 1200]] as const;
const EXTREMES: [number, number][] = [[0, 0], [1, 1], [-1, -1], [1, -1], [-1, 1]];
const seat = (w: number, h: number, pointer: [number, number] = [0, 0], growthSteps = 0, reducedMotion = false) =>
  worldCamera({ journey: 1, pointer, reducedMotion, width: w, height: h, growthSteps });
const lobby = (w: number, h: number, pointer: [number, number] = [0, 0]) =>
  worldCamera({ journey: 0, pointer, reducedMotion: false, width: w, height: h, growthSteps: 0 });
const inside = (cam: WorldCamera, w: number, h: number, x: number, y: number, z: number, slack = 1e-6) => {
  const [u, v] = projectWorld(cam, [x, y, z], { width: w, height: h });
  return u >= -slack && u <= w + slack && v >= -slack && v <= h + slack;
};

describe("multiplane layers", () => {
  it("orders discrete stage-side Z planes from camera to backcloth", () => {
    assert.deepEqual(MULTIPLANE_LAYERS.map((l) => l.z), [2.5, 1.2, 0, -0.3, -0.8, -1.5, -2.5, -4]);
    assert.equal(new Set(MULTIPLANE_LAYERS.map((l) => l.id)).size, MULTIPLANE_LAYERS.length);
    assert.deepEqual(LOBBY_LAYERS.map((l) => l.z), [1.5, 0.8, -1.45]);
  });
  it("keeps the frozen curtain depth as the stage-front layer", () => {
    // Stage front sits 0.3 behind the facade; the frozen curtains are at 2.2.
    assert.equal(PROSCENIUM_Z - 0.3, 2.2);
    assert.equal(LECTERN.z, PROSCENIUM_Z + 0.8);
    assert.equal(bannerLayout().length, 5);
  });
});

describe("empty stage behind the curtain", () => {
  const worldZ = (id: string) => PROSCENIUM_Z + MULTIPLANE_LAYERS.find((layer) => layer.id === id)!.z;

  it("opens and closes across the 1.5 second curtain travel", () => {
    assert.equal(CURTAIN_TRAVEL_MS, 1500);
    assert.equal(rehearsalCurtain(0, 0, 1).ratio, 0);
    assert.equal(rehearsalCurtain(0, 0, 1).titleVisible, true);
    assert.equal(rehearsalCurtain(0, 0, 1).moving, true);
    assert.equal(rehearsalCurtain(750, 0, 1).ratio, 0.5);
    assert.equal(rehearsalCurtain(750, 0, 1).titleVisible, false);
    assert.equal(rehearsalCurtain(1500, 0, 1).ratio, 1);
    assert.equal(rehearsalCurtain(1500, 0, 1).moving, false);
    assert.equal(rehearsalCurtain(1500, 0, 1).titleVisible, false);
    assert.equal(rehearsalCurtain(0, 1, 0).ratio, 1);
    assert.equal(rehearsalCurtain(750, 1, 0).ratio, 0.5);
    assert.equal(rehearsalCurtain(1500, 1, 0).ratio, 0);
    assert.equal(rehearsalCurtain(1500, 1, 0).titleVisible, true);
    // A half-open curtain finishes the remaining half in half the travel.
    assert.equal(rehearsalCurtain(0, 0.5, 1).ratio, 0.5);
    assert.equal(rehearsalCurtain(750, 0.5, 1).ratio, 1);
    assert.throws(() => rehearsalCurtain(-1, 0, 1), /curtain elapsed/);
  });

  it("sets the deck, groove wings and backcloth on the documented planes", () => {
    const cards = sceneLayout();
    const deck = cards.find((card) => card.id === "deck");
    assert.ok(deck && deck.kind === "deck");
    assert.deepEqual(deck.x, [-8, 8]);
    assert.equal(deck.y, 0);
    assert.deepEqual(deck.z, [-5.6, 2.4]);
    for (const id of ["groove-1", "groove-2", "scenic-flats"] as const) {
      const wings = cards.filter((card) => card.id === id);
      assert.equal(wings.length, 2, id);
      for (const wing of wings) {
        assert.equal(wing.kind, "wing");
        assert.equal(wing.z, worldZ(id));
        assert.deepEqual(wing.y, [OPENING.y[0], OPENING.y[1]]);
        assert.ok(wing.x[1] <= -6 || wing.x[0] >= 6, `${id} blocks the opening`);
      }
      assert.ok(wings[0]!.x[1] < 0 && wings[1]!.x[0] > 0);
    }
    const cloth = cards.find((card) => card.id === "backcloth");
    assert.ok(cloth && cloth.kind === "cloth");
    assert.equal(cloth.z, worldZ("backcloth"));
    assert.ok(cloth.x[0] <= OPENING.x[0] && cloth.x[1] >= OPENING.x[1]);
    assert.ok(cloth.y[0] <= OPENING.y[0] && cloth.y[1] >= OPENING.y[1]);
    const depths = ["groove-1", "groove-2", "scenic-flats", "backcloth"].map(worldZ);
    assert.deepEqual(depths, [...depths].sort((a, b) => b - a));
    assert.ok(cloth.z < deck.z[1] && cloth.z > deck.z[0]);
  });
});

describe("full-viewport seat framing", () => {
  for (const [w, h] of VIEWPORTS) {
    it(`fits the proscenium inside ${w}x${h}`, () => {
      const cam = seat(w, h);
      assert.equal(cam.aspect, w / h);
      const { halfWidth, top, bottom } = PROSCENIUM_FACADE;
      const corners = [[-halfWidth, top], [halfWidth, top], [-halfWidth, bottom], [halfWidth, bottom]] as const;
      const fitted = seatContainFitDepth(w, h);
      const pulledForward = cam.position[2] < fitted - 0.05;
      if (pulledForward) {
        const opening = [[OPENING.x[0], OPENING.y[0]], [OPENING.x[1], OPENING.y[0]], [OPENING.x[0], OPENING.y[1]], [OPENING.x[1], OPENING.y[1]]] as const;
        for (const [x, y] of opening) assert.ok(inside(cam, w, h, x, y, PROSCENIUM_Z), `opening ${x},${y}`);
      } else {
        for (const [x, y] of corners) assert.ok(inside(cam, w, h, x, y, PROSCENIUM_Z), `${x},${y}`);
        const margins = corners.map(([x, y]) => {
          const [u, v] = projectWorld(cam, [x, y, PROSCENIUM_Z], { width: w, height: h });
          return Math.min(u, w - u, v, h - v);
        });
        const packed = cam.position[2] > fitted + 0.05;
        if (!packed) {
          // A corner touches the viewport edge: the stage is as large as the viewport allows.
          assert.ok(Math.min(...margins) < 1e-6);
        }
      }
      // The stage opening's centre line is centered horizontally.
      assert.ok(Math.abs(projectWorld(cam, [0, OPENING.y[0], PROSCENIUM_Z], { width: w, height: h })[0] - w / 2) < 1e-6);
    });
  }
  it("rejects empty viewports", () => {
    assert.throws(() => seat(0, 720), /viewport/);
  });
  it("continues the floor below the proscenium to the bottom of the view", () => {
    assert.equal(PIT_FLOOR.z, PROSCENIUM_Z + 0.04);
    assert.ok(PIT_FLOOR.y[1] <= PROSCENIUM_FACADE.bottom);
    for (const [w, h] of VIEWPORTS) {
      for (const pointer of EXTREMES) {
        for (const steps of [0, 12]) {
          const cam = seat(w, h, pointer, steps);
          const bottom = PIT_FLOOR.x.map((x) => projectWorld(cam, [x, PIT_FLOOR.y[0], PIT_FLOOR.z], { width: w, height: h }));
          assert.ok(Math.min(...bottom.map((p) => p[0])) <= 0, `${w}x${h} floor misses the left edge`);
          assert.ok(Math.max(...bottom.map((p) => p[0])) >= w, `${w}x${h} floor misses the right edge`);
          for (const [, v] of bottom) assert.ok(v >= h - 1, `${w}x${h} floor stops at ${v} of ${h}`);
        }
      }
    }
  });
});

describe("side viewpoint at rest", () => {
  // The camera sits off the centre line and looks back at the frame centre,
  // so flat cards stand at a slight angle: the edge nearer the camera
  // projects taller than the far edge.
  const angle = (cam: WorldCamera, planeZ: number) => Math.atan2(Math.abs(cam.position[0]), cam.position[2] - planeZ);
  const nearOverFar = (cam: WorldCamera, rect: Rect3, w: number, h: number) => {
    const tall = (x: number) =>
      projectWorld(cam, [x, rect.y[0], rect.z], { width: w, height: h })[1] - projectWorld(cam, [x, rect.y[1], rect.z], { width: w, height: h })[1];
    const [left, right] = [tall(rect.x[0]), tall(rect.x[1])];
    return cam.position[0] < 0 ? left / right : right / left;
  };
  for (const [w, h] of VIEWPORTS) {
    it(`sees the stage and the box office from one side at ${w}x${h}`, () => {
      const [s, l] = [seat(w, h), lobby(w, h)];
      assert.ok(angle(s, PROSCENIUM_Z) >= (5 * Math.PI) / 180, "seat off the centre line");
      assert.ok(angle(l, LOBBY_FRAME.planeZ) >= (5 * Math.PI) / 180, "box office off the centre line");
      assert.equal(Math.sign(s.position[0]), Math.sign(l.position[0]));
      assert.ok(nearOverFar(s, { x: OPENING.x, y: OPENING.y, z: PROSCENIUM_Z }, w, h) > 1.02);
      if (w / h >= 1) assert.ok(nearOverFar(l, INSPECT, w, h) > 1.02);
    });
  }
});

describe("read-along centers the seat", () => {
  const view = (readAlong: number) =>
    worldCamera({ journey: 1, pointer: [0, 0], reducedMotion: true, width: 1280, height: 720, growthSteps: 0, readAlong });

  it("pans from the side view onto the centre line and back", () => {
    const down = view(0);
    const up = view(1);
    const side = (SIDE_VIEW_DEGREES * Math.PI) / 180;
    assert.ok(Math.abs(down.yaw - side) < 1e-12);
    assert.ok(down.position[0] < 0, "the resting camera stands house-left");
    assert.equal(up.yaw, 0);
    assert.equal(up.position[0], 0);
    assert.ok(up.position[2] < down.position[2], "read-along dollies toward the stage");
    assert.equal(up.focusZ, down.focusZ);
    const mid = view(0.5);
    assert.ok(Math.abs(mid.yaw - side / 2) < 1e-12);
    const df = mid.position[2] - mid.focusZ;
    assert.ok(Math.abs(mid.position[0] - df * Math.tan(mid.yaw)) < 1e-9);
  });

  it("slides stall rows off the aisle while the lectern is up", () => {
    assert.equal(stallShift(-1, 0), 0);
    assert.equal(stallShift(1, 0), 0);
    assert.ok(stallShift(-1, 1) === -STALL_ASIDE && stallShift(1, 1) === STALL_ASIDE);
    assert.ok(STALL_ASIDE > 2, "the rows travel, they do not nudge");
    for (const frame of STALL_FRAMES) {
      const leftInner = frame.seats[0].x[1] + stallShift(-1, 1);
      const rightInner = frame.seats[1].x[0] + stallShift(1, 1);
      assert.ok(leftInner <= -LECTERN.x[1], `left row still covers the lectern at ${leftInner}`);
      assert.ok(rightInner >= LECTERN.x[1], `right row still covers the lectern at ${rightInner}`);
    }
    assert.throws(() => stallShift(-1, 1.2), /stall lift/);
  });

  it("fits the stage column inside the viewport while read-along is up", () => {
    const column = (cam: WorldCamera, w: number, h: number): [number, number][] => {
      const top = PROSCENIUM_FACADE.top;
      const raisedBottom = LECTERN.y[0] + LECTERN_RAISE;
      const controls = bannerLayout();
      const play = controls[0]!.rect;
      const box = controls[controls.length - 1]!.rect;
      const points: [number, number, number][] = [
        [-OPENING.x[1], top, PROSCENIUM_Z],
        [OPENING.x[1], top, PROSCENIUM_Z],
        [play.x[0], play.y[1], play.z],
        [box.x[1], box.y[1], box.z],
        [LECTERN.x[0], raisedBottom, LECTERN.z],
        [LECTERN.x[1], raisedBottom, LECTERN.z],
      ];
      return points.map((point) => projectWorld(cam, point, { width: w, height: h }));
    };
    for (const [w, h] of VIEWPORTS) {
      const cam = worldCamera({
        journey: 1, pointer: [0, 0], reducedMotion: true, width: w, height: h, growthSteps: 0, readAlong: 1,
      });
      for (const [x, y] of column(cam, w, h)) {
        assert.ok(x >= 0 && x <= w && y >= 0 && y <= h, `${w}x${h} crops the stage column at ${x},${y}`);
      }
    }
    const wide = view(1);
    const square = worldCamera({
      journey: 1, pointer: [0, 0], reducedMotion: true, width: 1111, height: 1113, growthSteps: 0, readAlong: 1,
    });
    const squareRest = worldCamera({
      journey: 1, pointer: [0, 0], reducedMotion: true, width: 1111, height: 1113, growthSteps: 0,
    });
    assert.ok(wide.position[2] < view(0).position[2]);
    assert.ok(square.position[2] <= squareRest.position[2]);
  });
});

describe("inspection card in the grille window", () => {
  // Measured flat opening of the delivered ticket-window grille.
  const measured = { x0: -3.987, y0: 0.781, x1: 3.925, y1: 5.362 };
  const [x0, y0] = boothMap(measured.x0, measured.y0, 0);
  const [x1, y1] = boothMap(measured.x1, measured.y1, 0);
  const opening = { x0, y0, x1, y1 };
  it("centers the card inside the opening and keeps the buttons on it", () => {
    const placed = placeInspectCard(opening);
    const mid = (a: number, b: number) => (a + b) / 2;
    assert.ok(Math.abs(mid(placed.card.x[0], placed.card.x[1]) - mid(opening.x0, opening.x1)) < 1e-6);
    assert.ok(Math.abs(mid(placed.card.y[0], placed.card.y[1]) - mid(opening.y0, opening.y1)) < 1e-6);
    assert.ok(placed.card.x[0] > opening.x0 && placed.card.x[1] < opening.x1);
    assert.ok(placed.card.y[0] > opening.y0 && placed.card.y[1] < opening.y1);
    const aspect = (placed.card.x[1] - placed.card.x[0]) / (placed.card.y[1] - placed.card.y[0]);
    const authored = (INSPECT.x[1] - INSPECT.x[0]) / (INSPECT.y[1] - INSPECT.y[0]);
    assert.ok(Math.abs(aspect - authored) < 1e-6);
    assert.ok(INSPECT.y[1] > opening.y1, "the authored card rises through the grille frame");
    for (const btn of [placed.back, placed.seat]) {
      assert.ok(btn.x[0] >= placed.card.x[0] && btn.x[1] <= placed.card.x[1]);
      assert.ok(btn.y[0] >= placed.card.y[0] && btn.y[1] <= placed.card.y[1]);
      assert.ok(btn.y[1] < mid(placed.card.y[0], placed.card.y[1]));
    }
    assert.ok(placed.back.x[1] < placed.seat.x[0]);
  });
});

describe("lectern plaque", () => {
  for (const [w, h] of VIEWPORTS) {
    it(`keeps the plaque clear of the floor in ${w}x${h}`, () => {
      const vp = { width: w, height: h };
      for (const pointer of EXTREMES) {
        for (const steps of [0, 4, 12]) {
          const cam = seat(w, h, pointer, steps);
          const plaque = lecternRect(cam, vp, steps);
          if (w / h >= 16 / 9 - 0.02) {
            assert.ok(plaque.left >= 0 && plaque.left + plaque.width <= w, JSON.stringify(plaque));
            assert.ok(plaque.top >= 0 && plaque.top + plaque.height <= h, JSON.stringify(plaque));
          }
          const floor = Math.max(...OPENING.x.map((x) => projectWorld(cam, [x, OPENING.y[0], PROSCENIUM_Z], { width: w, height: h })[1]));
          assert.ok(plaque.top - floor >= 8, `floor clearance ${plaque.top - floor}`);
        }
      }
    });
  }
  it("rests in the pit and rises clear of the front row when read-along is on", () => {
    const rest = lecternWorld(0, 0);
    assert.deepEqual(rest.y, LECTERN.y);
    assert.equal(rest.z, LECTERN.z);
    const raised = lecternWorld(0, 1);
    assert.ok(Math.abs(raised.y[0] - (LECTERN.y[0] + LECTERN_RAISE)) < 1e-9);
    assert.ok(Math.abs(raised.y[1] - (LECTERN.y[1] + LECTERN_RAISE)) < 1e-9);
    assert.ok(LECTERN_RAISE > 0.5, "the reading pose is a real rise, not a nudge");
    const grown = lecternWorld(4, 1);
    assert.equal(grown.y[1], raised.y[1]);
    assert.ok(grown.y[0] < raised.y[0]);
    const screen = (cam: WorldCamera, vp: { width: number; height: number }, rect: Rect3) => {
      const pts = [
        [rect.x[0], rect.y[1]], [rect.x[1], rect.y[1]], [rect.x[0], rect.y[0]], [rect.x[1], rect.y[0]],
      ].map(([x, y]) => projectWorld(cam, [x!, y!, rect.z], vp));
      const xs = pts.map((p) => p[0]);
      const ys = pts.map((p) => p[1]);
      return { left: Math.min(...xs), right: Math.max(...xs), top: Math.min(...ys), bottom: Math.max(...ys) };
    };
    const overlaps = (a: { left: number; right: number; top: number; bottom: number }, b: typeof a) =>
      a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
    for (const [w, h] of VIEWPORTS) {
      const vp = { width: w, height: h };
      const cam = seat(w, h);
      const up = screen(cam, vp, raised);
      const down = screen(cam, vp, rest);
      const floor = projectWorld(cam, [0, OPENING.y[0], PROSCENIUM_Z], vp)[1];
      assert.ok(up.top < down.top && up.bottom < down.bottom, `${w}x${h}: reading pose is above the pit`);
      assert.ok(up.left >= 0 && up.right <= w && up.top >= 0 && up.bottom <= h - 8, `${w}x${h}: raised lectern is clipped ${JSON.stringify(up)}`);
      assert.ok(floor - up.top <= 24, `${w}x${h}: lectern climbs into the stage (${up.top} vs floor ${floor})`);
      for (const patron of PATRONS) {
        assert.ok(!overlaps(up, screen(cam, vp, patron)), `${w}x${h}: raised lectern meets a patron`);
      }
    }
  });

  it("eases the lectern between the pit and the reading pose", () => {
    assert.equal(lecternTravel(0, 0, 1).lift, 0);
    assert.equal(lecternTravel(0, 0, 1).moving, true);
    const quarter = lecternTravel(LECTERN_TRAVEL_MS / 4, 0, 1).lift;
    assert.ok(quarter > 0 && quarter < 0.25, `ease starts slow: ${quarter}`);
    assert.equal(lecternTravel(LECTERN_TRAVEL_MS, 0, 1).lift, 1);
    assert.equal(lecternTravel(LECTERN_TRAVEL_MS, 0, 1).moving, false);
    assert.equal(lecternTravel(0, 1, 0).lift, 1);
    assert.equal(lecternTravel(LECTERN_TRAVEL_MS, 1, 0).lift, 0);
    assert.equal(lecternTravel(LECTERN_TRAVEL_MS / 2, 0.5, 1).lift, 1);
    assert.throws(() => lecternTravel(-1, 0, 1), /lectern elapsed/);
  });

  it("grows the plaque downward and refits instead of shrinking text", () => {
    const vp = { width: 1280, height: 720 };
    const base = lecternRect(seat(1280, 720), vp, 0);
    const grown = lecternRect(seat(1280, 720, [0, 0], 8), vp, 8);
    assert.ok(grown.height / grown.width > base.height / base.width);
    const baseZ = seat(1280, 720).position[2];
    const grownZ = seat(1280, 720, [0, 0], 8).position[2];
    assert.ok(grownZ > baseZ, "a taller plaque refits the camera farther back");
  });
});

describe("playback controls hang on the proscenium banner", () => {
  // Hand-measured from the delivered facade art: the green pediment field
  // spans u 0.044-0.957 and v 0.052-0.211 of a 26 x 15.8 sheet whose top
  // edge is y 12.6. The pillar tops of the opening are at x +-8.
  const FIELD = { x: [-11.86, 11.82], y: [9.27, 11.78] } as const;
  const PILLARS = 8;

  it("keeps the five controls in rail order, in front of the facade", () => {
    const controls = bannerLayout();
    assert.deepEqual(controls.map((c) => c.id), RAIL_CONTROLS.map((c) => c.id));
    assert.equal(BANNER_Z, PROSCENIUM_Z + 0.07);
    for (const { id, rect } of controls) {
      assert.equal(rect.z, BANNER_Z, `${id} stands on the banner plane`);
      assert.ok(rect.z > PROSCENIUM_Z && rect.z - PROSCENIUM_Z < 0.2, `${id} is just in front of the facade`);
    }
  });

  it("rests on the bottom of the banner, between the pillar tops, with the seek as wide as the title", () => {
    assert.deepEqual(BANNER_FIELD.x, [-11.8, 11.8]);
    assert.deepEqual(BANNER_FIELD.y, [9.3, 11.76]);
    const layout = bannerLayout();
    const seek = layout.find((c) => c.id === "seek")!;
    assert.ok(Math.abs(seek.rect.x[1] - seek.rect.x[0] - BANNER_TITLE_WIDTH) < 1e-9, "seek matches PAPER THEATER");
    assert.ok(Math.abs((seek.rect.x[0] + seek.rect.x[1]) / 2) < 1e-9, "seek is centred on the title");
    for (const { id, rect } of layout) {
      assert.ok(rect.x[0] >= -PILLARS && rect.x[1] <= PILLARS, `${id} is between the pillars ${rect.x}`);
      assert.ok(rect.x[0] >= FIELD.x[0] && rect.x[1] <= FIELD.x[1], `${id} x ${rect.x}`);
      assert.equal(rect.y[0], BANNER_ROW_BOTTOM, `${id} sits on the valance cornice`);
      assert.ok(rect.y[0] > 8.9 && rect.y[1] < FIELD.y[1], `${id} overlays the lower banner`);
      const round = id !== "seek";
      if (round) assert.ok(Math.abs(rect.x[1] - rect.x[0] - BANNER_MEDALLION) < 1e-9, `${id} is half the old medallion`);
    }
    const play = layout.find((c) => c.id === "play")!.rect;
    const restart = layout.find((c) => c.id === "restart")!.rect;
    const read = layout.find((c) => c.id === "readalong")!.rect;
    const back = layout.find((c) => c.id === "return")!.rect;
    assert.ok(play.x[1] < restart.x[0] && restart.x[1] < seek.rect.x[0], "transport stands left of the gauge");
    assert.ok(seek.rect.x[1] < read.x[0] && read.x[1] < back.x[0], "house controls stand right of the gauge");
  });

  it("leaves a clear gap between neighbours and keeps one row height", () => {
    const rects = bannerLayout().map(({ rect }) => rect);
    const run = [...rects].sort((a, b) => a.x[0] - b.x[0]);
    for (let i = 1; i < run.length; i++) {
      assert.ok(run[i]!.x[0] - run[i - 1]!.x[1] >= 0.15, `gap ${run[i]!.x[0] - run[i - 1]!.x[1]}`);
    }
    const centres = rects.map((rect) => (rect.y[0] + rect.y[1]) / 2);
    assert.ok(Math.max(...centres) - Math.min(...centres) < 1e-9, "controls share one centre line");
    const heights = rects.map((rect) => rect.y[1] - rect.y[0]);
    assert.ok(Math.max(...heights) - Math.min(...heights) < 1e-9, "the row is one height");
  });

  it("keeps each card at the aspect of its delivered art", () => {
    for (const { id, rect } of bannerLayout()) {
      const art = HOUSE_ART_BRIEF[`banner.${id}`];
      assert.ok(art, `banner.${id} is a brief surface`);
      const world = (rect.x[1] - rect.x[0]) / (rect.y[1] - rect.y[0]);
      const texels = art.deliver_px[0] / art.deliver_px[1];
      assert.ok(Math.abs(world - texels) / texels < 0.01, `${id} world ${world} vs art ${texels}`);
    }
  });

  for (const [w, h] of VIEWPORTS) {
    it(`shows every control, large enough to hit, in ${w}x${h}`, () => {
      const vp = { width: w, height: h };
      const signs = bannerScreen(seat(w, h), vp);
      assert.deepEqual(signs.map((s) => s.id), RAIL_CONTROLS.map((c) => c.id));
      for (const { id, rect } of signs) {
        assert.ok(rect.left >= 0 && rect.left + rect.width <= w, `${id} x ${JSON.stringify(rect)}`);
        assert.ok(rect.top >= 0 && rect.top + rect.height <= h, `${id} y ${JSON.stringify(rect)}`);
        assert.ok(rect.height >= 24, `${id} is ${rect.height}px tall`);
      }
      const rail = railRect(seat(w, h), vp);
      for (const { rect } of signs) {
        assert.ok(rect.left >= rail.left - 1e-6 && rect.left + rect.width <= rail.left + rail.width + 1e-6);
        assert.ok(rect.top >= rail.top - 1e-6 && rect.top + rect.height <= rail.top + rail.height + 1e-6);
      }
      // The banner is above the stage opening, so it never covers the stage.
      const openingTop = projectWorld(seat(w, h), [0, OPENING.y[1], PROSCENIUM_Z], vp)[1];
      assert.ok(rail.top + rail.height <= openingTop, `banner ${rail.top + rail.height} over opening ${openingTop}`);
    });

    it(`moves with the proscenium under pointer parallax in ${w}x${h}`, () => {
      const vp = { width: w, height: h };
      const rest = bannerScreen(seat(w, h), vp);
      const layout = bannerLayout(bannerScale(vp));
      for (const pointer of EXTREMES) {
        const cam = seat(w, h, pointer);
        const moved = bannerScreen(cam, vp);
        for (const [i, sign] of moved.entries()) {
          // The facade point behind this card's left edge: parallax and foreshortening move it, so the card must follow it.
          const edge: [number, number, number] = [layout[i]!.rect.x[0], layout[i]!.rect.y[1], PROSCENIUM_Z];
          const facadeShift = projectWorld(cam, edge, vp)[0] - projectWorld(seat(w, h), edge, vp)[0];
          const was = rest[i]!.rect;
          const shift = sign.rect.left - was.left;
          assert.ok(Math.abs(shift - facadeShift) <= 0.6, `${sign.id} ${shift} vs facade ${facadeShift}`);
          assert.ok(Math.abs(sign.rect.width - was.width) <= was.width * 0.02, `${sign.id} keeps its size`);
        }
      }
    });
  }

  it("keeps the row full size across the seat viewports, including a tall window", () => {
    for (const [w, h] of VIEWPORTS) {
      const scale = bannerScale({ width: w, height: h });
      assert.ok(scale > 0 && scale <= 1, `${w}x${h} scale ${scale}`);
      if (w / h >= 16 / 9 - 0.02) assert.equal(scale, 1, `${w}x${h} shows the whole banner`);
    }
    const tall = bannerScale({ width: 800, height: 1200 });
    assert.equal(tall, 1, "the row sits between the pillars, so a tall window keeps it full size");
  });

  it("shrinks toward the centre, keeps rail order and stays between the pillars", () => {
    for (const scale of [1, 0.75, 0.5]) {
      const layout = bannerLayout(scale);
      assert.deepEqual(layout.map((c) => c.id), RAIL_CONTROLS.map((c) => c.id));
      for (const { id, rect } of layout) {
        assert.ok(rect.x[0] >= -PILLARS && rect.x[1] <= PILLARS, `${id} at ${scale} leaves the pillars`);
        assert.equal(rect.y[0], BANNER_ROW_BOTTOM, `${id} stays on the cornice while it shrinks`);
        const art = HOUSE_ART_BRIEF[`banner.${id}`]!;
        const world = (rect.x[1] - rect.x[0]) / (rect.y[1] - rect.y[0]);
        assert.ok(Math.abs(world - art.deliver_px[0] / art.deliver_px[1]) < 0.01 * world, `${id} keeps its art aspect at ${scale}`);
      }
      const row = layout.map((c) => c.rect);
      const widthOf = (r: Rect3) => r.x[1] - r.x[0];
      assert.ok(Math.abs(widthOf(row[0]!) - BANNER_MEDALLION * scale) < 1e-9, "medallions scale with the layout");
      const seek = layout.find((c) => c.id === "seek")!.rect;
      assert.ok(Math.abs((seek.x[0] + seek.x[1]) / 2) < 1e-6, "the gauge stays centred");
    }
    assert.deepEqual(bannerLayout(), bannerLayout(1));
  });

  it("stays on the banner while the plaque grows the view", () => {
    const vp = { width: 1280, height: 720 };
    for (const steps of [0, 4, 12]) {
      const signs = bannerScreen(seat(1280, 720, [0, 0], steps), vp);
      for (const { id, rect } of signs) {
        assert.ok(rect.left >= 0 && rect.left + rect.width <= 1280 && rect.top >= 0, `${id} at ${steps} steps`);
      }
    }
  });
});

describe("ticket hover zoom", () => {
  it("rests on the booth camera and centres a zoomed ticket", () => {
    const base = lobby(1539, 1360);
    const at: [number, number, number] = [-0.2, 4.4, PAGE_FACE_Z + 0.05];
    const rest = applyTicketZoom(base, { at, amount: 0 });
    assert.deepEqual(rest.position, base.position);
    assert.deepEqual(rest.shift, base.shift);
    const zoomed = applyTicketZoom(base, { at, amount: 1 });
    const restDistance = base.position[2] - at[2];
    const zoomDistance = zoomed.position[2] - at[2];
    assert.ok(zoomDistance < restDistance * 0.6, "the camera moves in on the ticket");
    assert.ok(zoomDistance > 0.3, "the camera stops short of the card");
    const held = ticketZoomIntent({ hovering: true, journey: 0, now: 0, offSince: null, holding: false });
    assert.equal(held.want, 1);
    assert.equal(held.offSince, null);
    const dwell = ticketZoomIntent({ hovering: false, journey: 0, now: 1000, offSince: null, holding: true });
    assert.equal(dwell.want, 1);
    assert.equal(dwell.offSince, 1000);
    const still = ticketZoomIntent({
      hovering: false, journey: 0, now: dwell.offSince + TICKET_ZOOM_HOLD_MS - 1, offSince: dwell.offSince, holding: true,
    });
    assert.equal(still.want, 1, "the zoom holds until the pointer has been away for two seconds");
    const release = ticketZoomIntent({
      hovering: false, journey: 0, now: dwell.offSince + TICKET_ZOOM_HOLD_MS, offSince: dwell.offSince, holding: true,
    });
    assert.equal(release.want, 0);
    const back = ticketZoomIntent({ hovering: true, journey: 0, now: 1500, offSince: 1000, holding: true });
    assert.equal(back.want, 1);
    assert.equal(back.offSince, null, "moving onto a ticket cancels the dwell");
    const left: [number, number, number] = [-0.2, 4.59, 16.98];
    const right: [number, number, number] = [0.2, 4.59, 16.98];
    const resting = ticketPanLock(left, left, null);
    assert.equal(resting.locked, null);
    const started = ticketPanLock(left, right, null);
    assert.deepEqual(started.locked, right);
    const midway: [number, number, number] = [0, 4.59, 16.98];
    const interrupted = ticketPanLock(midway, left, started.locked);
    assert.deepEqual(interrupted.at, right, "a pointer move back does not retarget the pan");
    assert.deepEqual(interrupted.locked, right);
    const abandoned = ticketPanLock(midway, null, started.locked);
    assert.deepEqual(abandoned.at, right, "leaving both tickets does not stop the pan");
    const arrived = ticketPanLock(right, null, started.locked);
    assert.equal(arrived.locked, null);
    const [u, v] = projectWorld(zoomed, at, { width: 1539, height: 1360 });
    assert.ok(Math.abs(u - 1539 / 2) < 2, `ticket x ${u}`);
    assert.ok(Math.abs(v - 1360 / 2) < 2, `ticket y ${v}`);
  });
});

describe("booth to seat fade", () => {
  it("is black at one second and clear again at two, with the switch on the black frame", () => {
    const out = boothSeatCrossfade(BOOTH_SEAT_FADE_MS / 2);
    assert.ok(Math.abs(out.veil - 0.5) < 1e-9);
    assert.equal(out.switched, false);
    const black = boothSeatCrossfade(BOOTH_SEAT_FADE_MS);
    assert.equal(black.veil, 1);
    assert.equal(black.switched, true);
    assert.equal(black.done, false);
    const inn = boothSeatCrossfade(BOOTH_SEAT_FADE_MS + BOOTH_SEAT_FADE_MS / 2);
    assert.ok(Math.abs(inn.veil - 0.5) < 1e-9);
    assert.equal(inn.switched, true);
    const done = boothSeatCrossfade(BOOTH_SEAT_FADE_MS * 2);
    assert.equal(done.veil, 0);
    assert.equal(done.done, true);
  });
});

describe("pointer parallax", () => {
  it("keeps the proscenium plane fixed while nearer planes shift further", () => {
    const vp = { width: 1280, height: 720 };
    const still = seat(1280, 720);
    const moved = seat(1280, 720, [1, 1]);
    const shift = (z: number) =>
      projectWorld(moved, [3, 2, z], vp)[0] - projectWorld(still, [3, 2, z], vp)[0];
    // Seen at an angle the proscenium holds to within a pixel.
    assert.ok(Math.abs(shift(PROSCENIUM_Z)) < 1);
    const zs = MULTIPLANE_LAYERS.map((l) => PROSCENIUM_Z + l.z);
    const shifts = zs.map(shift);
    for (let i = 1; i < shifts.length; i++) assert.ok(shifts[i - 1]! < shifts[i]!, `${zs[i]}`);
    // Near planes move against the camera, far planes with it.
    assert.ok(shifts[0]! < 0 && shifts[shifts.length - 1]! > 0);
  });
  it("stays inside the frozen camera pointer envelope at the seat", () => {
    const cam = seat(1920, 1080, [7, -7]);
    const base = seat(1920, 1080);
    // The side view turns the camera, so the move runs along its own axis.
    assert.ok(Math.abs(Math.hypot(cam.position[0] - base.position[0], cam.position[2] - base.position[2]) - CAMERA.pointer_x) < 1e-9);
    assert.ok(Math.abs(cam.position[1] - base.position[1] + CAMERA.pointer_y) < 1e-9);
  });
  it("is disabled under reduced motion", () => {
    const cam = seat(1280, 720, [1, 1], 0, true);
    assert.deepEqual(cam.position, seat(1280, 720).position);
    assert.deepEqual(cam.shift, [0, 0]);
  });
  it("caps device pixel ratio at 1.5", () => {
    assert.equal(viewportPixelRatio(3), 1.5);
    assert.equal(viewportPixelRatio(1), 1);
    assert.equal(viewportPixelRatio(undefined), 1);
  });
});

describe("box office and aisle journey", () => {
  it("keeps a quarter-turn of the page clear of the stall cards", () => {
    const width = PAGE_EDGE_X - PAGE_HINGE_X;
    const depth = PAGE_FACE_Z - FOYER_WALL_Z;
    const tipZ = PAGE_FACE_Z - Math.hypot(width, depth);
    const last = STALL_ROW_Z[STALL_ROW_Z.length - 1]!;
    assert.ok(tipZ > last, `page tip lands in the stalls at z ${tipZ}`);
    const [grilleLeft] = boothMap(-8, 0, PAGE_FACE_Z);
    const [grilleRight] = boothMap(8, 0, PAGE_FACE_Z);
    assert.ok(PAGE_HINGE_X < grilleLeft && PAGE_EDGE_X > grilleRight, "the page covers the grille");
    assert.ok(PAGE_FACE_Z > FOYER_WALL_Z + 0.02, "the page wall sits behind the grille");
    const sillY = boothMap(0, GRILLE_SILL_Y, PAGE_FACE_Z)[1];
    assert.ok(sillY < boothMap(0, 10.2, PAGE_FACE_Z)[1]);
    for (const [w, h] of VIEWPORTS) {
      const bounds = foyerWallBounds(w, h);
      const cam = worldCamera({ journey: 0, width: w, height: h, growthSteps: 0, pointer: [0, 0], reducedMotion: true });
      for (const [px, py] of [[0, h], [w, h]] as const) {
        const [x, y] = projectWorld(cam, [px === 0 ? bounds.x[0] : bounds.x[1], bounds.y[0], PAGE_FACE_Z], { width: w, height: h });
        assert.ok(y >= h - 1.5, `${w}x${h} floor bottom leaves the view at ${y}`);
        assert.ok(px === 0 ? x <= 1.5 : x >= w - 1.5, `${w}x${h} floor edge at ${x}`);
      }
    }
    for (const slot of TICKET_SLOTS) {
      assert.ok(slot.z < PAGE_FACE_Z - 0.02 && slot.z > FOYER_WALL_Z + 0.02, "tickets sit in the window");
    }
  });
  for (const [w, h] of VIEWPORTS) {
    it(`frames the counter, tickets and card in the lobby at ${w}x${h}`, () => {
      for (const pointer of EXTREMES) {
        const cam = lobby(w, h, pointer);
        for (const slot of TICKET_SLOTS) {
          for (const x of slot.x) for (const y of slot.y) assert.ok(inside(cam, w, h, x, y, slot.z));
        }
        for (const x of INSPECT.x) for (const y of INSPECT.y) assert.ok(inside(cam, w, h, x, y, INSPECT.z));
      }
    });
    it(`turns the page, then eases straight down the aisle at ${w}x${h}`, () => {
      const samples = Array.from({ length: 101 }, (_, i) => worldCamera({
        journey: i / 100, pointer: [0, 0], reducedMotion: true, width: w, height: h, growthSteps: 0,
      }));
      const startZ = samples[0]!.position[2];
      for (let i = 1; i < samples.length; i++) {
        assert.ok(samples[i]!.position[2] <= samples[i - 1]!.position[2] + 1e-6, `camera pulled back at ${i / 100}`);
        assert.ok(samples[i]!.position[0] < 4, `camera left the aisle at x ${samples[i]!.position[0]}`);
      }
      assert.equal(pageTurn(0), 0);
      let prevTurn = 0;
      let crossed = false;
      for (let i = 0; i < samples.length; i++) {
        const turn = pageTurn(i / 100);
        assert.ok(turn + 1e-9 >= prevTurn, "page turn reversed");
        prevTurn = turn;
        if (samples[i]!.position[2] <= PAGE_FACE_Z) {
          assert.ok(turn >= Math.PI * 0.55, `camera reached the page while it still filled the view at ${i / 100}`);
          crossed = true;
        }
      }
      const seatedPastThePage = seat(w, h).position[2] < PAGE_FACE_Z;
      assert.equal(crossed, seatedPastThePage, "the camera crosses the sheet only when the seat is on its stage side");
      assert.ok(pageTurn(1) > Math.PI / 2, "the page finishes past edge-on");
      assert.ok(startZ <= lobby(w, h).position[2] + 1e-6);
      assert.deepEqual(samples[100]!.position, seat(w, h).position);
      assert.ok(Math.abs(lobby(w, h).position[2] - seat(w, h).position[2] - BOOTH_FLIGHT) < 1e-6, "the booth sits one short step behind the seat");
      assert.ok(lobby(w, h).position[2] > PAGE_FACE_Z);
    });
  }
  it("packs three facing stall frames into a short stack and watches them at 64 degrees", () => {
    assert.equal(FOV_Y_DEGREES, 64);
    assert.equal(STALL_ROW_Z.length, 3);
    assert.equal(STALL_FRAMES.length, 3);
    for (let i = 1; i < STALL_ROW_Z.length; i++) {
      const gap = STALL_ROW_Z[i]! - STALL_ROW_Z[i - 1]!;
      assert.ok(gap > 0.8 && gap < 2, `row gap ${gap}`);
    }
    assert.ok(STALL_ROW_Z[0]! > PROSCENIUM_Z + 1.8, "front row clears the rail");
    assert.ok(STALL_ROW_Z[0]! < PROSCENIUM_Z + 6, "front row sits near the proscenium");
    const last = STALL_ROW_Z[STALL_ROW_Z.length - 1]!;
    assert.ok(last - PROSCENIUM_Z < 8, "the stall stack stays short");
    for (const frame of STALL_FRAMES) {
      assert.equal(frame.z, STALL_ROW_Z[STALL_FRAMES.indexOf(frame)]);
      assert.ok(frame.z > PROSCENIUM_Z && frame.z <= last, "frame stays inside the stall stack");
      assert.equal(frame.leftJamb.z, frame.z);
      assert.equal(frame.rightJamb.z, frame.z);
      assert.equal(frame.ceiling.z, frame.z);
      assert.equal(frame.floor.z, frame.z - 0.12);
      assert.ok(frame.floor.z < frame.seats[0].z && frame.floor.z < frame.seats[1].z, "floor sits behind both seat cards");
      assert.deepEqual(frame.leftJamb.y, [-3.2, 12.6]);
      assert.deepEqual(frame.rightJamb.y, [-3.2, 12.6]);
      assert.equal(frame.ceiling.y[1], 12.6);
      assert.ok(frame.leftJamb.x[1] <= frame.seats[0].x[0], "left jamb stays outside the seats");
      assert.ok(frame.rightJamb.x[0] >= frame.seats[1].x[1], "right jamb stays outside the seats");
      assert.ok(frame.ceiling.x[0] >= frame.leftJamb.x[1] && frame.ceiling.x[1] <= frame.rightJamb.x[0], "ceiling band fills the opening");
      assert.ok(frame.floor.x[0] <= frame.leftJamb.x[0] && frame.floor.x[1] >= frame.rightJamb.x[1], "floor band runs wall to wall");
      assert.equal(frame.seats[0].z, frame.z);
      assert.ok(frame.seats[1].z > frame.z, "house-right seats keep their stagger");
    }
    const watched = [[960, 540], [1280, 720], [1920, 1080], [1497, 1360], [2245, 2040]] as const;
    for (const [w, h] of watched) {
      const cam = worldCamera({
        journey: 1, pointer: [0, 0], reducedMotion: true, width: w, height: h, growthSteps: 0,
      });
      assert.equal(cam.fovYDegrees, 64);
      const seatZ = cam.position[2];
      assert.ok(last + 1.2 <= seatZ + 1e-6, `${w}x${h} camera sits on the stalls`);
      assert.ok(FOYER_WALL_Z > seatZ, `${w}x${h} foyer is behind the camera (${seatZ} vs ${FOYER_WALL_Z})`);
      const pageGap = PAGE_FACE_Z - seatZ;
      assert.ok(pageGap > 0.3 && pageGap < 2, `${w}x${h} page stays just behind the seat (gap ${pageGap})`);
      const fullView = seatContainFitDepth(16, 9);
      if (w / h >= 1.05) {
        assert.ok(Math.abs(seatZ - fullView) < 1e-6, `${w}x${h} camera is the full-view zoom`);
      } else {
        assert.ok(seatZ + 1e-6 >= fullView, `${w}x${h} narrow window only eases back from full view`);
      }
      const opening = [
        [OPENING.x[0], OPENING.y[0]],
        [OPENING.x[1], OPENING.y[0]],
        [OPENING.x[0], OPENING.y[1]],
        [OPENING.x[1], OPENING.y[1]],
      ] as const;
      for (const [x, y] of opening) assert.ok(inside(cam, w, h, x, y, PROSCENIUM_Z), `${w}x${h} misses the stage at ${x},${y}`);
      const lobbyZ = worldCamera({
        journey: 0, pointer: [0, 0], reducedMotion: true, width: w, height: h, growthSteps: 0,
      }).position[2];
      assert.ok(lobbyZ > PAGE_FACE_Z + 0.6, `${w}x${h} lobby camera clips the grille`);
    }
  });
  it("keeps front-row patrons on the proscenium side of the stalls", () => {
    assert.equal(PATRONS.length, 2);
    for (const rect of PATRONS) {
      assert.ok(rect.z > PROSCENIUM_Z + 1.8, "patrons clear the orchestra rail");
      assert.ok(rect.z < STALL_ROW_Z[0]!, "patrons sit stage-side of the stalls");
    }
  });
  it("rejects journeys outside [0, 1]", () => {
    assert.throws(() => worldCamera({ journey: 1.5, pointer: [0, 0], reducedMotion: false, width: 10, height: 10, growthSteps: 0 }), /journey/);
  });
});
