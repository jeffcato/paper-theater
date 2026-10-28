# Studio goal

**Objective:** complete the Required implementation checklist for a functional redesigned Studio showing actual runs through the shared player. Follow [RUNBOOK](RUNBOOK.md). Owner: the operator-assigned Studio redesign agent. Suggested model: Terra medium or Sol medium.

Own all `apps/studio/**`, including its tests/configuration, and this file. This is the existing temporary ownership override; server and player do not edit your app. Do not edit runtime/player/contracts/root dependencies or other agents' evidence. Freeze the functional design for the delivery plan; no second player, creative-selection editor or model/budget approval UI.

## Required implementation

- [ ] **STU-01 — Adopt current redesign and isolate its outputs.** Preserve current components/styles/tests. Establish the actual HTTP/SSE/player imports and source baseline. Update owned browser scripts to write new receipts/captures under Studio-owned evidence or a deliberately agreed handoff path; current scripts overwrite runtime-owned PNG/receipt paths. Keep historical runtime evidence intact and return exact cross-lane requests instead of editing their files.
- [ ] **STU-02 — Complete source/run navigation and actions.** Display actual available sources, run history and current run with idempotent start/cancel through existing contracts. Handle duplicate starts, unavailable configuration, waits, failure, cancellation and actual terminal state. No invented progress percentage or mock completed task; no human creative candidate/voice/art/spend approval flow.
- [ ] **STU-03 — Finish truthful snapshot/journal/SSE state.** Merge snapshot sequence N with events after N, deduplicate by committed identity, reconnect without losing committed work and recover invalid cursors. Guard stale fetch/selection completions, canceled runs and unmounted listeners. Render real causal decisions/tasks/artifact identities and concise diagnostics; recorded and live data use the same state/views.
- [ ] **STU-04 — Complete the artifact inspector and provenance walk.** Follow actual source→decision→image/mask/audio→candidate/package links using safe server-provided summaries and allowlisted media. Show missing/private/unavailable/diagnostic status honestly. Bound media loading/URLs, provide useful errors and dispose previews. Do not implement generic CAS access or expose private raw provider/accounting/evidence just to fill the inspector; ask server for a curated response.
- [ ] **STU-05 — Finish shared-player preview and viewport lifecycle.** Use only the public shared loader/mount/sink/plaque/resize/dispose APIs. Fix app-owned nonpositive/hidden viewport initialization, selection races, remount/resize, seek/play/pause and cleanup. Wait for real positive bounds without a fake fixed-size pass. Preserve explicit diagnostic rejection from production loading; player-owned defects go to its owner with a minimal case.
- [ ] **STU-06 — Provide coherent usable layouts and accessibility.** Complete desktop layout and supported content sizes, readable journal/inspector controls, keyboard/focus/error/loading states and reduced-motion treatment. Preserve player plaque clearance/text sizing and avoid hidden/zero-size canvas containers. Freeze cosmetic expansion once the functional handoff is ready; request root dependency changes through integration only.
- [ ] **STU-07 — Prove the real consumer flow and close app-side R12.** Use the built actual backend and shared player in a bounded local browser check, not only a mocked DOM. Cover source/start/events/reconnect/cancel/inspect/preview/seek/replay and navigation/disposal. Rerun title visibility, measured audio/highlight, nod, blackout and positive viewport/resize checks with current build/fixture identities and clean real console results. Keep strict assertions; old historical screenshots or flags do not close the gate.
- [ ] **STU-08 — Deliver the functional freeze and recording path.** Pass owned type/model/browser/build/identity checks, provide the exact launch/recorded-run workflow and stable explicit-path handoff to integration. Record current screenshots/receipt in owned paths and list actual limits/blocked backend/media fields. The walkthrough must show real saved provenance, distinguish replay from live generation and work without provider dispatch. Do not deploy or submit externally.

## Checks and dependencies

Use `npm run check --workspace @paper-theater/studio`, `npm run build --workspace @paper-theater/studio` and current browser/preview/strict-consumer scripts after reading the owned package manifest and building the required server/player. Use temporary data and available ports; do not kill other workers' sessions. No passing full-flow claim if a required real backend/browser step was not run. PLY-01/04 supplies the stable player API; SRV-07 supplies safe state/artifact views; INT-02 supplies shared contracts/root dependencies. Continue model/layout/reconnect work while a media or endpoint request waits.

## Operational validation gates

A genuinely generated accepted chapter and final actual-Mac walkthrough may arrive later. Finish and verify app behavior on honestly diagnostic saved runs, then consume the real run when available. A successful diagnostic preview cannot populate an audience ticket or authorize production. Submission format/portal confirmation and final upload belong to the operator.

## Evidence and status

| Item | Owned change / handoff | Exact check and outcome | Source/contract identity / limitation |
|---|---|---|---|
| — | Not started under this goal | — | Adopt current Studio edits, preserve runtime evidence |

## Dependency requests

| To | Exact missing input/action and reproducer | State |
|---|---|---|
| — | None recorded | — |
