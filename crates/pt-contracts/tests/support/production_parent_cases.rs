use pt_contracts::*;
use serde_json::json;
#[path = "production_configuration_cases.rs"]
mod configuration;

pub fn fixture() -> (ProductionParentPolicyV1, ProductionDerivationV1, Vec<u8>) {
    let original = configuration::configuration();
    let mut value = original.clone();
    value["accounting_scope_id"] = json!("inherited-production-ledger");
    value["source_sha256"] =
        json!(registered_source_catalog_v1().sources[1].canonical_source_sha256);
    value["compiler_sha256"] = json!(digest(b"frozen-compiler"));
    let source = &registered_source_catalog_v1().sources[1];
    value["imported_receipt_sha256s"] = json!([source.source_receipt_sha256]);
    let scope = value["operations"][0].as_object_mut().unwrap();
    scope.remove("unit_id");
    scope.remove("request_sha256");
    let policy: ProductionParentPolicyV1 = serde_json::from_value(value).unwrap();
    let bytes = serde_json::to_vec(&policy).unwrap();
    let cases = configuration::proposal_records();
    let row = &cases[0];
    let mut proposal: ProviderProposalV2 = serde_json::from_value(row["proposal"].clone()).unwrap();
    proposal.payload.file.sha256 = digest(b"{}");
    proposal.payload.file.bytes = 2;
    proposal.payload.provenance = Provenance::Autonomous;
    let source_snapshot: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/generation/source/pg36-frozen.json"
    ))
    .unwrap();
    let source_input = ArtifactRef {
        id: "original-source".into(),
        provenance: Provenance::Imported,
        file: FileRef {
            path: "private/source.txt".into(),
            sha256: source.canonical_source_sha256.clone(),
            bytes: source_snapshot["text"].as_str().unwrap().len() as u64,
            media_type: "text/plain".into(),
            dimensions: None,
        },
    };
    let mut input: StageInput = serde_json::from_value(row["input"].clone()).unwrap();
    input.inputs.push(source_input.clone());
    let derived = ProductionDerivationV1 {
        schema_version: 1,
        parent_policy: FileRef {
            path: "private/parent.json".into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        },
        scope_id: policy.operations[0].id.clone(),
        compiler_sha256: policy.compiler_sha256.clone(),
        source_sha256: policy.source_sha256.clone(),
        purpose: ProductionPurposeV1::Pilot,
        input,
        proposal,
        input_lineage: vec![ProductionInputLineageV1 {
            artifact: source_input,
            source_sha256: source.canonical_source_sha256.clone(),
            compiler_sha256: None,
            producer_receipt_sha256: source.source_receipt_sha256.clone(),
        }],
    };
    (policy, derived, bytes)
}

pub fn records() -> Vec<serde_json::Value> {
    let (policy, derived, bytes) = fixture();
    let base = json!({"id":"exact-compiler-parent", "valid":true, "policy":policy, "derived":derived,
        "policy_bytes":String::from_utf8(bytes).unwrap(), "logical_bytes":"{}"});
    let mut rows = vec![base.clone()];
    for (name, path, value) in [
        (
            "wrong-compiler",
            "/derived/compiler_sha256",
            json!(digest(b"wrong-compiler")),
        ),
        (
            "wrong-source",
            "/derived/source_sha256",
            json!(digest(b"wrong-source")),
        ),
        (
            "wrong-policy-hash",
            "/derived/parent_policy/sha256",
            json!(digest(b"wrong-policy")),
        ),
        ("changed-policy-bytes", "/policy_bytes", json!("{}")),
        ("changed-logical-bytes", "/logical_bytes", json!("{ }")),
        (
            "wrong-scope",
            "/derived/scope_id",
            json!("unregistered-scope"),
        ),
        ("wrong-purpose", "/derived/purpose", json!("bulk")),
        ("fixture-refusal", "/derived/input/mode", json!("fixture")),
        ("replay-refusal", "/derived/input/mode", json!("replay")),
        ("wrong-stage", "/derived/input/stage", json!("speech")),
        (
            "wrong-profile",
            "/derived/input/pins/profile_sha256",
            json!(digest(b"wrong-profile")),
        ),
        ("wrong-kind", "/derived/proposal/kind", json!("review")),
        (
            "wrong-model",
            "/derived/proposal/model",
            json!("wrong-model"),
        ),
        (
            "wrong-provider",
            "/derived/proposal/provider",
            json!("another-provider"),
        ),
        (
            "wrong-recipe",
            "/derived/proposal/recipe_sha256",
            json!(digest(b"wrong-recipe")),
        ),
        (
            "excess-tokens",
            "/derived/proposal/estimate/input_tokens",
            json!(101),
        ),
        (
            "excess-money",
            "/derived/proposal/estimate/money_microusd",
            json!(1001),
        ),
        (
            "excess-requests",
            "/derived/proposal/estimate/requests",
            json!(3),
        ),
        (
            "excess-stage-repairs",
            "/derived/input/limits/repairs",
            json!(1),
        ),
        (
            "diagnostic-request",
            "/derived/proposal/payload/provenance",
            json!("diagnostic"),
        ),
        ("missing-input-lineage", "/derived/input_lineage", json!([])),
        (
            "stale-input-root",
            "/derived/input_lineage/0/source_sha256",
            json!(digest(b"another-source")),
        ),
        (
            "unapproved-import",
            "/derived/input_lineage/0/producer_receipt_sha256",
            json!(digest(b"unregistered-reuse")),
        ),
        (
            "extra-derived-authority",
            "/derived/production_approved",
            json!(true),
        ),
        ("wrong-parent-version", "/policy/schema_version", json!(2)),
        (
            "missing-accounting",
            "/policy/accounting_original_sha256s",
            json!([]),
        ),
        (
            "invalid-accounting-group",
            "/policy/accounting_scope_id",
            json!(""),
        ),
        (
            "missing-authorization",
            "/policy/authorization_proof_sha256",
            json!("invalid"),
        ),
        ("wrong-parent-source", "/policy/book_id", json!("pg-55")),
        (
            "unbounded-parent",
            "/policy/limits/tokens",
            json!(9007199254740992u64),
        ),
        (
            "literal-secret",
            "/policy/operations/0/secret_env",
            json!("literal credential contents"),
        ),
        (
            "wrong-project-shape",
            "/policy/operations/0/context/project_id",
            json!(""),
        ),
        (
            "pilot-as-bulk-stage",
            "/policy/operations/0/stage",
            json!("speech"),
        ),
    ] {
        let mut row = base.clone();
        row["id"] = json!(name);
        row["valid"] = json!(false);
        if path == "/derived/production_approved" {
            row["derived"]["production_approved"] = value;
        } else {
            *row.pointer_mut(path).unwrap() = value;
        }
        if path.starts_with("/policy/") {
            let raw = serde_json::to_string(&row["policy"]).unwrap();
            row["derived"]["parent_policy"]["sha256"] = json!(digest(raw.as_bytes()));
            row["derived"]["parent_policy"]["bytes"] = json!(raw.len());
            row["policy_bytes"] = json!(raw);
        }
        rows.push(row);
    }
    let mut changed = base.clone();
    changed["id"] = json!("different-run-retains-parent-accounting");
    changed["derived"]["input"]["run_id"] = json!("next-run");
    changed["derived"]["input"]["unit_id"] = json!("next-unit");
    changed["logical_bytes"] = json!("{\"next\":true}");
    changed["derived"]["proposal"]["payload"]["file"]["sha256"] = json!(digest(b"{\"next\":true}"));
    changed["derived"]["proposal"]["payload"]["file"]["bytes"] = json!(13);
    rows.push(changed);
    let mut exact = base.clone();
    exact["id"] = json!("exact-metadata-cap-with-omitted-options");
    let derived: ProductionDerivationV1 = serde_json::from_value(exact["derived"].clone()).unwrap();
    let length = serde_json::to_vec(&derived).unwrap().len();
    let path = exact["derived"]["parent_policy"]["path"].as_str().unwrap();
    exact["derived"]["parent_policy"]["path"] = json!("p".repeat(path.len() + 65536 - length));
    fn omit_nulls(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(values) => {
                values.retain(|_, v| !v.is_null());
                for v in values.values_mut() {
                    omit_nulls(v)
                }
            }
            serde_json::Value::Array(values) => {
                for v in values {
                    omit_nulls(v)
                }
            }
            _ => {}
        }
    }
    omit_nulls(&mut exact["derived"]);
    rows.push(exact.clone());
    exact["id"] = json!("metadata-cap-plus-one-with-omitted-options");
    exact["valid"] = json!(false);
    let path = exact["derived"]["parent_policy"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    exact["derived"]["parent_policy"]["path"] = json!(format!("{path}p"));
    rows.push(exact);
    rows
}

pub fn check(row: &serde_json::Value) -> ContractResult<()> {
    let bad_shape = |_| ContractError {
        code: "shape".into(),
        field: "parent".into(),
        message: "invalid typed parent/derivation".into(),
    };
    let policy: ProductionParentPolicyV1 =
        serde_json::from_value(row["policy"].clone()).map_err(bad_shape)?;
    let derived: ProductionDerivationV1 =
        serde_json::from_value(row["derived"].clone()).map_err(bad_shape)?;
    policy.validate_derivation(
        &derived,
        row["policy_bytes"].as_str().unwrap().as_bytes(),
        row["logical_bytes"].as_str().unwrap().as_bytes(),
    )
}
