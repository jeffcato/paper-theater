// Selected rendering/layout profile paper-player-r186-v1.
// Selected, now fixture-validated at the schematic level by check:player -- shell.
// Any demonstrated defect requires a recorded profile revision; do not silently
// adjust these constants in different modules.
export const PROFILE_ID = "paper-player-r186-v1";
export const PROFILE_SCHEMA = 1;
export const CAPABILITY = "paper-player-v1";
export const SCHEMA_VERSION = 1;
export const SAMPLE_RATE = 48000;
export const DARK_HOLD_SAMPLES = 96000;
export const TITLE_HOLD_SAMPLES = 72000;
export const TITLE_DISMISS_SAMPLES = 24000;
export const CURTAIN_SAMPLES = 72000;
export const FADE_SAMPLES = 48000;
export const SIGN_DOWN_MS = 1200;
export const SIGN_UP_MS = 800;

export interface Vec3 extends Array<number> { 0: number; 1: number; 2: number }

export const OPENING = { x: [-8, 8] as const, y: [0, 9] as const, z: 1.3 };
export const OUTER = { x: [-11, 11] as const, y: [-3, 11] as const };
export const LAYER_Z = { inner: 1.3, columns: 1.8, apron: 2.4, plaque: 2.8, signage: 3.0 } as const;
export const DECK = { x: [-8, 8] as const, z: [-5.6, 2.4] as const, y: 0 };
export const APRON = { x: [-8, 8] as const, y: [-3, 0] as const, z: 2.4 };
export const PLAQUE = { x: [-7, 7] as const, y: [-2.8, -0.4] as const, z: 2.8 };
export const PLAQUE_SAFE = { x: [-6.5, 6.5] as const, y: [-2.55, -0.65] as const };
export const CAMERA = {
  fov_y_degrees: 40,
  up: [0, 1, 0] as [number, number, number],
  near: 0.1,
  far: 100,
  target_offset_x: 0.5,
  target_offset_y: 0.25,
  distance_pad: 2.0,
  pointer_x: 0.8,
  pointer_y: 0.4,
} as const;
export const FONT = {
  family: "Noto Serif",
  size_css_px: 22,
  line_height: 1.3,
  min_scale: 1.0,
  max_scale: 2.0,
} as const;
export const RENDER_DEFAULTS = {
  color_space: "srgb",
  working: "linear",
  tone_mapping: "none",
  cutout_alpha: 0.35,
  shell_alpha: 0.1,
  curtain_alpha: 0.05,
  shadow_map: "PCFShadowMap",
  shadow_max: 2048,
  texture_filter: "linear",
  mipmaps: false,
  image_max_px: 4096,
  graphics_budget_mib: 256,
  master_budget_mib: 256,
  dpr_max: 1.5,
} as const;
export const SIGN_BOUNDS = { x: [-4, 4] as const, y: [5, 7] as const, z: 3.0 } as const;
export const SIGN_ROPES_X = [-3, 3] as const;
export const SIGN_ROPE_TOP_Y = 11;

/** Base camera distance reserving 10% of frame dimensions (handoff formula). */
export function baseDistance(): number {
  const h = 7; // outer half-height incl. plaque/signs: (11 - -3) / 2
  const w = 11; // outer half-width: (11 - -11) / 2
  const tan20 = Math.tan((20 * Math.PI) / 180);
  const zMax = LAYER_Z.signage;
  return zMax + Math.max(h / (0.9 * tan20), w / (0.9 * (16 / 9) * tan20));
}

export function profileDigestInput(): string {
  return JSON.stringify({
    profile: PROFILE_ID,
    schema: PROFILE_SCHEMA,
    capability: CAPABILITY,
    sample_rate: SAMPLE_RATE,
    dark_hold: DARK_HOLD_SAMPLES,
    opening: OPENING,
    layers: LAYER_Z,
    deck: DECK,
    apron: APRON,
    plaque: PLAQUE,
    camera_fov: CAMERA.fov_y_degrees,
    font: FONT,
    render: RENDER_DEFAULTS,
  });
}
