# Continuous implementation goals

**Implementation-first policy (October 28 operator decision):** complete useful runnable slices, then perform focused verification before handoff. Test-first ordering and RED/GREEN receipts are not required; preserve existing tests, acceptance gates and ownership.

Prepared October 28, 2026. Give each existing agent its own list below and start a goal whose objective is to complete that list's **Required implementation** section. These lists implement the [seven-day delivery plan](../../plans/2026-10-28-hackathon-delivery.md). They do not create spending, bulk-generation, deployment or submission permission.

| Agent | Task list | Suggested starting setting |
|---|---|---|
| integration | [integration.md](integration.md) | Sol high; xhigh for a specific difficult authority/review problem |
| server | [server.md](server.md) | Sol high; medium for bounded adapters with frozen contracts |
| generation | [generation.md](generation.md) | Sol medium; high for sample mapping, source matching and evidence rules |
| player/audience owner | [player.md](player.md) | Sol medium; high for timing, rendering and recovery races |
| Studio redesign agent | [studio.md](studio.md) | Terra medium or Sol medium |

Settings are project recommendations, not benchmarks or changes to running agents. Use an available model in the operator's client. No agent should create replacement/subagents without an explicit disjoint operator assignment.

Assign the player list to the **single current owner** of `packages/player/**` and `apps/audience/**`. If another redesign agent currently owns those paths, give that agent the list or wait for an explicit handoff; do not launch a second overlapping writer. Studio already has its separate exclusive assignment.

## Paste into each agent's goal

Use the appropriate file name in this objective:

> Complete every item in the Required implementation section of docs/agent-handoffs/goals/AGENT.md, following docs/agent-handoffs/goals/RUNBOOK.md and AGENTS.md. Implement and verify each item, update only your assigned checklist, and immediately continue with the next unblocked item without asking me for another task. Preserve other agents' work. Do not mark blocked work complete or weaken acceptance. Perform live operations only within explicit recorded authorization; otherwise finish independent implementation and report the exact remaining external gate. Finish the goal only when all required implementation items are evidenced as complete.

Replace `AGENT.md` with `integration.md`, `server.md`, `generation.md`, `player.md` or `studio.md`. This is goal text, not a claim that a particular slash command exists. This preparation does not start any goals or model calls.

Exact short launch objectives, one per existing agent:

**Integration**

```text
Start a goal to complete all Required implementation items in docs/agent-handoffs/goals/integration.md. Follow docs/agent-handoffs/goals/RUNBOOK.md and AGENTS.md. Keep implementing the next unblocked item until the list is evidenced complete; update your checklist and report exact external gates without bypassing them.
```

**Server**

```text
Start a goal to complete all Required implementation items in docs/agent-handoffs/goals/server.md. Follow docs/agent-handoffs/goals/RUNBOOK.md and AGENTS.md. Keep implementing the next unblocked item until the list is evidenced complete; update your checklist, exclude apps/studio, and preserve spending/acceptance boundaries.
```

**Generation**

```text
Start a goal to complete all Required implementation items in docs/agent-handoffs/goals/generation.md. Follow docs/agent-handoffs/goals/RUNBOOK.md and AGENTS.md. Keep implementing the next unblocked item until the list is evidenced complete; update your checklist and produce exact runtime handoffs without inventing media, timing or approval.
```

**Current player/audience owner**

```text
Start a goal to complete all Required implementation items in docs/agent-handoffs/goals/player.md. Follow docs/agent-handoffs/goals/RUNBOOK.md and AGENTS.md. Keep implementing the next unblocked item until the list is evidenced complete; preserve the active redesign and accepted-only loading, and report genuine media/device gates separately.
```

**Studio redesign agent**

```text
Start a goal to complete all Required implementation items in docs/agent-handoffs/goals/studio.md. Follow docs/agent-handoffs/goals/RUNBOOK.md and AGENTS.md. Keep implementing the next unblocked item until the list is evidenced complete; own apps/studio only, use the shared player and real backend data, and preserve other agents' evidence.
```

## Coordination without another operator turn

Read the [runbook](RUNBOOK.md) once, your list and the relevant current contract. Workers may check off/update **their own** goal file and its evidence/dependency tables. Integration alone updates this index, RUNBOOK, root TODO and shared interfaces/Git. Read other lists to consume their latest handoffs; never edit another worker's status.

| First boundary | Producer | Consumers can implement meanwhile |
|---|---|---|
| Minimal request/receipt/acceptance/publication interfaces | INT-02, with exact owner proposals | Existing-trait doubles, native request/response parsers and negative cases |
| Real transport plus durable stage input/result registration | SRV-02–SRV-05 | GEN-02–GEN-10 compilers and saved-response tests |
| Primary audio and complete creative candidate | GEN-01–GEN-11; SRV-04 | PLY-02–PLY-05 render/load/capture mechanics and acceptance fixtures |
| Stable shared-player mount/load/resize API | PLY-01–PLY-03 | STU-01–STU-05 journal/state/inspector work |
| Executed acceptance and atomic publication | INT-04–INT-07; SRV-06 | Accepted-only audience/catalog negative tests and Studio replay |
| Actual approved media, budget and demo device | Operator/runtime | Every independent implementation item remains actionable |

The first milestone is an integrated excerpt; Oz is the first complete chapter. Keep code reusable for Wells. October 31 is the chapter-count feasibility gate; the operator-approved fallback is one complete accepted chapter plus the walkthrough. Feature/media freeze is November 2. Protect November 3–4 for delivery.

All new required items start unchecked. Adopt existing working code and credible current evidence before writing replacements. A completion receipt for a different tree is historical evidence, not a pass for current bytes. Live chapter acceptance remains separately visible in TODO even when an implementation goal finishes.
