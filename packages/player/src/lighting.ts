// Two lighting domains. The narrative stage takes the authored snapshot
// (ambient plus directional or spot, environment always null). The house
// keeps its own warm gaslight and does not follow the blackout gain.
// Lambert shades divide by π, so the sink scales intensity by π and an
// authored intensity of 1 matches the painted albedo under a head-on light.
import type { LightingSpec } from "./types.js";
import { PAGE_FACE_Z, PAGE_Y, SIDE_VIEW_DEGREES, worldCamera } from "./multiplane.js";

export const LAMBERT_COMPENSATION = Math.PI;

/** Level of each overhead booth lamp. */
export const BOOTH_HOUSE_LAMP = 0.55;
/** Seated level. Take Your Seat removes the booth lamp; the stage spots carry the opening. */
export const DIM_HOUSE_LAMP = 0;
/** Journey by which the overhead lamp is gone. Take Your Seat starts that walk. */
export const BOOTH_LAMP_OFF = 0.2;

const HOUSE_SHADOW = {
  map_size: 2048,
  bias: -0.00015,
  normal_bias: 0.002,
  near: 0.15,
  far: 12,
} as const;

const boothCam = worldCamera({
  journey: 0, width: 16, height: 9, growthSteps: 0, pointer: [0, 0], reducedMotion: true,
});
const boothYaw = SIDE_VIEW_DEGREES * Math.PI / 180;
const boothBack = [Math.sin(boothYaw), Math.cos(boothYaw)] as const;
const boothRight = [Math.cos(boothYaw), -Math.sin(boothYaw)] as const;
const boothLampAt = (side: number): [number, number, number] => [
  boothCam.position[0] + boothBack[0] * 0.7 + boothRight[0] * side * 0.9,
  boothCam.position[1] + 1.05,
  boothCam.position[2] + boothBack[1] * 0.7 + boothRight[1] * side * 0.9,
];
const grilleY = (PAGE_Y[0] + PAGE_Y[1]) / 2;

/**
 * Two overhead lamps behind the box-office camera, aimed down and forward
 * at the grille. Their range dies before the proscenium.
 */
export const HOUSE_LIGHTING: LightingSpec = {
  ambient: { color: "#cbbfae", intensity: 0.16 },
  environment: null,
  lights: ([-1, 1] as const).map((side) => ({
    id: side < 0 ? "booth-left" : "booth-right",
    type: "spot" as const,
    enabled: true,
    position: boothLampAt(side),
    target: [side * 0.35, grilleY, PAGE_FACE_Z] as [number, number, number],
    color: "#ffd7a8",
    intensity: BOOTH_HOUSE_LAMP,
    cast_shadow: true,
    shadow: HOUSE_SHADOW,
    angle_radians: 0.7,
    penumbra: 0.45,
    distance: 8,
    decay: 0,
  })),
};

const STAGE_SHADOW = {
  map_size: 2048,
  bias: -0.0002,
  normal_bias: 0.008,
  near: 0.3,
  far: 28,
} as const;

/** Two front-of-house spots, house-left and house-right, on the opening. */
export const STAGE_LIGHTING: LightingSpec = {
  ambient: { color: "#fff4e8", intensity: 0.06 },
  environment: null,
  lights: [
    {
      id: "stage-left",
      type: "spot",
      enabled: true,
      position: [-6.4, 9.6, 11],
      target: [-2.6, 4.2, 2.2],
      color: "#fff1d6",
      intensity: 1.5,
      cast_shadow: true,
      shadow: STAGE_SHADOW,
      angle_radians: 0.4,
      penumbra: 0.62,
      distance: 26,
      decay: 0,
    },
    {
      id: "stage-right",
      type: "spot",
      enabled: true,
      position: [6.4, 9.6, 11],
      target: [2.6, 4.2, 2.2],
      color: "#fff1d6",
      intensity: 1.5,
      cast_shadow: true,
      shadow: STAGE_SHADOW,
      angle_radians: 0.4,
      penumbra: 0.62,
      distance: 26,
      decay: 0,
    },
  ],
};

/** House scene: dim auditorium fill plus the two stage spots. */
export function houseSceneLighting(): LightingSpec {
  return {
    ambient: HOUSE_LIGHTING.ambient,
    environment: null,
    lights: [...HOUSE_LIGHTING.lights, ...STAGE_LIGHTING.lights],
  };
}

/** Overhead booth lamp. Full at the box office, gone once the audience takes a seat. */
export function houseLampIntensity(journey: number): number {
  const t = Math.max(0, Math.min(1, journey));
  if (t >= BOOTH_LAMP_OFF) return DIM_HOUSE_LAMP;
  const leave = t / BOOTH_LAMP_OFF;
  return BOOTH_HOUSE_LAMP * (1 - leave) + DIM_HOUSE_LAMP * leave;
}

/** Footlight halo level. Zero when every booth lamp is off, so the glow can dim. */
export function houseGlow(lighting: LightingSpec): number {
  const lamps = lighting.lights.filter((light) => light.id.startsWith("booth-"));
  const authored = BOOTH_HOUSE_LAMP || 1;
  const brightest = Math.max(0, ...lamps.map((light) => light.enabled ? light.intensity : 0));
  if (!(brightest > 0)) return 0;
  return Math.max(0, Math.min(1, brightest / authored));
}

type ThreeNS = typeof import("three");
type SceneLight = import("three").AmbientLight | import("three").DirectionalLight | import("three").SpotLight;

const TAG = "ptLight";

function tagged(scene: import("three").Scene): SceneLight[] {
  return scene.children.filter((child): child is SceneLight => child.userData[TAG] === true);
}

/**
 * Rebuild the scene's tagged lights to match one complete lighting state.
 * A second call with the same ids updates the existing lights in place.
 */
export function syncSceneLights(THREE: ThreeNS, scene: import("three").Scene, lighting: LightingSpec): void {
  const ids = new Set(lighting.lights.map((light) => light.id));
  for (const child of tagged(scene)) {
    if (child.userData.role === "ambient") continue;
    if (ids.has(String(child.userData.lightId))) continue;
    scene.remove(child);
    const target = (child as import("three").DirectionalLight).target;
    if (target) scene.remove(target);
  }

  let ambient = tagged(scene).find((child) => child.userData.role === "ambient") as import("three").AmbientLight | undefined;
  if (!ambient) {
    ambient = new THREE.AmbientLight();
    ambient.userData[TAG] = true;
    ambient.userData.role = "ambient";
    scene.add(ambient);
  }
  ambient.color.set(lighting.ambient.color);
  ambient.intensity = lighting.ambient.intensity * LAMBERT_COMPENSATION;

  for (const spec of lighting.lights) {
    let light = tagged(scene).find((child) => child.userData.lightId === spec.id) as
      | import("three").DirectionalLight
      | import("three").SpotLight
      | undefined;
    if (!light || light.userData.kind !== spec.type) {
      if (light) {
        scene.remove(light);
        scene.remove(light.target);
      }
      light = spec.type === "spot" ? new THREE.SpotLight() : new THREE.DirectionalLight();
      light.userData[TAG] = true;
      light.userData.lightId = spec.id;
      light.userData.kind = spec.type;
      scene.add(light);
      scene.add(light.target);
    }
    light.color.set(spec.color);
    light.intensity = spec.enabled ? spec.intensity * LAMBERT_COMPENSATION : 0;
    light.visible = spec.enabled;
    light.position.set(spec.position[0], spec.position[1], spec.position[2]);
    light.target.position.set(spec.target[0], spec.target[1], spec.target[2]);
    light.castShadow = spec.cast_shadow && spec.enabled;
    light.shadow.mapSize.set(spec.shadow.map_size, spec.shadow.map_size);
    light.shadow.bias = spec.shadow.bias;
    light.shadow.normalBias = spec.shadow.normal_bias;
    light.shadow.camera.near = spec.shadow.near;
    light.shadow.camera.far = spec.shadow.far;
    if (spec.type === "spot" && light instanceof THREE.SpotLight) {
      light.angle = spec.angle_radians;
      light.penumbra = spec.penumbra;
      light.distance = spec.distance;
      light.decay = spec.decay;
    } else {
      const cam = light.shadow.camera as import("three").OrthographicCamera;
      cam.left = -14;
      cam.right = 14;
      cam.top = 12;
      cam.bottom = -8;
    }
    light.shadow.camera.updateProjectionMatrix();
    light.target.updateMatrixWorld();
  }
}
