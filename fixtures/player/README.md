# Player fixtures (Agent 4 owned)

All fixtures here are **diagnostic/synthetic structural cases**. None carries
release permission: a schema-valid fixture with missing evidence fails
production loading (`loadRelease` rejects; `loadDiagnostic` reports missing
acceptance IDs without accepting).

- `two-scene-local/timeline.json` — synthetic 18-second arithmetic timeline
  from the player handoff (title hold/dismissal, curtain opening, scene A/B
  narration slots, one formal transition with an exact 96,000-sample dark
  hold, closing inside the master endpoint). Timing is arithmetic only: no
  measured speech, no release permission.
- `two-scene-local/manifest.diagnostic.json` + `acceptance.json` —
  diagnostic-provenance manifest whose reveal references resolve to missing
  acceptance IDs. The diagnostic entry point reports them; the production
  path rejects the manifest.
- `negatives/bad-hold.json` — transition with `fade_in_start` at 527999
  (dark hold 95,999). Must fail validation; neither rounding nor a longer
  fade repairs the hold.
- `negatives/unsupported-action.json` — cue with the legacy `pose` action,
  outside the production allowlist. Must fail validation.

Covered by `npm run check:player -- integration` (`tests/fixtures.test.ts`).
