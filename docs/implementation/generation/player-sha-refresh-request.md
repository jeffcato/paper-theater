# Request to Agent 4 (player): refresh pinned house-art hashes after flood-cut promotion

(Relays an operator-approved cross-lane proposal. Agent 3 does not edit
Agent 4's files; Agent 4 owns this change in its lane.)

You are the Paper Theater player agent. Read `docs/agent-handoffs/COMMON.md`,
`docs/agent-handoffs/INTERFACES.md`, and your role prompt
`docs/agent-handoffs/04-player-and-audience.md` before acting. This request
concerns only your owned paths.

## Background

Agent 3 cut the white backgrounds off two geometric-cut pieces with a
deterministic border flood-fill (zero provider calls, gates pass) and, with
operator approval, promoted the outputs to the canonical fixture names.
Dimensions are unchanged; bytes changed. Originals are preserved in place:

- `fixtures/generation/immersive-house-test-v2/house.facade-v2.png`
  now `247fba44cfd088e1b6d8272ca272e335e38bbbea85566ba1554365147490b40c`
  (was `4c4e5599220c3854666ebb08ecebfeb310d593f2e1286cc3646d95ef7a9f9c2d`;
  preserved as `house.facade-v2.pre-flood.png`), 2048x1245.
- `fixtures/generation/immersive-house-test-v2/lobby-grille.png`
  now `a5d5bb94987279281844be9346e0292681ab2593e5a78d879f34d4c8b1a578`
  (was `32f399b930dc7df30686d6dcc566535fa8a12d1786475ff616d586c0f10b218e`;
  preserved as `lobby-grille.pre-flood.png`), 2560x1792.

Proof of breakage (operator's console, audience dev harness):
`house_art_digest_mismatch: house.facade expected 4c4e5599…, got 247fba44…`
at `loadDevHouseArt (dev-art.ts:172)`. The loader is correct; the pins are
stale. The grille pin will mismatch next for the same reason.

## Task (your files only)

1. In `apps/audience/src/dev-art.ts`, replace the two pinned `sha256`
   strings: `house.facade` → `247fba44cfd088e1b6d8272ca272e335e38bbbea85566ba1554365147490b40c`;
   `lobby.grille` →
   `a5d5bb94987279281844be9346e0292681ab2593e5a78d879f34d4c8b1a578`.
2. In `docs/implementation/player/house-art-manifest.md`, apply the same
   two `sha256` replacements.
3. Do NOT touch `fixtures/generation/**`, `tools/generation/**`, or any
   other lane's paths. No action is needed in
   `packages/player/tests/house-art.test.ts` (it hashes fixture files at
   runtime and is self-consistent).

## Verification (report exact commands and output)

1. `grep` both files for the old hashes (`4c4e5599`, `32f399b9`): expect no
   remaining pins.
2. Load the audience dev harness through the path that produced the pasted
   error (`mountWorld` → `loadDevHouseArt`) and confirm both assets load
   with no `house_art_digest_mismatch`.
3. Run `python3 tools/check_product_identity.py` over every file you
   touched: must pass.
4. Report: files changed, verification results, and any further mismatch
   (e.g. additional pins of these bytes elsewhere in your lane).

## Standing constraints

- Diagnostic only; this changes no acceptance, registration, or catalog
  state.
- Shared tree: do not stage, commit, switch branches, or push (Agent 1
  owns git operations). Preserve other agents' edits.
- If either new hash fails to load for reasons beyond the pin itself
  (decode, size, alpha handling), stop, preserve the error, and report it
  instead of working around it in the loader.
