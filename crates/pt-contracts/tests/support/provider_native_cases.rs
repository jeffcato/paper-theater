use pt_contracts::*;
use serde_json::{Value, json};

pub fn originals() -> (ProviderOperationV2, ProviderResultV2) {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap();
    let mut op: ProviderOperationV2 = serde_json::from_value(
        cases
            .iter()
            .find(|c| c["id"] == "free-money-request")
            .unwrap()["wire"]
            .clone(),
    )
    .unwrap();
    let mut result: ProviderResultV2 = serde_json::from_value(
        cases
            .iter()
            .find(|c| c["id"] == "ambiguous-unknown-usage-retains-reservation")
            .unwrap()["wire"]
            .clone(),
    )
    .unwrap();
    op.operation.request_sha256 = digest(b"{}");
    op.operation.payload.file.sha256 = digest(b"{}");
    result.result.request_sha256 = op.operation.request_sha256.clone();
    (op, result)
}
pub fn file(path: &str, bytes: &[u8]) -> FileRef {
    FileRef {
        path: path.into(),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
        media_type: "application/json".into(),
        dimensions: None,
    }
}

fn scenario(
    mut op: ProviderOperationV2,
    mut result: ProviderResultV2,
    count: usize,
    cap: u64,
) -> Value {
    let raw = vec![b'x'; count];
    let raw_ref = (count > 0).then(|| ArtifactRef {
        id: "native-raw".into(),
        file: file("private/native-raw.json", &raw),
        provenance: Provenance::Diagnostic,
    });
    result.result.raw_response = raw_ref.clone();
    result.result.response_sha256 = raw_ref.as_ref().map(|r| r.file.sha256.clone());
    op.operation.payload.file.bytes = 2;
    let issued = serde_json::to_string(&op).unwrap();
    let wire = br#"{"native":"body"}"#;
    let record = ProviderNativeExchangeV1 {
        schema_version: 1,
        issued_operation: file("private/issued.json", issued.as_bytes()),
        codec_sha256: digest(b"pinned-native-codec"),
        origin_id: "fixed-origin".into(),
        path: "/v1/native".into(),
        wire_request: file("private/wire.json", wire),
        raw_response: raw_ref,
    };
    let policy = ProviderNativeExchangePolicyV1 {
        schema_version: 1,
        issued_operation: record.issued_operation.clone(),
        codec_sha256: record.codec_sha256.clone(),
        origin_id: record.origin_id.clone(),
        path: record.path.clone(),
        wire_request: record.wire_request.clone(),
        max_raw_response_bytes: cap,
    };
    json!({"id":"native-unknown-completion","valid":true,"record":record,"policy":policy,"operation":op,"result":result,
        "issued_bytes":issued,"logical_bytes":"{}","wire_bytes":String::from_utf8(wire.to_vec()).unwrap(),"raw_count":count})
}

pub fn records() -> Vec<Value> {
    let (op, result) = originals();
    let base = scenario(op.clone(), result.clone(), 0, 1024 * 1024);
    let with_raw = scenario(op.clone(), result.clone(), 32, 1024 * 1024);
    let mut rows = vec![base.clone(), with_raw.clone()];
    rows[1]["id"] = json!("bound-native-raw");
    let mut speech_op = serde_json::to_value(&op).unwrap();
    let mut speech_result = serde_json::to_value(&result).unwrap();
    speech_op["operation"]["kind"] = json!("speech");
    speech_op["operation"]["provider"] = json!("gemini");
    speech_op["operation"]["model"] = json!("gemini-3.8-flash-tts");
    for value in [&mut speech_op, &mut speech_result] {
        value["context"]["provider"] = json!("gemini");
        for w in value["quota_windows"].as_array_mut().unwrap() {
            w["scope"]["provider"] = json!("gemini");
        }
    }
    let mut native = scenario(
        serde_json::from_value(speech_op).unwrap(),
        serde_json::from_value(speech_result).unwrap(),
        1024 * 1024 + 1,
        12 * 1024 * 1024,
    );
    native["id"] = json!("large-gemini-speech-raw-role");
    rows.push(native.clone());
    let mut audio_op = serde_json::to_value(&op).unwrap();
    let mut audio_result = serde_json::to_value(&result).unwrap();
    audio_op["operation"]["kind"] = json!("transcription");
    audio_op["operation"]["provider"] = json!("gemini");
    audio_op["operation"]["model"] = json!("gemini-3.5-transcribe");
    for value in [&mut audio_op, &mut audio_result] {
        value["context"]["provider"] = json!("gemini");
        for w in value["quota_windows"].as_array_mut().unwrap() {
            w["scope"]["provider"] = json!("gemini");
        }
    }
    let mut audio = |id: &str, wire_len: usize, valid: bool| {
        let mut row = scenario(
            serde_json::from_value(audio_op.clone()).unwrap(),
            serde_json::from_value(audio_result.clone()).unwrap(),
            0,
            1024 * 1024,
        );
        let wire = "x".repeat(wire_len);
        let wire_ref = file("private/wire.json", wire.as_bytes());
        row["id"] = json!(id);
        row["valid"] = json!(valid);
        row["wire_bytes"] = json!(wire);
        row["record"]["wire_request"] = serde_json::to_value(&wire_ref).unwrap();
        row["policy"]["wire_request"] = serde_json::to_value(&wire_ref).unwrap();
        rows.push(row);
    };
    audio("audio-30s-transcription-role", 1_920_044, true);
    audio("audio-90s-transcription-role", 5_760_044, true);
    audio(
        "exact-audio-role-bound",
        MAX_NATIVE_AUDIO_WIRE_BYTES as usize,
        true,
    );
    audio(
        "audio-role-bound-plus-one",
        MAX_NATIVE_AUDIO_WIRE_BYTES as usize + 1,
        false,
    );
    let mut role_op = audio_op.clone();
    role_op["operation"]["kind"] = json!("direction");
    let mut role_row = scenario(
        serde_json::from_value(role_op).unwrap(),
        serde_json::from_value(audio_result.clone()).unwrap(),
        0,
        1024 * 1024,
    );
    let role_wire = "x".repeat(1_920_044);
    let role_ref = file("private/wire.json", role_wire.as_bytes());
    role_row["id"] = json!("wrong-role-large-wire");
    role_row["valid"] = json!(false);
    role_row["wire_bytes"] = json!(role_wire);
    role_row["record"]["wire_request"] = serde_json::to_value(&role_ref).unwrap();
    role_row["policy"]["wire_request"] = serde_json::to_value(&role_ref).unwrap();
    rows.push(role_row);
    let mut model_op = audio_op.clone();
    model_op["operation"]["model"] = json!("gemini-2.0-flash");
    let mut model_row = scenario(
        serde_json::from_value(model_op).unwrap(),
        serde_json::from_value(audio_result.clone()).unwrap(),
        0,
        1024 * 1024,
    );
    let model_wire = "x".repeat(1_920_044);
    let model_ref = file("private/wire.json", model_wire.as_bytes());
    model_row["id"] = json!("wrong-model-large-wire");
    model_row["valid"] = json!(false);
    model_row["wire_bytes"] = json!(model_wire);
    model_row["record"]["wire_request"] = serde_json::to_value(&model_ref).unwrap();
    model_row["policy"]["wire_request"] = serde_json::to_value(&model_ref).unwrap();
    rows.push(model_row);
    let mut changed = scenario(
        serde_json::from_value(audio_op.clone()).unwrap(),
        serde_json::from_value(audio_result.clone()).unwrap(),
        0,
        1024 * 1024,
    );
    let changed_wire = "x".repeat(1_920_044);
    let changed_ref = file("private/wire.json", changed_wire.as_bytes());
    changed["id"] = json!("changed-audio-identity");
    changed["valid"] = json!(false);
    changed["wire_bytes"] = json!("y".repeat(1_920_044));
    changed["record"]["wire_request"] = serde_json::to_value(&changed_ref).unwrap();
    changed["policy"]["wire_request"] = serde_json::to_value(&changed_ref).unwrap();
    rows.push(changed);
    let op: ProviderOperationV2 = serde_json::from_value(native["operation"].clone()).unwrap();
    let result: ProviderResultV2 = serde_json::from_value(native["result"].clone()).unwrap();
    let mut exact = scenario(
        op.clone(),
        result.clone(),
        12 * 1024 * 1024,
        12 * 1024 * 1024,
    );
    exact["id"] = json!("exact-native-role-bound");
    rows.push(exact);
    let mut overflow = scenario(op, result, 12 * 1024 * 1024 + 1, 12 * 1024 * 1024);
    overflow["id"] = json!("native-role-bound-plus-one");
    overflow["valid"] = json!(false);
    rows.push(overflow);
    for (id, pointer, value) in [
        ("record-version", "/record/schema_version", json!(2)),
        ("policy-version", "/policy/schema_version", json!(2)),
        ("substituted-issued-bytes", "/issued_bytes", json!("{}")),
        ("substituted-logical-bytes", "/logical_bytes", json!("{ }")),
        (
            "substituted-wire-bytes",
            "/wire_bytes",
            json!("{\"native\":\"different\"}"),
        ),
        (
            "substituted-codec",
            "/record/codec_sha256",
            json!(digest(b"other-codec")),
        ),
        (
            "substituted-origin",
            "/record/origin_id",
            json!("other-origin"),
        ),
        ("substituted-path", "/record/path", json!("/v1/other")),
        (
            "substituted-observer",
            "/operation/identity/epoch",
            json!(2),
        ),
        (
            "substituted-credential-context",
            "/operation/context/credential_context_id",
            json!("other-credential"),
        ),
        (
            "rewritten-issued-request",
            "/operation/operation/request_sha256",
            json!(digest(b"native wire replaces original")),
        ),
        (
            "unknown-charges-cleared",
            "/result/accounted_usage/tokens",
            json!(0),
        ),
        ("undeclared-raw-bytes", "/raw_count", json!(1)),
        ("zero-cap", "/policy/max_raw_response_bytes", json!(0)),
        (
            "ordinary-raw-cap-widened",
            "/policy/max_raw_response_bytes",
            json!(1024 * 1024 + 1),
        ),
        (
            "global-raw-cap-widened",
            "/policy/max_raw_response_bytes",
            json!(12 * 1024 * 1024 + 1),
        ),
    ] {
        let mut c = base.clone();
        c["id"] = json!(id);
        c["valid"] = json!(false);
        *c.pointer_mut(pointer).unwrap() = value;
        rows.push(c);
    }
    for (id, pointer, value) in [
        ("missing-native-raw-bytes", "/raw_count", json!(0)),
        ("altered-native-raw-bytes", "/raw_count", json!(31)),
        (
            "substituted-native-raw-ref",
            "/record/raw_response/id",
            json!("different-raw"),
        ),
    ] {
        let mut c = with_raw.clone();
        c["id"] = json!(id);
        c["valid"] = json!(false);
        *c.pointer_mut(pointer).unwrap() = value;
        rows.push(c);
    }
    let mut c = native;
    c["id"] = json!("native-raw-exceeds-selected-role");
    c["valid"] = json!(false);
    c["policy"]["max_raw_response_bytes"] = json!(1024 * 1024);
    rows.push(c);
    for (id, codec, origin, path) in [
        ("invalid-codec-digest", "bad", "fixed-origin", "/v1/native"),
        (
            "origin-is-not-a-url",
            "",
            "https://unselected.invalid",
            "/v1/native",
        ),
        (
            "protocol-relative-path",
            "",
            "fixed-origin",
            "//unselected.invalid/path",
        ),
        (
            "credential-query-path",
            "",
            "fixed-origin",
            "/v1/native?key=forbidden",
        ),
        (
            "encoded-relative-path",
            "",
            "fixed-origin",
            "/v1/%2e%2e/path",
        ),
        ("traversal-path", "", "fixed-origin", "/v1/../native"),
    ] {
        let mut c = base.clone();
        c["id"] = json!(id);
        c["valid"] = json!(false);
        for role in ["record", "policy"] {
            if !codec.is_empty() {
                c[role]["codec_sha256"] = json!(codec);
            }
            c[role]["origin_id"] = json!(origin);
            c[role]["path"] = json!(path);
        }
        rows.push(c);
    }
    for field in ["headers", "production_approved", "spending_authority"] {
        let mut c = base.clone();
        c["id"] = json!(format!("forbidden-{field}"));
        c["valid"] = json!(false);
        c["record"][field] = json!(true);
        rows.push(c);
    }
    let mut oversized_metadata = base.clone();
    oversized_metadata["id"] = json!("native-sidecar-metadata-stays-bounded");
    oversized_metadata["valid"] = json!(false);
    for role in ["record", "policy"] {
        oversized_metadata[role]["issued_operation"]["path"] = json!("x".repeat(65536));
    }
    rows.push(oversized_metadata);
    for escaped in [false, true] {
        let mut c = base.clone();
        c["id"] = json!(if escaped {
            "escaped-duplicate-issued-key"
        } else {
            "duplicate-issued-key"
        });
        c["valid"] = json!(false);
        let key = if escaped {
            "\\u0073chema_version"
        } else {
            "schema_version"
        };
        let bytes = c["issued_bytes"].as_str().unwrap().replacen(
            "\"schema_version\":2",
            &format!("\"schema_version\":2,\"{key}\":2"),
            1,
        );
        for role in ["record", "policy"] {
            c[role]["issued_operation"] = json!(file("private/issued.json", bytes.as_bytes()));
        }
        c["issued_bytes"] = json!(bytes);
        rows.push(c);
    }
    let mut omitted = base.clone();
    omitted["id"] = json!("legacy-optional-null-omission");
    let mut original: Value =
        serde_json::from_str(omitted["issued_bytes"].as_str().unwrap()).unwrap();
    original.as_object_mut().unwrap().remove("recovery");
    for key in ["repair", "transport_retry"] {
        original["identity"].as_object_mut().unwrap().remove(key);
    }
    let bytes = serde_json::to_string(&original).unwrap();
    for role in ["record", "policy"] {
        omitted[role]["issued_operation"] = json!(file("private/issued.json", bytes.as_bytes()));
    }
    omitted["issued_bytes"] = json!(bytes);
    rows.push(omitted);
    let originals: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap();
    let recovered = originals
        .iter()
        .find(|c| c["id"] == "recover-existing-job-without-new-attempt")
        .unwrap();
    let mut op: ProviderOperationV2 =
        serde_json::from_value(recovered["operation"].clone()).unwrap();
    let mut result: ProviderResultV2 = serde_json::from_value(recovered["wire"].clone()).unwrap();
    op.operation.request_sha256 = digest(b"{}");
    op.operation.payload.file.sha256 = digest(b"{}");
    op.recovery.as_mut().unwrap().request_sha256 = digest(b"{}");
    result.result.request_sha256 = digest(b"{}");
    result.recovery.as_mut().unwrap().request_sha256 = digest(b"{}");
    let mut recovery = scenario(op, result, 0, 1024 * 1024);
    recovery["id"] = json!("existing-job-is-not-a-native-submission");
    recovery["valid"] = json!(false);
    rows.push(recovery);
    rows
}

pub fn check(c: &Value) -> ContractResult<()> {
    fn decode<T: serde::de::DeserializeOwned>(v: &Value) -> ContractResult<T> {
        serde_json::from_value(v.clone()).map_err(|e| ContractError {
            code: "shape".into(),
            field: "record".into(),
            message: e.to_string(),
        })
    }
    let record: ProviderNativeExchangeV1 = decode(&c["record"])?;
    let policy: ProviderNativeExchangePolicyV1 = decode(&c["policy"])?;
    let operation: ProviderOperationV2 = decode(&c["operation"])?;
    let result: ProviderResultV2 = decode(&c["result"])?;
    let raw = vec![b'x'; c["raw_count"].as_u64().unwrap() as usize];
    record.validate_bytes(
        &policy,
        &operation,
        &result,
        ProviderNativeExchangeBytesV1 {
            issued_operation: c["issued_bytes"].as_str().unwrap().as_bytes(),
            logical_request: c["logical_bytes"].as_str().unwrap().as_bytes(),
            wire_request: c["wire_bytes"].as_str().unwrap().as_bytes(),
            raw_response: (!raw.is_empty()).then_some(raw.as_slice()),
        },
    )
}
