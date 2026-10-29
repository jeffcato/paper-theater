use pt_contracts::*;
use serde_json::{Value, json};

fn decode(v: &Value) -> ContractResult<TrustedPlayerLoadV1> {
    serde_json::from_value(v.clone()).map_err(|e| ContractError {
        code: "shape".into(),
        field: "trusted_player_load".into(),
        message: e.to_string(),
    })
}

pub fn fixture() -> TrustedPlayerLoadV1 {
    TrustedPlayerLoadV1 {
        schema_version: 1,
        profile_id: "paper-player-r186-v1".into(),
        checker_sha256: digest(b"trusted-checker"),
        acceptance_report_sha256: digest(b"verified-report"),
        catalog_entry_sha256: digest(b"catalog-entry"),
        release_id: "release-oz-1".into(),
        allow_diagnostic: false,
    }
}

pub fn records() -> Vec<Value> {
    let base = serde_json::to_value(fixture()).unwrap();
    let mut rows = vec![];
    let mut add = |id: &str, valid: bool, mutate: &dyn Fn(&mut Value)| {
        let mut row = json!({"id":id,"valid":valid,"load":base});
        mutate(&mut row["load"]);
        rows.push(row);
    };
    add("exact-trusted-player-pinning", true, &|_| {});
    add("zero-checker-hash-is-rejected", false, &|load| {
        load["checker_sha256"] = json!(ZERO_DIGEST);
    });
    add("zero-report-hash-is-rejected", false, &|load| {
        load["acceptance_report_sha256"] = json!(ZERO_DIGEST);
    });
    add("zero-catalog-hash-is-rejected", false, &|load| {
        load["catalog_entry_sha256"] = json!(ZERO_DIGEST);
    });
    add("missing-profile-is-rejected", false, &|load| {
        load["profile_id"] = json!("");
    });
    rows
}

pub fn check(row: &Value) -> ContractResult<()> {
    decode(&row["load"])?.validate()
}
