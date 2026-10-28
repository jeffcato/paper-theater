// house-art tests: manifest loader fail-closed rules, brief geometry conformance,
// mid-grey keying / aperture processing, and code-painted house fallback.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import * as THREE from "three";
import {
  loadHouseArtManifest,
  sha256Hex,
  HOUSE_ART_BRIEF,
  isMidGrey,
  keyOutMidGrey,
  cutAperture,
  detectFlatAperture,
  cutConnectedFlat,
  fillCurtainVelvet,
  detectInteriorOpening,
  measureNamePlate,
  paperEdgeInset,
  keyOutBorderGround,
  keyOutBorderGrey,
  extrudeCutoutColor,
  containFit,
} from "../src/index.js";
import { buildHouse } from "../src/house.js";
import { FOYER_VIEW, FOYER_WALL_Z, GRILLE_CARD, GRILLE_SILL_Y, PAGE_FACE_Z, PROSCENIUM_Z, boothMap, worldCamera } from "../src/multiplane.js";
import type { HouseArtManifest } from "../src/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..", "..");
const briefPath = join(root, "docs", "implementation", "player", "immersive-art-brief.json");
const wave2Dir = join(root, "fixtures", "generation", "immersive-house-test-v2");

function makeTestPng(width: number, height: number): Uint8Array {
  const buf = new Uint8Array(33);
  buf.set([137, 80, 78, 71, 13, 10, 26, 10]); // PNG signature
  const view = new DataView(buf.buffer);
  view.setUint32(8, 13); // IHDR chunk length
  buf[12] = 73; buf[13] = 72; buf[14] = 68; buf[15] = 82; // IHDR
  view.setUint32(16, width);
  view.setUint32(20, height);
  buf[24] = 8; // 8 bits
  buf[25] = 6; // RGBA
  buf[26] = 0; buf[27] = 0; buf[28] = 0;
  view.setUint32(29, 0); // crc
  return buf;
}

describe("house art: brief geometry and source of truth", () => {
  it("authoritative brief defines all 37 surfaces matching immersive-art-brief.json", () => {
    assert.ok(existsSync(briefPath), "immersive-art-brief.json exists");
    const briefJson = JSON.parse(readFileSync(briefPath, "utf-8"));
    assert.equal(briefJson.assets.length, 37);
    assert.equal(Object.keys(HOUSE_ART_BRIEF).length, 37);

    for (const a of briefJson.assets) {
      const brief = HOUSE_ART_BRIEF[a.id];
      assert.ok(brief, `missing asset in HOUSE_ART_BRIEF: ${a.id}`);
      assert.deepEqual(brief.deliver_px, a.deliver_px, `${a.id} deliver_px`);
      assert.equal(brief.alpha, a.alpha, `${a.id} alpha`);
      assert.deepEqual(brief.nine_slice_px, a.nine_slice_px, `${a.id} nine_slice_px`);
      assert.equal(brief.optional, a.optional, `${a.id} optional`);
    }
  });

  it("replaces the brass rail art with the five banner lithographs", () => {
    for (const id of ["rail.plate", "rail.lever", "rail.gauge", "rail.needle"]) {
      assert.equal(HOUSE_ART_BRIEF[id], undefined, `${id} is retired with the brass rail`);
    }
    const sizes: Record<string, readonly [number, number]> = {
      "banner.play": [512, 512],
      "banner.restart": [512, 512],
      "banner.readalong": [512, 512],
      "banner.return": [512, 512],
      "banner.seek": [1280, 240],
    };
    for (const [id, px] of Object.entries(sizes)) {
      const brief = HOUSE_ART_BRIEF[id];
      assert.ok(brief, `${id} is a brief surface`);
      assert.deepEqual(brief.deliver_px, px, `${id} deliver_px`);
      assert.equal(brief.alpha, "cutout", `${id} is a die-cut piece`);
      assert.equal(brief.optional, true, `${id} falls back to code painting`);
      assert.equal(brief.group, "Banner controls");
      assert.ok(brief.regions && "text" in brief.regions, `${id} names its ribbon region`);
    }
    assert.ok("track" in HOUSE_ART_BRIEF["banner.seek"]!.regions!, "the gauge names its scale track");
  });

  it("loads the delivered banner lithographs hash-bound against their provenance records", async () => {
    const records = JSON.parse(readFileSync(join(wave2Dir, "banner-art.records.json"), "utf-8")) as {
      asset: string; file: string; raw_file: string; raw_sha256: string; deliver_px: [number, number]; file_sha256: string;
    }[];
    assert.deepEqual(records.map((r) => r.asset).sort(), ["banner.play", "banner.readalong", "banner.restart", "banner.return", "banner.seek"]);
    const files = new Map<string, Uint8Array>();
    const manifestAssets: Record<string, { url: string; sha256: string; deliver_px: [number, number] }> = {};
    for (const rec of records) {
      const bytes = readFileSync(join(wave2Dir, rec.file));
      files.set(rec.file, bytes);
      assert.equal(await sha256Hex(bytes), rec.file_sha256, `${rec.asset} digest matches its record`);
      assert.equal(await sha256Hex(readFileSync(join(wave2Dir, rec.raw_file))), rec.raw_sha256, `${rec.asset} raw sheet kept`);
      manifestAssets[rec.asset] = { url: rec.file, sha256: rec.file_sha256, deliver_px: rec.deliver_px };
    }
    const manifestBytes = new TextEncoder().encode(JSON.stringify({
      schema_version: 1, brief: "immersive-house-art-v1", diagnostic: true, accepted: false, assets: manifestAssets,
    }));
    const loaded = await loadHouseArtManifest(manifestBytes, async (path: string) => files.get(path)!);
    assert.equal(loaded.assets.size, 5);
    for (const id of Object.keys(manifestAssets)) assert.equal(loaded.assets.get(id)?.alpha, "cutout");
  });
});

describe("house art loader: fail-closed contract", () => {
  it("bad digest rejected before render; never partially applies", async () => {
    const png = makeTestPng(2048, 1245);
    const correctHash = await sha256Hex(png);
    const badHash = "f".repeat(64);

    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      diagnostic: true,
      assets: {
        "house.facade": {
          url: "house.facade.png",
          sha256: badHash,
          deliver_px: [2048, 1245],
        },
      },
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const read = async () => png;

    await assert.rejects(
      loadHouseArtManifest(manifestBytes, read),
      /house_art_digest_mismatch/,
    );
  });

  it("geometry mismatch rejected: wrong deliver_px in manifest", async () => {
    const png = makeTestPng(2048, 1245);
    const hash = await sha256Hex(png);

    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      assets: {
        "house.facade": {
          url: "house.facade.png",
          sha256: hash,
          deliver_px: [1000, 500], // Wrong! Brief expects [2048, 1245]
        },
      },
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const read = async () => png;

    await assert.rejects(
      loadHouseArtManifest(manifestBytes, read),
      /house_art_geometry_mismatch/,
    );
  });

  it("geometry mismatch rejected: PNG header dimensions differ from brief", async () => {
    const wrongPng = makeTestPng(1000, 500); // Deliver size mismatch in actual file
    const hash = await sha256Hex(wrongPng);

    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      assets: {
        "house.facade": {
          url: "house.facade.png",
          sha256: hash,
          deliver_px: [2048, 1245],
        },
      },
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const read = async () => wrongPng;

    await assert.rejects(
      loadHouseArtManifest(manifestBytes, read),
      /PNG dimension mismatch|house_art_geometry_mismatch/,
    );
  });

  it("unknown asset rejected: asset not in brief", async () => {
    const png = makeTestPng(100, 100);
    const hash = await sha256Hex(png);

    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      assets: {
        "house.unknown-card": {
          url: "unknown.png",
          sha256: hash,
          deliver_px: [100, 100],
        },
      },
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const read = async () => png;

    await assert.rejects(
      loadHouseArtManifest(manifestBytes, read),
      /unknown_house_art_asset/,
    );
  });

  it("missing asset file rejected: read failure throws and never mounts", async () => {
    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      assets: {
        "house.facade": {
          url: "house.facade.png",
          sha256: "0".repeat(64),
          deliver_px: [2048, 1245],
        },
      },
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const read = async () => {
      throw new Error("file not found on disk");
    };

    await assert.rejects(
      loadHouseArtManifest(manifestBytes, read),
      /house_art_unavailable/,
    );
  });

  it("manifest digest mismatch rejected when expected digest supplied", async () => {
    const manifest: HouseArtManifest = {
      schema_version: 1,
      assets: {},
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const read = async () => new Uint8Array();

    await assert.rejects(
      loadHouseArtManifest(manifestBytes, "0".repeat(64), read),
      /House art manifest digest mismatch/,
    );
  });
});

describe("house art loader: missing asset falls back", () => {
  it("loads a valid partial manifest and falls back for unmanifested assets", async () => {
    const facadePng = makeTestPng(2048, 1245);
    const facadeHash = await sha256Hex(facadePng);

    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      diagnostic: true,
      assets: {
        "house.facade": {
          url: "house.facade.png",
          sha256: facadeHash,
          deliver_px: [2048, 1245],
        },
      },
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const files: Record<string, Uint8Array> = {
      "house.facade.png": facadePng,
    };
    const read = async (path: string) => {
      const f = files[path];
      if (!f) throw new Error("not found");
      return f;
    };

    const loaded = await loadHouseArtManifest(manifestBytes, read);
    assert.equal(loaded.assets.size, 1);
    assert.ok(loaded.assets.has("house.facade"));
    assert.equal(loaded.assets.get("house.facade")?.deliver_px[0], 2048);
    assert.equal(loaded.assets.get("house.facade")?.deliver_px[1], 1245);
    assert.equal(loaded.assets.has("lobby.grille"), false);
  });

  it("buildHouse uses manifest art for present assets and falls back for missing assets", async () => {
    const facadePng = makeTestPng(2048, 1245);
    const facadeHash = await sha256Hex(facadePng);

    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      diagnostic: true,
      assets: {
        "house.facade": {
          url: "house.facade.png",
          sha256: facadeHash,
          deliver_px: [2048, 1245],
        },
      },
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const loaded = await loadHouseArtManifest(manifestBytes, async () => facadePng);

    const createdTextures: { width: number; height: number }[] = [];
    const fakeCtx = {
      beginPath() {}, moveTo() {}, lineTo() {}, quadraticCurveTo() {}, bezierCurveTo() {},
      arc() {}, ellipse() {}, rect() {}, roundRect() {}, closePath() {}, fill() {}, stroke() {},
      strokeRect() {}, fillRect() {}, clearRect() {}, save() {}, restore() {}, clip() {},
      translate() {}, rotate() {}, scale() {}, setLineDash() {}, fillText() {},
      measureText() { return { width: 50 }; },
      createLinearGradient() { return { addColorStop() {} }; },
      createRadialGradient() { return { addColorStop() {} }; },
      getImageData() { return { data: new Uint8ClampedArray(16) }; },
      putImageData() {}, drawImage() {},
    } as unknown as CanvasRenderingContext2D;

    const prevDoc = (globalThis as unknown as { document?: unknown }).document;
    (globalThis as unknown as { document?: unknown }).document = {
      createElement: (tag: string) => {
        if (tag === "canvas") {
          const c = {
            width: 0,
            height: 0,
            getContext: () => fakeCtx,
          };
          createdTextures.push(c);
          return c;
        }
        return {};
      },
    };

    try {
      const rig = buildHouse(THREE, { houseArt: loaded });
      assert.ok(rig.nodes.length >= 3);
      // house.facade is in the manifest: texture was created with deliver_px [2048, 1245]
      const facadeTex = createdTextures.find((t) => t.width === 2048 && t.height === 1245);
      assert.ok(facadeTex, "facade used deliver_px 2048x1245 from manifest art");

      // lobby.grille is NOT in manifest: fallback size [1600, 1120] is used
      const grilleTex = createdTextures.find((t) => t.width === 1600 && t.height === 1120);
      assert.ok(grilleTex, "unmanifested grille fell back to 1600x1120 code-painted size");

      // house.curtain is NOT in manifest: fallback size [512, 576] is used
      const curtainTex = createdTextures.find((t) => t.width === 512 && t.height === 576);
      assert.ok(curtainTex, "unmanifested curtain fell back to 512x576 code-painted size");
    } finally {
      (globalThis as unknown as { document?: unknown }).document = prevDoc;
    }
  });

  it("loads actual Wave 2 diagnostic test batch (facade v2, grille, wallpaper, drape-left)", async () => {
    if (!existsSync(wave2Dir)) return;

    const facadeFile = join(wave2Dir, "house.facade-v2.png");
    const grilleFile = join(wave2Dir, "lobby-grille.png");
    const wallpaperFile = join(wave2Dir, "lobby-wallpaper.png");
    const drapeFile = join(wave2Dir, "lobby-drape-left.png");
    if (!existsSync(facadeFile) || !existsSync(grilleFile)) return;

    const facadeBytes = readFileSync(facadeFile);
    const grilleBytes = readFileSync(grilleFile);
    const wallpaperBytes = readFileSync(wallpaperFile);
    const drapeBytes = readFileSync(drapeFile);

    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      diagnostic: true,
      accepted: false,
      assets: {
        "house.facade": {
          url: "house.facade-v2.png",
          sha256: await sha256Hex(facadeBytes),
          deliver_px: [2048, 1245],
        },
        "lobby.grille": {
          url: "lobby-grille.png",
          sha256: await sha256Hex(grilleBytes),
          deliver_px: [2560, 1792],
        },
        "lobby.wallpaper": {
          url: "lobby-wallpaper.png",
          sha256: await sha256Hex(wallpaperBytes),
          deliver_px: [512, 512],
        },
        "lobby.drape-left": {
          url: "lobby-drape-left.png",
          sha256: await sha256Hex(drapeBytes),
          deliver_px: [352, 2112],
        },
      },
    };

    const files: Record<string, Uint8Array> = {
      "house.facade-v2.png": facadeBytes,
      "lobby-grille.png": grilleBytes,
      "lobby-wallpaper.png": wallpaperBytes,
      "lobby-drape-left.png": drapeBytes,
    };
    const read = async (path: string) => {
      const f = files[path];
      if (!f) throw new Error("not found");
      return f;
    };

    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const loaded = await loadHouseArtManifest(manifestBytes, read);
    assert.equal(loaded.assets.size, 4);
    assert.ok(loaded.assets.has("house.facade"));
    assert.ok(loaded.assets.has("lobby.grille"));
    assert.ok(loaded.assets.has("lobby.wallpaper"));
    assert.ok(loaded.assets.has("lobby.drape-left"));
    // Other assets are missing from manifest -> fall back to code-painted
    assert.equal(loaded.assets.has("house.curtain"), false);
    assert.equal(loaded.assets.has("house.lectern"), false);
  });
});

describe("house art processing: mid-grey keying and aperture cutting", () => {
  it("identifies flat mid-grey within tolerance and distinguishes colors", () => {
    // Mid-grey values near 128
    assert.equal(isMidGrey(128, 128, 128), true);
    assert.equal(isMidGrey(126, 126, 126), true);
    assert.equal(isMidGrey(131, 126, 123), true);
    assert.equal(isMidGrey(135, 128, 125), true);

    // Non-grey: carmine / crimson, gold, black, cream paper
    assert.equal(isMidGrey(143, 29, 31), false, "crimson");
    assert.equal(isMidGrey(201, 162, 74), false, "gold");
    assert.equal(isMidGrey(20, 13, 8), false, "dark shadow");
    assert.equal(isMidGrey(239, 226, 194), false, "paper");
  });

  it("keys out mid-grey pixels to alpha 0 while preserving illustrated artwork", () => {
    // 2x2 RGBA buffer
    // pixel 0: mid-grey (128, 128, 128, 255) -> keyed
    // pixel 1: crimson (143, 29, 31, 255) -> kept
    // pixel 2: dark ink (20, 13, 8, 255) -> kept
    // pixel 3: near-grey (126, 126, 126, 255) -> keyed
    const data = new Uint8Array([
      128, 128, 128, 255,
      143, 29, 31, 255,
      20, 13, 8, 255,
      126, 126, 126, 255,
    ]);
    keyOutMidGrey(data, 2, 2);

    assert.equal(data[3], 0, "pixel 0 alpha keyed");
    assert.equal(data[7], 255, "pixel 1 alpha kept");
    assert.equal(data[11], 255, "pixel 2 alpha kept");
    assert.equal(data[15], 0, "pixel 3 alpha keyed");
  });

  it("cuts aperture rectangle cleanly to alpha 0", () => {
    // 10x10 buffer filled with 255 alpha
    const W = 10;
    const H = 10;
    const data = new Uint8Array(W * H * 4);
    for (let i = 0; i < data.length; i += 4) {
      data[i] = 200;
      data[i + 1] = 150;
      data[i + 2] = 100;
      data[i + 3] = 255;
    }

    // Cut aperture from u: 0.2..0.8, v: 0.3..0.9
    cutAperture(data, W, H, 0.2, 0.3, 0.8, 0.9);

    // Inside rect: x=5, y=5 -> index (5*10 + 5)*4 = 220
    assert.equal(data[(5 * W + 5) * 4 + 3], 0, "inside aperture alpha is 0");

    // Outside rect: x=1, y=1 -> index (1*10 + 1)*4 = 44
    assert.equal(data[(1 * W + 1) * 4 + 3], 255, "outside aperture alpha is 255");
  });

  it("measures a flat booth window instead of the nominal brief rectangle", () => {
    const W = 200;
    const H = 140;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let i = 0; i < data.length; i += 4) {
      data[i] = 120;
      data[i + 1] = 72;
      data[i + 2] = 36;
      data[i + 3] = 255;
    }
    const x0 = 50;
    const y0 = 60;
    const x1 = 148;
    const y1 = 118;
    for (let y = y0; y < y1; y++) {
      for (let x = x0; x < x1; x++) {
        const i = (y * W + x) * 4;
        data[i] = 160;
        data[i + 1] = 159;
        data[i + 2] = 158;
      }
    }
    data[10] = 150;
    data[11] = 150;
    data[12] = 150;
    const found = detectFlatAperture(data, W, H);
    assert.ok(found);
    assert.ok(Math.abs(found.u0 - x0 / W) < 0.02);
    assert.ok(Math.abs(found.v0 - y0 / H) < 0.02);
    assert.ok(Math.abs(found.u1 - x1 / W) < 0.02);
    assert.ok(Math.abs(found.v1 - y1 / H) < 0.02);
    assert.equal(detectFlatAperture(new Uint8ClampedArray(8), 4, 4), null);
  });

  it("cuts the painted proscenium opening and leaves the columns", () => {
    const W = 100;
    const H = 60;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let i = 0; i < data.length; i += 4) {
      data[i] = 232;
      data[i + 1] = 190;
      data[i + 2] = 90;
      data[i + 3] = 255;
    }
    const grey = (x: number, y: number): void => {
      const i = (y * W + x) * 4;
      data[i] = 130;
      data[i + 1] = 130;
      data[i + 2] = 129;
    };
    for (let y = 16; y < 56; y++) for (let x = 28; x < 72; x++) grey(x, y);
    for (let y = 10; y < 16; y++) for (let x = 36; x < 64; x++) grey(x, y);
    for (let x = 36; x < 64; x++) {
      const i = (9 * W + x) * 4;
      data[i] = 156;
      data[i + 1] = 57;
      data[i + 2] = 62;
    }
    const found = cutConnectedFlat(data, W, H);
    assert.ok(found);
    assert.ok(found.u0 > 0.22, "the opening stays inside the columns");
    assert.ok(found.u1 < 0.78);
    const at = (x: number, y: number): number => data[(y * W + x) * 4 + 3]!;
    assert.equal(at(20, 30), 255, "a column inside the brief window stays");
    assert.equal(at(50, 30), 0, "the flat opening is clear");
    assert.equal(at(40, 12), 0, "the arch crown is clear");
    assert.equal(at(40, 9), 255, "the valance stays");
    assert.equal(at(30, 12), 255, "the arch corner stays");
  });

  it("fills a curtain panel with velvet and drops the paper margin", () => {
    const W = 40;
    const H = 48;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        const paper = x < 3 || y < 3 || x >= W - 3 || y >= H - 3;
        if (paper) {
          data[i] = 250;
          data[i + 1] = 248;
          data[i + 2] = 242;
        } else {
          data[i] = 140;
          data[i + 1] = 24;
          data[i + 2] = 28;
        }
        data[i + 3] = 255;
      }
    }
    assert.equal(fillCurtainVelvet(data, W, H), true);
    assert.ok(data[0]! < 180, "the paper margin is gone");
    assert.equal(data[3], 255);
    const mid = (24 * W + 20) * 4;
    assert.equal(data[mid], 140);
    assert.equal(data[mid + 3], 255);
    assert.equal(fillCurtainVelvet(new Uint8ClampedArray(8), 4, 4), false);
  });

  it("centers the box-office name on the blank plate above the window", () => {
    const W = 400;
    const H = 400;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let i = 0; i < data.length; i += 4) {
      data[i] = 90;
      data[i + 1] = 48;
      data[i + 2] = 24;
      data[i + 3] = 255;
    }
    const fill = (x0: number, y0: number, x1: number, y1: number, rgb: readonly [number, number, number]) => {
      for (let y = y0; y < y1; y++) {
        for (let x = x0; x < x1; x++) {
          const i = (y * W + x) * 4;
          data[i] = rgb[0];
          data[i + 1] = rgb[1];
          data[i + 2] = rgb[2];
        }
      }
    };
    // A short pale lip sits under the taller blank plate. The window is below both.
    fill(60, 270, 340, 276, [236, 216, 170]);
    fill(80, 220, 320, 260, [230, 198, 120]);
    fill(40, 300, 360, 380, [160, 160, 158]);
    const plate = measureNamePlate(data, W, H, 300 / H);
    assert.ok(plate);
    assert.ok(Math.abs(plate.u - 200 / W) < 0.03, `u ${plate.u}`);
    assert.ok(Math.abs(plate.v - 240 / H) < 0.03, `v ${plate.v}`);
    assert.ok(plate.vh > 30 / H);
    assert.equal(measureNamePlate(data, W, H, 0), null);
  });

  it("measures the arch opening and ignores the grey border and keystone", () => {
    const W = 100;
    const H = 160;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let i = 0; i < data.length; i += 4) {
      data[i] = 120;
      data[i + 1] = 72;
      data[i + 2] = 36;
      data[i + 3] = 255;
    }
    const paint = (x0: number, y0: number, x1: number, y1: number) => {
      for (let y = y0; y < y1; y++) {
        for (let x = x0; x < x1; x++) {
          const i = (y * W + x) * 4;
          data[i] = 122;
          data[i + 1] = 122;
          data[i + 2] = 122;
        }
      }
    };
    paint(0, 0, W, 3);
    paint(0, 0, 3, H);
    paint(W - 3, 0, W, H);
    paint(0, H - 3, W, H);
    paint(40, 8, 60, 16);
    paint(28, 50, 72, 150);
    const found = detectInteriorOpening(data, W, H);
    assert.ok(found);
    assert.ok(Math.abs(found.u0 - 28 / W) < 0.03, `u0 ${found.u0}`);
    assert.ok(Math.abs(found.u1 - 72 / W) < 0.03, `u1 ${found.u1}`);
    assert.ok(found.v0 > 0.2 && found.v0 < 0.4, `v0 ${found.v0}`);
    assert.ok(found.v1 > 0.9, `v1 ${found.v1}`);
    assert.equal(detectInteriorOpening(new Uint8ClampedArray(8), 4, 4), null);
  });

  it("knocks out only the paper ground that touches the border", () => {
    const W = 20;
    const H = 20;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        const edge = x < 2 || y < 2 || x >= W - 2 || y >= H - 2;
        data[i] = edge ? 250 : 140;
        data[i + 1] = edge ? 244 : 28;
        data[i + 2] = edge ? 230 : 32;
        data[i + 3] = 255;
      }
    }
    const inside = (10 * W + 10) * 4;
    data[inside] = 252;
    data[inside + 1] = 250;
    data[inside + 2] = 248;
    keyOutBorderGround(data, W, H);
    assert.equal(data[3], 0, "border paper is clear");
    assert.equal(data[(5 * W + 5) * 4 + 3], 255, "figure stays");
    assert.equal(data[inside + 3], 255, "interior white stays");
  });

  it("clears the grey field and leaves the cream die-cut flashing", () => {
    const W = 24;
    const H = 16;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        const grey = x < 3 || y < 3 || x >= W - 3 || y >= H - 3;
        const flash = x < 6 || y < 6 || x >= W - 6 || y >= H - 6;
        data[i] = grey ? 124 : flash ? 196 : 16;
        data[i + 1] = grey ? 124 : flash ? 176 : 12;
        data[i + 2] = grey ? 124 : flash ? 150 : 10;
        data[i + 3] = 255;
      }
    }
    keyOutBorderGrey(data, W, H);
    assert.equal(data[3], 0, "grey field is clear");
    assert.equal(data[(8 * W + 4) * 4 + 3], 255, "cream flashing stays");
    assert.equal(data[(8 * W + 4) * 4], 196, "flashing color is untouched");
    assert.equal(data[(8 * W + 12) * 4 + 3], 255, "ink stays");
  });

  it("copies ink color into cleared texels along the cut", () => {
    const W = 6;
    const H = 4;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let i = 0; i < data.length; i += 4) {
      data[i] = 128;
      data[i + 1] = 128;
      data[i + 2] = 128;
      data[i + 3] = 0;
    }
    const ink = (2 * W + 2) * 4;
    data[ink] = 20;
    data[ink + 1] = 12;
    data[ink + 2] = 8;
    data[ink + 3] = 255;
    extrudeCutoutColor(data, W, H);
    const beside = (2 * W + 3) * 4;
    assert.equal(data[beside + 3], 0, "cleared texel stays clear");
    assert.equal(data[beside], 20);
    assert.equal(data[beside + 1], 12);
    assert.equal(data[beside + 2], 8);
    assert.equal(data[0], 128, "a texel away from the ink keeps the paper color");
  });

  it("insets a near-white plate margin and leaves full-bleed art alone", () => {
    const W = 30;
    const H = 40;
    const data = new Uint8ClampedArray(W * H * 4);
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        const edge = x < 2 || y < 2 || x >= W - 2 || y >= H - 2;
        data[i] = edge ? 255 : 90;
        data[i + 1] = edge ? 254 : 40;
        data[i + 2] = edge ? 252 : 24;
        data[i + 3] = 255;
      }
    }
    const inset = paperEdgeInset(data, W, H);
    assert.ok(Math.abs(inset.u0 - 2 / W) < 0.02);
    assert.ok(Math.abs(inset.v0 - 2 / H) < 0.02);
    assert.ok(Math.abs(inset.u1 - 28 / W) < 0.02);
    assert.ok(Math.abs(inset.v1 - 38 / H) < 0.02);
    const bleed = new Uint8ClampedArray(W * H * 4);
    for (let i = 0; i < bleed.length; i += 4) {
      bleed[i] = 40;
      bleed[i + 1] = 20;
      bleed[i + 2] = 10;
      bleed[i + 3] = 255;
    }
    assert.deepEqual(paperEdgeInset(bleed, W, H), { u0: 0, v0: 0, u1: 1, v1: 1 });
  });

  it("contain-fits the foyer inside the measured booth window", () => {
    const content = { x0: -6.5, y0: -0.4, x1: 6.5, y1: 8 };
    const frame = { x0: -3.94, y0: 0.79, x1: 3.88, y1: 5.34 };
    const fit = containFit(content, frame, 0.03);
    const map = (x: number, y: number) => ({
      x: fit.cx + (x - fit.contentCx) * fit.scale,
      y: fit.cy + (y - fit.contentCy) * fit.scale,
    });
    for (const [x, y] of [[content.x0, content.y0], [content.x1, content.y1], [content.x0, content.y1], [content.x1, content.y0]] as const) {
      const p = map(x, y);
      assert.ok(p.x >= frame.x0 - 1e-6 && p.x <= frame.x1 + 1e-6, `x ${p.x}`);
      assert.ok(p.y >= frame.y0 - 1e-6 && p.y <= frame.y1 + 1e-6, `y ${p.y}`);
    }
    assert.ok(fit.scale < 1);
  });
});

describe("popup house cards face the audience", () => {
  it("keeps every house and lobby mesh on an XY plane and turns the page as one sheet", () => {
    const fakeCtx = {
      beginPath() {},
      moveTo() {},
      lineTo() {},
      quadraticCurveTo() {},
      bezierCurveTo() {},
      arc() {},
      ellipse() {},
      rect() {},
      roundRect() {},
      closePath() {},
      fill() {},
      stroke() {},
      strokeRect() {},
      fillRect() {},
      clearRect() {},
      save() {},
      restore() {},
      clip() {},
      translate() {},
      rotate() {},
      scale() {},
      setLineDash() {},
      fillText() {},
      measureText() {
        return { width: 50 };
      },
      createLinearGradient() {
        return { addColorStop() {} };
      },
      createRadialGradient() {
        return { addColorStop() {} };
      },
      getImageData() {
        return { data: new Uint8ClampedArray(16) };
      },
      putImageData() {},
      drawImage() {},
    } as unknown as CanvasRenderingContext2D;
    const prevDoc = (globalThis as unknown as { document?: unknown }).document;
    (globalThis as unknown as { document?: unknown }).document = {
      createElement: (tag: string) => {
        if (tag === "canvas") {
          return { width: 0, height: 0, getContext: () => fakeCtx };
        }
        return {};
      },
    };
    try {
      const rig = buildHouse(THREE);
      const turned: string[] = [];
      let doors = 0;
      const pages: import("three").Object3D[] = [];
      const visit = (node: import("three").Object3D): void => {
        if (node.userData.hinge === "page") pages.push(node);
        const mesh = node as import("three").Mesh;
        if (mesh.isMesh) {
          if (mesh.userData.hinge === "door") doors += 1;
          else if (mesh.rotation.x !== 0 || mesh.rotation.y !== 0) {
            turned.push(`${mesh.rotation.x},${mesh.rotation.y}`);
          }
        }
        for (const child of node.children) visit(child);
      };
      for (const node of rig.nodes) visit(node);
      assert.equal(doors, 0);
      assert.deepEqual(turned, []);
      assert.equal(pages.length, 1);
      const page = pages[0]!;
      rig.update(0, 1280, 720, 0);
      assert.equal(page.rotation.y, 0);
      assert.equal(page.visible, true);
      rig.update(1, 1280, 720, 0);
      assert.ok(page.rotation.y > Math.PI / 2, "the page finishes past edge-on");
      assert.equal(page.visible, false);
      const mapName = (mesh: import("three").Mesh): string => {
        const mat = mesh.material as import("three").MeshBasicMaterial;
        return mat.map?.name ?? "";
      };
      const jambs: import("three").Mesh[] = [];
      const prosceniumPaper: import("three").Mesh[] = [];
      const collect = (node: import("three").Object3D): void => {
        const mesh = node as import("three").Mesh;
        if (mesh.isMesh) {
          const geo = mesh.geometry as import("three").PlaneGeometry;
          const w = geo.parameters?.width ?? 0;
          const h = geo.parameters?.height ?? 0;
          if (Math.abs(h - 15.8) < 0.05 && w < 22) jambs.push(mesh);
          if (Math.abs(h - 27.2) < 0.05) prosceniumPaper.push(mesh);
        }
        for (const child of node.children) collect(child);
      };
      for (const node of rig.nodes) collect(node);
      assert.equal(jambs.length, 6);
      assert.ok(prosceniumPaper.length >= 2);
      for (const mesh of [...jambs, ...prosceniumPaper]) {
        assert.equal(mapName(mesh), "lobby.wallpaper", `wall ${mesh.geometry.type} uses ${mapName(mesh)}`);
      }
      const floors: import("three").Mesh[] = [];
      const findFloor = (node: import("three").Object3D): void => {
        const mesh = node as import("three").Mesh;
        if (mesh.isMesh && mesh.userData.role === "pit-floor") floors.push(mesh);
        for (const child of node.children) findFloor(child);
      };
      for (const node of rig.nodes) findFloor(node);
      assert.equal(floors.length, 1);
      const floor = floors[0]!;
      assert.equal(mapName(floor), "floor.carpet");
      assert.equal(floor.position.z, PROSCENIUM_Z + 0.04);
      assert.ok(Math.abs(floor.rotation.x) < 1e-6 && Math.abs(floor.rotation.y) < 1e-6);
      const geo = floor.geometry as import("three").PlaneGeometry;
      assert.ok(geo.parameters.height > 10, "the carpet continues below the proscenium");
      const counters: import("three").Mesh[] = [];
      const findCounter = (node: import("three").Object3D): void => {
        const mesh = node as import("three").Mesh;
        if (mesh.isMesh && mapName(mesh) === "lobby.counter") counters.push(mesh);
        for (const child of node.children) findCounter(child);
      };
      for (const node of rig.nodes) findCounter(node);
      assert.equal(counters.length, 0);
      const depthOf = (mesh: import("three").Object3D): number => {
        let z = 0;
        for (let node: import("three").Object3D | null = mesh; node; node = node.parent) z += node.position.z;
        return z;
      };
      const byRole = (role: string): import("three").Mesh[] => {
        const found: import("three").Mesh[] = [];
        const walk = (node: import("three").Object3D): void => {
          const mesh = node as import("three").Mesh;
          if (mesh.isMesh && mesh.userData.role === role) found.push(mesh);
          for (const child of node.children) walk(child);
        };
        for (const node of rig.nodes) walk(node);
        return found;
      };
      const grille = byRole("booth-grille");
      const boothWalls = byRole("booth-wall");
      const boothFloor = byRole("booth-floor");
      assert.equal(grille.length, 1);
      assert.equal(boothWalls.length, 1);
      assert.equal(boothFloor.length, 1);
      assert.equal(byRole("booth-reveal").length, 0);
      assert.equal(byRole("booth-back").length, 0, "the window looks through to the page wall");
      assert.equal(byRole("booth-sill").length, 0);
      assert.equal(mapName(grille[0]!), "lobby.grille");
      assert.equal(grille[0]!.castShadow, true);
      assert.equal(grille[0]!.receiveShadow, true);
      assert.equal(boothWalls[0]!.castShadow, true);
      assert.equal(boothWalls[0]!.receiveShadow, true);
      const grilleDepth = grille[0]!.customDepthMaterial as import("three").MeshDepthMaterial;
      assert.equal(grilleDepth.alphaTest, 0.5, "the grille shadow follows the die-cut");
      assert.equal(mapName(boothFloor[0]!), "floor.carpet");
      assert.equal(mapName(boothWalls[0]!), "lobby.wallpaper");
      const wallMat = boothWalls[0]!.material as import("three").MeshLambertMaterial;
      assert.equal(wallMat.map?.wrapS, THREE.RepeatWrapping);
      assert.equal(wallMat.map?.repeat.x, 1);
      assert.equal(wallMat.map?.repeat.y, 1);
      const wallZ = depthOf(boothWalls[0]!);
      const grilleZ = depthOf(grille[0]!);
      assert.ok(grilleZ > wallZ && grilleZ - wallZ < 0.03, "the grille sits on the wall plane");
      const palms = byRole("booth-palm").sort((a, b) => a.position.x - b.position.x);
      assert.equal(palms.length, 2);
      const grilleGeoX = grille[0]!.geometry as import("three").PlaneGeometry;
      const grilleLeft = grille[0]!.position.x - grilleGeoX.parameters.width / 2;
      const grilleRight = grille[0]!.position.x + grilleGeoX.parameters.width / 2;
      for (const palm of palms) {
        const palmGeo = palm.geometry as import("three").PlaneGeometry;
        const bottom = palm.position.y - palmGeo.parameters.height / 2;
        const carpet = boothMap(0, GRILLE_CARD.y[0], PAGE_FACE_Z)[1];
        const boothH = grilleGeoX.parameters.height;
        assert.ok(bottom < carpet - boothH * 0.04, "the vase sits forward of the plinth on the carpet");
        assert.ok(palmGeo.parameters.height > boothH, "the fronds reach over the grille");
        assert.ok(palmGeo.parameters.height < boothH * 1.4, "the plants are a step smaller than the first foreground size");
        assert.ok(depthOf(palm) > grilleZ + 0.08, "the palm stands closer than the grille");
        assert.equal((palm.material as import("three").MeshLambertMaterial).alphaTest, 0.5);
      }
      const leftPalm = palms[0]!;
      const rightPalm = palms[1]!;
      const leftGeo = leftPalm.geometry as import("three").PlaneGeometry;
      const rightGeo = rightPalm.geometry as import("three").PlaneGeometry;
      const leftInner = leftPalm.position.x + leftGeo.parameters.width / 2;
      const rightInner = rightPalm.position.x - rightGeo.parameters.width / 2;
      assert.ok(leftPalm.position.x < grilleLeft, "the left vase sits outside the frame");
      assert.ok(leftInner > grilleLeft + leftGeo.parameters.width * 0.2, "the left fronds cross the grille");
      assert.ok(rightPalm.position.x > grilleRight, "the right vase sits outside the frame");
      assert.ok(rightInner < grilleRight - rightGeo.parameters.width * 0.2, "the right fronds cross the grille");
      assert.ok(leftPalm.position.x < rightPalm.position.x);
      assert.equal((rightPalm.material as import("three").MeshLambertMaterial).map?.repeat.x, -1);
      const palmFloor = byRole("booth-palm-floor");
      assert.equal(palmFloor.length, 1);
      assert.equal(mapName(palmFloor[0]!), "floor.carpet");
      assert.ok(Math.abs(palmFloor[0]!.rotation.x) < 1e-6 && Math.abs(palmFloor[0]!.rotation.y) < 1e-6, "the floor band faces the audience");
      const bandGeo = palmFloor[0]!.geometry as import("three").PlaneGeometry;
      const bandTop = palmFloor[0]!.position.y + bandGeo.parameters.height / 2;
      const bandLeft = palmFloor[0]!.position.x - bandGeo.parameters.width / 2;
      const bandRight = palmFloor[0]!.position.x + bandGeo.parameters.width / 2;
      const leftBottom = leftPalm.position.y - leftGeo.parameters.height / 2;
      const rightBottom = rightPalm.position.y - rightGeo.parameters.height / 2;
      const leftOuter = leftPalm.position.x - leftGeo.parameters.width / 2;
      const rightOuter = rightPalm.position.x + rightGeo.parameters.width / 2;
      assert.ok(Math.abs(leftBottom - rightBottom) < 1e-4);
      const grilleFoot = grille[0]!.position.y - grilleGeoX.parameters.height / 2;
      assert.ok(bandTop <= grilleFoot + 1e-4, "the floor band stays below the grille");
      assert.ok(bandTop > leftBottom + leftGeo.parameters.height * 0.1, "the carpet overlaps the vase base");
      assert.ok(bandTop < leftBottom + leftGeo.parameters.height * 0.2, "the carpet stays on the vase base");
      assert.ok(bandLeft <= leftOuter + 1e-4 && bandRight >= rightOuter - 1e-4, "the band connects both vases");
      assert.ok(depthOf(palmFloor[0]!) < depthOf(leftPalm) - 0.01, "the vases stand in front of their floor");
      assert.ok(depthOf(palmFloor[0]!) > grilleZ, "the vase floor sits in front of the grille");
      const wallGeo = boothWalls[0]!.geometry as import("three").BufferGeometry;
      wallGeo.computeBoundingBox();
      const span = wallGeo.boundingBox!.max.x - wallGeo.boundingBox!.min.x;
      assert.equal(wallGeo.getAttribute("position").count / 4, 4, "one wall with a single opening");
      assert.ok(depthOf(boothFloor[0]!) > wallZ && depthOf(boothFloor[0]!) < grilleZ, "the floor is on the wall, under the frame");
      const base = boothMap(0, GRILLE_CARD.y[0], PAGE_FACE_Z)[1];
      const sill = boothMap(0, GRILLE_SILL_Y, PAGE_FACE_Z)[1];
      const floorGeo = boothFloor[0]!.geometry as import("three").PlaneGeometry;
      const floorTop = boothFloor[0]!.position.y + floorGeo.parameters.height / 2;
      const grilleGeo = grille[0]!.geometry as import("three").PlaneGeometry;
      const grilleBottom = grille[0]!.position.y - grilleGeo.parameters.height / 2;
      assert.ok(Math.abs(floorTop - base) < 1e-4, `floor top ${floorTop} vs pillar base ${base}`);
      assert.ok(Math.abs(grilleBottom - floorTop) < 1e-4, "the carpet meets the pillar feet");
      assert.ok(grilleBottom < sill, "the pillars continue below the window");
      assert.ok(floorGeo.parameters.width + 1e-4 >= FOYER_VIEW.x[1] - FOYER_VIEW.x[0], "the carpet covers the foyer view");
      assert.ok(span + 1e-4 >= FOYER_VIEW.x[1] - FOYER_VIEW.x[0], "the wallpaper covers the foyer view");
      const sheetSize = (mesh: import("three").Mesh): number => {
        const pos = mesh.geometry.getAttribute("position");
        const coords = mesh.geometry.getAttribute("uv");
        for (let i = 1; i < pos.count; i++) {
          const dx = Math.abs(pos.getX(i) - pos.getX(0));
          const du = Math.abs(coords.getX(i) - coords.getX(0));
          if (dx > 0.2 && du > 1e-4) return dx / du;
        }
        throw new Error("wallpaper has no measurable span");
      };
      const boothSheet = sheetSize(boothWalls[0]!);
      assert.ok(boothSheet < 1, `wallpaper sheet ${boothSheet} is still stretched soft`);
      const lobbyDistance = worldCamera({
        journey: 0, width: 16, height: 9, growthSteps: 0, pointer: [0, 0], reducedMotion: true,
      }).position[2] - PAGE_FACE_Z;
      const seatZ = worldCamera({
        journey: 1, width: 16, height: 9, growthSteps: 0, pointer: [0, 0], reducedMotion: true,
      }).position[2];
      const expectSheet = (mesh: import("three").Mesh, label: string): void => {
        const paperGeo = mesh.geometry as import("three").PlaneGeometry;
        const paperMat = mesh.material as import("three").MeshLambertMaterial;
        const tile = paperGeo.parameters.width / (paperMat.map?.repeat.x ?? 1);
        const expected = boothSheet * (seatZ - depthOf(mesh)) / lobbyDistance;
        assert.ok(Math.abs(tile - expected) < 1e-2, `${label} wallpaper ${tile} vs ${expected}`);
      };
      for (const mesh of jambs) expectSheet(mesh, "theater");
      for (const mesh of prosceniumPaper) expectSheet(mesh, "house");
      for (const mesh of [boothWalls[0]!, boothFloor[0]!, grille[0]!]) {
        assert.equal(mesh.rotation.x, 0);
        assert.equal(mesh.rotation.y, 0);
      }
      assert.equal(floor.castShadow, true);
      assert.equal(floor.receiveShadow, true);
      assert.equal((floor.material as import("three").Material).type, "MeshLambertMaterial");
      const glows: import("three").Mesh[] = [];
      const findGlow = (node: import("three").Object3D): void => {
        const mesh = node as import("three").Mesh;
        if (mesh.isMesh && mesh.userData.role === "footlight-glow") glows.push(mesh);
        for (const child of node.children) findGlow(child);
      };
      for (const node of rig.nodes) findGlow(node);
      assert.equal(glows.length, 1);
      const glow = glows[0]!;
      assert.equal(glow.castShadow, false);
      assert.equal((glow.material as import("three").Material).transparent, true);
      rig.setGlow(0);
      assert.equal((glow.material as import("three").Material).opacity, 0);
      rig.setGlow(1);
      assert.ok(((glow.material as import("three").Material).opacity ?? 0) > 0.5);
      const spot = rig.nodes.find((node) => node.userData.role === "ticket-spot") as import("three").SpotLight;
      assert.ok(spot?.isSpotLight);
      assert.equal(spot.castShadow, false);
      assert.equal(spot.visible, false);
      rig.setTickets([
        { id: "oz", title: "Oz", subtitle: "Chapter 1", ready: false },
        { id: "war", title: "War", subtitle: "Chapter I", ready: false },
      ], null);
      const left = rig.pickables.find((mesh) => mesh.userData.pick === "ticket:oz")!;
      const restY = left.position.y;
      rig.setHighlight("ticket:oz");
      assert.equal(left.rotation.z, 0, "a hovered ticket stays upright");
      assert.equal(left.position.y, restY, "a hovered ticket stays in place");
      const anchor = rig.ticketAnchor();
      assert.ok(anchor);
      assert.ok(Math.abs(spot.target.position.x - anchor[0]) < 1e-4);
      assert.ok(Math.abs(spot.target.position.y - anchor[1]) < 1e-4);
      assert.ok(spot.position.z > anchor[2], "the spotlight sits in front of the ticket");
      rig.setHoverZoom(1);
      assert.equal(spot.visible, true);
      const pageWall = byRole("booth-page");
      assert.equal(pageWall.length, 1);
      assert.equal(pageWall[0]!.receiveShadow, true);
      assert.equal(pageWall[0]!.layers.isEnabled(1), true, "the back wall receives the ticket spotlight");
      assert.equal(left.layers.isEnabled(1), true);
      assert.equal(spot.layers.isEnabled(0), true, "the view camera can see the spotlight");
      assert.equal(spot.layers.isEnabled(1), true);
      assert.equal(spot.shadow.camera.layers.isEnabled(1), true);
      assert.equal(spot.shadow.camera.layers.isEnabled(0), false);
      assert.equal(spot.castShadow, true, "the ticket spotlight casts onto the back wall");
      assert.ok(spot.shadow.camera.near < spot.position.z - anchor[2], "the ticket is inside the shadow frustum");
      assert.ok(spot.distance > spot.position.z - FOYER_WALL_Z, "the shadow reaches the back wall");
      assert.equal(left.castShadow, true);
      assert.ok(spot.intensity > 4 && spot.intensity < 9, "the ticket pool is a highlight, not a flood");
      assert.ok(spot.position.y > anchor[1] + 0.1, "the light sits high enough to drop a shadow");
      assert.ok(spot.angle > 0.25 && spot.angle < 0.4, "the pool hugs the card");
      rig.setHighlight("ticket:war");
      const other = rig.ticketAnchor();
      assert.ok(other && other[0] > anchor[0]);
      assert.equal(left.rotation.z, 0);
      rig.setHighlight(null);
      assert.equal(rig.ticketAnchor(), null);
      rig.setHoverZoom(0);
      assert.equal(spot.visible, false);
      assert.equal(spot.castShadow, false);
      assert.equal(left.rotation.z, 0);
    } finally {
      (globalThis as unknown as { document?: unknown }).document = prevDoc;
    }
  });
});

describe("house art fallback: no-manifest rendering unchanged", () => {
  it("buildHouse with no manifest produces unchanged fallback rig", () => {
    // Build house rig with mock canvas environment
    const fakeCtx = {
      beginPath() {},
      moveTo() {},
      lineTo() {},
      quadraticCurveTo() {},
      bezierCurveTo() {},
      arc() {},
      ellipse() {},
      rect() {},
      roundRect() {},
      closePath() {},
      fill() {},
      stroke() {},
      strokeRect() {},
      fillRect() {},
      clearRect() {},
      save() {},
      restore() {},
      clip() {},
      translate() {},
      rotate() {},
      scale() {},
      setLineDash() {},
      fillText() {},
      measureText() {
        return { width: 50 };
      },
      createLinearGradient() {
        return { addColorStop() {} };
      },
      createRadialGradient() {
        return { addColorStop() {} };
      },
      getImageData() {
        return { data: new Uint8ClampedArray(16) };
      },
      putImageData() {},
      drawImage() {},
    } as unknown as CanvasRenderingContext2D;

    const fakeCanvas = {
      width: 100,
      height: 100,
      getContext: () => fakeCtx,
    } as unknown as HTMLCanvasElement;

    const prevDoc = (globalThis as unknown as { document?: unknown }).document;
    (globalThis as unknown as { document?: unknown }).document = {
      createElement: (tag: string) => {
        if (tag === "canvas") return { ...fakeCanvas, width: 0, height: 0 };
        return {};
      },
    };

    try {
      const rigDefault = buildHouse(THREE);
      assert.ok(rigDefault.nodes.length >= 3);
      assert.ok(rigDefault.pickables.length > 0);
      assert.ok(rigDefault.textureBytes() > 0);

      const rigNull = buildHouse(THREE, null);
      assert.equal(rigDefault.nodes.length, rigNull.nodes.length);
      assert.equal(rigDefault.pickables.length, rigNull.pickables.length);
      assert.equal(rigDefault.textureBytes(), rigNull.textureBytes());

      const rigEmptyObj = buildHouse(THREE, { houseArt: null });
      assert.equal(rigDefault.nodes.length, rigEmptyObj.nodes.length);
      assert.equal(rigDefault.pickables.length, rigEmptyObj.pickables.length);
      assert.equal(rigDefault.textureBytes(), rigEmptyObj.textureBytes());
    } finally {
      (globalThis as unknown as { document?: unknown }).document = prevDoc;
    }
  });
});
