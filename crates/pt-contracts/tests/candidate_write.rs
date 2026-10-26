use pt_contracts::CandidateWrite;
use serde_json::json;

#[test]
fn candidate_write_preserves_optional_png_dimensions_and_legacy_json() {
    let mut wire = json!({"candidate_id":"png-1","relative_path":"image.png","media_type":"image/png","max_bytes":4096,"provenance":"diagnostic","dimensions":[128,64]});
    let request: CandidateWrite = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap()["dimensions"],
        json!([128, 64])
    );
    wire.as_object_mut().unwrap().remove("dimensions");
    let legacy: CandidateWrite = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(legacy).unwrap(), wire);
}
