# Native exchange boundary — services v1.10.0

INT-02 additive shared handoff. Rust/TypeScript now publish `ProviderNativeExchangeV1`, `ProviderNativeExchangePolicyV1` and `ProviderNativeExchangeBytesV1`. Existing provider-v2 records, service methods and explicit unavailable defaults retain their meanings; no legacy fallback or dispatch authority is added.

Rust calls `record.validate_bytes(&policy, &issued, &result, actual_bytes)`. TypeScript calls `await validateProviderNativeExchangeV1(record, policy, issued, result, actualBytes)`. Actual issued-operation, logical-request, native-request and declared raw-response bytes are required. Digest-looking strings alone cannot satisfy the byte checks.

## Frozen record and bounds

| Field | Meaning |
|---|---|
| `schema_version: 1` | Native exchange record/policy version |
| `issued_operation: FileRef` | Exact original private serialized provider-v2 issuance; full parsed descriptor must equal runtime's independently selected original |
| `codec_sha256` | Independently registered native codec identity |
| `origin_id` | Bounded identifier resolved through runtime's fixed HTTPS origin registry |
| `path` | Exact codec-selected relative provider path; query/fragment/encoded/traversal/protocol-relative routes fail |
| `wire_request: FileRef` | Separately pinned actual native JSON body; original logical request identity stays unchanged |
| `raw_response: ArtifactRef?` | Original result's genuine raw artifact, including its ID/provenance; missing response remains explicitly absent |

Policy contains the same independently selected issuance/codec/origin/path/native-request pins and `max_raw_response_bytes`; it has no `raw_response` field. Runtime obtains the expected raw identity from its original result. Never construct trusted policy from a submitted exchange record.

Metadata/issued-operation JSON is bounded to 64 KiB and native requests to 1 MiB. Ordinary raw responses stay at most 1 MiB; the separately selected Gemini Speech role may reach 12 MiB. The public `MAX_NATIVE_PROVIDER_RECORD_BYTES`, `MAX_NATIVE_PROVIDER_WIRE_BYTES` and `MAX_NATIVE_PROVIDER_RAW_BYTES` expose these ceilings. Streaming limits must be enforced before allocating/reading bodies. Other large native roles require their own reviewed handoff; ordinary stage/proposal/helper bounds are unchanged. Native audio decoding retains generation's separate 8 MiB bound.

The complete issued descriptor is checked, preserving provider/project/credential context, run/epoch/unit, operation/repair/retry identities, quotas and original request. Raw-response identity and conservative original result/accounting are checked without mutation. An existing-job descriptor (`issued.recovery != null`) cannot become a native submission; recovery still uses `recover_provider_v2` and its separate query receipt. Old optional null/omitted fields retain their actual v2 meaning. Duplicate JSON keys, including escaped aliases, fail in both languages. Existing coverage-ledger error codes are preserved through the shared strict parser.

## Exact server/generation adoption

1. Freeze the original immutable issuance JSON from the broker into private CAS with a genuine committed FileRef. Keep the original logical payload/request digest and all lineage/accounting intact.
2. Verify the committed logical payload bytes. Independently run the registered generation codec, pin its exact source/recipe and commit the exact encoded native body. Resolve origin/path through trusted configuration and the codec. Build expected policy from these authoritative records.
3. Dispatch those exact native bytes only through the original runtime-issued provider-v2 attempt after normal mode/authorization/reservation/epoch/secret checks. Keep original `request_sha256`; the separate native-body digest records the transport encoding. No implicit retry, descriptor repair or model substitution.
4. Capture original native raw bytes under the explicitly selected response role. Preserve partial/refused/unusable results and uncertain charges. Commit real raw/media refs; normalized observations remain separately identified derivatives. Codec/body validation supplies byte identities, with no listening/creative/production approval.
5. Validate the exchange against independently selected original issuance/result and actual verified reads. Persist privately under runtime's transaction/epoch rules. Late/canceled responses retain their true original epoch/accounting and cannot commit stage success, library updates or publication.
6. Supply the genuine exchange artifact through the original result transaction or a separately versioned private stage context. The record does not reference a result digest, avoiding a hash cycle. Do not rewrite a finalized old result merely to insert the new reference.
7. Retain original v2 recovery/query boundaries and Fixture/Replay zero-dispatch behavior. Native exchange JSON is private evidence, never a public audience export or serialized execution capability.

Runtime owns implementing the larger raw-role read/transport cap and durable adapter. Generation owns actual codecs, native parsers and stage observation adoption. Return exact frozen source/recipe/response examples before integration adopts those implementations. The shared record does not enable the currently unavailable production construction path.

## Evidence

Four public Rust tests and 39 Rust-produced cross-language cases pass. Cases cover actual-byte substitutions, original descriptor/context changes, unknown usage retention, fixed routes, metadata/raw role limits (including exact 12 MiB and plus one), forged grants, duplicate keys, historical optional fields and existing-job separation. Complete strict/native/packed contract checks pass. Raw red/green evidence and source pins are in `provider-native-exchange-ready.json`; combined-source CI is recorded separately when complete.

Matrix v3 stays **five implemented / 32 pending**. Byte binding supplies no provider-execution, independent listening, full creative verification, publication or actual-device authority. Production gates, NeedsRepair and all Missing/Uncertain/Fail findings remain. Zero provider calls and credential loading.
