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

Clean checkout `8ac0e5b`: copied from Git archive into a fresh temporary directory; SOURCE_ROOT unset for every command. `cargo test --workspace --locked --offline`, offline `npm ci`, contract/player/studio checks, both app/package builds, server build and identity scans all passed. A first attempt exposed the now-fixed candidate substitution regression and missing npm lockfile tarball identities; the second clean run used the corrected snapshot and exact cached tarballs. No source-workspace paths or symlinks are used.

Final code snapshot `41061a6`: fresh Git archive, SOURCE_ROOT unset, exact cached npm tarballs and an isolated Cargo dependency cache. Workspace tests, 14 shared contract cases, 43 player checks, studio check, both package/app builds, server build and all output identity scans passed. The same clean snapshot passed the Chrome saved-generation browser flow (start/cancel/reload/reconnect, replay zero POSTs, 39 ordered events and diagnostic NeedsRepair). Chrome 155.0.8059.85 on macOS 26.5.2; default viewport is not pinned for rendering acceptance. Mount/capture and actual device rendering remain pending.

Follow-up browser check also killed/restarted the server process and verified reuse of its committed source checkpoint before running the saved diagnostic package. It passed with no provider calls. The test-only follow-up and its actual screenshot are committed separately from the code snapshot.

Priority unblock: native Node package import first failed with ERR_MODULE_NOT_FOUND for src/types.js. A regression now imports @paper-theater/contracts by name; exporting dist/index.js with dist/index.d.ts makes it pass. An extracted npm tarball also imports in an isolated Node consumer. pt-services-v1.1.0 adds verified read_candidate; the initial integration test failed because the method/typed failure did not exist, then passed against the real runtime. Three integration cases cover correct bytes, separate execution scopes, forged hash/size/media/provenance, unknown IDs, escaping paths and cancelled epochs. Exact staged snapshot passed workspace tests, 14 shared cases, player/studio checks, app builds and identity scans. Canonical fixture bytes/digest are unchanged; provider calls: zero. See candidate-read-ready.json.
