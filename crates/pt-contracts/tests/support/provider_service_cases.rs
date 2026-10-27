use pt_contracts::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

pub fn cases() -> Vec<Value> {
    let metadata: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap();
    let find = |id: &str| metadata.iter().find(|c| c["id"] == id).unwrap().clone();
    let issued = find("free-money-request")["wire"].clone();
    let observer = json!({"schema_version":2,"run_id":"run-1","epoch":1,"unit_id":"unit-1"});
    let old = &issued["operation"];
    let proposal = json!({"schema_version":2,"kind":old["kind"],"provider":old["provider"],"model":old["model"],"recipe_sha256":old["recipe_sha256"],"payload":old["payload"],"inputs":old["inputs"],"estimate":old["estimate"],"timeout_ms":old["timeout_ms"]});
    let recovered = find("recover-existing-job-without-new-attempt");
    let request = json!({"schema_version":2,"original":recovered["operation"]});
    let recovery = json!({"schema_version":2,"observer":observer,"query_id":"query-1","original":recovered["wire"],"query_completion":"completed","query_reserved_usage":{"requests":1,"tokens":20,"money_microusd":10,"repairs":0},"query_accounted_usage":{"requests":1,"tokens":20,"money_microusd":10,"repairs":0},"query_observed_usage":{"requests":null,"tokens":null,"input_tokens":null,"output_tokens":null,"money_microusd":null},"query_started_at_ms":3000,"query_recorded_at_ms":4000});
    let mut cases = vec![];
    let mut add = |name: &str, kind: &str, wire: Value, valid: bool, code: &str| {
        cases.push(json!({"id":name,"kind":kind,"wire":wire,"proposal":proposal,"observer":observer,"request":request,"valid":valid,"code":code}));
    };
    add(
        "stage-proposal-only",
        "proposal",
        proposal.clone(),
        true,
        "",
    );
    add(
        "runtime-issued-descriptor",
        "issued",
        issued.clone(),
        true,
        "",
    );
    add("runtime-observer", "observer", observer.clone(), true, "");
    add(
        "existing-job-selector",
        "recovery_request",
        request.clone(),
        true,
        "",
    );
    add(
        "separate-query-unknown-usage",
        "recovery_result",
        recovery.clone(),
        true,
        "",
    );
    for field in [
        "run_id",
        "epoch",
        "unit_id",
        "attempt_id",
        "request_sha256",
        "context",
        "project_id",
        "credential_context_id",
        "identity",
        "repair",
        "transport_retry",
        "quota_windows",
        "authorization",
        "recovery",
        "mode",
        "limits",
        "spending_authority",
    ] {
        let mut v = proposal.clone();
        v[field] = json!("caller-authority");
        add(
            &format!("proposal-forbids-{field}"),
            "proposal",
            v,
            false,
            "shape",
        );
    }
    for (field, value, code) in [
        ("schema_version", json!(1), "version"),
        ("timeout_ms", json!(0), "bounds"),
        ("provider", json!("bad provider"), "id"),
        ("model", json!(""), "id"),
        ("recipe_sha256", json!("bad"), "digest"),
    ] {
        let mut v = proposal.clone();
        v[field] = value;
        add(&format!("proposal-{field}"), "proposal", v, false, code);
    }
    let mut v = proposal.clone();
    v["estimate"]["requests"] = json!(0);
    add(
        "zero-money-still-needs-request",
        "proposal",
        v,
        false,
        "usage",
    );
    let mut v = proposal.clone();
    v["estimate"]["input_tokens"] = json!(MAX_SAFE_INTEGER);
    add(
        "proposal-unsafe-combined-tokens",
        "proposal",
        v,
        false,
        "integer",
    );
    let mut v = proposal.clone();
    v["payload"]["file"]["bytes"] = json!(1024 * 1024 + 1);
    add("proposal-large-payload", "proposal", v, false, "bounds");
    let mut v = proposal.clone();
    v["inputs"] = json!(vec![proposal["payload"].clone(); 65]);
    add(
        "proposal-large-input-inventory",
        "proposal",
        v,
        false,
        "bounds",
    );
    let mut v = proposal.clone();
    v["payload"]["file"]["dimensions"] = Value::Null;
    add("nullable-file-wire-compatibility", "proposal", v, true, "");
    for field in [
        "model",
        "recipe_sha256",
        "timeout_ms",
        "payload",
        "inputs",
        "estimate",
    ] {
        let mut v = issued.clone();
        v["operation"][field] = match field {
            "model" => json!("other-model"),
            "recipe_sha256" => json!("b".repeat(64)),
            "timeout_ms" => json!(100),
            "payload" => {
                let mut a = old["payload"].clone();
                a["id"] = json!("other-payload");
                a
            }
            "inputs" => json!([old["payload"]]),
            _ => json!({"requests":2,"input_tokens":100,"output_tokens":20,"money_microusd":0}),
        };
        add(
            &format!("issued-conflicting-{field}"),
            "issued",
            v,
            false,
            "provider_binding",
        );
    }
    let mut v = issued.clone();
    v["operation"]["request_sha256"] = json!("b".repeat(64));
    add(
        "issued-wrong-request-bytes",
        "issued",
        v,
        false,
        "provider_binding",
    );
    for (field, value) in [("run_id", json!("other-run")), ("epoch", json!(2))] {
        let mut v = issued.clone();
        v["identity"][field] = value;
        add(
            &format!("issued-wrong-{field}"),
            "issued",
            v,
            false,
            "provider_binding",
        );
    }
    let mut v = issued.clone();
    v["operation"]["unit_id"] = json!("other-unit");
    add("issued-wrong-unit", "issued", v, false, "provider_binding");
    let mut v = issued.clone();
    v["recovery"] = request["original"]["recovery"].clone();
    add(
        "recovery-descriptor-cannot-submit",
        "issued",
        v,
        false,
        "recovery",
    );
    let mut v = observer.clone();
    v["epoch"] = json!(MAX_SAFE_INTEGER + 1);
    add("observer-unsafe-epoch", "observer", v, false, "integer");
    let mut v = observer.clone();
    v["unit_id"] = json!("");
    add("observer-invalid-unit", "observer", v, false, "id");
    let mut v = request.clone();
    v["original"]["recovery"] = Value::Null;
    add(
        "recovery-needs-existing-job",
        "recovery_request",
        v,
        false,
        "recovery",
    );
    let mut v = request.clone();
    v["observer"] = observer.clone();
    add(
        "caller-cannot-issue-observer",
        "recovery_request",
        v,
        false,
        "shape",
    );
    let mut v = request.clone();
    v["original"]["recovery"]["attempt_id"] = json!("other-attempt");
    add(
        "recovery-wrong-attempt-selector",
        "recovery_request",
        v,
        false,
        "recovery",
    );
    for field in ["run_id", "unit_id"] {
        let mut v = recovery.clone();
        v["observer"][field] = json!("other-observer");
        add(
            &format!("recovery-wrong-observer-{field}"),
            "recovery_result",
            v,
            false,
            "provider_binding",
        );
    }
    let mut v = recovery.clone();
    v["observer"]["epoch"] = json!(2);
    add(
        "recovery-wrong-observer-epoch",
        "recovery_result",
        v,
        false,
        "provider_binding",
    );
    let mut v = recovery.clone();
    v["original"]["identity"]["run_id"] = json!("new-run");
    add(
        "recovery-preserves-original-run",
        "recovery_result",
        v,
        false,
        "provider_binding",
    );
    let mut v = recovery.clone();
    v["original"]["result"]["attempt_id"] = json!("new-attempt");
    add(
        "recovery-preserves-original-attempt",
        "recovery_result",
        v,
        false,
        "provider_binding",
    );
    let mut v = recovery.clone();
    v["original"]["recovery"]["provider_job_id"] = json!("different-job");
    add(
        "recovery-preserves-original-job",
        "recovery_result",
        v,
        false,
        "recovery",
    );
    for field in ["query_reserved_usage", "query_accounted_usage"] {
        let mut v = recovery.clone();
        v[field]["requests"] = json!(0);
        add(
            &format!("query-request-cost-{field}"),
            "recovery_result",
            v,
            false,
            "usage",
        );
        let mut v = recovery.clone();
        v[field]["repairs"] = json!(1);
        add(
            &format!("query-no-creative-repair-{field}"),
            "recovery_result",
            v,
            false,
            "usage",
        );
    }
    let mut v = recovery.clone();
    v["query_accounted_usage"]["money_microusd"] = json!(0);
    add(
        "query-unknown-money-retained",
        "recovery_result",
        v,
        false,
        "usage",
    );
    let mut v = recovery.clone();
    v["query_observed_usage"] = json!({"tokens":20,"input_tokens":15,"output_tokens":10});
    add(
        "query-conflicting-token-split",
        "recovery_result",
        v,
        false,
        "usage",
    );
    let mut v = recovery.clone();
    v["query_observed_usage"] = json!({"tokens":50});
    add(
        "query-overage-retained",
        "recovery_result",
        v,
        false,
        "usage",
    );
    let mut v = recovery.clone();
    v["query_completion"] = json!("ambiguous");
    v["query_observed_usage"] = json!({"requests":0,"tokens":0,"money_microusd":0});
    v["query_accounted_usage"]["tokens"] = json!(0);
    add(
        "query-ambiguous-zero-retains-reservation",
        "recovery_result",
        v,
        false,
        "usage",
    );
    let mut v = recovery.clone();
    v["query_recorded_at_ms"] = json!(2000);
    add("query-invalid-time", "recovery_result", v, false, "retry");
    let mut v = recovery.clone();
    v["query_id"] = json!("");
    add(
        "query-runtime-identity-required",
        "recovery_result",
        v,
        false,
        "id",
    );
    let mut v = recovery.clone();
    v["query_id"] = recovery["original"]["result"]["attempt_id"].clone();
    add(
        "query-cannot-reuse-original-attempt-identity",
        "recovery_result",
        v,
        false,
        "recovery",
    );
    // Observing a canceled older run must not rewrite the original result into the observer's run.
    let mut v = recovery.clone();
    v["observer"]["run_id"] = json!("new-observer-run");
    v["observer"]["unit_id"] = json!("observer-unit");
    v["original"]["epoch_disposition"] = json!("obsolete");
    let mut c = json!({"id":"new-observer-retains-obsolete-original","kind":"recovery_result","wire":v,"proposal":proposal,"observer":v["observer"],"request":request,"valid":true,"code":""});
    cases.push(c.clone());
    c["id"] = json!("unknown-query-observations-remain-omitted");
    c["wire"]["query_observed_usage"] = json!({});
    cases.push(c);
    cases
}
fn decode<T: DeserializeOwned>(v: &Value) -> ContractResult<T> {
    serde_json::from_value(v.clone()).map_err(|e| ContractError {
        code: "shape".into(),
        field: "record".into(),
        message: e.to_string(),
    })
}
fn encode(v: impl Serialize) -> ContractResult<Value> {
    Ok(serde_json::to_value(v).unwrap())
}
pub fn check(c: &Value) -> ContractResult<Value> {
    match c["kind"].as_str().unwrap() {
        "proposal" => {
            let v: ProviderProposalV2 = decode(&c["wire"])?;
            v.validate()?;
            encode(v)
        }
        "observer" => {
            let v: ProviderObserverV2 = decode(&c["wire"])?;
            v.validate()?;
            encode(v)
        }
        "issued" => {
            let v: ProviderOperationV2 = decode(&c["wire"])?;
            v.validate_for_proposal(&decode(&c["proposal"])?, &decode(&c["observer"])?)?;
            encode(v)
        }
        "recovery_request" => {
            let v: ProviderRecoveryRequestV2 = decode(&c["wire"])?;
            v.validate()?;
            encode(v)
        }
        "recovery_result" => {
            let v: ProviderRecoveryResultV2 = decode(&c["wire"])?;
            v.validate_for_request(&decode(&c["request"])?, &decode(&c["observer"])?)?;
            encode(v)
        }
        _ => panic!("unknown fixture kind"),
    }
}
