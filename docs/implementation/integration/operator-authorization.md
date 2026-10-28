# Operator authorization inputs (recorded 2026-10-27)

The operator established the following standing inputs for the production
spending/request/repair authorization. They are inputs only: the server lane
persists them as enforced broker limits and reconciles the ledger (R4/R18).
Nothing here dispatches a provider call or approves bulk synthesis by itself.

## 1. Spend ceiling: $3.50

- Operator holds $3.98 in Gemini credit for the hackathon window.
- Authorized production money cap: **$3.50 (3,500,000 microUSD)**.
- The $0.48 remainder is an untouched buffer so retries/repairs cannot strand
  a chapter at zero mid-run. No other model, key, or allowance extends this cap.
- Credentials live in the product-root `.env` (gitignored, never committed):
  `GEMINI_API_KEY` is the operative key for TTS. `META_API_KEY` has no
  consumer in this tree yet; it authorizes nothing.

## 2. Scope: metered pilot unit first, full bulk later

- Approved now: **one small metered pilot unit** (recommended: a small Wells
  unit per TODO D3) to measure real burn rate before any chapter-scale spend.
- Full bulk synthesis for either chapter is **not approved**. It requires a
  separate explicit operator decision after measured cost and cleared pilot
  listening/continuity evidence are in hand.
- `bulk_speech_approved` remains `false` in every registration until then.
  A passing pilot never implies it.

## 3. Voice A/B: Flash Lite TTS vs Flash TTS

- Operator decision (2026-10-27): before pinning the final TTS model, run an
  **A/B audition of Gemini 3.8 Flash Lite TTS against Gemini 3.8 Flash TTS**
  inside pilot scope. The known Flash model id in-tree is
  `gemini-3.8-flash-tts`; the exact Lite model id is pinned at audition time
  by the director/server lane and recorded here once established.
- Decision rule: the **director judges whether the quality bump is worth the
  measured cost delta**. Voice direction is a core hackathon deliverable, not
  a cost-minimization default — Lite wins only if the director says the
  difference doesn't matter on the ear.
- Both arms meter spend against the $3.50 cap through the broker. Existing
  accepted Lite recordings keep their real identities; no silent
  re-synthesis under either outcome.

## Preconditions (unchanged, owned by lanes)

- Live dispatch is still physically disabled (test-only broker construction);
  the server lane wires the live transport and persists these limits first.
- Pilot listening/continuity evidence gates are still red; full bulk approval
  is blocked on evidence, not just money.
- Studio never approves artifacts or spend; these inputs take effect only
  through persisted runtime authorization, never through UI action.
