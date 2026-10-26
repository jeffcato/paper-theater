# Frozen fixture capture sampling inventory

Prepared from `fixtures/shared/two-scene-v1`, contract `pt-contracts-v1.0.0`, fixture digest `5a662fb9d1687fd61a692acdd4265bc12aac9d5bfacb8e5f75e58b472dbf776c`, manifest `04f11a892cb775590120c371c7b8effbf6ea6460ca2d800e5f159d7ed46c1f1d`. The profile remains `paper-player-r186-v1`, selected/unverified.

[Inventory](../../../fixtures/generation/capture/inventory.json) contains **48 distinct sample positions and 144 ready CaptureRequest records**, covering 960×540, 1280×720 and 1920×1080 at DPR 1. Integer samples use the 48 kHz delivery master. Event boundaries include sample−1, sample and sample+1 within bounds. The terminal sample 3,262,560 is an explicit ended-state evaluation; it is not another audio frame.

| Coverage | Sample interval or point | Seconds |
|---|---:|---:|
| Title hold | [0, 72000) | 0–1.5 |
| Title dismiss | [72000, 96000) | 1.5–2 |
| Introductory prose, curtains closed | [96000, 168000) | 2–3.5 |
| First reveal / opening starts | 168000 | 3.5 |
| Opening curtains | [168000, 240000) | 3.5–5 |
| First full picture hold | [240000, 1416000) | 5–29.5 |
| Fade out | [1416000, 1464000) | 29.5–30.5 |
| Exact dark hold | [1464000, 1560000) | 30.5–32.5 |
| Incoming fade in | [1560000, 1608000) | 32.5–33.5 |
| Second full picture interval | [1608000, 3190560) | 33.5–66.47 |
| Second reveal / nod starts | 1656480 | 34.51 |
| `nod-em` action | [1656480, 1680480) | 34.51–35.01 |
| Last prose/action / closing begins | 3190560 | 66.47 |
| Closing curtains | [3190560, 3262560) | 66.47–67.97 |
| Exact terminal state | 3262560 | 67.97 |

Every hold carries its measured duration and first/middle/last-active sample references. `nod-em` additionally has 25%, 50%, 75%, before/start/end samples, an explicit two-frame motion pair and a contextual clip interval. Backward-seek sequences revisit each reveal from terminal, checking that later contributors disappear. Terminal expectations bind the authored terminal snapshot, empty instance list and fully closed curtains.

Requests bind the frozen manifest/profile, a content-derived fixture candidate ID, the exact existing compiled player JS inventory and the inventory generator bytes. The candidate ID identifies this diagnostic fixture; it is not an invented runtime database ID. Player tree digests use sorted-path file records, UTF-8 sorted-key compact JSON and LF. The browser runner must verify those bytes, record its actually executed bundle/browser/device identities and provide actual image hashes. The checker pin identifies this sampling generator, not an executed creative reviewer.

The [state probe](../../../fixtures/generation/capture/state-probe.json) executed all 144 requests through the compiled player's actual `describeCapture` API with zero provider calls. It contains evaluated states only. No PNG, browser rendering, full-duration visual review or production acceptance is claimed. A state digest cannot establish motion: the current digest omits gesture/rotation fields, so the inventory requires actual target pose/pixel comparisons.

Two issues must remain visible to player capture:

- **Incoming-picture readiness fails.** Scene 2 fades in at sample 1,560,000, but `em-2` becomes diagnostically eligible at 1,656,480: 96,480 samples (2.01 s) after fade-in starts, and 48,480 samples (1.01 s) after fade-in finishes. Requests sample both sides of these boundaries. The inventory preserves the authored timing and reports `readiness: fail`.
- **The nod is absent from evaluated gesture state.** At samples 1,662,480, 1,668,480 and 1,674,480, the target `em-2` is present but has `gesture: null` and intensity 0. The probe records `action:nod-em:gesture-state` as fail. Reproducer below; Agent 4 owns resolution. `evaluate.ts` indexes cue recipients by asset ID while the authored cue targets instance ID `em-2`. No timeline retiming or cue omission was made.

All image-scope, fact, voice, narrative and rendered-review acceptance gates remain unresolved. Synthetic diagnostic reveal assertions do not grant production permissions. Sampled stills do not certify an entire hold; capture must retain duration labels and actual sequence observations. Scene-transition aperture checks must include emissive pixels/shadows while shell/text remain readable.

```sh
python3 -B -m unittest discover -s tools/generation -p test_capture_inventory.py
python3 -B tools/generation/capture_inventory.py --output fixtures/generation/capture/inventory.json
node tools/generation/probe_capture_inventory.mjs --inventory fixtures/generation/capture/inventory.json --output fixtures/generation/capture/state-probe.json
python3 tools/check_product_identity.py tools/generation fixtures/generation/capture docs/implementation/generation
```

Three new tests passed after failing for the absent generator/validator/probe. Tests reject missing frames/requests, stale manifest/player pins, missing motion pairs, invalid hold durations, out-of-bounds samples and a silently relabeled readiness failure. The probe accepts all 144 requests and reports the gesture-state failure without calling a provider. Changes remain in generation-owned paths; Agent 1 owns Git integration under the shared workspace rules.

Next: Agent 4 uses the requests with the real browser capture interface, returns exact request/state/image receipts and resolves the missing gesture state; readiness and narrative defects remain diagnostic failures until an authored revision is accepted. This work does not depend on the acoustic candidate-reader/helper API changes.
