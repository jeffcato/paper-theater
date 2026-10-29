#[path = "../tests/support/acceptance_fixture.rs"]
mod fixture;
use pt_contracts::*;
use serde_json::json;

fn bundle(scope: AcceptanceScope) -> serde_json::Value {
    let mut f = fixture::Fixture::new();
    f.policy.scope = scope;
    let execution = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    let ledger = execution.coverage_ledger().unwrap();
    let report = serde_json::to_vec(execution.report()).unwrap();
    let catalog = br#"{"catalog":"diagnostic","entries":[]}"#.to_vec();
    let release = br#"{"release":"oz-1"}"#.to_vec();
    let allowlist = PublicExportAllowlistV1 {
        schema_version: 1,
        entries: vec![
            PublicExportEntryV1 {
                path: "acceptance-report.json".into(),
                sha256: digest(&report),
                bytes: report.len() as u64,
                media_type: "application/json".into(),
            },
            PublicExportEntryV1 {
                path: "release.json".into(),
                sha256: digest(&release),
                bytes: release.len() as u64,
                media_type: "application/json".into(),
            },
            PublicExportEntryV1 {
                path: "catalog.json".into(),
                sha256: digest(&catalog),
                bytes: catalog.len() as u64,
                media_type: "application/json".into(),
            },
        ],
    };
    let files = vec![
        ("acceptance-report.json", report.clone()),
        ("release.json", release),
        ("catalog.json", catalog.clone()),
    ];
    let borrowed: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(p, b)| (*p, b.as_slice()))
        .collect();
    let expected = verify_accepted_publication(
        &ledger,
        &report,
        &acceptance_binding_checker(),
        &allowlist,
        &digest(&catalog),
        &borrowed,
        &[digest(b"private sentinel")],
    )
    .unwrap();
    json!({
        "ledger": ledger,
        "report": String::from_utf8(report.clone()).unwrap(),
        "checker": acceptance_binding_checker(),
        "allowlist_json": serde_json::to_string(&allowlist).unwrap(),
        "catalog_sha256": digest(&catalog),
        "files": files.iter().map(|(p, b)| json!({"path": p, "text": String::from_utf8(b.clone()).unwrap()})).collect::<Vec<_>>(),
        "private_digest": digest(b"private sentinel"),
        "expected": expected,
    })
}

fn main() {
    println!(
        "{}",
        serde_json::to_string(&json!({
            "diagnostic": bundle(AcceptanceScope::Diagnostic),
            "chapter": bundle(AcceptanceScope::ProductionChapter),
        }))
        .unwrap()
    );
}
