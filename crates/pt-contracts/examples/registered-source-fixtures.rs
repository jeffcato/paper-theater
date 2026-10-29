#[path = "../tests/support/registered_source_cases.rs"]
mod cases;
#[path = "../tests/support/acceptance_fixture.rs"]
mod fixture;
fn main() {
    let records = cases::records();
    for c in &records {
        let report: pt_contracts::AcceptanceVerificationReport =
            serde_json::from_value(c["report"].clone()).unwrap();
        let checker = serde_json::from_value(c["checker"].clone()).unwrap();
        assert_eq!(
            report.validate(&checker).is_ok(),
            c["valid"] == true,
            "{}",
            c["id"]
        );
    }
    println!(
        "{}",
        serde_json::json!({"catalog":pt_contracts::registered_source_catalog_v1(),"catalog_sha256":pt_contracts::registered_source_catalog_sha256_v1(),"records":records})
    );
}
