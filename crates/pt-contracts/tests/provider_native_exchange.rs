use pt_contracts::*;
use serde_json::json;

#[path = "support/provider_native_cases.rs"]
mod cases;
use cases::{file, originals};

#[test]
fn shared_native_exchange_cases_preserve_all_original_identities_and_limits() {
    for c in cases::records() {
        let r = cases::check(&c);
        assert_eq!(r.is_ok(), c["valid"] == true, "{}: {r:?}", c["id"]);
    }
}

#[test]
fn native_byte_binding_preserves_logical_request_and_uncertain_accounting() {
    let (op, result) = originals();
    op.validate().unwrap();
    result.validate_for_operation(&op).unwrap();
    let issued = serde_json::to_vec(&op).unwrap();
    let wire = br#"{"native":"body"}"#;
    let record = ProviderNativeExchangeV1 {
        schema_version: 1,
        issued_operation: file("private/issued.json", &issued),
        codec_sha256: digest(b"pinned-codec"),
        origin_id: "fixture-origin".into(),
        path: "/v1/native".into(),
        wire_request: file("private/wire.json", wire),
        raw_response: None,
    };
    let policy = ProviderNativeExchangePolicyV1 {
        schema_version: 1,
        issued_operation: record.issued_operation.clone(),
        codec_sha256: record.codec_sha256.clone(),
        origin_id: record.origin_id.clone(),
        path: record.path.clone(),
        wire_request: record.wire_request.clone(),
        max_raw_response_bytes: 1024 * 1024,
    };
    let original = json!({"op":op,"result":result});
    record
        .validate_bytes(
            &policy,
            &op,
            &result,
            ProviderNativeExchangeBytesV1 {
                issued_operation: &issued,
                logical_request: b"{}",
                wire_request: wire,
                raw_response: None,
            },
        )
        .unwrap();
    assert_eq!(json!({"op":op,"result":result}), original);
    assert_eq!(result.result.completion, ProviderCompletion::Ambiguous);
    assert!(result.result.usage.is_none());
    assert_ne!(op.operation.request_sha256, record.wire_request.sha256);
}

#[test]
fn native_byte_binding_rejects_unbounded_roles_even_with_matching_bytes() {
    let (op, result) = originals();
    let issued = serde_json::to_vec(&op).unwrap();
    let wire = b"{}";
    let record = ProviderNativeExchangeV1 {
        schema_version: 1,
        issued_operation: file("issued.json", &issued),
        codec_sha256: digest(b"codec"),
        origin_id: "fixed-origin".into(),
        path: "/v1/native".into(),
        wire_request: file("wire.json", wire),
        raw_response: None,
    };
    let mut policy = ProviderNativeExchangePolicyV1 {
        schema_version: 1,
        issued_operation: record.issued_operation.clone(),
        codec_sha256: record.codec_sha256.clone(),
        origin_id: record.origin_id.clone(),
        path: record.path.clone(),
        wire_request: record.wire_request.clone(),
        max_raw_response_bytes: 0,
    };
    for cap in [0, 1024 * 1024 + 1, 12 * 1024 * 1024 + 1] {
        policy.max_raw_response_bytes = cap;
        assert!(
            record
                .validate_bytes(
                    &policy,
                    &op,
                    &result,
                    ProviderNativeExchangeBytesV1 {
                        issued_operation: &issued,
                        logical_request: b"{}",
                        wire_request: wire,
                        raw_response: None,
                    }
                )
                .is_err(),
            "ordinary response role accepted cap {cap}"
        );
    }
    policy.max_raw_response_bytes = 1024 * 1024;
    let mut padded = issued.clone();
    padded.resize(64 * 1024 + 1, b' ');
    let mut large = record.clone();
    large.issued_operation = file("issued.json", &padded);
    policy.issued_operation = large.issued_operation.clone();
    assert!(
        large
            .validate_bytes(
                &policy,
                &op,
                &result,
                ProviderNativeExchangeBytesV1 {
                    issued_operation: &padded,
                    logical_request: b"{}",
                    wire_request: wire,
                    raw_response: None,
                }
            )
            .is_err()
    );
    policy.issued_operation = record.issued_operation.clone();
    let oversized = vec![b' '; 1024 * 1024 + 1];
    let mut large = record.clone();
    large.wire_request = file("wire.json", &oversized);
    policy.wire_request = large.wire_request.clone();
    assert!(
        large
            .validate_bytes(
                &policy,
                &op,
                &result,
                ProviderNativeExchangeBytesV1 {
                    issued_operation: &issued,
                    logical_request: b"{}",
                    wire_request: &oversized,
                    raw_response: None,
                }
            )
            .is_err()
    );
}

#[test]
fn native_binding_requires_a_selected_codec_and_fixed_relative_route() {
    let (op, result) = originals();
    let issued = serde_json::to_vec(&op).unwrap();
    let wire = b"{}";
    for (codec, origin, path) in [
        ("bad", "fixed-origin", "/v1/native"),
        (
            &digest(b"codec"),
            "https://unselected.invalid",
            "/v1/native",
        ),
        (
            &digest(b"codec"),
            "fixed-origin",
            "//unselected.invalid/native",
        ),
        (
            &digest(b"codec"),
            "fixed-origin",
            "/v1/native?key=forbidden",
        ),
        (&digest(b"codec"), "fixed-origin", "/v1/../native"),
    ] {
        let record = ProviderNativeExchangeV1 {
            schema_version: 1,
            issued_operation: file("issued.json", &issued),
            codec_sha256: codec.into(),
            origin_id: origin.into(),
            path: path.into(),
            wire_request: file("wire.json", wire),
            raw_response: None,
        };
        let policy = ProviderNativeExchangePolicyV1 {
            schema_version: 1,
            issued_operation: record.issued_operation.clone(),
            codec_sha256: record.codec_sha256.clone(),
            origin_id: record.origin_id.clone(),
            path: record.path.clone(),
            wire_request: record.wire_request.clone(),
            max_raw_response_bytes: 1024 * 1024,
        };
        assert!(
            record
                .validate_bytes(
                    &policy,
                    &op,
                    &result,
                    ProviderNativeExchangeBytesV1 {
                        issued_operation: &issued,
                        logical_request: b"{}",
                        wire_request: wire,
                        raw_response: None,
                    }
                )
                .is_err(),
            "invalid codec/origin/path accepted: {path}"
        );
    }
}

#[test]
fn audio_wire_replay_is_exact_and_idempotent() {
    let row = cases::records()
        .into_iter()
        .find(|c| c["id"] == "audio-30s-transcription-role")
        .unwrap();
    let record: ProviderNativeExchangeV1 = serde_json::from_value(row["record"].clone()).unwrap();
    let policy: ProviderNativeExchangePolicyV1 =
        serde_json::from_value(row["policy"].clone()).unwrap();
    let op: ProviderOperationV2 = serde_json::from_value(row["operation"].clone()).unwrap();
    let result: ProviderResultV2 = serde_json::from_value(row["result"].clone()).unwrap();
    let wire = row["wire_bytes"].as_str().unwrap().as_bytes();
    let bytes = || ProviderNativeExchangeBytesV1 {
        issued_operation: row["issued_bytes"].as_str().unwrap().as_bytes(),
        logical_request: row["logical_bytes"].as_str().unwrap().as_bytes(),
        wire_request: wire,
        raw_response: None,
    };
    record
        .validate_bytes(&policy, &op, &result, bytes())
        .unwrap();
    record
        .validate_bytes(&policy, &op, &result, bytes())
        .unwrap();
}
