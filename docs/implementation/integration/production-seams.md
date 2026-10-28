# Production seams: first dependency handoff

2026-10-28. INT-02 is in progress; this is the bounded HTTPS dependency response to [runtime's request](../runtime/production-dependencies.md). Provider-v2 remains the issuance/execution/recovery service. No new dispatch or spending authority is introduced.

## Runtime HTTPS dependency

`pt-production` now has exact `reqwest = 0.13.5`, default features disabled, `rustls` and `stream` enabled. The root Cargo lock is resolved offline. Runtime owns the transport implementation and its tests; generation owns vendor codecs. Integration supplies no alternate networking path.

The dependency contract test initially failed with unresolved `reqwest` (three compiler errors), then passed after the manifest/lock update:

```sh
cargo test -p pt-production --test https_dependency --locked --offline
```

The test builds a TLS-only client with proxy, redirects and retries disabled, bounded connect/read/total timeouts, and proves a cleartext request fails as a builder error before any connection. This proves the dependency/construction seam only; it does not certify runtime transport or network behavior.

Runtime construction must retain `.https_only(true)`, `.no_proxy()`, `.redirect(reqwest::redirect::Policy::none())`, `.retry(reqwest::retry::never())`, and nonzero bounded connect/read/total timeouts. Reqwest otherwise has default retry behavior: broker issuance must remain the only request/retry authority. Consume bounded chunks with `Response::chunk()`, checking cumulative bytes and cancellation; `Content-Length` alone is insufficient. Do not use unbounded `bytes()`/`text()` for an untrusted body. Validate the fixed provider origin and issued path; no arbitrary model-supplied retrieval. Preserve raw bytes/partial outcomes and uncertain accounting when a sent request is canceled, times out or disconnects. Fixture/Replay must not construct a dispatch path or read credentials.

Authoritative dependency references: [ClientBuilder](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html), [retry policy](https://docs.rs/reqwest/0.13.5/reqwest/retry/index.html), [Response chunks](https://docs.rs/reqwest/0.13.5/reqwest/struct.Response.html#method.chunk). Confirmed against the installed 0.13.5 source. No HTTP/provider requests executed by the test.

## Administrative configuration API

Rust exports `ProductionConfigurationV1`, `ProductionOperationScopeV1`, `ProductionPurposeV1`, `ProductionConfigurationV1::validate()` and `validate_proposal(scope_id, &StageInput, &ProviderProposalV2)`. TypeScript exports the matching records, `validateProductionConfigurationV1` and `validateConfiguredProviderProposalV1`. This is a schema-1 additive seam; old provider-v2 implementors and receipt versions retain their meanings.

The wire shape matches runtime's proposed `ProductionConfiguration`/`ProductionOperationScope`: selected book/chapter, recipe/profile pins, original accounting digests, authorization-proof digest, global limits and exact operation/provider/project/credential-context/model/request/recipe/secret-environment-name scope. All fields are strict. Empty/duplicate history, duplicate operations, unsafe integers, widened limits, relabeled Pilot scope and serialized secret/grant fields fail. Both configured Oz and Wells use the same boundary.

`validate_proposal` additionally checks Live mode, selected stage/unit, frozen pins, exact request payload digest, operation/model/provider/recipe, timeout and aggregate estimated tokens/request/money against the configured scope. It does not issue work, read credentials, inspect balances or authenticate proof digests. Runtime must still verify original accounting inputs and persisted authorization, source selection, committed request bytes, active epoch, reservations, quotas and unresolved attempts before issuance. `allow_recovery` is administrative intent; existing-job queries still require separately issued durable recovery authority. No fallback to legacy execution is introduced.

**Exact server adoption:** replace or alias the duplicate private wire structs with these shared records; call shared validation at configuration import and proposal selection, then retain runtime's `authority(db)` reconciliation and all issuance/execution/recovery guards. Map `ContractError` into the existing administrative error response. Preserve immutable imports, uncertain reservations and exhausted repairs. Do not serialize or return a capability from these records. Return a frozen source/test handoff before integration adopts the runtime implementation.

Four Rust public-boundary tests and 44 Rust-produced TypeScript cases pass, including exact proposal selection and rejection of altered request/recipe/model/provider/stage/unit/profile, Fixture/Replay dispatch and widened estimates. Full contracts, strict declarations and native/packed exports pass. Red evidence includes missing Rust API, accepted empty history/widened scope and missing TypeScript export; all remain archived in the integration check receipts.

## Remaining common boundary

Trusted review policy, full executed verifier authority, current publication envelope and frozen player registration remain explicit pending seams. I21's private verification capability does not authorize publication. The [exact-report coverage ledger](acceptance-coverage-ledger.md) is implemented; no pending verifier is counted as implemented by this handoff.

## Native raw-response adoption clarification

The shared provider-v2 result already permits a large `raw_response: ArtifactRef`; its 64 KiB metadata envelope stores the reference, not the response body. The 1 MiB raw-body restriction in runtime's current `provider_v2.rs` is a runtime-owned role limit, rather than a shared-schema limit. Runtime may implement the requested separately bounded Gemini Speech native-response role (12 MiB original raw; 8 MiB decoded native audio, per the producer-owned `docs/implementation/generation/provider-codecs.md` handoff in the shared checkout (not part of this frozen source)) without widening ordinary proposal/stage/helper JSON or request limits. This is implementation guidance, not dispatch authority or proof that the runtime cap has changed.

Preserve the original committed proposal payload and issued `request_sha256`. Commit the exact encoded native request bytes and codec/source digest as separately identified private evidence before dispatch, then use those exact bytes. Keep the actual vendor raw body in the result's original `raw_response`; decoding failures, refusals and partial bodies do not erase that artifact or accounting. A normalized observation is a new derivative with original request/wire/raw identities; it cannot replace the original response. Return the precise exchange/observation schema and bounded adapter tests for any new cross-lane record before adoption. Existing-job recovery remains separately issued and cannot resubmit a native request.

The moving worker tree also reports a historical Direction-plan compatibility failure in the new primary-registration tests. The frozen R19/G25 candidate in PR #3 retains its exact original compiler bytes and passes that earlier aggregate; this does not certify the later worker edits. Generation/runtime must return a versioned historical-recipe reader or an explicit rejection/migration handoff preserving original request/result/usage identities. Do not relabel old plans as outputs of the new compiler.
