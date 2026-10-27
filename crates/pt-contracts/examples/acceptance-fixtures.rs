#[path = "../tests/support/acceptance_fixture.rs"]
mod acceptance_fixture;
use pt_contracts::*;
use serde_json::{Value, json};
fn main() {
    let mut f = acceptance_fixture::Fixture::new();
    f.doc
        .acceptance
        .required_check_ids
        .push("complete-source-coverage".into());
    f.doc.acceptance.required_check_ids.sort();
    f.doc.acceptance.required_check_ids.dedup();
    f.save();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 2 && args[0] == "--write-cli-fixture" {
        let out = std::path::Path::new(&args[1]);
        std::fs::create_dir_all(out).unwrap();
        fn copy(a: &std::path::Path, b: &std::path::Path) {
            std::fs::create_dir_all(b).unwrap();
            for e in std::fs::read_dir(a).unwrap() {
                let e = e.unwrap();
                if e.path().is_dir() {
                    copy(&e.path(), &b.join(e.file_name()))
                } else {
                    std::fs::copy(e.path(), b.join(e.file_name())).unwrap();
                }
            }
        }
        copy(&f.root, &out.join("package"));
        copy(&f.private, &out.join("private"));
        for (name, value) in [
            ("document.json", serde_json::to_value(&f.doc).unwrap()),
            ("policy.json", serde_json::to_value(&f.policy).unwrap()),
            ("evidence.json", json!([])),
        ] {
            std::fs::write(out.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
        }
        return;
    }
    let first = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    first
        .report()
        .validate(&acceptance_binding_checker())
        .unwrap();
    let mut cases = vec![
        json!({"id":"diagnostic-executed-plumbing-only","valid":true,"report":first.report()}),
    ];
    let mut bad = serde_json::to_value(first.report()).unwrap();
    bad["production_approved"] = json!(true);
    cases.push(json!({"id":"forged-production-approval","valid":false,"report":bad}));
    for (id, field) in [
        ("missing-required-coverage", "checks"),
        ("missing-requirement-and-row", "requirements"),
    ] {
        let mut v = serde_json::to_value(first.report()).unwrap();
        v[field].as_array_mut().unwrap().remove(0);
        if field == "requirements" {
            v["checks"].as_array_mut().unwrap().remove(0);
        }
        cases.push(json!({"id":id,"valid":false,"report":v}));
    }
    let mut v = serde_json::to_value(first.report()).unwrap();
    let c = v["checks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["executed"] == false)
        .unwrap();
    c["outcome"] = json!("pass");
    c["executed"] = json!(true);
    c["executed_checker"] = serde_json::to_value(acceptance_binding_checker()).unwrap();
    cases.push(json!({"id":"forged-semantic-checker-execution","valid":false,"report":v}));
    let mut v = serde_json::to_value(first.report()).unwrap();
    v["checker"]["sha256"] = json!("0".repeat(64));
    cases.push(json!({"id":"stale-checker","valid":false,"report":v}));
    let mut v = serde_json::to_value(first.report()).unwrap();
    v["blockers"] = json!([]);
    cases.push(json!({"id":"omitted-adverse-and-pending","valid":false,"report":v}));
    let mut v = serde_json::to_value(first.report()).unwrap();
    v["raw_source"] = json!("must never be included");
    cases.push(json!({"id":"private-data-in-unrecognized-field","valid":false,"report":v}));
    f.policy.scope = AcceptanceScope::ProductionChapter;
    let chapter = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    chapter
        .report()
        .validate(&acceptance_binding_checker())
        .unwrap();
    cases.push(
        json!({"id":"production-chapter-missing-verifiers","valid":true,"report":chapter.report()}),
    );
    f.policy.scope = AcceptanceScope::AutonomousExcerpt;
    let excerpt = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    excerpt
        .report()
        .validate(&acceptance_binding_checker())
        .unwrap();
    cases
        .push(json!({"id":"excerpt-distinct-from-chapter","valid":true,"report":excerpt.report()}));
    let legacy: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/acceptance-report-v1.json"
    ))
    .unwrap();
    cases.push(json!({"id":"archived-v1-receipt-retains-identity","valid":true,"report":legacy["report"],"checker":legacy["checker"]}));
    let mut forged = serde_json::to_value(first.report()).unwrap();
    forged["schema_version"] = json!(1);
    forged["matrix_version"] = json!("pt-acceptance-required-v1");
    forged["checker"] = legacy["checker"].clone();
    forged["binding"]["checker_sha256"] = legacy["checker"]["sha256"].clone();
    for row in forged["checks"].as_array_mut().unwrap() {
        if row["executed"] == true {
            row["executed_checker"] = legacy["checker"].clone();
        }
    }
    cases.push(json!({"id":"v1-cannot-claim-source-execution","valid":false,"report":forged,"checker":legacy["checker"]}));
    let mut wrong_pair = serde_json::to_value(first.report()).unwrap();
    wrong_pair["matrix_version"] = json!("pt-acceptance-required-v1");
    cases.push(json!({"id":"mixed-report-matrix-versions","valid":false,"report":wrong_pair}));
    f.doc.timeline.scenes[0].source_span[1] -= 1;
    let file = acceptance_fixture::put(
        &f.root,
        &f.doc.manifest.entries.timeline.path,
        &serde_json::to_vec(&f.doc.timeline).unwrap(),
        "application/json",
    );
    f.doc.manifest.entries.timeline = file;
    f.save();
    let gap = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    assert!(
        gap.report()
            .checks
            .iter()
            .any(|c| c.check_id == "complete-source-coverage"
                && c.executed
                && c.outcome == ReviewOutcome::Fail
                && c.reasons.contains(&AcceptanceReason::SourceCoverageGap))
    );
    cases.push(json!({"id":"independent-scene-gap-retained","valid":true,"report":gap.report()}));
    for c in &cases {
        let checker: AcceptanceCheckerIdentity = serde_json::from_value(
            c.get("checker")
                .cloned()
                .unwrap_or_else(|| serde_json::to_value(acceptance_binding_checker()).unwrap()),
        )
        .unwrap();
        let valid = serde_json::from_value::<AcceptanceVerificationReport>(c["report"].clone())
            .ok()
            .is_some_and(|r| r.validate(&checker).is_ok());
        assert_eq!(valid, c["valid"] == Value::Bool(true), "{}", c["id"]);
    }
    println!(
        "{}",
        json!({"matrix":acceptance_check_matrix(),"checker":acceptance_binding_checker(),"cases":cases})
    );
}
