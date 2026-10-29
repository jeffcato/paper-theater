# Paper Theater — Remaining Tasks (independent completion audit)

Audit date: 2026-10-27. Auditor: independent completion auditor (read-only; no code, index, or history changes made).
Product root: `~/Projects/paper-theater` (relative paths below resolve from there).

Authoritative sources consulted: `AGENTS.md`, `docs/agent-handoffs/COMMON.md`,
`docs/agent-handoffs/INTERFACES.md`, `docs/agent-handoffs/README.md`,
`docs/spec/contracts.md`, `docs/spec/playback.md`, `docs/spec/server.md`,
`TODO.md`, `docs/implementation/**` handoffs and ready-receipts, plus the
external read-only workspace via transient source-relative paths only —
`plans/2026-10-24-paper-theater-implementation.md`, `architecture/system.md`,
`architecture/generation/*`, `architecture/playback/*` and related contracts.
Latest operator decisions and `COMMON.md` drift resolutions supersede older
spec text. External paths and retired identities are never recorded here.

## 0. Recorded tree state

- `HEAD = 2e22ffd46e21ca2f295b8fe7860c9389ffa47f99` (branch
  `integration/contracts-v1`, tracking `origin/integration/contracts-v1`).
  Local `main = 6fc6285`; remote `origin = https://github.com/jeffcato/paper-theater.git`
  (fetch+push). Remote public visibility / default-branch could not be verified offline.
- Working tree at audit time: ~60 modified (`M`) + ~100 untracked (`??`) files —
  dirty, uncommitted, spanning all four lanes. All observed dirty paths fall
  inside their lane's `README.md` ownership table; no cross-lane violation detected.
- Checkout is a single shared working directory on `integration/contracts-v1`,
  so only `integration` (Agent 1) may stage/commit/switch/push. Other agents are
  active (notably Claude Code owns the player UI redesign under `packages/player/**`
  and `apps/audience/**`); that work is preserved and excluded from backend tasks.
- Identity gate: `python3 tools/check_product_identity.py .` passed
  (~1630 files) at audit time.

## 1. Current-state verdict

The frozen diagnostic boundary (`pt-contracts-v1.0.0` / `pt-services-v1.8.0`,
fixture SHA `5a662fb9…776c`) is implemented and verified offline with zero
provider calls: durable fixture/replay runs, journal/SSE/cancel/restart, the
shared-player diagnostic slice (geometry, read-along, staging, lifecycle,
context-loss recovery, chapter-WAV loading), and 5 of 37 acceptance verifiers
execute. Required chapter sources are frozen (Oz Ch.1; Wells Bk One Ch.I).
(Update 2026-10-28: pilot-recording-review and delivery-fact-mapping now
execute under matrix v5 — 7 of 37, all gates green, uncommitted.)
**But there is no accepted autonomous release of any kind**: catalog holds zero
packages, `var/` holds zero runs, both audience tickets read “Under rehearsal”,
the studio strict check (R12) is red, 30 of 37 acceptance verifiers are
explicitly unimplemented (`production-verifier-pending` retained), no live
authorization/accounting exists, and no demo-device measurement exists. The
shortest path to release is: I20 coherent backend checkpoint → R18+G24
Direction/Pilot/Speech wiring → remaining stage adapters (G1–G10) + player
device/profile gates (P6–P9) + independent verifier/publication (I4/R5) →
accepted excerpt (M5/G11) → both full chapters (M6/D3) → delivery rehearsal
and submission prep (M7/D5/D6).

## 2. Requirement-status counts

| Scope | Status |
|---|---|
| Acceptance verifiers (excerpt/chapter matrix, 37 required) | 7 execute, 30 missing |
| Diagnostic matrix checks | 3/3 execute |
| `pt-contracts` Rust suite | 48 pass, 0 fail (10 suites, offline) |
| Player unit scope (`check:player`) | 134/134 reported (P18 headless) |
| Audience catalog rules | 8 pass (reported) |
| Scoped backend reports | R17 165, R16 217, R15 184, R14 134, R13 120 scoped checks reported; G23 103+19, G22 84, G21 130, G20 101 reported; no coherent full aggregate green (aggregate Pilot failures retained) |
| Provider calls / credentials / live dispatch | 0 / unloaded / disabled |
| Durable runs in `var/`, catalog packages, accepted releases | 0 / 0 / 0 |
| Milestones M0–M4 | diagnostic slice done; production adapters partial |
| Milestones M5 excerpt, M6 chapters, M7 delivery | all pending |
| Dirty uncommitted files | ~160 (multi-lane, pending Agent 1 review) |

## 3. Requirements → implementation / evidence matrix

Legend: C complete · P partial · M missing · B blocked · U unverified (reported, not independently rerun here).

### 3.1 Sources, chapters, coverage

| Requirement (source ref) | Implementation / evidence | Status |
|---|---|---|
| Oz Ch.1 = Gutenberg #55; Wells Bk One Ch.I “The Eve of the War” = Gutenberg #36 (`COMMON.md` drift fix; `docs/spec/contracts.md`; `plans/2026-10-24-paper-theater-implementation.md`) | `fixtures/generation/source/pg55-frozen.json` (Ch.I “The Cyclone”, span [2319,8277]) and `pg36-frozen.json` (heading I., 12654 chars); raw/clean/frozen hashes + boundary regressions | C |
| Verbatim complete prose in order, no invention; speaker identity persistent (`COMMON.md`; `architecture/generation/*`) | Source freeze + G16 fact matching + G18 delivery mapping; G19/G21 chapter import bounds | P |
| Independent Unicode source/read-along/scene coverage; pinned full-source/slice relations (I17/I18; `architecture/playback/*`) | Services v1.6.0/v1.7.0 verifiers; 9 source tests; nonzero-offset returns Missing | P (file-local only; global/chapter coverage unverified) |
| Thunder Child excluded from required chapters (diagnostic-only) (`COMMON.md`) | Preserved as negative-case/diagnostic evidence | C |
| Two complete accepted first-chapter performances (`TODO.md` D3; plan M6) | No accepted package; catalog empty; R16 outcome `needs_repair`, `production_approved: false` | M |

### 3.2 Direction, research, voices, pilot, speech

| Requirement (source ref) | Implementation / evidence | Status |
|---|---|---|
| Source-backed direction: cast/beat coverage, scenes/pictures, 1 canonical image per subject type + art profile (`COMMON.md`; `architecture/generation/*`) | G22 offline Direction slice (84 checks); real research/reference/semantic review missing | P |
| Voice pilots: Flash TTS audition first for new finals; keep accepted Lite identities; adjacent-take review, safe joins, scored passage before bulk (G2; `COMMON.md`; `architecture/generation/*`) | G23 offline Pilot (103+19 checks); `bulk_speech_approved: false`; no real auditions | P |
| Speech/acoustic: brokered synthesis, primary + independent whole-interval transcription, 0.75 s tolerance on measured end, per-fact evidence (`COMMON.md`; `INTERFACES.md`) | G13/G16/G18/G19 offline stages; 176 words/170 groups preserved; `NeedsRepair`; G24 speech compiler pending | P |
| Segmentation, resampling, trims/gaps, presentation mapping, 48 kHz stereo PCM16 master, sample-exact mappings (G6; `COMMON.md`) | G18 mapping (4 mapped/3 unresolved facts); R14 runtime routing; R11/G17/P14 256 MiB master capacity | P |
| Music: director-owned bible, phrase-aware reuse, −18 LUFS / ≤−1 dBTP single master (G7) | Not implemented | M |

### 3.3 Visuals, choreography, review, shell

| Requirement (source ref) | Implementation / evidence | Status |
|---|---|---|
| Paper-look illustrations; 1 canonical sprite per book; book-local reuse; scene-art specifics (G5; `COMMON.md`; `docs/implementation/generation/scene-art-style.md`) | Saved-figure pipeline; registered pivots; immersive-house test docs v1/v2; real Image/SAM adapters missing | P |
| Concrete subjects/actions; no symbolic substitutes, empty entrances, duplicates, silent omissions (G4/G8/G11; `architecture/generation/*`) | Preflight cases; readiness v2 correction (v1 failure preserved); full adapters pending | P |
| Choreography: supported actions only (enter/exit/move/facing/nod/…); impossible actions fail (G8; `INTERFACES.md`) | Evaluator + P11 instance cues + P12 nod change-and-return hashes | P (diagnostic scope) |
| Automated acceptance: sampling inventory → player captures → 5 image criteria, acoustic, continuity, narrative readability (G9; `INTERFACES.md`) | G14 48-position/144-request inventory (`rendered: false`); P12 144/144 v2 captures; semantic reviews unimplemented | P |
| Final shell via Agent 4 briefs; bounded studio shell + 35-surface immersive house (G10; `INTERFACES.md`; shell/immersive briefs) | P13 house pass; immersive world (`multiplane`/`house`/`immersive.ts`, 163/163 checks reported); provider-backed house art still diagnostic | P |

### 3.4 Runtime, providers, orchestration

| Requirement (source ref) | Implementation / evidence | Status |
|---|---|---|
| One active run, fixed 12-stage order, SQLite sole authority, content-addressed files, per-unit checkpoints, bounded repairs; no DAG engine (`COMMON.md`; `INTERFACES.md`; `architecture/system.md`) | `runtime/` store/runner/broker/schema; fixture checkpoints/restart/cancel/epochs | P |
| Single authorization/reservation broker; money/request/repair separate; fixture/replay rejects provider ops pre-credential; no live without persisted authorization (R4; `COMMON.md`; `INTERFACES.md`) | R13 private adapter (120 tests, fake transports); R17 public methods (165 scoped checks); live construction/credentials/publication disabled | P |
| Quotas, retries, recovery, inherited accounting reconciliation; uncertain/late charges preserved | Counted fake-transport cases; real reconciliation + live authorization closed | M |
| Cancellation epochs, restart reuse, deterministic recorded replay with zero redispatch | R10/R14/R16 replay proofs; journal/SSE cursors; 39-event Chrome journal check | P |
| Publication split: generation proposes, runtime activates, player verifies independently; hash order payload→acceptance→manifest→catalog (R5; `INTERFACES.md`) | Candidate write/read (v1.1/v1.2); trusted private acceptance (R15); catalog activation + recovery missing | P |
| HTTP/SSE: books/runs/cancel/events/journal/artifacts; idempotency conflicts fail; no fabricated progress (`docs/spec/server.md`; `INTERFACES.md`) | Loopback endpoints + origin/allowlist checks; full walkthrough (R7) pending | P |
| Studio: start/cancel/inspect/preview/recorded replay; autonomous, never asks creative approval (`architecture/system.md`; `COMMON.md`) | Thin studio views; R6 diagnostic mount; R12 strict check RED (Nonpositive viewport on seek) | P/B |

### 3.5 Acceptance, evidence, publication

| Requirement (source ref) | Implementation / evidence | Status |
|---|---|---|
| Independent acceptance binds bytes/recipes/profiles/checker versions; diagnostic success never authorizes production (I4; `COMMON.md`) | Services v1.4.0→v1.8.0; 5 verifiers execute; `production-verifier-pending` retained; forged/deserialized passes rejected | P |
| 30 missing semantic/device/provider verifiers (acceptance-ready `unimplemented_production_checks`) | Explicitly unimplemented, incl. provenance, chapter completeness, image/speech/music/story/action/render reviews, profile/device, publication-recovery, spend authorization, export boundary; pilot + fact-mapping now execute (matrix v4/v5) | M |
| Evidence freshness; missing/stale/ambiguous/uncertain fails; private/public separation; atomic publication (`COMMON.md`; `INTERFACES.md`) | Private reports outside payloads; NeedsRepair retained; activation missing | P |
| Production verifiers currently unimplemented (per task brief) | Covered by RT-08/RT-09 below | M |

### 3.6 Player, studio, audience, devices

| Requirement (source ref) | Implementation / evidence | Status |
|---|---|---|
| `loadRelease` verified handle or typed failure; `mount` play/pause/seek/restart/volume/read-along/dispose; pure `evaluate`; diagnostic entry studio-only (P2/P3/P5; `INTERFACES.md`) | LoaderEnvelope with production rejection of diagnostic fixture; `mountPlayer`; lifecycle/teardown proofs | P |
| Timing: title 1.5/0.5 s; curtains 1.5 s; fades 1 s + exactly 96000 dark samples; pause sign 1.2/0.8 s; all phases inside master (`COMMON.md`; `docs/spec/playback.md`) | Exact-sample checks (dark hold 96000; negative 95999 fails); P15 burst/phase proofs | P |
| Read-along: every word, active 2–3-line chunk only, atomic groups, `point-following-same-boundary-v1` (`COMMON.md`) | P4 mount-owned plaque, Noto Serif measured layout, refit, 24-pose agreement | P (diagnostic scope) |
| Stage/house domains; blackout keeps shell/sign readable; unrevealed filtered incl. seeks (`COMMON.md`) | P13 independent house pass; aperture-0/house-lit receipts | P |
| Profile: 16:9, DPR ≤1.5, images ≤4096, graphics ≤256 MiB, master ≤256 MiB; `paper-player-r186-v1` selected_unverified (P9; `COMMON.md`) | Caps enforced in code; headless geometry P18 (0.028 px, ≤8.9e−16); profile unverified for production | P |
| Graphics recovery, control races, lifecycle teardown (P6/P15/P16) | Recovery + lifecycle receipts headless; demo-device gates open | P |
| Title visible over closed curtains (P17/R12) | Player fix complete with pixels; server strict R12 rerun RED (Nonpositive viewport) — historical title-zero receipt superseded as failure evidence | B |
| Audience: two-ticket flow, accepted-only playback, static routes/MIME/cache, no provider need (M7/D5; `architecture/playback/*`) | Booth/routes/unavailable states; catalog empty; tickets gated on accepted packages; rehearsal copy missing | P |
| Demo-device: 60 s p95 ≤33.3 ms, fullscreen, 100/200% text, 10 cycles, narrative captures (P7/P8; `docs/implementation/player/checks.md` pending list) | Headless SwiftShader only (p95 50–150 ms under load, not a device result); 97.1 MB accounted graphics at 1280×720 DPR1 | M |
| Accessibility (read-along toggle/scales, reduced motion, plaque readability) | Toggles, 100/150/200% layout, reduced-motion parallax zeroing, sign/plaque readability receipts | P |

### 3.7 Delivery, submission, repo

| Requirement (source ref) | Implementation / evidence | Status |
|---|---|---|
| M5 autonomous excerpt; M6 both chapters; M7 delivery rehearsal on delivery computer without providers (README gates; D3/D5) | None accepted; no rehearsal/submission release doc | M |
| Public `jeffcato/paper-theater`, default `main` (AGENTS.md; README; I7) | Operator decision recorded; remote URL matches; offline visibility/default unverified | U |
| Submission materials: rules/reuse confirmation, recording/export practice, frozen release, walkthrough (D6) | Not started | M |

## 4. Remaining task list (executable increments)

Implementation gaps (need code), then verification / authorization / device / delivery work.
Each item: ID · requirements · owner · deliverable · dependencies · blockers ·
done-criteria (observable tests/evidence).

### A. Integration checkpoint (do first)

- **RT-01 — I20 coherent backend checkpoint** · I20; `backend-checkpoint-task.md` ·
  owner `integration` · deliverable: isolated exact-tree snapshot freezing R17/G23
  pins, resolved aggregate Pilot outcome (green or retained explicit blocker),
  full scoped Rust/generation/Python/contracts/export aggregate + conformance/
  restart/replay evidence, exact-head CI, published exclusion list ·
  deps: R17/G23 handoffs in hand · blockers: concurrent-design drift, moving UI
  files must be excluded · done: `backend-checkpoint-ready.json` published, CI
  green at the frozen head, aggregate result stated coherently.
- **RT-02 — Commit the dirty tree** · README Git rules · owner `integration` ·
  deliverable: reviewed explicit-path commits of the ~160 dirty files on
  `integration/contracts-v1`, pushed, CI recorded · deps: RT-01 scoping ·
  blockers: Claude UI redesign in flight — coordinate a stable frozen handoff,
  never sweep moving files/locks (I14) · done: clean `git status`, pushed head,
  CI record bound to that head.

### B. Runtime / server / studio

- **RT-03 — R18 durable Direction/Pilot flow** · R18; `server-direction-pilot-task.md`;
  G22/G23 runtime requests · owner `server` · deliverable: I19 native WAV wrapper
  adopted with parity tests; Source→Direction→Pilot through namespaced two-phase
  committed requests/observations/real ArtifactRefs, atomic checkpoints, saved
  replay, NeedsRepair preserved; pilot grants never authorize bulk speech ·
  deps: RT-01 pins · blockers: none (backend-only) · done: `presentation-runtime`
  -style ready-record + byte-identical replay proof, zero provider calls.
- **RT-04 — R4 live dispatch + accounting reconciliation** · R4/D1; `COMMON.md`
  broker rule · owner `server` · deliverable: every provider/helper op routed via
  broker with reservations; inherited ledger reconciled; money/request/repair
  enforced simultaneously; uncertain/late charges preserved ·
  deps: RT-03, OP-01/OP-02 · blockers: operator authorization · done: live-gated
  tests with injected transports + persisted authorization record; still zero
  *real* calls until OP-02 bulk approval.
- **RT-05 — R5 publication/recovery** · R5/I4; `INTERFACES.md` hash order · owner
  `server` · deliverable: consume frozen `PackageCandidate` + verifier, validate
  frozen bytes, recover interrupted export/activation, update catalog last ·
  deps: RT-08, RT-03 · blockers: acceptance verifiers · done: interrupted-activation
  recovery test + catalog-last ordering proof.
- **RT-06 — R12 studio strict check green** · R12/P17 · owner `server` (backend) +
  `player` (UI/resize only) · deliverable: Nonpositive-viewport failure diagnosed
  and fixed without weakening the assertion; title pixels >20 at 3 viewports;
  studio mounts shared player for candidate inspection ·
  deps: stable UI handoff (I14) · blockers: redesign coordination · done:
  `docs/implementation/runtime/studio-player/verification.json` regenerated green
  at current head, P17 title receipt retained.
- **RT-07 — R7 full journal walkthrough** · R7 · owner `server` · deliverable:
  source→decision→artifact views for live and saved events; candidate replay
  provably changes no accounting/catalog and dispatches nothing ·
  deps: RT-03 · blockers: none · done: browser walkthrough receipt with event
  counts and zero-POST replay proof.

### C. Generation pipeline

- **RT-08 — G24 bounded Speech compiler + saved takes** · G24/G3;
  `generation-speech-task.md` · owner `generation` · deliverable: verbatim
  source/speaker units, bounded request/take/transcription manifests, no prose
  hints in payloads, no fabricated refs/timestamps, private exact replay ·
  deps: RT-01 pins · blockers: bulk-synthesis approval stays closed · done:
  ready-record + saved/fake-audio tests green, `bulk_speech_approved` still false.
- **RT-09 — Remaining stage adapters G1–G10** · G1–G10; `architecture/generation/*` ·
  owner `generation` (+ `server` broker wiring, + `player` capture interface) ·
  deliverable, in order: research/canonical-image binding; voice-pilot auditions;
  speech/transcription approval; preflight blocking; Image/SAM visual +
  registration; presentation/master mapping; music bible + mix; choreography
  accounting; sampling-inventory review loop; shell generation via broker ·
  deps: RT-03/RT-08 · blockers: OP-02 (live media), profile freeze (RT-13) ·
  done: per-stage ready-records with real-broker attempts, NeedsRepair cleared
  per stage, zero invented evidence.
- **RT-10 — M5/G11 autonomous excerpt** · G11/M5 · owner `generation` (candidate) +
  `server` (durable run) + `player` (captures) · deliverable: accepted immutable
  Oz excerpt package (narrator/character handoff, 2 sets, later reveal, reused
  figure, score transition, final shell, saved journal) passing all executable
  verifiers · deps: RT-03…RT-09, RT-12 · blockers: missing verifiers for covered
  gates must be implemented first · done: catalog holds 1 accepted autonomous
  package; audience still rejects (excerpt ≠ chapter release).

### D. Player / audience (preserve redesign ownership)

- **RT-11 — I14/P1 stable UI handoff integration** · I14/P1 · owner `integration`
  (applies) + `player` (freezes) · deliverable: exact reviewed player/audience/UI
  paths + locked installs integrated; shared-type adoption verified; old red
  evidence preserved · deps: Claude Code frozen handoff · blockers: moving
  redesign snapshot · done: exact-head review + CI; no lockfile regeneration
  from unidentified state.
- **RT-12 — Independent acceptanceverifier set for player gates** · I4 (player
  rows: profile-r186-browser, graphics-recovery, control-lifecycle, title-visibility,
  public-export-boundary) · owner `player` proposes, `integration` freezes ·
  deliverable: executable checkers + fixtures for the 5 player-owned missing
  verifiers · deps: RT-11 · blockers: none · done: new verifiers execute in
  `verify_acceptance` with actual behavior-level verification results; test-first ordering is not required.
- **RT-13 — P9 production profile freeze** · P9 · owner `player` + `integration` ·
  deliverable: browser-proven geometry/defaults frozen as production profile
  revision replacing `selected_unverified` · deps: RT-11, RT-16 · blockers:
  demo-device data · done: profile revision pinned in contracts + registration
  rerun under it.
- **RT-14 — P3 audience accepted-release playback** · P3 · owner `player` ·
  deliverable: accepted-release loading, transport controls, teardown; diagnostic
  bypass unreachable from audience routes · deps: RT-10 (first accepted bytes) ·
  blockers: none · done: audience tests with accepted fixture pass; diagnostic
  fixture still rejected on production path.

### E. Acceptance / verification

- **RT-15 — Implement the 30 missing verifiers** · I4 unimplemented list; owner
  split: `generation` (provenance, chapter, research, image×5, acoustic×2,
  narrator, joins, bible, mix, story, readiness, actions, creative/render,
  opening/full-audio coverage, speech, primary),
  `player` (font, profile, device, graphics, lifecycle, title),
  `server` (publication-recovery, spend-authorization), `integration`
  (source-coverage already done; export-boundary, matrix freeze) ·
  deliverable: executable verifiers in `verify_acceptance`, no `-pending`
  escapes for implemented gates · deps: RT-09/RT-12/RT-05 · blockers: profile
  (RT-13), device data (RT-16) · done: 37/37 execute; chapter/excerpt matrices
  green on real packages only.
- **RT-16 — Rerun + reconcile all claimed-but-unrerun checks** · TODO verification
  record · owner `integration` · deliverable: exact-head rerun log for workspace
  Rust, `check:contracts`, `check:player`, chrome/inventory/lifecycle/recovery/
  title/chapter/staging/theater/plaque suites + identity scans; receipt-vs-head
  table marking stale claims · deps: RT-02 · blockers: none · done: single
  dated verification report bound to one head; stale receipts labeled historical.

### F. Chapters / delivery / submission

- **RT-17 — D3 both complete chapters** · M6/D3/D4 · owners `generation` →
  `server` → `integration` → `player` · deliverable: two accepted immutable
  chapter packages (Oz Ch.1; Wells Bk One Ch.I) with full prose, evidence,
  music, read-along; recorded journals; sprite/voice reuse exercised ·
  deps: RT-15, RT-04/OP-02, RT-13 · blockers: bulk-synthesis authorization,
  demo-device acceptance · done: catalog holds 2 accepted autonomous chapter
  releases; both tickets live; small-Wells-unit pilot run first per TODO.
- **RT-18 — D5 delivery rehearsal** · M7/D5 · owner `player` + `server` ·
  deliverable: static routes/MIME/cache proof, hash checks, bounded load/retry,
  complete playback, resize/fullscreen, reduced motion, graphics recovery,
  repeat visits for both packages; local rehearsal copy + saved journals;
  provider-free walkthrough proof · deps: RT-17, RT-19 · blockers: demo-device
  access · done: rehearsal receipt with request counts (0 remote/provider).
- **RT-19 — Demo-device measurement (P7/P8)** · P7/P8/D2 · owner `player` ·
  deliverable: Chrome/OS/hardware record; 60 s warmed playback at 1280×720
  DPR ≤1.5 with p95 ≤33.3 ms; graphics ≤256 MiB; master ≤256 MiB; images
  ≤4096; fullscreen + 100/200% text + 10 entry/exit cycles + narrative captures ·
  deps: RT-11 · blockers: OP-03 device access · done: `checks.md` pending rows
  filled with device numbers (headless figures explicitly not substituted).
- **RT-20 — D6 submission preparation** · D6 · owner operator + `integration` ·
  deliverable: event rules/reuse confirmation, recording/export practice, frozen
  demo release, source→decisions→artifacts→playback walkthrough, allowlisted
  public exports · deps: RT-18 · blockers: event timing/rules · done: frozen
  release hash + walkthrough recording archived.

## 5. Work-class separation

- Implementation gaps: RT-03, RT-04, RT-05, RT-07, RT-08, RT-09, RT-10,
  RT-12, RT-13, RT-14, RT-17.
- Verification (no new behavior): RT-01, RT-02, RT-06 (rerun half), RT-15, RT-16.
- Authorization-gated: RT-04 (live), RT-09 media half, RT-10, RT-17 (need
  OP-01/OP-02); nothing here authorizes spend by itself.
- Device-measurement: RT-19 (needs OP-03); RT-13 blocked on it.
- Delivery: RT-18, RT-20.

## 6. Dependency graph / critical path / parallel lanes

```text
RT-01 checkpoint ─┬─► RT-03 R18 ───┬─► RT-04 live/acct ──► RT-05 publication ─► RT-17 chapters ─► RT-18 rehearsal ─► RT-20 submission
                  │                └─► RT-08 G24 ──► RT-09 stages ──► RT-10 excerpt ─────────────────┘
                  ├─► RT-02 commit (unblocks all integration CI)
                  └─► RT-11 UI handoff ─► RT-12 player verifiers ─► RT-13 profile ─►┐
RT-06 R12 fix ──► studio green ────────────────────────────────────────────────────┤ (feeds RT-13/RT-18)
RT-16 rerun ──► trustworthy baseline (any time after RT-02)                        │
RT-19 device ──► profile/device evidence ──────────────────────────────────────────┘
RT-15 verifiers grow alongside RT-09/RT-12/RT-05; must finish before RT-10/RT-17.
```

Critical path: RT-01 → RT-03/RT-08 → RT-09 → RT-15 → RT-10 → RT-17 → RT-18 → RT-20.
Safe parallel assignments right now: `server`→RT-03 prep (fixture-only, no
snapshot mutation before I20 handoff); `generation`→RT-08 planning/new-file work
(same constraint); `player`→RT-11 freeze prep + RT-19 planning (no backend edits);
`integration`→RT-01. R12 diagnosis (RT-06) can proceed in parallel if it touches
only reproduction files, coordinated with the redesign owner. No lane edits
another lane's paths; shared-schema/lockfile proposals go to `integration`.

## 7. Required operator inputs

- **OP-01 — Live spending/request/repair authorization**: production spend ceiling
  + scoped request/repair caps for Agent 2 to persist; inherited experiment
  allowances do not transfer. Blocks RT-04/RT-10/RT-17 live legs.
- **OP-02 — Credentials + bulk-synthesis approval**: provider credentials via the
  approved secret channel (never serialized); Flash TTS voice pins; explicit
  bulk-speech go-ahead (G24 alone must not authorize it). Blocks RT-09/RT-17 media.
- **OP-03 — Demo-device access + target confirmation**: actual delivery computer
  (provisional: current Mac) with Chrome version/OS/hardware recorded; exclusive
  measurement window (headless numbers cannot substitute). Blocks RT-13/RT-19.
- **OP-04 — Repository confirmation**: verify `jeffcato/paper-theater` is public
  and `main` is default (I7, recorded 2026-10-26). Unblocks submission (RT-20).
- **OP-05 — Event/submission terms**: current hackathon rules, reuse requirements,
  submission timing/format. Blocks RT-20 scope.

## 8. Completion claims lacking sufficient evidence

1. Any “pass” older than the dirty tree: TODO verification rows bound to heads
   (`306059f`, `41061a6`, `5419a3f`, …) predate ~160 uncommitted files — covered
   by RT-02/RT-16, not to be quoted as current-head results.
2. Scoped-check totals (R13 120, R14 134, R15 184, R16 217, R17 165, G19–G23
   suites) are focused runs; aggregate Pilot failures are retained and no
   coherent full aggregate is claimed — RT-01 must not accept focused green.
3. `check:player` 134/134, inventory 144/144, lifecycle/recovery/title/chapter/
   staging/theater/plaque receipts are headless SwiftShader proofs, not
   target-device performance, memory, or fullscreen acceptance (RT-19).
4. Capture inventory state probes report `rendered: false`; inventory im...
[truncated 911 chars]