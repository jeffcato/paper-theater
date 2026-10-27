#[path = "../tests/support/source_slice_fixture.rs"]
mod fixture;
use fixture::*;
use pt_contracts::*;
use serde_json::{Value, json};
fn main() {
    let x = Fixture::new("🧭é\u{301} “前”\n", 600);
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 2 && args[0] == "--write-cli-fixture" {
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
        let out = std::path::Path::new(&args[1]);
        copy(&x.f.root, &out.join("package"));
        copy(&x.f.private, &out.join("private"));
        for (name, value) in [
            ("document.json", serde_json::to_value(&x.f.doc).unwrap()),
            ("policy.json", serde_json::to_value(&x.policy).unwrap()),
            ("evidence.json", json!([])),
        ] {
            std::fs::write(out.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
        }
        return;
    }
    let v = x.verify();
    v.report().validate(&acceptance_binding_checker()).unwrap();
    let mut cases = vec![json!({"id":"mapped-unicode-slice","valid":true,"report":v.report()})];
    let mut x = Fixture::new("prefix", 42);
    x.policy.relationship = None;
    let v = x.verify();
    assert_eq!(source(v.report()).outcome, ReviewOutcome::Missing);
    cases.push(json!({"id":"missing-mapping-stays-missing","valid":true,"report":v.report()}));
    for f in ["full_source_sha256", "relationship_sha256"] {
        let mut r = serde_json::to_value(cases[0]["report"].clone()).unwrap();
        r["source_slice"][f] = json!("0".repeat(64));
        cases.push(json!({"id":format!("forged-unconsumed-{f}"),"valid":false,"report":r}));
    }
    let mut r = cases[0]["report"].clone();
    r["source_slice"]["global_origin"] = json!(0);
    cases.push(json!({"id":"wrong-origin-summary","valid":false,"report":r}));
    let mut r = cases[0]["report"].clone();
    r["source_slice"]["full_source"] = json!({"path":"private/source.txt"});
    cases.push(json!({"id":"private-file-ref-in-summary","valid":false,"report":r}));
    let legacy: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/acceptance-report-v2.json"
    ))
    .unwrap();
    cases.push(json!({"id":"archived-v2-unmodified","valid":true,"report":legacy["report"],"checker":legacy["checker"]}));
    let mut r = legacy["report"].clone();
    r["source_slice"] = cases[0]["report"]["source_slice"].clone();
    cases.push(json!({"id":"v2-cannot-claim-mapped-execution","valid":false,"report":r,"checker":legacy["checker"]}));
    for c in &cases {
        let checker: AcceptanceCheckerIdentity = serde_json::from_value(
            c.get("checker")
                .cloned()
                .unwrap_or_else(|| json!(acceptance_binding_checker())),
        )
        .unwrap();
        let valid = serde_json::from_value::<AcceptanceVerificationReport>(c["report"].clone())
            .ok()
            .is_some_and(|r| r.validate(&checker).is_ok());
        assert_eq!(valid, c["valid"] == true, "{}", c["id"]);
    }
    let x = Fixture::new("🧭é\u{301}", 10);
    let mut records = vec![];
    for (id, wire, valid) in [
        ("relationship", json!(x.relationship), true),
        ("policy", json!(x.policy), true),
    ] {
        records.push(json!({"id":id,"kind":id,"wire":wire,"valid":valid}));
    }
    for field in ["global_origin", "local_origin", "slice_scalar_count"] {
        let mut wire = json!(x.relationship);
        wire[field] = json!(if field == "local_origin" { 1 } else { 0 });
        records.push(
            json!({"id":format!("bad-{field}"),"kind":"relationship","wire":wire,"valid":false}),
        );
    }
    let mut wire = json!(x.relationship);
    wire["relationships"] = json!([x.relationship, x.relationship]);
    records.push(
        json!({"id":"duplicate-relationship","kind":"relationship","wire":wire,"valid":false}),
    );
    for field in ["authorization", "run_id"] {
        let mut wire = json!(x.policy);
        wire[field] = json!("caller-authority");
        records.push(json!({"id":format!("policy-forbids-{field}"),"kind":"policy","wire":wire,"valid":false}));
    }
    for c in &records {
        let valid = if c["kind"] == "policy" {
            serde_json::from_value::<AcceptanceVerificationPolicyV2>(c["wire"].clone())
                .ok()
                .is_some_and(|p| p.validate().is_ok())
        } else {
            serde_json::from_value::<SourceSliceRelationshipV1>(c["wire"].clone())
                .ok()
                .is_some_and(|r| r.validate_for_policy(&x.policy).is_ok())
        };
        assert_eq!(valid, c["valid"] == true, "{}", c["id"]);
    }
    println!(
        "{}",
        json!({"checker":acceptance_binding_checker(),"cases":cases,"records":records,"expected_policy":x.policy})
    );
}
