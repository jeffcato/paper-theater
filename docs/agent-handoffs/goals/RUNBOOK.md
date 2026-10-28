# Rules for a continuous implementation goal

Read root `AGENTS.md`, [COMMON](../COMMON.md), [INTERFACES](../INTERFACES.md), your role prompt and [the delivery plan](../../plans/2026-10-28-hackathon-delivery.md). Then use your assigned goal file as the ordered work queue. Older bootstrap tasks are historical; adopt already implemented behavior instead of redoing it.

## Work loop

1. Inspect current owned changes and the relevant handoff. Record any starting edits you must preserve. Find the first unchecked implementation item whose dependencies permit useful work.
2. If it is already implemented, verify the relevant current behavior and record exact evidence, then check it off. Do not regenerate unchanged media, archives or whole test matrices just to repeat a receipt.
3. For missing behavior, use contract-first incremental TDD: one failing public-behavior test, the smallest implementation, green, then the next behavior. Keep applicable negative cases; a smaller model does not get weaker checks. Routine choices inside the assigned task need no additional operator turn.
4. Run the affected checks and identity scan; record actual results and remaining limits in your own goal file. A checkbox means code and its required checks are complete, not merely designed, proposed or handed off.
5. Publish the smallest usable interface/artifact handoff as soon as it exists, before the entire lane finishes. Re-read dependency requests in the other lists at each completed item/integration checkpoint; reply with an owned contract/implementation handoff, not edits to their checklists.
6. Immediately continue to the next unblocked item. Do not end after one completion report or ask what task comes next. If a dependency blocks one item, record it and advance independent work; use saved/test-double inputs to verify mechanics without claiming genuine production acceptance.

## Blockers and goal completion

After two failed approaches or 45 minutes on a boundary, record the minimal reproducer, requested owner/action and what remains usable. Continue another independent item. Do not busy-poll files, repeatedly run the same failing aggregate or silently widen scope. If no useful work remains, give an exact blocker report and follow the goal runtime's waiting/blocked rules. Do not invent approval, pause a goal without a user request, or mark a goal complete to escape a dependency.

The goal objective is the **Required implementation** checklist, including its tests and required code-adoption seams. All of those items must be checked with evidence before reporting implementation complete. Dependencies may therefore leave the goal blocked until another lane finishes. A budget/time limit is not completion.

Each list separately names **Operational validation gates**. Actual paid runs, accepted chapter media, device access and external submission are not fabricated prerequisites to writing the code. If unavailable, report them as pending and finish all independent implementation. Completion of the implementation goal means precisely that; it does not mean the project, chapter or production validation is complete. If the operator explicitly adds a live/device gate to the goal objective, that gate also becomes required for goal completion.

## Ownership and Git

- Edit only your permanent implementation paths and your own goal file. Studio exclusively owns `apps/studio/**` while its temporary assignment is active; server excludes it. Preserve the separate player/audience owner. No second writer for the same UI paths.
- Integration owns shared Rust/TS contracts, production manifest/module root, shared fixtures, root configuration/lockfiles and shared Git. Send exact interface/dependency proposals; workers must not fork public wire records or change root dependencies themselves.
- In the shared checkout only integration stages/commits/switches/pushes, using explicit reviewed paths. A worker can finish owned implementation with unstaged code and evidence; integration's commits satisfy the shared commit rule. Never sweep the index, reset another lane or kill another worker's server/browser.
- Use temporary test data and available/dynamic ports. Own all generated output paths. In particular, Studio scripts currently write runtime-owned PNG/receipt paths: move new runs to Studio-owned evidence or coordinate an explicit output handoff, preserving historical receipts.

## Authority and evidence

The [recorded authorization](../../implementation/integration/operator-authorization.md) is a $3.50 cap and scoped Pilot with bulk separately gated. Preserve existing consumption/reservations/uncertain attempts; an API key, test grant, new model, or this goal is not additional authorization. Implement guarded production construction and transports, but perform a real call only through runtime after its operation/model/request/repair scope and limits are actually established. Credential access remains absent from offline tests/replay. Helpers cannot create their own requests/retries/ledgers. Never disclose secrets.

Generation proposes; runtime commits and publishes; player verifies/renders. Retain complete prose, stable identities, measured timing, independent fact support, source-scoped references, concrete narrative coverage and all adverse findings. A matrix flag or model's Pass string is not independent verification. Keep diagnostic/synthetic provenance and missing/uncertain checks explicit; no audience bypass. Checkpoint mappings must not rewrite original issued/raw/usage identities.

## Efficient verification and reporting

Use focused checks while editing; broad aggregates belong to a frozen integration candidate and major relevant changes. Coordinate heavy Rust/browser runs to avoid shared-machine timeout artifacts. Keep production deadlines unchanged and record test concurrency. Integration verifies exact source/build identities and CI; do not inherit green from an earlier commit or receipt.

Use your goal file's evidence table for normal updates. Link existing handoffs or concise receipts; do not produce another full archive/README series per checkbox. The final report contains completed IDs, owned paths, exact commands/results, consumed contract/profile/source identities and pending external gates. Do not claim two chapters if the fallback shipped one. External deployment/submission is performed by the operator or under separate explicit authorization.
