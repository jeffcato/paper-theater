#[path = "../tests/support/audio_fixture.rs"]
mod audio_fixture;
#[path = "../tests/support/acceptance_fixture.rs"]
mod fixture;
#[path = "../tests/support/wav_cases.rs"]
mod wav;
use pt_contracts::*;
use serde_json::{Value, json};
fn main() {
    let f = audio_fixture::saved();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 2 && args[0] == "--write-cli-fixture" {
        let out = std::path::Path::new(&args[1]);
        let package = out.join("package");
        std::fs::create_dir_all(&package).unwrap();
        // Copy only the frozen diagnostic public allowlist; private document/inputs stay separate.
        for file in f
            .doc
            .manifest
            .entries
            .files()
            .into_iter()
            .chain(f.doc.manifest.assets.iter())
            .chain(std::iter::once(&f.doc.manifest.acceptance))
        {
            let dest = package.join(&file.path);
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::copy(f.root.join(&file.path), dest).unwrap();
        }
        for name in ["inventory.json", "manifest.json"] {
            std::fs::copy(f.root.join(name), package.join(name)).unwrap();
        }
        fn private_copy(a: &std::path::Path, b: &std::path::Path) {
            std::fs::create_dir_all(b).unwrap();
            for e in std::fs::read_dir(a).unwrap() {
                let e = e.unwrap();
                if e.path().is_dir() {
                    private_copy(&e.path(), &b.join(e.file_name()))
                } else {
                    std::fs::copy(e.path(), b.join(e.file_name())).unwrap();
                }
            }
        }
        private_copy(&f.private, &out.join("private"));
        for (name, value) in [
            ("document.json", serde_json::to_value(&f.doc).unwrap()),
            ("policy.json", serde_json::to_value(&f.policy).unwrap()),
            ("evidence.json", json!([])),
        ] {
            std::fs::write(out.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
        }
        return;
    }
    let v = audio_fixture::verify(&f);
    v.report().validate(&acceptance_binding_checker()).unwrap();
    let mut cases = vec![json!({"id":"actual-saved-delivery","valid":true,"report":v.report()})];
    let mut counts = vec![];
    for case in wav::cases() {
        let f = audio_fixture::with_audio(&case.bytes, 1);
        let v = audio_fixture::verify(&f);
        assert_eq!(
            audio_fixture::audio(v.report()).outcome == ReviewOutcome::Pass,
            case.delivery,
            "{}",
            case.id
        );
        v.report().validate(&acceptance_binding_checker()).unwrap();
        cases.push(json!({"id":case.id,"valid":true,"report":v.report()}));
        counts.push(json!({"id":case.id,"native":case.native,"delivery":case.delivery}));
    }
    for field in ["frame_count", "data_bytes", "sample_rate"] {
        let mut r = cases[0]["report"].clone();
        r["delivery_audio"]["format"][field] = json!(1);
        cases.push(json!({"id":format!("forged-{field}"),"valid":false,"report":r}));
    }
    let mut r = cases[0]["report"].clone();
    r["delivery_audio"]["timeline_sha256"] = json!("0".repeat(64));
    cases.push(json!({"id":"unconsumed-timeline","valid":false,"report":r}));
    let mut r = cases[0]["report"].clone();
    r.as_object_mut().unwrap().remove("delivery_audio");
    cases.push(json!({"id":"pass-without-inspection","valid":false,"report":r}));
    let legacy: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/acceptance-report-v3.json"
    ))
    .unwrap();
    cases.push(json!({"id":"archived-v3-unchanged","valid":true,"report":legacy["report"],"checker":legacy["checker"]}));
    let mut r = legacy["report"].clone();
    r["delivery_audio"] = cases[0]["report"]["delivery_audio"].clone();
    cases.push(json!({"id":"v3-cannot-claim-inspection","valid":false,"report":r,"checker":legacy["checker"]}));
    let mut r = cases[0]["report"].clone();
    r["schema_version"] = json!(3);
    r["matrix_version"] = json!("pt-acceptance-required-v2");
    r["checker"] = legacy["checker"].clone();
    r["binding"]["checker_sha256"] = legacy["checker"]["sha256"].clone();
    r.as_object_mut().unwrap().remove("delivery_audio");
    for item in r["requirements"].as_array_mut().unwrap() {
        if item["check_id"] == "audio-format-review" {
            item["target"] = json!({"kind":"candidate"});
        }
    }
    for item in r["checks"].as_array_mut().unwrap() {
        if item["executed"] == true {
            item["executed_checker"] = legacy["checker"].clone();
        }
        if item["check_id"] == "audio-format-review" {
            item["target"] = json!({"kind":"candidate"});
        }
    }
    cases.push(json!({"id":"v3-audio-execution-remains-unimplemented","valid":false,"report":r,"checker":legacy["checker"]}));
    for c in &cases {
        let checker: AcceptanceCheckerIdentity = serde_json::from_value(
            c.get("checker")
                .cloned()
                .unwrap_or_else(|| json!(acceptance_binding_checker())),
        )
        .unwrap();
        let valid = serde_json::from_value::<AcceptanceVerificationReport>(c["report"].clone())
            .ok()
            .is_some_and(|r| r.validate(&checker).is_ok());
        assert_eq!(valid, c["valid"] == true, "{}", c["id"]);
    }
    println!(
        "{}",
        json!({"checker":acceptance_binding_checker(),"cases":cases,"inspector_cases":counts})
    );
}
