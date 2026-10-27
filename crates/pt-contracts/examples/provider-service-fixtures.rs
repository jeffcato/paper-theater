#[path = "../tests/support/provider_service_cases.rs"]
mod cases;
fn main() {
    let records = cases::cases()
        .into_iter()
        .map(|c| {
            let r = cases::check(&c);
            assert_eq!(r.is_ok(), c["valid"] == true, "{}: {r:?}", c["id"]);
            serde_json::json!({"case":c,"rust_error":r.as_ref().err(),"roundtrip":r.as_ref().ok()})
        })
        .collect::<Vec<_>>();
    println!("{}", serde_json::to_string(&records).unwrap());
}
