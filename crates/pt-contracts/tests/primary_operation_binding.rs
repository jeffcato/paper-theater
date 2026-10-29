use pt_contracts::*;
#[path = "support/primary_operation_cases.rs"]
mod cases;

#[test]
fn cross_language_primary_operation_cases_agree() {
    for row in cases::records() {
        assert_eq!(
            cases::check(&row).is_ok(),
            row["valid"] == true,
            "{}",
            row["id"]
        );
    }
}

#[test]
fn exact_primary_operation_binding_passes() {
    let (provenance, op, result, bytes) = cases::fixture();
    provenance
        .validate_for_operation(&op, &result, &bytes)
        .unwrap();
}

#[test]
fn forged_operation_attempt_is_rejected() {
    let (mut provenance, op, result, bytes) = cases::fixture();
    provenance.operation_attempt_id = "forged-attempt".into();
    assert!(
        provenance
            .validate_for_operation(&op, &result, &bytes)
            .is_err()
    );
}

#[test]
fn stale_operation_request_is_rejected() {
    let (mut provenance, op, result, bytes) = cases::fixture();
    provenance.operation_request_sha256 = digest(b"other-request");
    assert!(
        provenance
            .validate_for_operation(&op, &result, &bytes)
            .is_err()
    );
}

#[test]
fn substituted_recorded_response_is_rejected() {
    let (mut provenance, op, result, bytes) = cases::fixture();
    provenance.result_response_sha256 = Some(digest(b"forged-response"));
    assert!(
        provenance
            .validate_for_operation(&op, &result, &bytes)
            .is_err()
    );
}
