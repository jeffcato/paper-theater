use pt_contracts::*;
use serde_json::json;
#[path = "support/production_parent_cases.rs"]
mod cases;
use cases::fixture;

#[test]
fn cross_language_parent_cases_preserve_exact_bytes_and_scope() {
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
fn frozen_compiler_derives_a_new_exact_request_without_wildcard_or_accounting_reset() {
    let (policy, mut derived, bytes) = fixture();
    policy.validate_derivation(&derived, &bytes, b"{}").unwrap();
    let group = policy.accounting_scope_id.clone();
    derived.input.run_id = "another-run".into();
    derived.input.unit_id = "another-unit".into();
    derived.proposal.payload.file.sha256 = digest(b"{\"next\":true}");
    derived.proposal.payload.file.bytes = 13;
    policy
        .validate_derivation(&derived, &bytes, b"{\"next\":true}")
        .unwrap();
    assert_eq!(policy.accounting_scope_id, group);
}

#[test]
fn another_compiler_cannot_derive_under_the_parent() {
    let (policy, mut derived, bytes) = fixture();
    derived.compiler_sha256 = digest(b"unregistered-compiler");
    assert!(policy.validate_derivation(&derived, &bytes, b"{}").is_err());
}

#[test]
fn another_source_cannot_be_substituted_during_derivation() {
    let (policy, mut derived, bytes) = fixture();
    derived.source_sha256 = digest(b"different-source");
    assert!(policy.validate_derivation(&derived, &bytes, b"{}").is_err());
}

#[test]
fn a_derived_request_cannot_exceed_its_selected_scope_limits() {
    let (policy, mut derived, bytes) = fixture();
    derived.proposal.estimate.input_tokens = policy.operations[0].limits.tokens + 1;
    assert!(policy.validate_derivation(&derived, &bytes, b"{}").is_err());
}

#[test]
fn every_committed_input_needs_exact_source_bound_provenance() {
    let (policy, mut derived, bytes) = fixture();
    derived.input_lineage.clear();
    assert!(policy.validate_derivation(&derived, &bytes, b"{}").is_err());
}

#[test]
fn invalid_parent_policy_cannot_be_rebound_to_a_fresh_valid_file_hash() {
    let (policy, derived, _) = fixture();
    let base = serde_json::to_value(policy).unwrap();
    for (path, value) in [
        ("/schema_version", json!(2)),
        ("/accounting_scope_id", json!("")),
        ("/authorization_proof_sha256", json!("not-a-proof")),
        ("/accounting_original_sha256s", json!([])),
        ("/operations", json!([])),
        ("/operations/0/context/project_id", json!("")),
        ("/operations/0/secret_env", json!("literal secret contents")),
        ("/operations/0/limits/requests", json!(3)),
        ("/operations/0/stage", json!("speech")),
        ("/book_id", json!("pg-55")),
        ("/limits/tokens", json!(9007199254740992u64)),
    ] {
        let mut changed = base.clone();
        *changed.pointer_mut(path).unwrap() = value;
        let policy: ProductionParentPolicyV1 = serde_json::from_value(changed).unwrap();
        let bytes = serde_json::to_vec(&policy).unwrap();
        let mut request = derived.clone();
        request.parent_policy.sha256 = digest(&bytes);
        request.parent_policy.bytes = bytes.len() as u64;
        assert!(
            policy.validate_derivation(&request, &bytes, b"{}").is_err(),
            "{path}"
        );
    }
}

#[test]
fn diagnostic_request_bytes_cannot_be_promoted_by_a_parent_policy() {
    let (policy, mut derived, bytes) = fixture();
    derived.proposal.payload.provenance = Provenance::Diagnostic;
    assert!(policy.validate_derivation(&derived, &bytes, b"{}").is_err());
}

#[test]
fn an_empty_input_inventory_cannot_claim_source_bound_compilation() {
    let (policy, mut derived, bytes) = fixture();
    derived.input.inputs.clear();
    derived.proposal.inputs.clear();
    derived.input_lineage.clear();
    assert!(policy.validate_derivation(&derived, &bytes, b"{}").is_err());
}
