#[path = "../tests/support/production_parent_cases.rs"]
mod cases;
fn main() {
    let rows = cases::records();
    for row in &rows {
        assert_eq!(
            cases::check(row).is_ok(),
            row["valid"] == true,
            "{}",
            row["id"]
        );
    }
    println!("{}", serde_json::to_string(&rows).unwrap());
}
