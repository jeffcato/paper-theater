#[path = "../tests/support/acceptance_coverage_cases.rs"]
mod cases;
#[path = "../tests/support/acceptance_fixture.rs"]
mod fixture;
fn main() {
    println!("{}", serde_json::to_string(&cases::records()).unwrap());
}
