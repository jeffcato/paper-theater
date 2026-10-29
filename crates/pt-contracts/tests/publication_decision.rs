use pt_contracts::*;
#[path = "support/acceptance_fixture.rs"]
mod fixture;

struct Bundle {
    _f: fixture::Fixture,
    report: Vec<u8>,
    ledger: AcceptanceCoverageLedgerV1,
    checker: AcceptanceCheckerIdentity,
    allowlist: PublicExportAllowlistV1,
    catalog: Vec<u8>,
    payload: Vec<(String, Vec<u8>)>,
    private: Vec<Digest>,
}

fn entry(path: &str, bytes: &[u8], media_type: &str) -> PublicExportEntryV1 {
    PublicExportEntryV1 {
        path: path.into(),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
        media_type: media_type.into(),
    }
}

fn bundle() -> Bundle {
    bundle_scoped(AcceptanceScope::Diagnostic)
}

fn bundle_scoped(scope: AcceptanceScope) -> Bundle {
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
            entry("acceptance-report.json", &report, "application/json"),
            entry("release.json", &release, "application/json"),
            entry("catalog.json", &catalog, "application/json"),
        ],
    };
    let payload = vec![
        ("acceptance-report.json".to_string(), report.clone()),
        ("release.json".to_string(), release),
        ("catalog.json".to_string(), catalog.clone()),
    ];
    Bundle {
        _f: f,
        report,
        ledger,
        checker: acceptance_binding_checker(),
        allowlist,
        catalog,
        payload,
        private: vec![digest(b"private sentinel")],
    }
}

fn borrowed(p: &[(String, Vec<u8>)]) -> Vec<(&str, &[u8])> {
    p.iter().map(|(p, b)| (p.as_str(), b.as_slice())).collect()
}

fn decide(b: &Bundle, payload: &[(&str, &[u8])]) -> ContractResult<AcceptedPublicationV1> {
    verify_accepted_publication(
        &b.ledger,
        &b.report,
        &b.checker,
        &b.allowlist,
        &digest(&b.catalog),
        payload,
        &b.private,
    )
}

#[test]
fn genuine_execution_with_exact_payload_is_accepted() {
    let b = bundle();
    let files = borrowed(&b.payload);
    let accepted = decide(&b, &files).unwrap();
    assert_eq!(accepted.schema_version, 1);
    assert_eq!(accepted.report_sha256, b.ledger.report_sha256);
    assert_eq!(accepted.catalog_sha256, digest(&b.catalog));
    assert_eq!(accepted.file_count, 3);
}

#[test]
fn forged_report_bytes_are_rejected() {
    let b = bundle();
    let mut report = b.report.clone();
    let mid = report.len() / 2;
    report[mid] ^= 0x01;
    let files = borrowed(&b.payload);
    assert!(
        verify_accepted_publication(
            &b.ledger,
            &report,
            &b.checker,
            &b.allowlist,
            &digest(&b.catalog),
            &files,
            &b.private,
        )
        .is_err()
    );
}

#[test]
fn changed_payload_bytes_are_rejected() {
    let b = bundle();
    let mut payload = b.payload.clone();
    payload[1].1[1] ^= 0x01;
    assert!(decide(&b, &borrowed(&payload)).is_err());
}

#[test]
fn ledger_joined_to_another_execution_payload_is_stale() {
    let a = bundle();
    let c = bundle_scoped(AcceptanceScope::ProductionChapter);
    let files = borrowed(&c.payload);
    assert!(
        verify_accepted_publication(
            &a.ledger,
            &a.report,
            &a.checker,
            &c.allowlist,
            &digest(&c.catalog),
            &files,
            &a.private,
        )
        .is_err()
    );
}

#[test]
fn incomplete_coverage_is_rejected() {
    let b = bundle();
    let mut ledger = b.ledger.clone();
    ledger.rows.pop();
    let files = borrowed(&b.payload);
    assert!(
        verify_accepted_publication(
            &ledger,
            &b.report,
            &b.checker,
            &b.allowlist,
            &digest(&b.catalog),
            &files,
            &b.private,
        )
        .is_err()
    );
}

#[test]
fn partial_payload_is_rejected() {
    let b = bundle();
    let files = borrowed(&b.payload[..2]);
    assert!(decide(&b, &files).is_err());
}

#[test]
fn catalog_outside_the_payload_is_rejected() {
    let b = bundle();
    let files = borrowed(&b.payload);
    assert!(
        verify_accepted_publication(
            &b.ledger,
            &b.report,
            &b.checker,
            &b.allowlist,
            &digest(b"elsewhere"),
            &files,
            &b.private,
        )
        .is_err()
    );
}

fn load(report_sha256: Digest, checker_sha256: Digest, catalog: Digest) -> TrustedPlayerLoadV1 {
    TrustedPlayerLoadV1 {
        schema_version: 1,
        profile_id: "paper-player-r186-v1".into(),
        checker_sha256,
        acceptance_report_sha256: report_sha256,
        catalog_entry_sha256: catalog,
        release_id: "oz-1".into(),
        allow_diagnostic: true,
    }
}

#[test]
fn trusted_load_authenticates_the_accepted_ledger() {
    let b = bundle();
    let catalog = digest(&b.catalog);
    load(
        b.ledger.report_sha256.clone(),
        b.checker.sha256.clone(),
        catalog.clone(),
    )
    .authenticate_acceptance(&b.ledger, &catalog)
    .unwrap();
}

#[test]
fn trusted_load_rejects_forged_bindings() {
    let b = bundle();
    let catalog = digest(&b.catalog);
    assert!(
        load(
            digest(b"other-report"),
            b.checker.sha256.clone(),
            catalog.clone()
        )
        .authenticate_acceptance(&b.ledger, &catalog)
        .is_err()
    );
    assert!(
        load(
            b.ledger.report_sha256.clone(),
            b.checker.sha256.clone(),
            digest(b"other-catalog"),
        )
        .authenticate_acceptance(&b.ledger, &catalog)
        .is_err()
    );
}
