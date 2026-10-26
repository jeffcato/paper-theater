// Shared player API (INTERFACES.md): loadRelease returns a verified
// immutable handle or typed failure; mount provides play/pause/seek/restart/
// setVolume/setReadAlong/notifications/dispose; evaluate is pure. An explicit
// diagnostic entry point renders valid fixtures while reporting missing
// acceptance IDs; it is unavailable from the audience production path.
//
// NOTE (contracts pending): wire record types here are Agent 4-local and
// provisional. They mirror the handoff names so adopting Agent 1's
// contracts-ready revision is mechanical. Never fork the public schema.
export {
  PROFILE_ID,
  CAPABILITY,
  SAMPLE_RATE,
  DARK_HOLD_SAMPLES,
  RENDER_DEFAULTS,
  baseDistance,
} from "./profile.js";
export type {
  CompiledTimeline,
  FileRef,
  Manifest,
  ReadAlongGroup,
  ReadAlongWord,
  ReleaseRef,
  RevealGroup,
  SceneTransition,
  StageSnapshot,
  StoryCue,
} from "./types.js";
export {
  sampleToSeconds,
  mediaTimeToSample,
  pivotTopToBottom,
  planeOffsetForPivot,
  planeOffsetDirect,
  assertNativeAspect,
  applyCropToSource,
  flipPointZ,
  flipMatrixZ,
  eulerToMatrixZFlipped,
  scalarToUtf16Map,
  sliceByScalar,
} from "./convert.js";
export {
  evaluate,
  evaluateEasing,
  eligibleContributors,
  validateTimeline,
  validateTransitionHold,
  lightingWithGain,
} from "./evaluate.js";
export type { EvaluatedState, EvaluatedActor, PhaseKind } from "./evaluate.js";
export {
  loadRelease,
  loadDiagnostic,
  inspectRelease,
  inspectWav,
  inspectPng,
  isAcceptedHandle,
  safePackagePath,
  sha256Hex,
  MAX_FILE_BYTES,
  MAX_MANIFEST_BYTES,
  MAX_TOTAL_BYTES,
} from "./loader.js";
export type { LoadPolicy, LoadedRelease, FileReader, Inspection } from "./loader.js";
export { createTransport, IntermissionSign } from "./transport.js";
export type {
  MountHandle,
  PauseReason,
  SignPose,
  TransportEvents,
  IntermissionOptions,
} from "./transport.js";
export {
  validateSchematicShell,
  baseCameraPose,
  envelopeCameraPose,
  plaqueGrowthSteps,
  solveHomography,
  compositeStagePixel,
  blackoutHolds,
  createStageRenderer,
  configureColorTexture,
  configureMaskTexture,
} from "./shell.js";
export { groupWords, layoutChunks, ReadAlongModel } from "./readalong.js";
export type { Chunk } from "./readalong.js";
export { describeCapture, boundarySamples } from "./capture.js";
export type { CaptureRequest, CaptureResult } from "./capture.js";
export { toCueView, toLightView } from "./shared.js";
export type { SharedCueView, FlatLight } from "./shared.js";
