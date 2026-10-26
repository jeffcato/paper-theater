# Studio and server v1

The studio controls start/cancel/inspect/preview/recorded replay. Creative selection and review are autonomous. It never asks a person to pick candidates or approve artifacts. The audience is a static two-ticket application with an honest unavailable state until accepted chapters exist.

Loopback, same-origin endpoints: `GET /api/v1/books`, `POST/GET /api/v1/runs`, `GET /api/v1/runs/{id}`, `POST /api/v1/runs/{id}/cancel`, run `/events` and `/journal`, and allowlisted `GET /api/v1/artifacts/{id}`. An idempotency key reused with different request bytes conflicts. Mutation Host/Origin validation, bounded bodies and allowlisted artifact IDs are runtime responsibilities.

Run states are queued, running, waiting_provider, recovering, succeeded, failed and cancelled. SQLite is the sole metadata/event/budget/library authority. Artifacts are content addressed; durable bytes precede metadata references. State and its event commit in one transaction before broadcast. Events contain schema/run/sequence/epoch/revision/time/type and causal references; large payloads remain behind artifact refs. A snapshot includes last_event_seq. Streaming after that sequence and deduplication lose no committed event. Invalid cursors require a fresh snapshot. Replay reads recorded events/artifacts, with zero dispatch or accounting/publication mutations.

Cancellation advances the epoch and stops dispatch. Late work cannot commit success, library bindings, acceptance or publication; actual late charges remain recorded. Recovery preserves reservations and ambiguous attempts. Runtime owns final filesystem activation/reconciliation and catalog updates; generation cannot activate a candidate. A valid diagnostic package can be inspected without becoming audience eligible.
