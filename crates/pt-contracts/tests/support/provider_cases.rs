use pt_contracts::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> ContractResult<T> {
    serde_json::from_value(value.clone()).map_err(|e| ContractError {
        code: "shape".into(),
        field: "record".into(),
        message: e.to_string(),
    })
}
fn encode(value: impl Serialize) -> ContractResult<Value> {
    Ok(serde_json::to_value(value).unwrap())
}
pub fn check(case: &Value) -> ContractResult<Value> {
    match case["kind"].as_str().unwrap() {
        "operation" => {
            let v: ProviderOperationV2 = decode(&case["wire"])?;
            v.validate()?;
            encode(v)
        }
        "result" => {
            let op: ProviderOperationV2 = decode(&case["operation"])?;
            let v: ProviderResultV2 = decode(&case["wire"])?;
            v.validate_for_operation(&op)?;
            encode(v)
        }
        "legacy_operation" => encode(decode::<ProviderOperation>(&case["wire"])?),
        "legacy_result" => encode(decode::<ProviderResult>(&case["wire"])?),
        _ => panic!("unknown fixture kind"),
    }
}
