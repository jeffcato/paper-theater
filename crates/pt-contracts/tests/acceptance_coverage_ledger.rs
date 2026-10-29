use pt_contracts::*;
#[path = "support/acceptance_coverage_cases.rs"]
mod cases;
#[path = "support/acceptance_fixture.rs"]
mod fixture;

#[test]
fn shared_ledger_cases_reject_forged_or_incomplete_coverage() {
    for row in cases::records() {
        let checker: AcceptanceCheckerIdentity =
            serde_json::from_value(row["expected_checker"].clone()).unwrap();
        let valid = serde_json::from_value::<AcceptanceCoverageLedgerV1>(row["ledger"].clone())
            .ok()
            .is_some_and(|l| {
                l.validate_for_report_bytes(
                    row["report_bytes"].as_str().unwrap().as_bytes(),
                    &checker,
                )
                .is_ok()
            });
        assert_eq!(valid, row["valid"].as_bool().unwrap(), "{}", row["id"]);
    }
}

#[test]
fn executed_ledger_covers_every_selected_target_without_promoting_missing_verifiers() {
    let mut f = fixture::Fixture::new();
    f.policy.scope = AcceptanceScope::ProductionChapter;
    let execution = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    let ledger = execution.coverage_ledger().unwrap();
    let report = execution.report();
    assert_eq!(ledger.rows.len(), report.requirements.len());
    for (row, check) in ledger.rows.iter().zip(&report.checks) {
        assert_eq!(row.result.check_id, check.check_id);
        assert_eq!(row.result.target, check.target);
        assert_eq!(row.result.outcome, check.outcome);
        assert_eq!(row.result.executed, check.executed);
    }
    assert!(
        ledger
            .rows
            .iter()
            .any(|r| r.result.check_id == "independent-acoustic-evidence"
                && !r.result.executed
                && r.result.outcome != ReviewOutcome::Pass)
    );
    assert!(!ledger.production_approved);
    assert_eq!(
        ledger.binding.inventory_sha256,
        report.binding.inventory_sha256
    );
    assert_eq!(
        ledger.report_sha256,
        digest(&serde_json::to_vec(report).unwrap())
    );
}

#[test]
fn ledger_validation_binds_exact_report_bytes_and_preserves_adverse_results() {
    let mut f = fixture::Fixture::new();
    f.doc.acceptance.findings[0].outcome = ReviewOutcome::Fail;
    f.save();
    let execution = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    let mut ledger = execution.coverage_ledger().unwrap();
    let report_bytes = serde_json::to_vec(execution.report()).unwrap();
    let checker = acceptance_binding_checker();
    ledger
        .validate_for_report_bytes(&report_bytes, &checker)
        .unwrap();
    assert!(
        ledger
            .rows
            .iter()
            .any(|r| r.result.outcome == ReviewOutcome::Fail)
    );
    let original = ledger.clone();
    ledger.rows.pop();
    assert!(
        ledger
            .validate_for_report_bytes(&report_bytes, &checker)
            .is_err()
    );
    ledger = original.clone();
    let adverse = ledger
        .rows
        .iter_mut()
        .find(|r| r.result.outcome == ReviewOutcome::Fail)
        .unwrap();
    adverse.result.outcome = ReviewOutcome::Pass;
    assert!(
        ledger
            .validate_for_report_bytes(&report_bytes, &checker)
            .is_err()
    );
    let pretty = serde_json::to_vec_pretty(execution.report()).unwrap();
    assert!(
        original
            .validate_for_report_bytes(&pretty, &checker)
            .is_err()
    );
}

#[test]
fn ledger_cannot_be_issued_after_consumed_source_changes() {
    let f = fixture::Fixture::new();
    let execution = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    execution.coverage_ledger().unwrap();
    let source = &f
        .policy
        .inputs
        .iter()
        .find(|i| i.role == AcceptanceInputRole::Source)
        .unwrap()
        .file;
    std::fs::write(f.private.join(&source.path), b"changed source").unwrap();
    assert!(execution.coverage_ledger().is_err());
}
