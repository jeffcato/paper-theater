# First diagnostic integration verification

2026-10-26. Installed Rust 1.99.0, Node 24.20.0, npm 11.19.0, Python 3.14.3. Rust minimum is the actually tested toolchain; it is not inherited from an experimental manifest. Root npm packages were queried before pinning TypeScript 7.0.2, Vite 8.3.5 and React 19.3.1. Three.js and declarations resolve exactly to 0.186.0. Original-version receipts are not upgraded receipts.

Contract-first red/green evidence after the root instruction update:

- Exact 96,000-sample dark hold: missing validator failed compilation; implemented validator passes and rejects 95,999 samples.
- Portable diagnostic fixture: missing public validators failed; lighting flattening and typed-file binding defects were exposed and fixed; saved package now passes exact-file checks while production remains blocked.
- Read-along: malformed scalar spans, unsafe sample, omitted group and moved point were accepted before validation; measured word/group validation rejects them and accepts all saved words/groups.
- Reveal evidence: missing eligibility/validator failed; checks now reject unresolved/stale evidence, repeated/ambiguous terminal occurrences and early bounds. Null remains ineligible.
- Registration/cues: invalid pivot was accepted before geometry validation; registration, mirroring and bounded cue checks now reject it. The new cue-slot check exposed the fixture's misplaced nod; fixture construction was fixed before its first freeze.
- Rust/TypeScript exchange: check failed without the Rust serializer example. It now serializes 14 records from actual Rust structs; TypeScript accepts both valid diagnostic cases and rejects all 12 invalid cases with matching failure codes and useful fields.

Actual results: `cargo test --workspace --offline` passed; `npm run check:contracts` passed 14 records; `npm run check:player` passed 43 tests; `npm run build` built both apps and both packages; identity scans passed on sources, app outputs and the server binary.

Actual Chrome check: `node apps/studio/scripts/browser-check.mjs` passed start/cancel/reload/reconnect and replay with zero replay POSTs; all 39 committed events arrived once and in order. Screenshot is in the runtime lane. Local loopback browser execution required sandbox network permission. This check does not prove shared-player mounting or rendered stage acceptance. The studio currently reports mounting pending when that API is absent.

Pending: full diagnostic saved-media mount/capture through the studio; player migration to frozen TS contracts; complete production acceptance verifier; provider quota/retry/recovery metadata refinements; new PNG candidate metadata; live authorization/accounting; autonomous excerpt and both full chapters; target-device geometry/blackout/performance measurements. The canonical fixture is intentionally diagnostic and remains rejected by production loaders.
