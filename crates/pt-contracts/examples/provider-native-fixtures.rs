#[path = "../tests/support/provider_native_cases.rs"]
mod cases;
fn main() {
    let mut records = cases::records();
    for c in &mut records {
        let result = cases::check(c);
        assert_eq!(result.is_ok(), c["valid"] == true, "{}", c["id"]);
        c["rust_error"] = serde_json::to_value(result.err()).unwrap();
    }
    println!("{}", serde_json::to_string(&records).unwrap());
}
