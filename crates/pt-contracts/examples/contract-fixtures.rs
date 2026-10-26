use pt_contracts::*;
use serde_json::{Value, json};
fn main() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/shared/two-scene-v1");
    let d: PlaybackDocument =
        serde_json::from_slice(&std::fs::read(root.join("fixture.json")).unwrap()).unwrap();
    validate_playback(&d).unwrap();
    verify_package_files(&d, &root).unwrap();
    let value = serde_json::to_value(&d).unwrap();
    let mut cases = vec![json!({"id":"saved-media-diagnostic","valid":true,"document":value})];
    let negatives: Vec<(&str, &str, Box<dyn Fn(&mut Value)>)> = vec![
        (
            "unknown-action",
            "action",
            Box::new(|v| v["timeline"]["cues"][0]["action"] = json!("hop")),
        ),
        (
            "short-dark-hold",
            "dark_hold",
            Box::new(|v| {
                let n = v["timeline"]["transitions"][0]["fade_in_start"]
                    .as_u64()
                    .unwrap();
                v["timeline"]["transitions"][0]["fade_in_start"] = json!(n - 1);
            }),
        ),
        (
            "wrong-audio-hash",
            "audio_hash",
            Box::new(|v| v["timeline"]["delivery_audio_sha256"] = json!("0".repeat(64))),
        ),
        (
            "invalid-scalar-span",
            "source_span",
            Box::new(|v| v["read_along"]["words"][0]["source_span"][1] = json!(9)),
        ),
        (
            "unsafe-sample",
            "interval",
            Box::new(|v| v["read_along"]["words"][0]["end_sample"] = json!(MAX_SAFE_INTEGER + 1)),
        ),
        (
            "missing-attestation",
            "attestation_missing",
            Box::new(|v| {
                v["acceptance"]["attestations"]
                    .as_array_mut()
                    .unwrap()
                    .remove(0);
            }),
        ),
        (
            "stale-attestation",
            "attestation_hash",
            Box::new(|v| {
                v["acceptance"]["attestations"][1]["delivery_audio_sha256"] = json!("0".repeat(64))
            }),
        ),
        (
            "repeated-terminal",
            "terminal_ambiguous",
            Box::new(|v| {
                let x = v["acceptance"]["attestations"][1]["terminal_word_ids"][0].clone();
                v["acceptance"]["attestations"][1]["terminal_word_ids"]
                    .as_array_mut()
                    .unwrap()
                    .push(x);
            }),
        ),
        (
            "ambiguous-terminal",
            "terminal_ambiguous",
            Box::new(|v| v["acceptance"]["attestations"][1]["terminal_word_ids"] = json!([])),
        ),
        (
            "early-reveal",
            "reveal_early",
            Box::new(|v| v["manifest"]["reveals"][0]["not_before_sample"] = json!(0)),
        ),
        (
            "wrong-pivot",
            "registration",
            Box::new(|v| v["actors"][0]["pivot_top_left"][0] = json!(-0.1)),
        ),
        (
            "missing-word-group",
            "word_coverage",
            Box::new(|v| {
                v["read_along"]["groups"].as_array_mut().unwrap().pop();
            }),
        ),
    ];
    for (id, code, change) in negatives {
        let mut v = value.clone();
        change(&mut v);
        let failure = match serde_json::from_value::<PlaybackDocument>(v.clone()) {
            Ok(d) => validate_playback(&d).unwrap_err(),
            Err(e) => ContractError {
                code: "action".into(),
                field: "timeline.cues.action".into(),
                message: e.to_string(),
            },
        };
        assert_eq!(failure.code, code, "{id}: {failure}");
        cases.push(json!({"id":id,"valid":false,"code":code,"field":failure.field,"document":v}));
    }
    let mut unknown = value.clone();
    unknown["manifest"]["reveals"][0]["not_before_sample"] = Value::Null;
    unknown["acceptance"]["attestations"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    validate_playback(&serde_json::from_value(unknown.clone()).unwrap()).unwrap();
    cases.push(json!({"id":"unknown-bound-ineligible","valid":true,"document":unknown}));
    println!("{}", serde_json::to_string(&cases).unwrap());
}
