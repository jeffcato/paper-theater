#[path = "../tests/support/provider_cases.rs"]
mod provider_cases;
use pt_contracts::*;
use serde_json::json;
fn main() {
    let mut records = Vec::new();
    for case in provider_cases::cases() {
        let result = provider_cases::check(&case);
        assert_eq!(
            result.is_ok(),
            case["valid"] == true,
            "{}: {result:?}",
            case["id"]
        );
        let keys = if case["valid"] == true && case["kind"] == "operation" {
            serde_json::from_value::<ProviderOperationV2>(case["wire"].clone())
                .unwrap()
                .quota_windows
                .iter()
                .map(|w| w.scope.key())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        records.push(json!({"case":case,"rust_error":result.as_ref().err(),"roundtrip":result.ok(),"scope_keys":keys}));
    }
    println!("{}", serde_json::to_string(&records).unwrap());
}
