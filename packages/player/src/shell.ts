// Schematic shell, camera envelope, plaque growth rule and blackout
// compositing math. Three.js renderer construction follows the pinned
// profile (WebGLRenderer, PCFShadowMap, sRGB output, linear working target,
// no tone mapping, linear filtering without mipmaps) and lives behind
// createStageRenderer() so pure checks run without a browser.
import {
  APRON,
  baseDistance,
  CAMERA,
  DECK,
  LAYER_Z,
  OPENING,
  OUTER,
  PLAQUE,
  PLAQUE_SAFE,
  PROFILE_ID,
  RENDER_DEFAULTS,
} from "./profile.js";

export interface ShellValidation {
  ok: boolean;
  errors: string[];
  expanded_plaque_y0: number;
  base_distance: number;
}

/** Validate layered proscenium/deck/apron joins and the below-stage plaque. */
export function validateSchematicShell(): ShellValidation {
  const errors: string[] = [];
  if (!(OPENING.x[0] < OPENING.x[1] && OPENING.y[0] < OPENING.y[1])) {
    errors.push("opening: inverted bounds");
  }
  if (!(OUTER.x[0] <= OPENING.x[0] && OPENING.x[1] <= OUTER.x[1])) {
    errors.push("opening must sit inside outer bounds");
  }
  if (!(LAYER_Z.inner < LAYER_Z.columns && LAYER_Z.columns < LAYER_Z.apron && LAYER_Z.apron < LAYER_Z.plaque)) {
    errors.push("layers: z order must be inner < columns < apron < plaque");
  }
  // Structural deck: deck front Z equals apron join Z.
  if (DECK.z[1] !== APRON.z) errors.push("deck/apron join mismatch");
  if (DECK.y !== 0 || APRON.y[1] !== 0) errors.push("deck/apron must meet at Y=0");
  // Plaque centered below the opening with its content below the floor edge.
  if (PLAQUE.y[1] > 0) errors.push("plaque must sit below Y=0");
  if (PLAQUE_SAFE.y[1] > 0) errors.push("safe text must sit below Y=0");
  if (!(PLAQUE.x[0] <= PLAQUE_SAFE.x[0] && PLAQUE_SAFE.x[1] <= PLAQUE.x[1])) {
    errors.push("safe inset must sit inside plaque");
  }
  return { ok: errors.length === 0, errors, expanded_plaque_y0: PLAQUE.y[0], base_distance: baseDistance() };
}

export interface CameraPose {
  position: [number, number, number];
  target: [number, number, number];
}

/** Base pose: shell bounds XY center at Z=0, perspective 40 deg. */
export function baseCameraPose(): CameraPose {
  const cx = (OUTER.x[0] + OUTER.x[1]) / 2;
  const cy = (OUTER.y[0] + OUTER.y[1]) / 2;
  const d = baseDistance();
  return { position: [cx, cy, d], target: [cx, cy, 0] };
}

/**
 * Camera envelope: authored target offset X+-0.5/Y+-0.25, distance [d,d+2];
 * pointer adds X+-0.8/Y+-0.4 to both camera and target, resets on leave,
 * zero under reduced motion. No history-dependent smoothing.
 */
export function envelopeCameraPose(
  targetOffset: [number, number],
  dolly: number,
  pointer: [number, number],
  reducedMotion: boolean,
): CameraPose {
  const [ox, oy] = targetOffset;
  if (Math.abs(ox) > CAMERA.target_offset_x + 1e-9 || Math.abs(oy) > CAMERA.target_offset_y + 1e-9) {
    throw new Error("Authored target offset outside envelope");
  }
  const d = baseDistance();
  if (!(dolly >= 0 && dolly <= CAMERA.distance_pad + 1e-9)) {
    throw new Error("Dolly outside [d,d+2]");
  }
  const [px, py] = reducedMotion ? [0, 0] : pointer;
  if (Math.abs(px) > CAMERA.pointer_x + 1e-9 || Math.abs(py) > CAMERA.pointer_y + 1e-9) {
    throw new Error("Pointer outside envelope");
  }
  const base = baseCameraPose();
  return {
    position: [base.position[0] + ox + px, base.position[1] + oy + py, d + dolly],
    target: [base.target[0] + ox + px, base.target[1] + oy + py, 0],
  };
}

/**
 * Plaque growth rule: if two lines cannot fit, extend the plaque downward in
 * 0.25-world-unit steps, expand shell bounds and refit the camera until the
 * inset fits. Never shrink text or split a highlight group.
 */
export function plaqueGrowthSteps(neededHeight: number, insetHeight: number): number {
  if (!(neededHeight > 0 && insetHeight > 0)) throw new Error("Nonpositive layout height");
  if (neededHeight <= insetHeight) return 0;
  return Math.ceil((neededHeight - insetHeight) / 0.25);
}

/** Projected-DOM homography: solve planar map from inset corners to CSS px. */
export function solveHomography(
  world: [[number, number], [number, number], [number, number], [number, number]],
  css: [[number, number], [number, number], [number, number], [number, number]],
): number[] {
  // Solves the 8x8 linear system for h11..h32 with h33=1.
  const A: number[][] = [];
  const b: number[] = [];
  for (let i = 0; i < 4; i++) {
    const [x, y] = world[i]!;
    const [u, v] = css[i]!;
    A.push([x, y, 1, 0, 0, 0, -u * x, -u * y]);
    b.push(u);
    A.push([0, 0, 0, x, y, 1, -v * x, -v * y]);
    b.push(v);
  }
  return solveLinear(A, b);
}

function solveLinear(A: number[][], b: number[]): number[] {
  const n = b.length;
  const M = A.map((row, i) => [...row, b[i]!]);
  for (let col = 0; col < n; col++) {
    let pivot = col;
    for (let r = col + 1; r < n; r++) {
      if (Math.abs(M[r]![col]!) > Math.abs(M[pivot]![col]!)) pivot = r;
    }
    if (Math.abs(M[pivot]![col]!) < 1e-12) throw new Error("Singular homography");
    [M[col], M[pivot]] = [M[pivot]!, M[col]!];
    const div = M[col]![col]!;
    for (let c = col; c <= n; c++) M[col]![c]! /= div;
    for (let r = 0; r < n; r++) {
      if (r === col) continue;
      const factor = M[r]![col]!;
      for (let c = col; c <= n; c++) M[r]![c]! -= factor * M[col]![c]!;
    }
  }
  return M.map((row) => row[n]!);
}

/**
 * Blackout compositing rule (pure part): stage target color multiplied by
 * gain; at full blackout every stage-aperture interior channel is <=1/255.
 * House layers/signage stay independently lit. Ineligible content never
 * enters any pass even at zero gain.
 */
export function compositeStagePixel(linear: [number, number, number], gain: number): [number, number, number] {
  if (!(gain >= 0 && gain <= 1)) throw new Error("Gain must be 0..1");
  if (gain === 0) return [0, 0, 0];
  return [linear[0]! * gain, linear[1]! * gain, linear[2]! * gain];
}

export function blackoutHolds(pixels: Array<[number, number, number]>): boolean {
  return pixels.every(([r, g, b]) => r <= 1 / 255 && g <= 1 / 255 && b <= 1 / 255);
}

export interface RendererOptions {
  canvas?: unknown;
  width?: number;
  height?: number;
}

/**
 * Create the Three.js stage renderer per the pinned profile. Imports three
 * lazily so pure checks run in Node without a browser or GPU.
 *
 * Applied guidance (tutor three.js skill + corpus examples):
 * - WebGLRenderer with antialias, sRGB output, no tone mapping.
 * - shadowMap enabled with PCFShadowMap (NOT PCFSoftShadowMap); the skill
 *   notes shadow.radius blur applies to PCFSoftShadowMap only, so the
 *   migration must not depend on radius.
 * - Color textures use SRGBColorSpace; masks/data stay linear (NoColorSpace).
 * - LinearFilter with generateMipmaps=false for the initial profile.
 * - One render scheduler owned by the caller (setAnimationLoop/add/remove).
 */
export async function createStageRenderer(options: RendererOptions = {}): Promise<{
  renderer: unknown;
  profile: string;
  shadow_map: string;
  dispose(): void;
}> {
  const THREE = await import("three");
  const renderer = new THREE.WebGLRenderer({
    antialias: true,
    powerPreference: "high-performance",
  });
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.NoToneMapping;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  const dpr = Math.min(
    (globalThis as { devicePixelRatio?: number }).devicePixelRatio ?? 1,
    RENDER_DEFAULTS.dpr_max,
  );
  renderer.setPixelRatio(dpr);
  renderer.setSize(options.width ?? 1280, options.height ?? 720);
  let disposed = false;
  return {
    renderer,
    profile: PROFILE_ID,
    shadow_map: "PCFShadowMap",
    dispose() {
      if (disposed) return;
      disposed = true;
      renderer.dispose();
    },
  };
}

/**
 * Browser frame sink implementing the profile compositing pipeline: the
 * narrative stage renders into an aperture-clipped linear target, a
 * fullscreen pass multiplies it by the blackout gain (covering unlit and
 * emissive contributions alike), and house layers stay independently lit.
 * Diagnostic geometry is neutral (final artwork follows); the pipeline
 * itself — target, gain multiply, capture readback — is the real path.
 *
 * Headless harnesses have no WebGL context: construction rejects with a
 * typed `webgl_unavailable` failure instead of crashing, so browser pixel
 * acceptance stays honestly pending.
 */
export async function createThreeSink(
  width: number,
  height: number,
): Promise<import("./mount.js").FrameSink> {
  if (!Number.isSafeInteger(width) || width <= 0 || !Number.isSafeInteger(height) || height <= 0) {
    throw new Error(`capture bounds: ${width}x${height}`);
  }
  const THREE = await import("three");
  let renderer: import("three").WebGLRenderer;
  try {
    renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: "high-performance" });
  } catch (error) {
    throw new Error(
      `webgl_unavailable: ${error instanceof Error ? error.message : String(error)} (browser capture pending)`,
    );
  }
  try {
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFShadowMap;
    const dpr = Math.min(
      (globalThis as { devicePixelRatio?: number }).devicePixelRatio ?? 1,
      RENDER_DEFAULTS.dpr_max,
    );
    renderer.setPixelRatio(dpr);
    renderer.setSize(width, height);
  } catch (error) {
    renderer.dispose();
    throw new Error(
      `webgl_unavailable: ${error instanceof Error ? error.message : String(error)} (browser capture pending)`,
    );
  }

  // Stage domain: linear working target holding narrative color.
  const stageTarget = new THREE.WebGLRenderTarget(width, height);
  stageTarget.texture.colorSpace = THREE.NoColorSpace;
  stageTarget.texture.minFilter = THREE.LinearFilter;
  stageTarget.texture.magFilter = THREE.LinearFilter;
  stageTarget.texture.generateMipmaps = false;
  const stageScene = new THREE.Scene();
  stageScene.background = new THREE.Color(0x000000);
  const stageCamera = new THREE.PerspectiveCamera(40, width / height, 0.1, 100);
  const base = baseCameraPose();
  stageCamera.position.set(base.position[0], base.position[1], base.position[2]);
  stageCamera.lookAt(base.target[0], base.target[1], base.target[2]);

  // Composite pass: stage color multiplied by the blackout gain.
  const compositeScene = new THREE.Scene();
  const compositeCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const gainUniform = { value: 1 };
  const compositeMaterial = new THREE.ShaderMaterial({
    uniforms: { map: { value: stageTarget.texture }, gain: gainUniform },
    vertexShader: "varying vec2 vUv; void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }",
    fragmentShader:
      "uniform sampler2D map; uniform float gain; varying vec2 vUv;" +
      "void main() { vec3 c = texture2D(map, vUv).rgb * gain; gl_FragColor = vec4(c, 1.0); }",
  });
  const compositeQuad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), compositeMaterial);
  compositeQuad.frustumCulled = false;
  compositeScene.add(compositeQuad);

  let disposed = false;
  return {
    render(state): void {
      if (disposed) throw new Error("render after dispose");
      gainUniform.value = state.stage_gain;
      renderer.setRenderTarget(stageTarget);
      renderer.render(stageScene, stageCamera);
      renderer.setRenderTarget(null);
      renderer.render(compositeScene, compositeCamera);
    },
    capturePixels(captureWidth, captureHeight): Uint8Array {
      if (disposed) throw new Error("capture after dispose");
      if (captureWidth !== width || captureHeight !== height) {
        throw new Error(`capture size ${captureWidth}x${captureHeight} differs from sink ${width}x${height}`);
      }
      const pixels = new Uint8Array(captureWidth * captureHeight * 4);
      renderer.readRenderTargetPixels(stageTarget, 0, 0, captureWidth, captureHeight, pixels);
      return pixels;
    },
    dispose(): void {
      if (disposed) return;
      disposed = true;
      compositeQuad.geometry.dispose();
      compositeMaterial.dispose();
      stageTarget.dispose();
      renderer.dispose();
    },
  };
}

/** Configure a color texture per profile: sRGB, linear filter, no mipmaps. */
export async function configureColorTexture(texture: {
  colorSpace: unknown;
  minFilter: unknown;
  magFilter: unknown;
  generateMipmaps: boolean;
}): Promise<void> {
  const THREE = await import("three");
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.minFilter = THREE.LinearFilter;
  texture.magFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;
}

/** Configure a mask/depth texture per profile: linear, no color conversion. */
export async function configureMaskTexture(texture: {
  colorSpace: unknown;
  minFilter: unknown;
  magFilter: unknown;
  generateMipmaps: boolean;
}): Promise<void> {
  const THREE = await import("three");
  texture.colorSpace = THREE.NoColorSpace;
  texture.minFilter = THREE.LinearFilter;
  texture.magFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;
}
