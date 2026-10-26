# Runtime integration requests

Pending R2-001: add `pub mod runtime;` and production dependencies `rusqlite = { version = "0.32", features = ["bundled"] }`, `serde` derive, `serde_json`, `sha2 = "0.10"`, `tokio` (rt-multi-thread, macros, time, sync, process, io-util), `tempfile`; dev dependency `tempfile`. Runtime uses only these libraries. Server owns its Axum dependencies.

Pending R2-002: publish StageExecutor/RunServices/StageInput/StageOutput and PackageCandidate definitions. Need explicit operation identity, quota scope (provider/project/model), estimated money/requests/tokens, repair identity, retry class, recovery job ID and structured retry timestamp. StageOutput needs explicit outcome, proposed byte artifacts, decisions with source refs, library expected revision, reviewed package inventory and acceptance/evidence refs. Pending definitions are private runtime records, never copied into another lane.

Pending R2-003: studio imports `@paper-theater/player` diagnostic API and contracts. Need diagnostic loader/mount signature, fixture URL/inventory and exact package exports. Studio will never supply creative acceptance.

Pending R2-004: add studio to root npm workspace/lock and aliases `check:studio` and runtime/server checks. Runtime isolated check is `cargo test --manifest-path fixtures/runtime/checks/Cargo.toml`; remove isolated workspace once root contracts/dependencies are available.

No live dispatch is authorized. Broker remains disabled pending neutral inherited accounting reconciliation and separately established product authorization. Fixture execution needs neither credentials nor accounting imports.

Update: consumed callable `pt-services-v1.0.0` / `pt-contracts-v1.0.0`. Runtime now implements shared `RunServices` and accepts shared `StageExecutor`. First contract test observed RED (private Executor mismatch) before replacing the preliminary boundary. R2-001 remains needed for root builds; isolated harness compiles actual owned runtime source against shared contracts.

Pending R2-005: PackageCandidate needs explicit book/source binding and acceptance document inventory schema. Runtime's preliminary activation adapter currently fails shared package outputs closed. Need a shared exact acceptance verifier or schema to avoid inventing semantic acceptance in runtime. ProviderOperation lacks project fingerprint, repair/retry class and retry/job recovery data; runtime can bind project server-side, but ProviderResult needs structured retry time/job identity to preserve recovery evidence.

Agent 1: please stage/commit only reviewed lane paths after checks; updated AGENTS.md requires clean commits, while shared-index ownership stays with you. Do not stage isolated `fixtures/runtime/checks/target` outputs.

R2-001 production dependencies are now present; root `--locked --offline` runtime tests run. Server manifest now adds Axum 0.8, async-stream 0.3, Tokio net/signal, serde/json; dev tower 0.5 util + tempfile. Agent 1 please refresh root Cargo.lock for these owned manifest additions. Studio will add scripts/typecheck/browser tests in its own manifest; root aliases already observed.

R2-003 concrete player gap: `packages/player/src/index.ts` currently exports loadDiagnostic/createTransport but no `mount(container,handle)` or diagnostic mount. Studio will call shared loadDiagnostic and mount only; please expose the diagnostic mount and compatible fixture loader so studio contains no rendering/timeline code. Current player LoadPolicy uses hashes while shared Acceptance uses version IDs. Bind to frozen shared records before browser acceptance.
