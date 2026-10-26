# Version and corpus provenance (Agent 4)

Selected target: Three.js **0.186.0** with `WebGLRenderer`, per the player
handoff. Pinned in `packages/player/package.json`:

- `three`: `0.186.0` (exact)
- `@types/three`: `0.186.0` (exact)

## Provenance records (October 26, 2026)

- Registry: `npm view three@0.186.0` resolves `0.186.0` with dist integrity
  `sha512-cr/fIM2ddMSVbYVgkfD4jLJv7Fh/8ZTjvo+7gQeSVGUZHxpx9FDwoL5iC7hUz/LiRA8wMbqfnb90xKfm1/HHkQ==`
  (dist-tags latest at the time: `0.186.1`; the player pins `0.186.0`, not latest).
- `@types/three@0.186.0` resolves with dist integrity
  `sha512-mxYSBpDC+D0pLfSP6sW4WZTcT+nrtmZcimMqnVmy36Hte3XpeYSrvgg4TRdaM1GemGog1AWzI5qL2VoIfMXbJQ==`.
- A transient read-only local checkout reported `package.json` version
  `0.186.0` at commit `5efce9dcb3b6d063e4e96772374ba4c5bcbe2626`.
  (The checkout's machine path is intentionally not recorded: external source
  locations are transient and must never be serialized into the product.)
- The product never depends on that checkout at build/runtime: no path
  dependency, no symlink, no serialized absolute path.

## Tutor index status

Tutor lookups used for API guidance only (`WebGLRenderer`,
`WebGLRenderTarget`, `Texture`, `PCFShadowMap` examples, the threejs skill
references for lighting/textures/fundamentals/interaction). `tutor_status`
reports the active corpus as `three.js` but exposes **no source revision**,
so the corpus name is not accepted as revision proof. No separately named
0.186.0 corpus was built in this lane; if Agent 1 wants a pinned corpus,
the request is recorded in [integration-request.md](integration-request.md).

## Migration audit (0.170 → 0.186)

- `PCFSoftShadowMap` → `PCFShadowMap` (`src/shell.ts`,
  `createStageRenderer`). The threejs skill notes `shadow.radius` blur
  applies to `PCFSoftShadowMap` only, so the player does not depend on it.
- `outputColorSpace = SRGBColorSpace`, `toneMapping = NoToneMapping`
  (profile: sRGB output, linear working target, no tone mapping).
- Color textures: `SRGBColorSpace`, `LinearFilter`, `generateMipmaps = false`.
  Mask/depth textures: `NoColorSpace` (no color conversion).
- `WebGLRenderer` retained; single render scheduler owned by the caller;
  no WebGPU/TSL migration.
- Guard: `tests/integration.test.ts` fails the build if `PCFSoftShadowMap`
  appears in player source code (comments documenting the migration exempt).
