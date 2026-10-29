# Shared instructions for every Paper Theater agent

Read this file, [INTERFACES.md](INTERFACES.md), and your assigned prompt. Implement your assigned work; do not stop at a plan. Work only inside your owned product paths. The experiment workspace is a read-only source of reusable code and evidence.

## Product identity and extraction rule

The product is **Paper Theater**. Use `paper-theater`, `pt-*`, and `@paper-theater/*` for new project identities. Existing generic reusable crate names may remain.

The former experiment's project name, its case variants, and every name starting with that retired prefix must be absent from the new product. The supplied checker contains the exact prefix encoded as character codes. This rule includes application UI, source, comments, tests, documentation, manifests, package/lock metadata, environment examples, module/cache identifiers, generated prompts, logs, source maps, paths, URLs, database metadata, build artifacts and release exports. Do not reproduce the retired identity even in a prohibition, migration note, attribution note or test string. Generate a negative test's prohibited bytes programmatically when needed.

Use `SOURCE_ROOT` only as an external, transient reading location. Never serialize its value, depend on it at build/runtime, create a symlink to it, or embed the source machine's absolute paths. Imported code must compile from copied product-local files. Adapt include paths, subprocess paths, cache locations and fixture lookups. Product data defaults under a configurable product-local `var/`; retain no experiment-specific environment/default paths.

Record reuse provenance using neutral component IDs, source-relative file paths, source content hashes, applicable license notices, transformation descriptions and destination hashes. Preserve truthful authorship and imported/synthetic/live status. Do not record the old repository name or URL. Source provenance can say `experimental-source-snapshot` with a digest. Preserve required attribution; report any unavoidable conflict rather than silently falsifying a notice.

Immutable inputs require special care. If an original response or ledger contains forbidden identity text, keep that original in the read-only evidence workspace. A cleaned derivative has a new hash, explicit transformation provenance and an original-content digest; it is never called the original raw response. If acceptance needs unavailable original bytes, keep that gate unresolved. Do not purchase replacement evidence under this instruction alone. Never clear historical spending, reservations, failed attempts or exhausted retries during migration; Agent 2 owns accounting reconciliation through a neutral hash-bound import.

Copy selected code and tests, not old repository scaffolding, archives, dependency caches, built bundles or historical reports wholesale. Check copied/generated content with `tools/check_product_identity.py` before handing it off.

## Authority and current decisions

October 28 workflow update: implement complete useful slices first and run focused verification before handoff. Mandatory TDD/test-first ordering and RED/GREEN receipts are no longer required, including in older briefs or optional skills. Existing tests, shared interface ownership, negative coverage for critical behavior, acceptance, provenance and spending rules remain in force; see root AGENTS.md for the authoritative policy.

The latest user requirements and these ownership instructions supersede stale mechanisms in source documents. Consult only the implementation plan plus the owning specification needed for your task, using `SOURCE_ROOT` transiently. Agent 1 publishes clean product specifications under `docs/spec/` as they are needed.

Resolve known source-document drift as follows:

- Sources are Gutenberg #55, Oz Chapter 1; and Gutenberg #36, Wells, Book One Chapter I, “The Eve of the War.” The Thunder Child material is experimental reuse/negative-case evidence, not either required chapter.
- New final dramatic recordings audition the selected Flash TTS model first. Accepted existing Lite recordings retain their real model/voice identity. No automatic regeneration or silent substitution.
- Introductory prose may establish context with curtains closed. Required first-picture contributors must already be eligible before curtain opening begins. Scene blackouts remain narration-free.
- Passed capability probes are reusable evidence, not new kickoff experiments or proof of production acceptance. No additional audience-appeal probe or dedicated singing/humming detector is required.
- The existing reduced-scope preview and its relaxed visual trial do not meet production acceptance. Failed narrator continuity, missing story coverage, diagnostic mattes and omitted required actions remain failures.
- Target Three.js 0.186.0 and matching declarations are selected but unverified. Agent 4 verifies availability/compatibility and preserves original 0.170.0 receipts as historical evidence only. Report a concrete unavailable dependency; do not silently select another version.

## Fixed architecture

Three product Rust crates: `pt-contracts`, `pt-production`, `pt-server`. Reuse generic core crates selectively. Two TypeScript packages: `contracts`, `player`; two React/Vite apps: `studio`, `audience`. Bounded Python acoustic/media helpers are allowed under `tools/generation/`. No runtime or build dependency on the experiment tree.

One active production run, a fixed ordered stage list, SQLite as sole run/event/budget/library authority, content-addressed files, per-unit checkpoints, and bounded repairs. Keep ordinary functions and small interfaces; no generic DAG engine, distributed workers, accounts, public generation service, code-generation framework or second player.

Generation is autonomous. The studio provides start/cancel/inspect/preview/recorded replay; it never asks a person to choose creative candidates or approve artifacts. Muse directs; deterministic validators and independent automated reviewers accept or return concrete failures. Missing/uncertain required evidence fails. Recorded replay makes zero provider calls and changes no accounting or publication.

Start in fixture/replay mode with zero provider dispatch. Do not load credentials to run offline checks. All provider operations, including research, transcription, verification, image/SAM, music, reviews and token-count calls, go through Agent 2's single authorization/reservation broker. Money, request quotas and repair allowances are separate limits; another key for the same project does not create quota. Existing scoped experiment allowances do not authorize new product production.

## Product requirements to retain

- Original complete prose in order; no invented dialogue, vocal tags or dropped words. Characters receive voices only when the source supplies spoken performances. Persistent speaker identity is separate from scene-specific acting.
- One immutable canonical character sprite per book production; book-local identity lookup and reuse across chapters. One canonical external image per applicable researched subject type, plus a separate common art profile. All image assets — theater dressing and scene assets alike — are illustrations that read as die-cut paper: hand-drawn line and flat colour on a paper ground, clean cut contour edges, no photographic texture, no 3D or glossy CG rendering, no baked lettering. Generic scenery may use style alone; unrelated references must not be attached just to fill a field. Scene-asset specifics live in the [scene art style](../implementation/generation/scene-art-style.md). One internal exception: the immersive house set may attach its own reviewed/accepted `house.facade` (a product-generated member of the same set, not an external researched reference) as the style reference for the remaining house surfaces, as specified in the [immersive house art brief](../implementation/player/immersive-art-brief.md).
- Concrete, recognizable subjects, locations and actions. Source coverage drives scenes/pictures; two-scene fixtures impose no creative quota. No symbolic substitutes, unexplained empty entrances, duplicated instances or silent required cue omission.
- Source/evidence-scoped visual prompts and references. Disable unreviewed image-provider search/shell tools. A whole-chapter bible never leaks into an earlier asset request.
- Every narrative reveal has complete per-fact source coverage and independent acoustic evidence. Unknown/stale/ambiguous bounds are ineligible. The inherited independent-anchor tolerance is 0.75 seconds; use the later measured end. It is not a word-highlight tolerance.
- Preserve measured point annotations and `point-following-same-boundary-v1`. A point may share the following positive interval only at the same measured start; never invent word durations. Read-along covers every source word and shows only the active two-to-three-line chunk, with groups kept atomic.
- One delivered 48 kHz stereo PCM16 WAV master and one sampled media playhead for story, captions and highlighting. Keep explicit source-to-presentation sample mappings through resampling, trims and gaps. The user-pause sign alone animates on UI time.
- Title hold/dismiss minimums 1.5/0.5 seconds; opening/closing curtains 1.5 seconds; scene fade out/in 1 second each with exactly 96,000 dark-hold samples at 48 kHz; pause sign down/up 1.2/0.8 seconds. All authored phases fit inside the master.
- Separate narrative-stage and house-lighting domains. Blackout includes emissive pixels and shadows while shell/text/signage stay readable. Filter unrevealed contributors before every pass, including in darkness and after backward seeks.
- Initial resource profile: 16:9, DPR ≤1.5, images ≤4096 pixels per dimension, resident graphics ≤256 MiB including targets/shadows, delivery master ≤256 MiB. Actual target-device measurement remains required.
- Multi-take voice continuity, safe joined speech, a real director-owned musical bible and phrase-aware score reuse are required before bulk work. Existing audio defects are not repaired merely by global loudness normalization or crossfading.
- Production acceptance binds actual bytes, recipes, profiles, player/checker versions and full required coverage. Diagnostic success cannot authorize production publication.

## Coordination

Agent 1 alone edits public contracts, shared fixtures, root configuration and lockfiles. Submit an exact field/API/dependency proposal with the failing case instead of editing those files. Agent 2 owns durable side effects and catalog activation. Agent 3 owns authored presentation/cue generation. Agent 4 owns playback interpretation and browser evidence. These boundaries are detailed in `INTERFACES.md`.

After two failed approaches or 45 minutes on one boundary, preserve a reproducer, report the exact issue and continue independent work. Do not expand budgets or weaken gates. Preserve other workers' edits. Do not spawn additional agents without the operator assigning disjoint ownership.
