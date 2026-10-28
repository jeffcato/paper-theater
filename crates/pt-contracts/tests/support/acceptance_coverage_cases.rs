use super::fixture;
use pt_contracts::*;
use serde_json::{Value, json};

pub fn records() -> Vec<Value> {
    let mut f = fixture::Fixture::new();
    f.policy.scope = AcceptanceScope::ProductionChapter;
    for (id, outcome) in [
        ("narrator-continuity", ReviewOutcome::Fail),
        ("speech-joins", ReviewOutcome::Uncertain),
    ] {
        f.doc.acceptance.findings.push(Finding {
            id: id.into(),
            field: "acceptance".into(),
            outcome,
            message: "synthetic adverse regression; no production approval".into(),
            evidence: vec![],
        });
        f.doc.acceptance.required_check_ids.push(id.into());
    }
    f.doc.acceptance.required_check_ids.sort();
    f.doc.acceptance.required_check_ids.dedup();
    f.save();
    let execution = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    let ledger = execution.coverage_ledger().unwrap();
    let report = serde_json::to_string(execution.report()).unwrap();
    let baseline = json!({"id":"executed-selected-scope","valid":true,"ledger":ledger,
        "report_bytes":report,"expected_checker":acceptance_binding_checker()});
    let mut rows = vec![baseline.clone()];
    for name in [
        "missing-row",
        "duplicate-row",
        "changed-target",
        "invented-execution",
        "downgraded-fail",
        "wrong-report-hash",
        "wrong-checker",
        "wrong-owner",
        "unknown-owner",
        "production-flag",
        "extra-grant",
        "report-whitespace",
        "duplicate-report-key",
        "escaped-duplicate-report-key",
    ] {
        let mut v = baseline.clone();
        v["id"] = json!(name);
        v["valid"] = json!(false);
        match name {
            "missing-row" => {
                v["ledger"]["rows"].as_array_mut().unwrap().pop();
            }
            "duplicate-row" => {
                let row = v["ledger"]["rows"][0].clone();
                v["ledger"]["rows"].as_array_mut().unwrap().push(row);
            }
            "changed-target" => {
                v["ledger"]["rows"][0]["result"]["target"] = json!({"kind":"source","span":[0,1]});
            }
            "invented-execution" => {
                let row = v["ledger"]["rows"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["result"]["executed"] == false)
                    .unwrap();
                row["result"]["executed"] = json!(true);
            }
            "downgraded-fail" => {
                let row = v["ledger"]["rows"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["result"]["outcome"] == "fail")
                    .unwrap();
                row["result"]["outcome"] = json!("missing");
            }
            "wrong-report-hash" => {
                v["ledger"]["report_sha256"] = json!(digest(b"different-report"))
            }
            "wrong-checker" => {
                v["expected_checker"]["sha256"] = json!(digest(b"different-checker"))
            }
            "wrong-owner" => v["ledger"]["rows"][0]["evidence_owners"] = json!(["unassigned"]),
            "unknown-owner" => v["ledger"]["rows"][0]["evidence_owners"] = json!(["automatic"]),
            "production-flag" => v["ledger"]["production_approved"] = json!(true),
            "extra-grant" => v["ledger"]["publication_grant"] = json!(true),
            "duplicate-report-key" | "escaped-duplicate-report-key" => {
                let key = if name == "duplicate-report-key" {
                    "private"
                } else {
                    "\\u0070rivate"
                };
                let bytes = v["report_bytes"].as_str().unwrap().replacen(
                    "\"private\":true",
                    &format!("\"private\":true,\"{key}\":true"),
                    1,
                );
                v["ledger"]["report_sha256"] = json!(digest(bytes.as_bytes()));
                v["report_bytes"] = json!(bytes);
            }
            _ => v["report_bytes"] = json!(format!("{}\n", v["report_bytes"].as_str().unwrap())),
        }
        rows.push(v);
    }
    rows
}
