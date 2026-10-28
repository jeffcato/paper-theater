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
  SIGN_BOUNDS,
  SIGN_ROPE_TOP_Y,
  SIGN_ROPES_X,
} from "./profile.js";
import { nodAngle, pivotOffset } from "./assets.js";
import { buildHouse } from "./house.js";
import { LAMBERT_COMPENSATION, STAGE_LIGHTING, houseLampIntensity, houseSceneLighting, syncSceneLights } from "./lighting.js";
import type { ControlsView, InspectView, TicketView } from "./house.js";
import type { LoadedHouseArt } from "./house-art.js";
import { prepareHouseArt } from "./house-art.js";
import { applyTicketZoom, lecternRect, railRect, sceneLayout, ticketPanLock, ticketZoomIntent, worldCamera } from "./multiplane.js";
import type { CssRect, WorldCamera, WorldView } from "./multiplane.js";
import type { FrameSink } from "./mount.js";

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

/** Shell framing a camera pose resolves against: XY center + distance. */
export interface FramedShell {
  centerX: number;
  centerY: number;
  distance: number;
}

/** Base framing: outer-bounds center at the profile base distance. */
export function baseShell(): FramedShell {
  return {
    centerX: (OUTER.x[0] + OUTER.x[1]) / 2,
    centerY: (OUTER.y[0] + OUTER.y[1]) / 2,
    distance: baseDistance(),
  };
}

/**
 * Camera refit for plaque growth: frame the shell expanded downward by the
 * growth steps (0.25 world units each) plus the full downward envelope
 * margin (authored target offset + pointer), so the grown plaque and every
 * camera/pointer extreme stay inside the frame. Same 10%-reserve rule as
 * the base distance; horizontal framing is unchanged (growth is vertical).
 */
export function refitShell(growthSteps: number): FramedShell {
  if (!Number.isSafeInteger(growthSteps) || growthSteps < 0) {
    throw new Error(`growth steps: ${growthSteps}`);
  }
  const expansion = growthSteps * 0.25;
  const margin = CAMERA.target_offset_y + CAMERA.pointer_y;
  const bottom = OUTER.y[0] - expansion - margin;
  const top = OUTER.y[1];
  const halfH = (top - bottom) / 2;
  const tan20 = Math.tan((20 * Math.PI) / 180);
  const w = (OUTER.x[1] - OUTER.x[0]) / 2;
  return {
    centerX: (OUTER.x[0] + OUTER.x[1]) / 2,
    centerY: (top + bottom) / 2,
    distance: LAYER_Z.signage + Math.max(halfH / (0.9 * tan20), w / (0.9 * (16 / 9) * tan20)),
  };
}

/**
 * Camera envelope: authored target offset X+-0.5/Y+-0.25, distance [d,d+2];
 * pointer adds X+-0.8/Y+-0.4 to both camera and target, resets on leave,
 * zero under reduced motion. No history-dependent smoothing. Poses resolve
 * against the base shell unless a refit shell is supplied.
 */
export function envelopeCameraPose(
  targetOffset: [number, number],
  dolly: number,
  pointer: [number, number],
  reducedMotion: boolean,
  shell: FramedShell = baseShell(),
): CameraPose {
  const [ox, oy] = targetOffset;
  if (Math.abs(ox) > CAMERA.target_offset_x + 1e-9 || Math.abs(oy) > CAMERA.target_offset_y + 1e-9) {
    throw new Error("Authored target offset outside envelope");
  }
  const d = shell.distance;
  if (!(dolly >= 0 && dolly <= CAMERA.distance_pad + 1e-9)) {
    throw new Error("Dolly outside [d,d+2]");
  }
  const [px, py] = reducedMotion ? [0, 0] : pointer;
  if (Math.abs(px) > CAMERA.pointer_x + 1e-9 || Math.abs(py) > CAMERA.pointer_y + 1e-9) {
    throw new Error("Pointer outside envelope");
  }
  return {
    position: [shell.centerX + ox + px, shell.centerY + oy + py, d + dolly],
    target: [shell.centerX + ox + px, shell.centerY + oy + py, 0],
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

export interface ViewportCss {
  width: number;
  height: number;
}

export interface PlaqueScreenRect {
  left: number;
  top: number;
  width: number;
  height: number;
  /** Projected CSS y of the deck/apron join (world Y=0) at plaque depth. */
  floorY: number;
}

/**
 * Pinhole projection of a stage point to CSS px for a camera pose. Pure
 * (no three.js): the camera basis comes from position/target/up exactly as
 * the stage PerspectiveCamera orients itself, then a perspective divide
 * with the profile fov over the viewport aspect. Points at or behind the
 * camera throw instead of projecting.
 */
export function projectToCss(
  point: [number, number, number],
  pose: CameraPose,
  viewport: ViewportCss,
  fovYDegrees = CAMERA.fov_y_degrees,
): [number, number] {
  if (!(viewport.width > 0 && viewport.height > 0)) throw new Error("Nonpositive viewport");
  const sub = (a: [number, number, number], b: [number, number, number]): [number, number, number] =>
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const norm = (v: [number, number, number]): [number, number, number] => {
    const len = Math.hypot(v[0], v[1], v[2]);
    if (!(len > 0)) throw new Error("Degenerate camera basis");
    return [v[0] / len, v[1] / len, v[2] / len];
  };
  const cross = (a: [number, number, number], b: [number, number, number]): [number, number, number] => [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
  const dot = (a: [number, number, number], b: [number, number, number]): number =>
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  // three.js convention: the camera looks down its local -Z.
  const zAxis = norm(sub(pose.position, pose.target));
  const xAxis = norm(cross(CAMERA.up as [number, number, number], zAxis));
  const yAxis = cross(zAxis, xAxis);
  const rel = sub(point, pose.position);
  const depth = -dot(rel, zAxis);
  if (!(depth > 0)) throw new Error("Point at or behind camera");
  const tan = Math.tan(((fovYDegrees * Math.PI) / 180) / 2);
  const aspect = viewport.width / viewport.height;
  const ndcX = dot(rel, xAxis) / (tan * aspect * depth);
  const ndcY = dot(rel, yAxis) / (tan * depth);
  return [((ndcX + 1) / 2) * viewport.width, ((1 - ndcY) / 2) * viewport.height];
}

/**
 * Projected below-stage placement for the plaque overlay: the profile
 * safe-text inset (PLAQUE_SAFE at plaque depth) plus the projected floor
 * line the text must clear. DOM overlays position from this rect; the
 * measured chunk (not the character-count proxy) decides the final height.
 */
export function projectPlaqueSafe(pose: CameraPose, viewport: ViewportCss): PlaqueScreenRect {
  const z = LAYER_Z.plaque;
  const topLeft = projectToCss([PLAQUE_SAFE.x[0], PLAQUE_SAFE.y[1], z], pose, viewport);
  const bottomRight = projectToCss([PLAQUE_SAFE.x[1], PLAQUE_SAFE.y[0], z], pose, viewport);
  const [, floorY] = projectToCss([0, 0, z], pose, viewport);
  return {
    left: topLeft[0],
    top: topLeft[1],
    width: bottomRight[0] - topLeft[0],
    height: bottomRight[1] - topLeft[1],
    floorY,
  };
}

/**
 * Curtain panel center-x for an open ratio in 0..1. Each 8-wide panel
 * covers half the opening when closed and slides fully clear when open.
 */
export function curtainPanelX(side: "left" | "right", openRatio: number): number {
  if (!(openRatio >= 0 && openRatio <= 1)) throw new Error(`curtain ratio: ${openRatio}`);
  const closed = side === "left" ? OPENING.x[0] / 2 : OPENING.x[1] / 2;
  const travel = side === "left" ? OPENING.x[0] : OPENING.x[1];
  return closed + travel * openRatio;
}

/**
 * INTERMISSION board center-y for sign progress 0..1. Visible inside
 * SIGN_BOUNDS (center 6); hidden lifts clear above the framed shell top.
 */
export function signBoardY(progress: number): number {
  if (!(progress >= 0 && progress <= 1)) throw new Error(`sign progress: ${progress}`);
  return (SIGN_BOUNDS.y[0] + SIGN_BOUNDS.y[1]) / 2 + (1 - progress) * 8;
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
export interface FigureSource {
  asset_id: string;
  url: string;
  pixel_size: [number, number];
  pivot_top_left: [number, number];
  stage_size: [number, number];
  allow_mirror: boolean;
}

export interface TitleCard {
  heading: string;
  sub: string;
}

export interface ThreeSinkOptions {
  /** Registered figures bound to scene instances. Omitted entries fail closed. */
  figures?: FigureSource[];
  /** Neutral calibration fixture (default true; existing evidence depends on it). */
  calibration?: boolean;
  /** Opening title card strings (default neutral; harness passes authored). */
  title?: TitleCard;
  /**
   * Caller-owned canvas for same-canvas graphics recovery. Defaults to a
   * fresh canvas; recovery recreates the surface on the supplied element.
   */
  canvas?: HTMLCanvasElement;
  /**
   * Full-viewport multiplane house (audience immersion): box office, aisle,
   * auditorium facade, lectern and brass rail in the house pass, with the
   * world camera from setView. Absent keeps the bounded preview framing.
   */
  immersive?: boolean;
  /** Registered-style house art manifest for the multiplane house. */
  houseArt?: LoadedHouseArt | null;
}

/** Pick result: house id plus the hit's horizontal texture coordinate. */
export interface HousePick {
  id: string;
  u: number;
}

/** Accounted GPU allocation (RGBA8 + depth; MSAA x4 drawing buffer). */
export interface GraphicsMemory {
  textures: number;
  targets: number;
  drawingBuffer: number;
  total: number;
}

/** Immersive sink: the shared stage sink plus world-camera/house controls. */
export interface ImmersiveSink extends FrameSink {
  canvas: HTMLCanvasElement;
  resize(width: number, height: number): void;
  setView(view: WorldView): void;
  camera(): WorldCamera;
  pick(cssX: number, cssY: number): HousePick | null;
  setHighlight(id: string | null): void;
  setTickets(tickets: TicketView[], notice: string | null): void;
  setInspect(card: InspectView | null): void;
  setControls(state: ControlsView): void;
  setTitle(title: TitleCard): void;
  /** Render the house with no performers (no mount). Curtain ratio defaults closed. */
  renderIdle(titleVisible: boolean, curtainRatio?: number): void;
  lecternRect(): CssRect;
  railRect(): CssRect;
  memory(): GraphicsMemory;
}

/** Native crop aspect for a figure source: stage matches pixels. */
function assertFigureAspect(figure: FigureSource): void {
  const [pw, ph] = figure.pixel_size;
  const [sw, sh] = figure.stage_size;
  if (!(pw > 0 && ph > 0 && sw > 0 && sh > 0)) {
    throw new Error(`figure_bounds: ${figure.asset_id}`);
  }
  const left = sw * ph;
  const right = sh * pw;
  const tol = 1e-9 * Math.max(1, Math.abs(left), Math.abs(right));
  if (Math.abs(left - right) > tol) {
    throw new Error(`figure_aspect: ${figure.asset_id} stage ${sw}/${sh} vs pixels ${pw}/${ph}`);
  }
}

export async function createThreeSink(
  width: number,
  height: number,
  options: ThreeSinkOptions = {},
): Promise<import("./mount.js").FrameSink> {
  if (!Number.isSafeInteger(width) || width <= 0 || !Number.isSafeInteger(height) || height <= 0) {
    throw new Error(`capture bounds: ${width}x${height}`);
  }
  const size = { width, height };
  const THREE = await import("three");
  let renderer: import("three").WebGLRenderer;
  try {
    renderer = new THREE.WebGLRenderer({
      antialias: true,
      powerPreference: "high-performance",
      ...(options.canvas ? { canvas: options.canvas } : {}),
    });
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
  const owned: { dispose(): void }[] = [];
  // Diagnostic deck plane (built first) for the deck/apron join proof.
  let diagDeck: import("three").Mesh | null = null;
  if (options.calibration !== false) {
    const diag = buildDiagnosticFixture(THREE);
    diagDeck = diag[0] ?? null;
    for (const mesh of diag) {
      stageScene.add(mesh);
      owned.push(mesh.geometry, mesh.material as { dispose(): void });
      // Material disposal never frees its map: the diagnostic quads own
      // two canvas textures that must join the owned set explicitly.
      const map = (mesh.material as unknown as { map?: { dispose(): void } | null }).map;
      if (map) owned.push(map);
    }
  }

  // Authored registered figures, loaded before the sink exists: a failure
  // here rejects the mount instead of showing incomplete assets mid-play.
  const loader = new THREE.TextureLoader();
  const figureTemplates = new Map<string, {
    texture: import("three").Texture;
    geometry: import("three").BufferGeometry;
    layout: {
      width: number;
      height: number;
      offsetX: number;
      offsetY: number;
      bottomY: number;
      allow_mirror: boolean;
    };
  }>();
  for (const figure of options.figures ?? []) {
    let texture: import("three").Texture;
    try {
      texture = await loader.loadAsync(figure.url);
    } catch (error) {
      throw new Error(
        `figure_unavailable: ${figure.asset_id} (${error instanceof Error ? error.message : String(error)})`,
      );
    }
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = false;
    const [sw, sh] = figure.stage_size;
    if (!(sw > 0 && sh > 0)) throw new Error(`figure_bounds: ${figure.asset_id}`);
    assertFigureAspect(figure);
    const [offsetX, offsetY] = pivotOffset(figure.pivot_top_left, figure.stage_size);
    const geometry = new THREE.PlaneGeometry(sw, sh);
    geometry.translate(offsetX, offsetY, 0);
    figureTemplates.set(figure.asset_id, {
      texture,
      geometry,
      layout: {
        width: sw,
        height: sh,
        offsetX,
        offsetY,
        bottomY: -sh / 2 + offsetY,
        allow_mirror: figure.allow_mirror,
      },
    });
    owned.push(texture, geometry);
  }
  const figureMeshes = new Map<string, import("three").Mesh>();
  const bottomByInstance = new Map<string, number>();
  const flip = new THREE.Matrix4().makeScale(1, 1, -1);
  const scratch = {
    matrix: new THREE.Matrix4(),
    flipped: new THREE.Matrix4(),
    position: new THREE.Vector3(),
    quaternion: new THREE.Quaternion(),
    euler: new THREE.Euler(),
    scale: new THREE.Vector3(),
  };

  // Composite pass: stage color multiplied by the blackout gain.
  // Neutral stage architecture (always built; the calibration flag covers
  // only the diagnostic fixture): the narrative aperture (inner back wall)
  // composites through the stage target under the blackout gain, while
  // permanent house geometry (columns, header, apron facing, plaque
  // backing, opening curtains) and the pause-driven sign render in the
  // independent house pass, staying readable through blackouts. The title
  // card also lives in the house pass — in front of the closed curtains —
  // and carries the same gain on its own material so dark holds stay dark.
  // The house scene carries its own gaslight and ignores the blackout gain.
  // The profile shell/curtain alphas stay reserved for later house moods.
  const staging = buildStaging(
    THREE,
    options.title ?? { heading: "Paper Theater", sub: "" },
  );
  for (const mesh of staging.stage) stageScene.add(mesh);
  const houseScene = new THREE.Scene();
  for (const node of staging.house) houseScene.add(node);
  for (const resource of staging.resources) owned.push(resource);
  // Immersive dressing joins the house pass only; the bounded shell pieces
  // it replaces are hidden (their geometry still answers debugGeometry).
  const preparedHouseArt = options.houseArt ? await prepareHouseArt(options.houseArt) : null;
  const house = options.immersive ? buildHouse(THREE, preparedHouseArt ?? options.houseArt) : null;
  syncSceneLights(THREE, stageScene, STAGE_LIGHTING);
  if (options.immersive) {
    const wall = staging.stage[0];
    if (wall) wall.visible = false;
    const paper = (draw: (ctx: CanvasRenderingContext2D, w: number, h: number) => void): import("three").MeshBasicMaterial => {
      const canvas = document.createElement("canvas");
      canvas.width = 256;
      canvas.height = 256;
      const ctx = canvas.getContext("2d");
      if (!ctx) throw new Error("2d canvas unavailable");
      draw(ctx, canvas.width, canvas.height);
      const texture = new THREE.CanvasTexture(canvas);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      texture.generateMipmaps = false;
      owned.push(texture);
      const mat = new THREE.MeshBasicMaterial({ map: texture });
      owned.push(mat);
      return mat;
    };
    const tone: Record<string, string> = {
      "groove-1": "#efe2c4",
      "groove-2": "#e0cba8",
      "scenic-flats": "#d2b892",
      backcloth: "#f6edd4",
      deck: "#c4a574",
    };
    for (const piece of sceneLayout()) {
      const mat = paper((ctx, w, h) => {
        ctx.fillStyle = tone[piece.id] ?? "#f3e6c8";
        ctx.fillRect(0, 0, w, h);
        ctx.strokeStyle = "rgba(42, 33, 24, 0.55)";
        ctx.lineWidth = 6;
        if (piece.kind === "deck") {
          for (let x = 0; x < w; x += 32) ctx.strokeRect(x, 0, 32, h);
        } else {
          ctx.strokeRect(10, 10, w - 20, h - 20);
          ctx.strokeStyle = "rgba(42, 33, 24, 0.08)";
          for (let x = 40; x < w; x += 56) {
            ctx.beginPath();
            ctx.moveTo(x, 16);
            ctx.lineTo(x, h - 16);
            ctx.stroke();
          }
        }
      });
      if (piece.kind === "deck") {
        const geo = new THREE.PlaneGeometry(piece.x[1] - piece.x[0], piece.z[1] - piece.z[0]);
        owned.push(geo);
        const mesh = new THREE.Mesh(geo, mat);
        mesh.rotation.x = -Math.PI / 2;
        mesh.position.set((piece.x[0] + piece.x[1]) / 2, piece.y, (piece.z[0] + piece.z[1]) / 2);
        stageScene.add(mesh);
        continue;
      }
      const geo = new THREE.PlaneGeometry(piece.x[1] - piece.x[0], piece.y[1] - piece.y[0]);
      owned.push(geo);
      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set((piece.x[0] + piece.x[1]) / 2, (piece.y[0] + piece.y[1]) / 2, piece.z);
      stageScene.add(mesh);
    }
  }
  syncSceneLights(THREE, houseScene, houseSceneLighting());
  if (house) {
    for (const node of house.nodes) houseScene.add(node);
    for (const resource of house.resources) owned.push(resource);
    for (const node of staging.bounded) node.visible = false;
    for (const curtain of [staging.curtainL, staging.curtainR]) {
      const mat = curtain.material as import("three").MeshBasicMaterial;
      mat.map = house.curtainMap;
      mat.needsUpdate = true;
    }
  }

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
  // The composite quad never writes or tests depth: the house scene draws
  // after it against cleared depth on every destination.
  compositeMaterial.depthTest = false;
  compositeMaterial.depthWrite = false;
  const compositeQuad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), compositeMaterial);
  compositeQuad.frustumCulled = false;
  compositeScene.add(compositeQuad);
  const veilMat = new THREE.MeshBasicMaterial({
    color: 0x000000,
    transparent: true,
    opacity: 0,
    depthTest: false,
    depthWrite: false,
  });
  const veilQuad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), veilMat);
  veilQuad.frustumCulled = false;
  const veilScene = new THREE.Scene();
  veilScene.add(veilQuad);
  owned.push(veilMat, veilQuad.geometry);
  // Explicit clears: the house pass shares each destination with the
  // composite quad, so autoClear cannot own the clear anymore.
  renderer.autoClear = false;

  // Final composited output lives in its own target: capture reads these
  // exact bytes (post blackout gain), and the same bytes go to the screen.
  const compositeTarget = new THREE.WebGLRenderTarget(width, height);
  compositeTarget.texture.colorSpace = THREE.NoColorSpace;
  compositeTarget.texture.minFilter = THREE.LinearFilter;
  compositeTarget.texture.magFilter = THREE.LinearFilter;
  compositeTarget.texture.generateMipmaps = false;

  let disposed = false;
  let signProgress = 0;
  let view: WorldView = { journey: 1, pointer: [0, 0], reducedMotion: false, width, height, growthSteps: 0 };
  let worldCam: WorldCamera = worldCamera(view);
  let ticketMix = 0;
  let zoomAt: [number, number, number] | null = null;
  let panLocked: [number, number, number] | null = null;
  let offSince: number | null = null;
  let veil = 0;
  let zoomClock = performance.now();
  const applyView = (): void => {
    const now = performance.now();
    const dt = Math.max(0, Math.min(50, now - zoomClock));
    zoomClock = now;
    house?.syncReadAlong({ now, reduced: view.reducedMotion });
    const aimed: WorldView = { ...view, readAlong: house?.lecternLift() ?? 0 };
    worldCam = worldCamera(aimed);
    veil = Math.max(0, Math.min(1, view.veil ?? 0));
    const anchor = house?.ticketAnchor() ?? null;
    if (view.journey >= 0.12) {
      zoomAt = null;
      panLocked = null;
      ticketMix = 0;
      offSince = null;
    } else if (!zoomAt || view.reducedMotion) {
      panLocked = null;
      if (anchor) zoomAt = [anchor[0], anchor[1], anchor[2]];
    } else {
      const pan = ticketPanLock(zoomAt, anchor, panLocked);
      panLocked = pan.locked;
      if (pan.locked && pan.at) {
        const glide = view.reducedMotion ? 1 : 1 - Math.exp(-dt / 180);
        zoomAt = [
          zoomAt[0] + (pan.at[0] - zoomAt[0]) * glide,
          zoomAt[1] + (pan.at[1] - zoomAt[1]) * glide,
          zoomAt[2] + (pan.at[2] - zoomAt[2]) * glide,
        ];
      } else if (anchor) {
        zoomAt = [anchor[0], anchor[1], anchor[2]];
      }
    }
    // While a ticket holds the camera, pointer parallax would shove the dolly
    // and can cancel the pan. The pointer only chooses the ticket.
    if (panLocked || ticketMix > 0.02) worldCam = worldCamera({ ...aimed, pointer: [0, 0] });
    const intent = ticketZoomIntent({
      hovering: anchor !== null || panLocked !== null,
      journey: view.journey,
      now,
      offSince,
      holding: zoomAt !== null,
    });
    offSince = intent.offSince;
    const want = intent.want;
    const step = view.reducedMotion ? 1 : 1 - Math.exp(-dt / 220);
    ticketMix += (want - ticketMix) * step;
    if (ticketMix < 0.001) ticketMix = 0;
    if (want === 1 && ticketMix > 0.999) ticketMix = 1;
    worldCam = applyTicketZoom(worldCam, zoomAt && ticketMix > 0 ? { at: zoomAt, amount: ticketMix } : null);
    if (ticketMix === 0) {
      zoomAt = null;
      panLocked = null;
      offSince = null;
    }
    house?.setHoverZoom(ticketMix, zoomAt);
    const [x, y, z] = worldCam.position;
    stageCamera.fov = worldCam.fovYDegrees;
    stageCamera.aspect = worldCam.aspect;
    stageCamera.position.set(x, y, z);
    stageCamera.rotation.set(0, worldCam.yaw, 0);
    // Off-axis window: holds the frame centre while the camera translates.
    stageCamera.setViewOffset(
      size.width, size.height,
      (-worldCam.shift[0] * size.width) / 2, (worldCam.shift[1] * size.height) / 2,
      size.width, size.height,
    );
    stageCamera.updateProjectionMatrix();
    house?.update(view.journey, size.width, size.height, view.growthSteps, worldCam, { now, reduced: view.reducedMotion });
    const level = houseLampIntensity(view.journey);
    let boothLamps = 0;
    for (const child of houseScene.children) {
      if (!String(child.userData.lightId ?? "").startsWith("booth-")) continue;
      const lamp = child as import("three").SpotLight;
      lamp.intensity = level * LAMBERT_COMPENSATION * (1 - ticketMix * 0.4);
      lamp.castShadow = level > 0;
      lamp.visible = level > 0;
      boothLamps += 1;
    }
    if (boothLamps > 0) house?.setGlow(Math.max(level / houseLampIntensity(0), 0.5));
  };
  if (house) applyView();
  const raycaster = new THREE.Raycaster();
  const ndc = new THREE.Vector2();
  const shown = (node: import("three").Object3D | null): boolean => {
    for (let n = node; n; n = n.parent) if (!n.visible) return false;
    return true;
  };
  const requireHouse = (): NonNullable<typeof house> => {
    if (!house) throw new Error("immersive house unavailable (bounded sink)");
    if (disposed) throw new Error("house call after dispose");
    return house;
  };
  const draw = (gain: number, ratio: number, titleVisible: boolean): void => {
    staging.curtainL.position.x = curtainPanelX("left", ratio);
    staging.curtainR.position.x = curtainPanelX("right", ratio);
    // Immersive velvet gathers its folds as it opens; the panel geometry
    // and coverage stay exactly the frozen slide.
    if (house) house.curtainMap.repeat.x = 1 + ratio * 0.8;
    staging.title.visible = titleVisible;
    // The house pass ignores the composite gain, so the title carries
    // the same blackout factor on its own material.
    staging.titleMat.color.setScalar(gain);
    const boardY = signBoardY(signProgress);
    staging.sign.group.visible = signProgress > 0.001;
    staging.sign.board.position.set(0, boardY, SIGN_BOUNDS.z);
    staging.sign.board.rotation.z = Math.sin(signProgress * Math.PI) * (3 * Math.PI / 180);
    const ropeLen = Math.max(0.01, SIGN_ROPE_TOP_Y - (boardY + 1));
    staging.sign.ropeL.position.set(SIGN_ROPES_X[0], SIGN_ROPE_TOP_Y, SIGN_BOUNDS.z);
    staging.sign.ropeR.position.set(SIGN_ROPES_X[1], SIGN_ROPE_TOP_Y, SIGN_BOUNDS.z);
    staging.sign.ropeL.scale.set(1, ropeLen, 1);
    staging.sign.ropeR.scale.set(1, ropeLen, 1);
    renderer.setRenderTarget(stageTarget);
    renderer.clear();
    renderer.render(stageScene, stageCamera);
    renderer.setRenderTarget(compositeTarget);
    renderer.clear();
    renderer.render(compositeScene, compositeCamera);
    renderer.render(houseScene, stageCamera);
    veilMat.opacity = veil;
    if (veil > 0.001) renderer.render(veilScene, compositeCamera);
    renderer.setRenderTarget(null);
    renderer.clear();
    renderer.render(compositeScene, compositeCamera);
    renderer.render(houseScene, stageCamera);
    if (veil > 0.001) renderer.render(veilScene, compositeCamera);
  };
  const sink: ImmersiveSink = {
    canvas: renderer.domElement,
    setCameraPose(pose: CameraPose): void {
      if (disposed) throw new Error("setCameraPose after dispose");
      stageCamera.position.set(pose.position[0], pose.position[1], pose.position[2]);
      stageCamera.lookAt(pose.target[0], pose.target[1], pose.target[2]);
    },
    setSign(_pose: unknown, progress: number): void {
      if (disposed) throw new Error("setSign after dispose");
      signProgress = Math.max(0, Math.min(1, progress));
    },
    render(state): void {
      if (disposed) throw new Error("render after dispose");
      gainUniform.value = state.stage_gain;
      // Authored instances first: hide every figure, then show exactly the
      // evaluated (presence- and reveal-filtered) actors with their own
      // state. Unknown assets fail closed instead of rendering wrong.
      for (const mesh of figureMeshes.values()) mesh.visible = false;
      for (const actor of state.actors) {
        const template = figureTemplates.get(actor.asset_id);
        if (!template) {
          throw new Error(`unknown asset: ${actor.asset_id} (no bound figure)`);
        }
        let mesh = figureMeshes.get(actor.instance_id);
        if (!mesh) {
          const material = new THREE.MeshLambertMaterial({ map: template.texture });
          mesh = new THREE.Mesh(template.geometry, material);
          mesh.matrixAutoUpdate = false;
          mesh.castShadow = true;
          mesh.receiveShadow = true;
          stageScene.add(mesh);
          figureMeshes.set(actor.instance_id, mesh);
          bottomByInstance.set(actor.instance_id, template.layout.bottomY);
          owned.push(material);
        }
        const [rx, ry, rz] = actor.rotation_xyz;
        // Supported rigid nod around the registered pivot (the mesh origin):
        // progress comes from the evaluated cue window via the actor record.
        const nod = actor.gesture === "nod"
          ? nodAngle(actor.gesture_progress ?? 0, actor.gesture_intensity)
          : 0;
        scratch.euler.set(rx + nod, ry, rz);
        scratch.quaternion.setFromEuler(scratch.euler);
        scratch.position.set(actor.position[0], actor.position[1], actor.position[2]);
        const mirrorX = actor.mirrored && template.layout.allow_mirror ? -1 : 1;
        scratch.scale.set(mirrorX, 1, 1);
        scratch.matrix.compose(scratch.position, scratch.quaternion, scratch.scale);
        // V3 +Z-away becomes +Z-toward: local matrices convert as F*M*F.
        scratch.flipped.copy(flip).multiply(scratch.matrix).multiply(flip);
        mesh.matrix.copy(scratch.flipped);
        mesh.visible = true;
      }
      // Staging follows the evaluated media state; the sign follows the
      // pause-driven UI clock through setSign (ratios clamp, never throw).
      draw(state.stage_gain, Math.max(0, Math.min(1, state.curtain_open_ratio)), state.title_visible);
    },
    renderIdle(titleVisible: boolean, curtainRatio = 0): void {
      if (disposed) throw new Error("render after dispose");
      for (const mesh of figureMeshes.values()) mesh.visible = false;
      const ratio = Math.max(0, Math.min(1, curtainRatio));
      draw(1, ratio, titleVisible);
    },
    resize(nextWidth: number, nextHeight: number): void {
      if (disposed) throw new Error("resize after dispose");
      if (!Number.isSafeInteger(nextWidth) || nextWidth <= 0 || !Number.isSafeInteger(nextHeight) || nextHeight <= 0) {
        throw new Error(`resize bounds: ${nextWidth}x${nextHeight}`);
      }
      size.width = nextWidth;
      size.height = nextHeight;
      renderer.setSize(nextWidth, nextHeight, false);
      stageTarget.setSize(nextWidth, nextHeight);
      compositeTarget.setSize(nextWidth, nextHeight);
      stageCamera.aspect = nextWidth / nextHeight;
      if (house) {
        view = { ...view, width: nextWidth, height: nextHeight };
        applyView();
      } else {
        stageCamera.updateProjectionMatrix();
      }
    },
    setView(next: WorldView): void {
      requireHouse();
      view = { ...next, pointer: [next.pointer[0], next.pointer[1]], width: size.width, height: size.height };
      applyView();
    },
    camera(): WorldCamera {
      requireHouse();
      return worldCam;
    },
    pick(cssX: number, cssY: number): HousePick | null {
      const rig = requireHouse();
      ndc.set((cssX / size.width) * 2 - 1, 1 - (cssY / size.height) * 2);
      raycaster.setFromCamera(ndc, stageCamera);
      for (const hit of raycaster.intersectObjects(rig.pickables, false)) {
        const id = hit.object.userData.pick as string | undefined;
        if (!id || !shown(hit.object)) continue;
        const u = hit.uv?.x ?? 0.5;
        // Gauge picks report the fraction along the engraved track.
        const track = hit.object.userData.track as [number, number] | undefined;
        return { id, u: track ? Math.max(0, Math.min(1, (u - track[0]) / (track[1] - track[0]))) : u };
      }
      return null;
    },
    setHighlight(id: string | null): void {
      requireHouse().setHighlight(id);
    },
    setTickets(tickets: TicketView[], notice: string | null): void {
      requireHouse().setTickets(tickets, notice);
    },
    setInspect(card: InspectView | null): void {
      requireHouse().setInspect(card);
    },
    setControls(state: ControlsView): void {
      requireHouse().setControls(state);
    },
    setTitle(title: TitleCard): void {
      requireHouse();
      staging.repaintTitle(title);
    },
    lecternRect(): CssRect {
      return lecternRect(worldCam, size, view.growthSteps, requireHouse().lecternLift());
    },
    railRect(): CssRect {
      requireHouse();
      return railRect(worldCam, size);
    },
    memory(): GraphicsMemory {
      const dprNow = renderer.getPixelRatio();
      let textures = (house?.textureBytes() ?? 0) + staging.textureBytes;
      for (const template of figureTemplates.values()) {
        const image = template.texture.image as { width?: number; height?: number } | null;
        textures += (image?.width ?? 0) * (image?.height ?? 0) * 4;
      }
      // Two linear targets, each RGBA8 color + 32-bit depth.
      const targets = 2 * size.width * size.height * 8;
      // Default framebuffer: front/back RGBA8 + depth, 4x MSAA resolve.
      const px = Math.round(size.width * dprNow) * Math.round(size.height * dprNow);
      const drawingBuffer = px * (4 * 2 + 4 * 4 + 4 * 4);
      return { textures, targets, drawingBuffer, total: textures + targets + drawingBuffer };
    },
    capturePixels(captureWidth, captureHeight): Uint8Array {
      if (disposed) throw new Error("capture after dispose");
      if (captureWidth !== size.width || captureHeight !== size.height) {
        throw new Error(`capture size ${captureWidth}x${captureHeight} differs from sink ${size.width}x${size.height}`);
      }
      const pixels = new Uint8Array(captureWidth * captureHeight * 4);
      renderer.readRenderTargetPixels(compositeTarget, 0, 0, captureWidth, captureHeight, pixels);
      return pixels;
    },
    dispose(): void {
      if (disposed) return;
      disposed = true;
      for (const mesh of figureMeshes.values()) stageScene.remove(mesh);
      figureMeshes.clear();
      for (const resource of owned) resource.dispose();
      compositeQuad.geometry.dispose();
      compositeMaterial.dispose();
      stageTarget.dispose();
      compositeTarget.dispose();
      renderer.dispose();
    },
    resourceInfo(): { geometries: number; textures: number } {
      return {
        geometries: renderer.info.memory.geometries,
        textures: renderer.info.memory.textures,
      };
    },
    debugGeometry(): import("./mount.js").SinkGeometry {
      if (disposed) throw new Error("debugGeometry after dispose");
      // Instance pivots/bottoms: mesh.matrix maps authoring-local points to
      // render world (V3 +Z-away negated); negating Z returns authoring frame.
      const point = new THREE.Vector3();
      const instances = [...figureMeshes.entries()].map(([instance_id, mesh]) => {
        const bottomY = bottomByInstance.get(instance_id) ?? 0;
        point.set(0, 0, 0).applyMatrix4(mesh.matrix);
        const pivotWorld: [number, number, number] = [point.x, point.y, -point.z];
        point.set(0, bottomY, 0).applyMatrix4(mesh.matrix);
        const bottomWorld: [number, number, number] = [point.x, point.y, -point.z];
        return { instance_id, pivotWorld, bottomWorld };
      });
      // Static joins read through the real scene-graph matrices. Deck front
      // edge: 16x8 plane rotated flat, front-center local (0,-4,0). Apron top
      // edge: 16x3 facing, top-center local (0,1.5,0). Column bases: 3x14x0.6
      // boxes, base-center local (0,-7,0).
      const joint = (
        mesh: import("three").Object3D | null,
        local: [number, number, number],
      ): [number, number, number] => {
        if (!mesh) throw new Error("debugGeometry unavailable without the calibration fixture");
        mesh.updateWorldMatrix(true, false);
        const v = new THREE.Vector3(local[0], local[1], local[2]).applyMatrix4(mesh.matrixWorld);
        return [v.x, v.y, v.z];
      };
      return {
        instances,
        joints: {
          deckFrontEdge: joint(diagDeck, [0, -4, 0]),
          apronTopEdge: joint(staging.apron, [0, 1.5, 0]),
          columnBaseL: joint(staging.columnL, [0, -7, 0]),
          columnBaseR: joint(staging.columnR, [0, -7, 0]),
        },
      };
    },
  };
  return sink;
}

/** Full-viewport immersive sink (audience): same stage, multiplane house. */
export async function createImmersiveSink(
  width: number,
  height: number,
  options: Omit<ThreeSinkOptions, "immersive"> = {},
): Promise<ImmersiveSink> {
  return (await createThreeSink(width, height, { ...options, immersive: true })) as ImmersiveSink;
}

/**
 * Neutral diagnostic stage fixture, drawn entirely in code: a deck plane,
 * one asymmetric orientation quad (warm-red left / cool-blue right, bright
 * top strip, dark bottom strip, pale marker top-left) and a smaller actor
 * card. No story facts; per-actor registered artwork binds later. Unlit
 * basic materials deliberately prove the blackout gain covers unlit pixels.
 */
function buildDiagnosticFixture(THREE: typeof import("three")): import("three").Mesh[] {
  const colorTexture = (canvas: HTMLCanvasElement): import("three").CanvasTexture => {
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = false;
    return texture;
  };
  const paint = (w: number, h: number, draw: (ctx: CanvasRenderingContext2D) => void): HTMLCanvasElement => {
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("2d canvas unavailable");
    draw(ctx);
    return canvas;
  };

  const deck = new THREE.Mesh(
    new THREE.PlaneGeometry(16, 8),
    new THREE.MeshBasicMaterial({ color: 0x4a4a4a }),
  );
  deck.rotation.x = -Math.PI / 2;
  deck.position.set(0, 0, -1.6);

  // Asymmetric orientation quad: horizontal thirds red/neutral/blue against
  // the V3 flip (never negate just actor Z), vertical bright-top/dark-bottom
  // against pivot mistakes, pale marker top-left against mirroring.
  const orientationCanvas = paint(96, 144, (ctx) => {
    ctx.fillStyle = "#a83a2c";
    ctx.fillRect(0, 0, 32, 144);
    ctx.fillStyle = "#5a5a5a";
    ctx.fillRect(32, 0, 32, 144);
    ctx.fillStyle = "#2c5fa8";
    ctx.fillRect(64, 0, 32, 144);
    ctx.fillStyle = "#f2ede2";
    ctx.fillRect(0, 0, 96, 18);
    ctx.fillStyle = "#0b0b0b";
    ctx.fillRect(0, 126, 96, 18);
    ctx.fillStyle = "#f2ede2";
    ctx.fillRect(4, 22, 12, 12);
  });
  const orientation = new THREE.Mesh(
    new THREE.PlaneGeometry(2, 3),
    new THREE.MeshBasicMaterial({ map: colorTexture(orientationCanvas) }),
  );
  orientation.position.set(-1.5, 2.2, 0);

  const cardCanvas = paint(48, 96, (ctx) => {
    ctx.fillStyle = "#2ca87a";
    ctx.fillRect(0, 0, 24, 96);
    ctx.fillStyle = "#2c9aa8";
    ctx.fillRect(24, 0, 24, 96);
    ctx.fillStyle = "#f2ede2";
    ctx.fillRect(0, 0, 48, 10);
    ctx.fillStyle = "#0b0b0b";
    ctx.fillRect(0, 86, 48, 10);
  });
  const card = new THREE.Mesh(
    new THREE.PlaneGeometry(1, 2),
    new THREE.MeshBasicMaterial({ map: colorTexture(cardCanvas) }),
  );
  card.position.set(2.2, 1.6, 0.4);

  return [deck, orientation, card];
}

interface StagingRig {
  stage: import("three").Mesh[];
  house: import("three").Object3D[];
  resources: { dispose(): void }[];
  curtainL: import("three").Mesh;
  curtainR: import("three").Mesh;
  /** Apron facing mesh (top edge meets the deck front at Y=0, Z=2.4). */
  apron: import("three").Mesh;
  columnL: import("three").Mesh;
  columnR: import("three").Mesh;
  title: import("three").Mesh;
  /** Title card material: tinted by the blackout gain every render. */
  titleMat: import("three").MeshBasicMaterial;
  /** Bounded-shell pieces the immersive facade/lectern replace. */
  bounded: import("three").Object3D[];
  repaintTitle(title: TitleCard): void;
  textureBytes: number;
  sign: {
    group: import("three").Group;
    board: import("three").Mesh;
    ropeL: import("three").Mesh;
    ropeR: import("three").Mesh;
  };
}

/**
 * Neutral stage architecture plus the house rigs, all drawn in code
 * (shell.json ships style neutral-v1 with null images). The narrative
 * aperture (back wall, title card) composites through the blackout gain;
 * permanent house geometry (columns, header, apron facing, plaque backing,
 * opening curtains — motion still media-driven) and the rope-hung
 * INTERMISSION board (pause-driven through setSign) enter the
 * independently-lit house scene and stay readable through blackouts.
 */
function buildStaging(
  THREE: typeof import("three"),
  title: TitleCard,
): StagingRig {
  const resources: { dispose(): void }[] = [];
  const track = <T extends { dispose(): void }>(resource: T): T => {
    resources.push(resource);
    return resource;
  };
  const basic = (color: number): import("three").MeshBasicMaterial =>
    track(new THREE.MeshBasicMaterial({ color }));
  const codeTexture = (w: number, h: number, draw: (ctx: CanvasRenderingContext2D) => void): import("three").Texture => {
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("2d canvas unavailable");
    draw(ctx);
    const texture = track(new THREE.CanvasTexture(canvas));
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = false;
    return texture;
  };
  const flat = (
    w: number, h: number, color: number, x: number, y: number, z: number,
  ): import("three").Mesh => {
    const mesh = new THREE.Mesh(track(new THREE.PlaneGeometry(w, h)), basic(color));
    mesh.position.set(x, y, z);
    return mesh;
  };

  // Narrative aperture: back wall (V3 +Z-away negates into three.js
  // distance, as F*M*F maps actor positions) plus the title card. These
  // composite through the blackout gain.
  const stage: import("three").Mesh[] = [
    flat(22, 14, 0x101010, 0, 4, -LAYER_Z.inner),
  ];
  // Permanent house geometry: proscenium, apron facing, plaque backing.
  // Depths keep three.js-near values matching the verified fixture space.
  const apron = flat(16, 3, 0x181818, 0, -1.5, LAYER_Z.apron);
  const plaqueBacking = flat(PLAQUE.x[1] - PLAQUE.x[0], PLAQUE.y[1] - PLAQUE.y[0], 0x0c0c0c, 0, (PLAQUE.y[0] + PLAQUE.y[1]) / 2, LAYER_Z.plaque);
  const house: import("three").Object3D[] = [apron, plaqueBacking];
  const headerGeo = (): import("three").BoxGeometry => track(new THREE.BoxGeometry(3, 14, 0.6));
  const columns: import("three").Mesh[] = [];
  for (const side of [-1, 1]) {
    const column = new THREE.Mesh(headerGeo(), basic(0x232323));
    column.position.set(side * 9.5, 4, LAYER_Z.columns);
    house.push(column);
    columns.push(column);
  }
  const header = new THREE.Mesh(track(new THREE.BoxGeometry(22, 2, 0.6)), basic(0x232323));
  header.position.set(0, 10, LAYER_Z.columns);
  house.push(header);

  const curtainTexture = codeTexture(256, 288, (ctx) => {
    ctx.fillStyle = "#5e1f1f";
    ctx.fillRect(0, 0, 256, 288);
    for (let x = 0; x < 256; x += 32) {
      ctx.fillStyle = "#471515";
      ctx.fillRect(x, 0, 14, 288);
      ctx.fillStyle = "#722929";
      ctx.fillRect(x + 14, 0, 3, 288);
    }
    ctx.fillStyle = "#3a1010";
    ctx.fillRect(0, 0, 256, 10);
    ctx.fillRect(0, 272, 256, 16);
  });
  const curtainHeight = OPENING.y[1] - OPENING.y[0];
  const curtainY = (OPENING.y[0] + OPENING.y[1]) / 2;
  const curtainGeo = track(new THREE.PlaneGeometry(8, curtainHeight));
  const curtainMatL = track(new THREE.MeshLambertMaterial({ map: curtainTexture }));
  const curtainMatR = track(new THREE.MeshLambertMaterial({ map: curtainTexture }));
  const curtainL = new THREE.Mesh(curtainGeo, curtainMatL);
  curtainL.position.set(curtainPanelX("left", 0), curtainY, 2.2);
  curtainL.castShadow = true;
  curtainL.receiveShadow = true;
  const curtainR = new THREE.Mesh(curtainGeo, curtainMatR);
  curtainR.position.set(curtainPanelX("right", 0), curtainY, 2.2);
  curtainR.castShadow = true;
  curtainR.receiveShadow = true;
  house.push(curtainL, curtainR);

  const paintTitle = (ctx: CanvasRenderingContext2D, title: TitleCard): void => {
    ctx.fillStyle = "#131009";
    ctx.fillRect(0, 0, 512, 160);
    ctx.fillStyle = "#8a6d2f";
    ctx.fillRect(40, 14, 432, 4);
    ctx.fillRect(40, 142, 432, 4);
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    // Shrink long lettering to the card instead of clipping it.
    const fitted = (text: string, px: number, y: number, color: string): void => {
      ctx.fillStyle = color;
      do ctx.font = `${px}px Georgia, serif`; while (ctx.measureText(text).width > 452 && --px > 12);
      ctx.fillText(text, 256, y);
    };
    fitted(title.heading.slice(0, 40), 46, 66, "#eadfc2");
    fitted(title.sub.slice(0, 48), 26, 114, "#b9a87e");
  };
  const titleTexture = codeTexture(512, 160, (ctx) => paintTitle(ctx, title));
  const titleMat = track(new THREE.MeshBasicMaterial({ map: titleTexture, transparent: true }));
  const titleCard = new THREE.Mesh(
    track(new THREE.PlaneGeometry(10, 3.125)),
    titleMat,
  );
  // In front of the closed curtains (2.2); visible only while they hide
  // the stage behind it. The card lives in the house pass so closed
  // curtains can never cover it; the render loop tints it by the blackout
  // gain so dark holds stay dark.
  titleCard.position.set(0, 4.5, 2.5);
  house.push(titleCard);

  const signTexture = codeTexture(512, 128, (ctx) => {
    ctx.fillStyle = "#1f1a12";
    ctx.fillRect(0, 0, 512, 128);
    ctx.strokeStyle = "#8a6d2f";
    ctx.lineWidth = 10;
    ctx.strokeRect(8, 8, 496, 112);
    ctx.fillStyle = "#ecdfbe";
    ctx.font = "bold 54px Georgia, serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    try {
      (ctx as CanvasRenderingContext2D & { letterSpacing?: string }).letterSpacing = "8px";
    } catch {
      // Letterspacing is decorative; the board reads without it.
    }
    // Fit the lettering inside the frame (letterspacing widens it).
    let px = 54;
    while (ctx.measureText("INTERMISSION").width > 440 && px > 24) ctx.font = `bold ${--px}px Georgia, serif`;
    ctx.fillText("INTERMISSION", 256, 68);
  });
  const signGroup = new THREE.Group();
  const board = new THREE.Mesh(
    track(new THREE.PlaneGeometry(
      SIGN_BOUNDS.x[1] - SIGN_BOUNDS.x[0],
      SIGN_BOUNDS.y[1] - SIGN_BOUNDS.y[0],
    )),
    track(new THREE.MeshBasicMaterial({ map: signTexture })),
  );
  const ropeGeo = track(new THREE.BoxGeometry(0.09, 1, 0.09));
  ropeGeo.translate(0, -0.5, 0);
  const ropeMat = basic(0x3d3d3d);
  const ropeL = new THREE.Mesh(ropeGeo, ropeMat);
  const ropeR = new THREE.Mesh(ropeGeo, ropeMat);
  board.position.set(0, signBoardY(1), SIGN_BOUNDS.z);
  signGroup.add(board, ropeL, ropeR);
  signGroup.visible = false;
  house.push(signGroup);
  return {
    stage,
    house,
    resources,
    curtainL,
    curtainR,
    title: titleCard,
    titleMat,
    bounded: [apron, plaqueBacking, ...columns, header],
    repaintTitle(next: TitleCard): void {
      const canvas = titleTexture.image as HTMLCanvasElement;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      paintTitle(ctx, next);
      titleTexture.needsUpdate = true;
    },
    textureBytes: (256 * 288 + 512 * 160 + 512 * 128) * 4,
    sign: { group: signGroup, board, ropeL, ropeR },
    apron,
    columnL: columns[0]!,
    columnR: columns[1]!,
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
