# Player baseline (Agent 4)

Source baseline: the staging theater example (`docs/examples/oz_scene/theater/`,
~3,600 lines across engine/compiler/release/types) was read transiently and is
preserved as reusable patterns: fail-closed release inspection with immutable
`ValidatedRelease` handles, deterministic evaluation with synchronous
narrative cleanup on seek, `point-following-same-boundary-v1` grouping, and
the two-second dark-hold compilation. The staging workspace was not mutated;
no absolute source paths are serialized in the product.

Original-version receipts: staging preflight results describe Three.js
0.170.x (`three`/`@types/three` `^0.170.0` in the staging example manifest).
Those receipts are preserved as historical evidence for 0.170.x only and are
not relabeled as 0.186 results.

Reproducible product baseline (this lane):

- `npm run check:player` in `packages/player` — clean `tsc` build followed by
  `node --test` against freshly compiled `dist` modules. Groups: `contracts`,
  `loader`, `transport`, `shell`, `timeline`, `readalong`, `integration`
  (includes `tests/fixtures.test.ts` over `fixtures/player`), then the
  aggregate. See [checks.md](checks.md) for the current results.
- `npm test` in `apps/audience` — ticket-catalog rule tests (5 tests).
- `vite build` in `apps/audience` — production bundle smoke: `index.html`
  serves 200, `tickets/catalog.json` 404s, so the booth shows the honest
  unavailable state with retry.
