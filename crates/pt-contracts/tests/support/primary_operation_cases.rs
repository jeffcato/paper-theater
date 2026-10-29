use pt_contracts::*;
use serde_json::{Value, json};

fn metadata() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap()
}

fn decode<T: serde::de::DeserializeOwned>(v: &Value, field: &str) -> ContractResult<T> {
    serde_json::from_value(v.clone()).map_err(|e| ContractError {
        code: "shape".into(),
        field: field.into(),
        message: e.to_string(),
    })
}

pub fn fixture() -> (
    PrimaryProvenanceV1,
    ProviderOperationV2,
    ProviderResultV2,
    Vec<u8>,
) {
    let wires = metadata();
    let issued = wires
        .iter()
        .find(|c| c["id"] == "free-money-request")
        .unwrap()["wire"]
        .clone();
    let op: ProviderOperationV2 = serde_json::from_value(issued.clone()).unwrap();
    let text = "{\"primary\":true}";
    let bytes = text.as_bytes().to_vec();
    let mut windows = issued["quota_windows"].clone();
    windows[0]["accounted"] = json!({"money_microusd":0,"requests":1,"tokens":120,"repairs":0});
    let result: ProviderResultV2 = serde_json::from_value(json!({
        "schema_version": 2,
        "result": {
            "attempt_id": op.operation.attempt_id,
            "completion": "completed",
            "request_sha256": op.operation.request_sha256,
            "response_sha256": null,
            "raw_response": null,
            "outputs": [],
            "usage": null,
            "started_at": "primary-recorded",
            "elapsed_ms": 1,
            "diagnostic": ""
        },
        "context": issued["context"],
        "identity": issued["identity"],
        "quota_windows": windows,
        "observed_usage": {"requests":null,"tokens":null,"input_tokens":null,"output_tokens":null,"money_microusd":null},
        "accounted_usage": {"money_microusd":0,"requests":1,"tokens":120,"repairs":0},
        "started_at_ms": 1,
        "recorded_at_ms": 2,
        "epoch_disposition": "current",
        "retry": null,
        "recovery": null
    }))
    .unwrap();
    let provenance = PrimaryProvenanceV1 {
        schema_version: 1,
        operation_attempt_id: op.operation.attempt_id.clone(),
        operation_request_sha256: op.operation.request_sha256.clone(),
        primary_request: ArtifactRef {
            id: "primary-request-1".into(),
            file: FileRef {
                path: "private/primary-request.json".into(),
                sha256: digest(&bytes),
                bytes: bytes.len() as u64,
                media_type: "application/json".into(),
                dimensions: None,
            },
            provenance: Provenance::Diagnostic,
        },
        result_response_sha256: None,
    };
    (provenance, op, result, bytes)
}

pub fn records() -> Vec<Value> {
    let (provenance, op, result, bytes) = fixture();
    let text = String::from_utf8(bytes.clone()).unwrap();
    let op_json = serde_json::to_value(&op).unwrap();
    let result_json = serde_json::to_value(&result).unwrap();
    let base = json!({"provenance":provenance,"operation":op_json,"result":result_json,"primary_text":text});
    let mut rows = vec![];
    let mut add = |id: &str, valid: bool, mutate: &dyn Fn(&mut Value)| {
        let mut row = base.clone();
        row["id"] = json!(id);
        row["valid"] = json!(valid);
        mutate(&mut row);
        rows.push(row);
    };
    add("exact-primary-operation-binding", true, &|_| {});
    add("forged-operation-attempt-is-rejected", false, &|row| {
        row["provenance"]["operation_attempt_id"] = json!("forged-attempt");
    });
    add("stale-operation-request-is-rejected", false, &|row| {
        row["provenance"]["operation_request_sha256"] = json!(digest(b"other-request"));
    });
    add("substituted-recorded-response-is-rejected", false, &|row| {
        row["provenance"]["result_response_sha256"] = json!(digest(b"forged-response"));
    });
    add("tampered-committed-operation-is-rejected", false, &|row| {
        row["operation"]["operation"]["payload"]["file"]["sha256"] = json!(digest(b"tampered"));
    });
    add("altered-primary-bytes-are-rejected", false, &|row| {
        row["primary_text"] = json!("{\"primary\":false}");
    });
    rows
}

pub fn check(row: &Value) -> ContractResult<()> {
    let provenance: PrimaryProvenanceV1 = decode(&row["provenance"], "provenance")?;
    let op: ProviderOperationV2 = decode(&row["operation"], "operation")?;
    let result: ProviderResultV2 = decode(&row["result"], "result")?;
    let text = row["primary_text"].as_str().unwrap_or("");
    provenance.validate_for_operation(&op, &result, text.as_bytes())
}
