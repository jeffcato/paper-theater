#[path = "support/provider_cases.rs"]
mod provider_cases;

#[test]
fn provider_metadata_wire_cases() {
    for case in provider_cases::cases() {
        let result = provider_cases::check(&case);
        assert_eq!(
            result.is_ok(),
            case["valid"] == true,
            "{}: {result:?}",
            case["id"]
        );
        if let Some(code) = case["code"].as_str() {
            assert_eq!(result.unwrap_err().code, code, "{}", case["id"]);
        }
    }
}

#[test]
fn quota_keys_keep_tuple_boundaries_and_share_credential_capacity() {
    let key = |name: &str| {
        let case = provider_cases::cases()
            .into_iter()
            .find(|c| c["id"] == name)
            .unwrap();
        let op: pt_contracts::ProviderOperationV2 =
            serde_json::from_value(case["wire"].clone()).unwrap();
        op.validate().unwrap();
        op.quota_windows[0].scope.key()
    };
    assert_eq!(
        key("free-money-request"),
        key("second-credential-shares-project-scope")
    );
    assert_ne!(key("scope-tuple-a"), key("scope-tuple-b"));
}
