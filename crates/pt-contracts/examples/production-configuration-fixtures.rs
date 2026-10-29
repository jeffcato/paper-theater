#[path = "../tests/support/production_configuration_cases.rs"]
mod cases;
fn main() {
    println!(
        "{}",
        serde_json::json!({"configurations":cases::records(),"proposals":cases::proposal_records()})
    );
}
