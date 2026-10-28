// banner-controls (shell): the playback controls are lithographed paper
// cards that hang on the proscenium banner in world space, not signs pinned
// to the viewport. Checks their pose and picks, the delivered art path and
// its keyed paper rim, the code-painted fallback, state painting and the
// seek pointer. Expectations come from the brief and the layout plan.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import * as THREE from "three";
import { buildHouse } from "../src/house.js";
import type { HouseRig } from "../src/house.js";
import { BANNER_SEEK_TRACK, BANNER_Z, LECTERN, LECTERN_RAISE, LECTERN_TRAVEL_MS, PROSCENIUM_Z, bannerLayout, bannerScale, lecternWorld, stallShift, worldCamera } from "../src/multiplane.js";
import type { PreparedHouseArt, PreparedHouseAsset } from "../src/house-art.js";
import { HOUSE_ART_BRIEF } from "../src/house-brief.js";

type Mesh = import("three").Mesh;
type Object3D = import("three").Object3D;

interface Log {
  texts: string[];
  images: unknown[];
  glows: number[];
  washes: number;
  canvases: { width: number; height: number }[];
  puts: { data: Uint8ClampedArray; w: number; h: number }[];
}

/** A 2D context that records what the controls draw. `scene` fakes decoded art pixels. */
function installCanvas(scene?: (w: number, h: number) => Uint8ClampedArray): { log: Log; restore: () => void } {
  const log: Log = { texts: [], images: [], glows: [], washes: 0, canvases: [], puts: [] };
  let last: { data: Uint8ClampedArray; w: number; h: number } | null = null;
  const noop = () => {};
  const ctx = {
    beginPath: noop, moveTo: noop, lineTo: noop, quadraticCurveTo: noop, bezierCurveTo: noop, arc: noop,
    ellipse: noop, rect: noop, roundRect: noop, closePath: noop, fill: noop, stroke: noop, strokeRect: noop,
    fillRect: noop, clearRect: noop, save: noop, restore: noop, clip: noop, translate: noop, rotate: noop,
    scale: noop, setLineDash: noop, putImageData: noop, drawImage: noop,
    fillText(text: string) { log.texts.push(text); },
    measureText() { return { width: 50 }; },
    createLinearGradient() { return { addColorStop: noop }; },
    createRadialGradient() { return { addColorStop: noop }; },
    getImageData(_x: number, _y: number, w: number, h: number) {
      last = { data: scene ? scene(w, h) : new Uint8ClampedArray(16), w, h };
      return { data: last.data };
    },
    set shadowBlur(value: number) { log.glows.push(value); },
    set globalCompositeOperation(value: string) { if (value === "source-atop") log.washes += 1; },
  } as unknown as CanvasRenderingContext2D;
  (ctx as unknown as { drawImage: (image: unknown) => void }).drawImage = (image) => { log.images.push(image); };
  (ctx as unknown as { putImageData: () => void }).putImageData = () => {
    if (last) log.puts.push(last);
  };
  const prev = (globalThis as unknown as { document?: unknown }).document;
  (globalThis as unknown as { document?: unknown }).document = {
    createElement: (tag: string) => {
      if (tag !== "canvas") return {};
      const canvas = { width: 0, height: 0, getContext: () => ctx };
      log.canvases.push(canvas);
      return canvas;
    },
  };
  return { log, restore: () => { (globalThis as unknown as { document?: unknown }).document = prev; } };
}

const walk = (rig: HouseRig, visit: (node: Object3D) => void): void => {
  const go = (node: Object3D): void => {
    visit(node);
    node.children.forEach(go);
  };
  rig.nodes.forEach(go);
};
const rails = (rig: HouseRig): Mesh[] => rig.pickables.filter((m) => String(m.userData.pick).startsWith("rail:"));
const rail = (rig: HouseRig, id: string): Mesh => {
  const found = rails(rig).find((m) => m.userData.pick === `rail:${id}`);
  assert.ok(found, `rail:${id} is pickable`);
  return found;
};
const worldPos = (mesh: Object3D): [number, number, number] => {
  const sum: [number, number, number] = [0, 0, 0];
  for (let node: Object3D | null = mesh; node; node = node.parent) {
    sum[0] += node.position.x;
    sum[1] += node.position.y;
    sum[2] += node.position.z;
  }
  return sum;
};
const shown = (mesh: Object3D): boolean => {
  for (let node: Object3D | null = mesh; node; node = node.parent) if (!node.visible) return false;
  return true;
};
const size = (mesh: Mesh): [number, number] => {
  const p = (mesh.geometry as import("three").PlaneGeometry).parameters;
  return [p.width * mesh.scale.x, p.height * mesh.scale.y];
};
const idle = { enabled: false, playing: false, readAlong: false, progress: 0, fullscreen: false } as const;
const cam = (journey: number, pointer: [number, number]) =>
  worldCamera({ journey, pointer, reducedMotion: false, width: 1280, height: 720, growthSteps: 0 });

describe("banner controls: pose and picks", () => {
  it("hangs five pickable cards on the banner plane, facing the audience", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      const meshes = rails(rig);
      assert.deepEqual(meshes.map((m) => m.userData.pick).sort(), ["rail:play", "rail:restart", "rail:readalong", "rail:return", "rail:seek"].sort());
      for (const { id, rect } of bannerLayout()) {
        const mesh = rail(rig, id);
        const [x, y, z] = worldPos(mesh);
        assert.ok(Math.abs(x - (rect.x[0] + rect.x[1]) / 2) < 1e-6, `${id} x`);
        assert.ok(Math.abs(y - (rect.y[0] + rect.y[1]) / 2) < 1e-6, `${id} y`);
        assert.equal(z, BANNER_Z, `${id} z`);
        assert.ok(z > PROSCENIUM_Z, `${id} stands in front of the facade`);
        assert.equal(mesh.rotation.x, 0);
        assert.equal(mesh.rotation.y, 0);
        const [w, h] = size(mesh);
        assert.ok(w >= rect.x[1] - rect.x[0] - 1e-6 && w <= (rect.x[1] - rect.x[0]) * 1.25, `${id} width ${w}`);
        assert.ok(h >= rect.y[1] - rect.y[0] - 1e-6 && h <= (rect.y[1] - rect.y[0]) * 1.5, `${id} height ${h}`);
      }
    } finally {
      restore();
    }
  });

  it("never follows the camera: the cards are world cards, not viewport signs", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]));
      const before = rails(rig).map((m) => worldPos(m));
      const scaleBefore = rails(rig).map((m) => size(m));
      rig.update(1, 1280, 720, 0, cam(1, [1, -1]));
      rig.update(1, 1920, 1080, 4, cam(1, [-1, 1]));
      assert.deepEqual(rails(rig).map((m) => worldPos(m)), before);
      assert.deepEqual(rails(rig).map((m) => size(m)), scaleBefore);
    } finally {
      restore();
    }
  });

  it("keeps the row full size in a tall window and does not follow the camera", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      const tallCam = (pointer: [number, number]) =>
        worldCamera({ journey: 1, pointer, reducedMotion: false, width: 800, height: 1200, growthSteps: 0 });
      rig.update(1, 800, 1200, 0, tallCam([0, 0]));
      const scale = bannerScale({ width: 800, height: 1200 });
      assert.equal(scale, 1, "a tall window still shows the row between the pillars");
      for (const { id, rect } of bannerLayout(scale)) {
        const mesh = rail(rig, id);
        const [x, y, z] = worldPos(mesh);
        assert.ok(Math.abs(x - (rect.x[0] + rect.x[1]) / 2) < 1e-6, `${id} x`);
        assert.ok(Math.abs(y - (rect.y[0] + rect.y[1]) / 2) < 1e-6, `${id} y`);
        assert.equal(z, BANNER_Z);
        const [w] = size(mesh);
        assert.ok(w <= (rect.x[1] - rect.x[0]) * 1.25, `${id} keeps its card width`);
      }
      const before = rails(rig).map((m) => worldPos(m));
      rig.update(1, 800, 1200, 0, tallCam([1, -1]));
      assert.deepEqual(rails(rig).map((m) => worldPos(m)), before, "pointer parallax does not move or resize the cards");
      rig.update(1, 1280, 720, 0);
      for (const { id, rect } of bannerLayout()) {
        assert.ok(Math.abs(worldPos(rail(rig, id))[0] - (rect.x[0] + rect.x[1]) / 2) < 1e-6, `${id} returns to full size in a wide window`);
      }
    } finally {
      restore();
    }
  });

  it("is only live at the seat: hidden in the box office so rays cannot reach it", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      rig.update(0, 1280, 720, 0);
      for (const mesh of rails(rig)) assert.equal(shown(mesh), false, `${mesh.userData.pick} hidden at the box office`);
      rig.update(1, 1280, 720, 0);
      for (const mesh of rails(rig)) assert.equal(shown(mesh), true, `${mesh.userData.pick} shown at the seat`);
    } finally {
      restore();
    }
  });

  it("maps a seek pick across the engraved scale of the gauge art", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      const mesh = rail(rig, "seek");
      const track = mesh.userData.track as [number, number];
      assert.ok(track && track[0] > 0 && track[1] < 1 && track[0] < track[1]);
      const rect = bannerLayout().find((c) => c.id === "seek")!.rect;
      const [w] = size(mesh);
      const worldAt = (u: number) => mesh.position.x + (u - 0.5) * w;
      const art = (u: number) => rect.x[0] + u * (rect.x[1] - rect.x[0]);
      assert.ok(Math.abs(worldAt(track[0]) - art(BANNER_SEEK_TRACK[0])) < 1e-6, "track start on the scale");
      assert.ok(Math.abs(worldAt(track[1]) - art(BANNER_SEEK_TRACK[1])) < 1e-6, "track end on the scale");
    } finally {
      restore();
    }
  });
});

describe("banner controls: seek pointer", () => {
  it("slides along the scale with the playhead and hides while the gauge is dead", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      rig.update(1, 1280, 720, 0);
      const needles: Mesh[] = [];
      walk(rig, (node) => { if (node.userData.role === "seek-needle") needles.push(node as Mesh); });
      assert.equal(needles.length, 1);
      const needle = needles[0]!;
      const rect = bannerLayout().find((c) => c.id === "seek")!.rect;
      const at = (p: number) => rect.x[0] + (BANNER_SEEK_TRACK[0] + (BANNER_SEEK_TRACK[1] - BANNER_SEEK_TRACK[0]) * p) * (rect.x[1] - rect.x[0]);
      rig.setControls({ ...idle, progress: 0.5 });
      assert.equal(needle.visible, false, "no performance, no pointer");
      for (const p of [0, 0.25, 0.5, 1]) {
        rig.setControls({ ...idle, enabled: true, progress: p });
        assert.equal(needle.visible, true);
        assert.ok(Math.abs(worldPos(needle)[0] - at(p)) < 1e-6, `pointer at ${p}`);
        assert.ok(worldPos(needle)[2] > BANNER_Z, "the pointer rides in front of the gauge card");
      }
      rig.setControls({ ...idle, enabled: true, progress: 2 });
      assert.ok(Math.abs(worldPos(needle)[0] - at(1)) < 1e-6, "progress clamps to the scale end");
    } finally {
      restore();
    }
  });
});

describe("banner controls: painting", () => {
  it("typesets the state on the ribbons in the bundled serif", () => {
    const { log, restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      log.texts.length = 0;
      rig.setControls({ ...idle, rehearsal: true });
      for (const label of ["PLAY", "RESTART", "SEEK", "READ-ALONG", "BOX OFFICE"]) {
        assert.ok(log.texts.includes(label), `${label} typeset (${log.texts.join("|")})`);
      }
      log.texts.length = 0;
      rig.setControls({ ...idle, rehearsal: true, playing: true });
      assert.ok(log.texts.includes("PAUSE") && !log.texts.includes("PLAY"), "play shows PAUSE while playing");
      assert.ok(!log.texts.includes("READ-ALONG OFF"), "the old plate wording is gone");
    } finally {
      restore();
    }
  });

  it("dims what is not live and wakes it with a performance", () => {
    const { log, restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      log.washes = 0;
      rig.setControls({ ...idle, rehearsal: true });
      assert.equal(log.washes, 2, "rehearsal wakes only the play card: restart and seek stay dim");
      log.washes = 0;
      rig.setControls({ ...idle, enabled: true });
      assert.equal(log.washes, 0, "a mounted performance wakes every card");
      log.washes = 0;
      rig.setControls({ ...idle });
      assert.equal(log.washes, 3, "play, restart and seek are dim without a performance or rehearsal");
    } finally {
      restore();
    }
  });

  it("glows the hovered card with a halo drawn from its own silhouette", () => {
    const { log, restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      rig.setControls({ ...idle, rehearsal: true });
      log.glows.length = 0;
      rig.setHighlight("rail:play");
      assert.ok(log.glows.some((g) => g > 0), "halo blur set for the lit card");
      log.glows.length = 0;
      rig.setHighlight(null);
      assert.ok(!log.glows.some((g) => g > 0), "the halo clears with the hover");
    } finally {
      restore();
    }
  });

  it("falls back to code-painted medallions at the brief's delivered sizes", () => {
    const { log, restore } = installCanvas();
    try {
      buildHouse(THREE);
      const dims = (id: string) => HOUSE_ART_BRIEF[id]!.deliver_px;
      for (const id of ["banner.play", "banner.restart", "banner.readalong", "banner.return", "banner.seek"]) {
        const [w, h] = dims(id);
        assert.ok(log.canvases.some((c) => c.width === w && c.height === h), `${id} fallback ${w}x${h}`);
      }
    } finally {
      restore();
    }
  });

  it("draws the delivered lithographs and keeps their cream die-cut rim", () => {
    // Fake decoded art: mid-grey ground, a 24px cream paper rim, a green field inside.
    const scene = (w: number, h: number): Uint8ClampedArray => {
      const data = new Uint8ClampedArray(w * h * 4);
      for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++) {
          const edge = Math.min(x, y, w - 1 - x, h - 1 - y);
          const rgb = edge < 12 ? [128, 128, 128] : edge < 36 ? [232, 220, 190] : [40, 80, 56];
          data.set([...rgb, 255], (y * w + x) * 4);
        }
      }
      return data;
    };
    const { log, restore } = installCanvas(scene);
    try {
      const ids = ["banner.play", "banner.restart", "banner.readalong", "banner.return", "banner.seek"];
      const images = new Map(ids.map((id) => [id, { id }]));
      const assets = new Map<string, PreparedHouseAsset>(ids.map((id) => [id, {
        id,
        url: `${id}.png`,
        sha256: "0".repeat(64) as PreparedHouseAsset["sha256"],
        deliver_px: HOUSE_ART_BRIEF[id]!.deliver_px,
        alpha: "cutout",
        nine_slice_px: null,
        regions: null,
        bytes: new Uint8Array(),
        image: images.get(id) as unknown as CanvasImageSource,
      }]));
      const houseArt: PreparedHouseArt = { assets };
      buildHouse(THREE, { houseArt });
      for (const id of ids) assert.ok(log.images.includes(images.get(id)), `${id} art drawn`);
      const keyed = log.puts.filter((p) => p.w === 512 && p.h === 512);
      assert.equal(keyed.length, 4, "four medallions were keyed");
      for (const { data, w, h } of keyed) {
        const px = (x: number, y: number) => data[(y * w + x) * 4 + 3]!;
        assert.equal(px(2, 2), 0, "the grey ground is cleared");
        assert.equal(px(24, h / 2), 255, "the cream paper rim stays");
        assert.equal(px(w / 2, h / 2), 255, "the picture stays");
      }
    } finally {
      restore();
    }
  });
});

describe("read-along lectern", () => {
  const lectern = (rig: HouseRig): Mesh => {
    let found: Mesh | null = null;
    for (const node of rig.nodes) node.traverse((child) => {
      if (child.userData.role === "lectern") found = child as Mesh;
    });
    assert.ok(found, "lectern card");
    return found;
  };

  it("rises into the reading pose when read-along turns on and returns to the pit when it turns off", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      const mesh = lectern(rig);
      const restY = (LECTERN.y[0] + LECTERN.y[1]) / 2;
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: 0, reduced: false });
      assert.ok(Math.abs(worldPos(mesh)[1] - restY) < 1e-6);
      rig.setControls({ ...idle, readAlong: true });
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: 0, reduced: false });
      assert.ok(Math.abs(worldPos(mesh)[1] - restY) < 1e-6, "the rise starts from the pit");
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: LECTERN_TRAVEL_MS, reduced: false });
      const up = lecternWorld(0, 1);
      assert.ok(Math.abs(worldPos(mesh)[1] - (up.y[0] + up.y[1]) / 2) < 1e-6);
      assert.equal(rig.lecternLift(), 1);
      rig.setControls({ ...idle, readAlong: false });
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: LECTERN_TRAVEL_MS, reduced: true });
      assert.ok(Math.abs(worldPos(mesh)[1] - restY) < 1e-6);
      assert.equal(rig.lecternLift(), 0);
    } finally {
      restore();
    }
  });

  it("parts the stall rows with the lectern and brings them back", () => {
    const { restore } = installCanvas();
    try {
      const rig = buildHouse(THREE);
      const seats: Mesh[] = [];
      for (const node of rig.nodes) node.traverse((child) => {
        if (child.userData.role === "stall-seat") seats.push(child as Mesh);
      });
      assert.equal(seats.length, 6);
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: 0, reduced: false });
      const rest = seats.map((mesh) => worldPos(mesh)[0]);
      rig.setControls({ ...idle, readAlong: true });
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: 0, reduced: false });
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: LECTERN_TRAVEL_MS, reduced: false });
      seats.forEach((mesh, index) => {
        const side = mesh.userData.side as -1 | 1;
        const dx = worldPos(mesh)[0] - rest[index]!;
        assert.ok(Math.abs(dx - stallShift(side, 1)) < 1e-9);
        assert.ok(side < 0 ? dx < 0 : dx > 0);
      });
      rig.syncReadAlong({ now: LECTERN_TRAVEL_MS, reduced: false });
      assert.equal(rig.lecternLift(), 1);
      rig.setControls({ ...idle, readAlong: false });
      rig.update(1, 1280, 720, 0, cam(1, [0, 0]), { now: LECTERN_TRAVEL_MS + 1, reduced: true });
      seats.forEach((mesh, index) => assert.ok(Math.abs(worldPos(mesh)[0] - rest[index]!) < 1e-6));
    } finally {
      restore();
    }
  });
});
