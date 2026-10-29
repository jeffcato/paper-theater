use pt_contracts::*;
use serde_json::json;

pub fn configuration() -> serde_json::Value {
    json!({
        "schema_version":1,"id":"offline-pilot","book_id":"pg-36","source_id":"pg-36-book-1-chapter-1",
        "pins":{"recipe_sha256":digest(b"recipe"),"profile_sha256":digest(b"profile"),"profile_id":"profile-v1"},
        "authorization_proof_sha256":digest(b"synthetic-proof-not-authorization"),
        "accounting_original_sha256s":[digest(b"synthetic-original-not-accounting")],
        "limits":{"money_microusd":1000,"requests":2,"tokens":100,"repairs":0,"wall_time_ms":30000},
        "operations":[{"id":"pilot-speech","unit_id":"pilot-unit","stage":"pilot","purpose":"pilot","kind":"speech",
            "context":{"provider":"fixture","project_id":"project","credential_context_id":"credential-reference"},
            "model":"fixture-model","recipe_sha256":digest(b"speech-recipe"),"request_sha256":digest(b"exact-request"),
            "secret_env":"PT_OFFLINE_CREDENTIAL","limits":{"money_microusd":1000,"requests":2,"tokens":100,"repairs":0,"wall_time_ms":30000},"allow_recovery":false}]
    })
}

pub fn records() -> Vec<serde_json::Value> {
    let mut rows = vec![json!({"id":"exact-offline-pilot","valid":true,"value":configuration()})];
    let mut oz = configuration();
    oz["book_id"] = json!("pg-55");
    oz["source_id"] = json!("pg-55-chapter-1");
    rows.push(json!({"id":"oz-same-boundary","valid":true,"value":oz}));
    for (name, pointer, value) in [
        ("wrong-version", "/schema_version", json!(2)),
        ("wrong-chapter", "/source_id", json!("pg-55-chapter-1")),
        ("absent-history", "/accounting_original_sha256s", json!([])),
        ("no-operations", "/operations", json!([])),
        (
            "changed-proof-shape",
            "/authorization_proof_sha256",
            json!("unverified"),
        ),
        ("unsafe-cap", "/limits/tokens", json!(9007199254740992u64)),
        ("zero-timeout", "/limits/wall_time_ms", json!(0)),
        ("excess-timeout", "/limits/wall_time_ms", json!(300001)),
        (
            "widened-requests",
            "/operations/0/limits/requests",
            json!(3),
        ),
        (
            "widened-money",
            "/operations/0/limits/money_microusd",
            json!(1001),
        ),
        ("widened-repairs", "/operations/0/limits/repairs", json!(1)),
        (
            "widened-timeout",
            "/operations/0/limits/wall_time_ms",
            json!(30001),
        ),
        ("zero-requests", "/operations/0/limits/requests", json!(0)),
        (
            "pilot-as-bulk-speech",
            "/operations/0/stage",
            json!("speech"),
        ),
        (
            "secret-contents",
            "/operations/0/secret_env",
            json!("literal secret contents"),
        ),
        (
            "numeric-env-name",
            "/operations/0/secret_env",
            json!("2SECRET"),
        ),
        (
            "wrong-project-shape",
            "/operations/0/context/project_id",
            json!(""),
        ),
        (
            "request-not-digest",
            "/operations/0/request_sha256",
            json!("request"),
        ),
        (
            "unknown-purpose",
            "/operations/0/purpose",
            json!("automatic"),
        ),
        (
            "recovery-not-boolean",
            "/operations/0/allow_recovery",
            json!("yes"),
        ),
    ] {
        let mut v = configuration();
        *v.pointer_mut(pointer).unwrap() = value;
        rows.push(json!({"id":name,"valid":false,"value":v}));
    }
    for name in [
        "duplicate-history",
        "duplicate-operation",
        "unknown-grant",
        "unknown-secret-field",
    ] {
        let mut v = configuration();
        match name {
            "duplicate-history" => {
                let h = v["accounting_original_sha256s"][0].clone();
                v["accounting_original_sha256s"]
                    .as_array_mut()
                    .unwrap()
                    .push(h);
            }
            "duplicate-operation" => {
                let o = v["operations"][0].clone();
                v["operations"].as_array_mut().unwrap().push(o);
            }
            "unknown-grant" => {
                v["production_approved"] = json!(true);
            }
            _ => {
                v["operations"][0]["secret_value"] = json!("synthetic forbidden field");
            }
        }
        rows.push(json!({"id":name,"valid":false,"value":v}));
    }
    rows
}

pub fn proposal_records() -> Vec<serde_json::Value> {
    let c = configuration();
    let o = &c["operations"][0];
    let input = json!({"schema_version":1,"run_id":"run","stage":"pilot","unit_id":"pilot-unit",
        "epoch":1,"revision":0,"inputs":[],"pins":c["pins"],"mode":"live","limits":o["limits"],"library_revision":0});
    let proposal = json!({"schema_version":2,"kind":"speech","provider":"fixture","model":"fixture-model",
        "recipe_sha256":o["recipe_sha256"],"payload":{"id":"committed-request","provenance":"diagnostic",
            "file":{"path":"private/request.json","sha256":o["request_sha256"],"bytes":13,"media_type":"application/json","dimensions":null}},
        "inputs":[],"estimate":{"requests":1,"input_tokens":30,"output_tokens":20,"money_microusd":10},"timeout_ms":1000});
    let baseline = json!({"id":"exact-configured-proposal","valid":true,"configuration":c,"scope_id":"pilot-speech","input":input,"proposal":proposal});
    let mut rows = vec![baseline.clone()];
    for (name, pointer, value) in [
        ("unknown-scope", "/scope_id", json!("unconfigured")),
        ("fixture-does-not-dispatch", "/input/mode", json!("fixture")),
        ("replay-does-not-dispatch", "/input/mode", json!("replay")),
        ("wrong-stage", "/input/stage", json!("speech")),
        ("wrong-unit", "/input/unit_id", json!("other-unit")),
        (
            "wrong-profile",
            "/input/pins/profile_sha256",
            json!(digest(b"different-profile")),
        ),
        ("widened-stage", "/input/limits/requests", json!(3)),
        (
            "wrong-request",
            "/proposal/payload/file/sha256",
            json!(digest(b"different-request")),
        ),
        (
            "wrong-recipe",
            "/proposal/recipe_sha256",
            json!(digest(b"different-recipe")),
        ),
        (
            "wrong-provider",
            "/proposal/provider",
            json!("other-provider"),
        ),
        ("wrong-model", "/proposal/model", json!("other-model")),
        ("wrong-kind", "/proposal/kind", json!("review")),
        (
            "token-overflow",
            "/proposal/estimate/input_tokens",
            json!(9007199254740991u64),
        ),
        (
            "tokens-exceed-scope",
            "/proposal/estimate/input_tokens",
            json!(101),
        ),
        (
            "requests-exceed-scope",
            "/proposal/estimate/requests",
            json!(3),
        ),
        (
            "money-exceeds-scope",
            "/proposal/estimate/money_microusd",
            json!(1001),
        ),
        (
            "timeout-exceeds-scope",
            "/proposal/timeout_ms",
            json!(30001),
        ),
    ] {
        let mut v = baseline.clone();
        v["id"] = json!(name);
        v["valid"] = json!(false);
        *v.pointer_mut(pointer).unwrap() = value;
        rows.push(v);
    }
    rows
}
