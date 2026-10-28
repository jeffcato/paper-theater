// House gaslight and authored stage lights. Expectations come from the
// playback profile: environment stays null, shadows are PCF at most 2048,
// and the house rig does not follow the stage blackout.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import * as THREE from "three";
import {
  BOOTH_HOUSE_LAMP,
  BOOTH_LAMP_OFF,
  DIM_HOUSE_LAMP,
  HOUSE_LIGHTING,
  LAMBERT_COMPENSATION,
  STAGE_LIGHTING,
  houseGlow,
  houseLampIntensity,
  houseSceneLighting,
  syncSceneLights,
} from "../src/lighting.js";
import { PAGE_FACE_Z, SIDE_VIEW_DEGREES, worldCamera } from "../src/multiplane.js";

describe("house gaslight", () => {
  it("keeps two overhead lamps behind the booth camera and short of the stage", () => {
    assert.equal(HOUSE_LIGHTING.environment, null);
    assert.ok(HOUSE_LIGHTING.ambient.intensity > 0 && HOUSE_LIGHTING.ambient.intensity < 0.25);
    assert.equal(HOUSE_LIGHTING.lights.length, 2);
    const cam = worldCamera({
      journey: 0, width: 16, height: 9, growthSteps: 0, pointer: [0, 0], reducedMotion: true,
    });
    const yaw = SIDE_VIEW_DEGREES * Math.PI / 180;
    const back = [Math.sin(yaw), Math.cos(yaw)] as const;
    const [left, right] = HOUSE_LIGHTING.lights;
    assert.ok(left!.position[0] < right!.position[0], "the lamps flank the camera");
    for (const lamp of HOUSE_LIGHTING.lights) {
      assert.equal(lamp.type, "spot");
      assert.equal(lamp.intensity, BOOTH_HOUSE_LAMP);
      assert.equal(lamp.cast_shadow, true);
      const dx = lamp.position[0] - cam.position[0];
      const dz = lamp.position[2] - cam.position[2];
      assert.ok(dx * back[0] + dz * back[1] > 0.3, "lamp sits behind the booth camera");
      assert.ok(lamp.position[1] > cam.position[1] + 0.4, "lamp is overhead");
      assert.ok(lamp.position[1] > lamp.target[1], "the lamp shines downward");
      assert.ok(lamp.target[2] < lamp.position[2], "the lamp shines forward");
      assert.ok(Math.abs(lamp.target[2] - PAGE_FACE_Z) < 0.05, "the lamp is aimed at the grille");
      assert.ok(lamp.distance < PAGE_FACE_Z - 2.2, "the lamp dies before the proscenium");
      assert.ok(lamp.shadow.map_size <= 2048 && lamp.shadow.map_size >= 1024);
    }
    assert.equal(houseGlow(HOUSE_LIGHTING), 1);
    assert.equal(houseLampIntensity(0), BOOTH_HOUSE_LAMP);
    assert.equal(DIM_HOUSE_LAMP, 0);
    assert.equal(houseLampIntensity(BOOTH_LAMP_OFF), DIM_HOUSE_LAMP);
    assert.equal(houseLampIntensity(1), DIM_HOUSE_LAMP);
    assert.ok(houseLampIntensity(BOOTH_LAMP_OFF / 2) < BOOTH_HOUSE_LAMP);
  });

  it("dims the footlight halo when the house lamps are off", () => {
    const dark = structuredClone(HOUSE_LIGHTING);
    for (const light of dark.lights) light.enabled = false;
    assert.equal(houseGlow(dark), 0);
    for (const light of dark.lights) {
      light.enabled = true;
      light.intensity = BOOTH_HOUSE_LAMP / 2;
    }
    assert.ok(Math.abs(houseGlow(dark) - 0.5) < 1e-9);
  });
});

describe("stage spots", () => {
  it("uses two spots from either side of the house, brighter than the dim auditorium", () => {
    assert.equal(STAGE_LIGHTING.environment, null);
    assert.equal(STAGE_LIGHTING.lights.length, 2);
    assert.ok(STAGE_LIGHTING.lights.every((light) => light.type === "spot"));
    const [left, right] = STAGE_LIGHTING.lights;
    assert.ok(left!.position[0] < 0 && right!.position[0] > 0);
    assert.ok(left!.target[0] < 0 && right!.target[0] > 0);
    for (const spot of STAGE_LIGHTING.lights) {
      assert.ok(spot.position[1] > spot.target[1], "spot hangs above the opening");
      assert.ok(spot.position[2] > spot.target[2], "spot is in front of the opening");
      assert.ok(spot.intensity > BOOTH_HOUSE_LAMP);
      assert.ok(spot.intensity > HOUSE_LIGHTING.ambient.intensity);
      assert.ok(spot.angle_radians > 0 && spot.angle_radians <= Math.PI / 2);
      assert.equal(spot.cast_shadow, true);
      assert.ok(spot.shadow.map_size <= 2048 && spot.shadow.map_size >= 1024);
    }
    const combined = houseSceneLighting();
    assert.equal(combined.lights.length, 4);
    assert.equal(combined.ambient.intensity, HOUSE_LIGHTING.ambient.intensity);
  });
});

describe("scene light sync", () => {
  it("scales Lambert intensity by π and updates the same lights in place", () => {
    const scene = new THREE.Scene();
    syncSceneLights(THREE, scene, STAGE_LIGHTING);
    const spots = scene.children.filter((child) => (child as import("three").SpotLight).isSpotLight) as import("three").SpotLight[];
    assert.equal(spots.length, 2);
    assert.ok(Math.abs(spots[0]!.intensity - STAGE_LIGHTING.lights[0]!.intensity * LAMBERT_COMPENSATION) < 1e-9);
    assert.equal(spots[0]!.shadow.mapSize.x, STAGE_LIGHTING.lights[0]!.shadow.map_size);
    const count = scene.children.length;
    syncSceneLights(THREE, scene, STAGE_LIGHTING);
    assert.equal(scene.children.length, count);
    const dimmed = structuredClone(STAGE_LIGHTING);
    dimmed.lights[0]!.enabled = false;
    syncSceneLights(THREE, scene, dimmed);
    assert.equal(spots[0]!.visible, false);
    assert.equal(spots[0]!.intensity, 0);
  });
});
