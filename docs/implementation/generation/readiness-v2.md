# Readiness correction: two-scene-readiness-v2

Published as a separate **diagnostic** revision in [two-scene-readiness-v2](../../../fixtures/generation/two-scene-readiness-v2/revision.json). The frozen shared v1 fixture and its manifest remain unchanged. This publication creates a versioned fixture and review proposal; it does not activate a production catalog entry or grant acceptance.

The new manifest SHA-256 is `2070bb5f88d5d68fe6156aa4df17673634fc110584ec5f7c03d82bf5bddc502b`; its complete file-index digest is `40dde4e6f8bd36bd2db478c6a6ea8449372a7577cff5407dccb732807cde992c`. Parent fixture digest: `5a662fb9d1687fd61a692acdd4265bc12aac9d5bfacb8e5f75e58b472dbf776c`.

The correction moves the existing four-second transition into the measured sentence pause after “hole.” It chooses source sample **801849**, an actual PCM16 zero nearest the pause midpoint. The surrounding measured gap is [792240, 811440) at 24 kHz. The observed ±120-sample window peaks at 86 PCM16 units (about −51.6 dBFS); this is an edit observation, not an accepted listening review.

| Event | v1 sample / seconds | v2 sample / seconds |
|---|---:|---:|
| Figure eligibility | 1656480 / 34.51 | 1464480 / 30.51 |
| Fade out starts | 1416000 / 29.5 | 1699698 / 35.410375 |
| Dark hold starts / incoming set swap | 1464000 / 30.5 | 1747698 / 36.410375 |
| Fade in starts | 1560000 / 32.5 | 1843698 / 38.410375 |
| Fade in ends / full picture | 1608000 / 33.5 | 1891698 / 39.410375 |
| Nod | [1656480, 1680480) / 34.51–35.01 | [1891698, 1915698) / 39.410375–39.910375 |
| Terminal state | 3262560 / 67.97 | 3262560 / 67.97 |

The figure is eligible **379218 samples (7.900375 s) before fade-in starts**. Both required contributors pass the deterministic diagnostic entry-readiness check. The previous 2.01-second late entry remains visible in v1.

The eligibility change follows the presentation map: the same source fact is now narrated before the relocated transition. Its raw acoustic measurement is unchanged. No earlier acoustic endpoint is invented. The word/point intervals are remapped by exact integer arithmetic; all 176 source word identities, 170 highlight memberships, six measured points, punctuation and source order remain intact.

Every retained stereo PCM frame is byte-identical after removing the old/new inserted transition gaps. The existing 192000 silent samples move as one block; narration is absent through the one-second fade out, exact 96000-sample dark hold and one-second fade in. The new master is still 48 kHz stereo PCM16 and 67.97 seconds. Its SHA-256 is `3d2ca213cf76882d89241ab19e25ef2c7663fdc64b315be416d5f565a15f1e37`. Original source WAV and alignment bytes are included unchanged as evidence.

Scene source spans now partition at scalar 548, before “Dorothy.” The nod retains its ID, target, action, easing and 24000-sample duration; [correction.json](../../../fixtures/generation/two-scene-readiness-v2/correction.json) explicitly records its revision to the new full-picture entrance. No required action is silently dropped. The opening, closing, profile, sprite and total sample count remain unchanged.

The [v2 capture inventory](../../../fixtures/generation/capture-readiness-v2/inventory.json) has 48 sample positions / 144 requests with its own manifest, profile, player and checker bindings. The [state-only probe](../../../fixtures/generation/capture-readiness-v2/state-probe.json) accepts all requests and still reports `action:nod-em:gesture-state` as fail in the current compiled player. Actual browser images, audible joins, long-hold narrative readability, source/image compatibility and rendered motion still require review. The existing incoming-picture-readiness production finding remains Missing; a diagnostic arithmetic pass cannot certify those reviews. Synthetic independent/image attestations retain their real provenance, and production loading remains blocked.

The v1 capture inventory/probe are preserved as historical receipts for the earlier generator/player snapshots. The version-aware generator rejects stale bindings; generate fresh requests in a separate output directory for any current rerun. It does not overwrite either published fixture.

Reproduce and verify:

```sh
python3 -B tools/generation/readiness_correction.py --output fixtures/generation/two-scene-readiness-v2
python3 -B tools/generation/capture_inventory.py --fixture fixtures/generation/two-scene-readiness-v2 --output fixtures/generation/capture-readiness-v2/inventory.json
node tools/generation/probe_capture_inventory.mjs --inventory fixtures/generation/capture-readiness-v2/inventory.json --output fixtures/generation/capture-readiness-v2/state-probe.json
python3 -B -m unittest discover -s tools/generation -p 'test_*.py'
cargo test -p pt-production generation:: --offline
python3 tools/check_product_identity.py .
```

Results: **17 Python tests and 5 Rust generation tests pass**, including shared `validate_playback` / `verify_package_files`, original PCM equality, unchanged v1 hashes, exact blackout phases, preserved point/group membership, independent v2 capture bindings and immutable publication. Tests first failed for the absent correction generator/publication API, absent v2 fixture and missing version-aware inventory parameter. Publication is idempotent and refuses changed or additional bytes in an existing version. Zero provider calls; no new recordings, spending, credential access or production publication.

Agent 4 can now capture v2's incoming picture at samples 1843697 / 1843698 / 1843699 and full picture at 1891697 / 1891698 / 1891699, plus the rescheduled nod's paired/interior frames. Keep v1's failure evidence for comparison. The operator explicitly requested this separate publication; its Git changes are restricted to the generation lane.
