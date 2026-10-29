use super::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn put(
    input: &mut StageInput,
    services: &mut OfflineServices,
    path: &str,
    b: Vec<u8>,
) -> ArtifactRef {
    let a = ArtifactRef {
        id: format!("committed:speech:{}", path.replace('/', ":")),
        file: FileRef {
            path: path.into(),
            sha256: digest(&b),
            bytes: b.len() as u64,
            media_type: if path.ends_with(".wav") {
                "audio/wav"
            } else {
                "application/json"
            }
            .into(),
            dimensions: None,
        },
        provenance: Provenance::Synthetic,
    };
    input.inputs.retain(|v| v.file.path != path);
    input.inputs.push(a.clone());
    services.inputs.insert(path.into(), b);
    a
}
fn value(services: &OfflineServices, path: &str) -> Value {
    parse(
        &services
            .writes
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(c, _)| c.file.path == path)
            .expect(path)
            .1,
    )
    .unwrap()
}
fn export_case(label: &str, input: &StageInput, services: &OfflineServices, out: &StageOutput) {
    let Ok(destination) = std::env::var("PT_SPEECH_EXPORT") else {
        return;
    };
    assert!(safe_path(&destination) && destination.starts_with("fixtures/generation/"));
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = project.join(&destination).join(label);
    std::fs::create_dir_all(&root).unwrap();
    let mut borrowed = BTreeMap::new();
    for name in ["saved-audio", "dialogue", "request-phase"] {
        let base = project.join("fixtures/generation/pilot-v1").join(name);
        let index: Vec<Value> =
            parse(&std::fs::read(base.join("input-index.json")).unwrap()).unwrap();
        for row in index {
            borrowed.insert(
                row["artifact"]["file"]["sha256"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                row["source"].as_str().unwrap().to_string(),
            );
        }
        for file in ["stage-input.json", "candidates/private/pilot-plan.json"] {
            let path = format!("fixtures/generation/pilot-v1/{name}/{file}");
            let b = std::fs::read(project.join(&path)).unwrap();
            borrowed.insert(digest(&b), path);
        }
    }
    std::fs::write(
        root.join("stage-input.json"),
        super::super::json(input).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("stage-output.json"),
        super::super::json(out).unwrap(),
    )
    .unwrap();
    let mut index = vec![];
    for a in &input.inputs {
        let source = if let Some(path) = borrowed.get(&a.file.sha256) {
            a.file
                .verify(&std::fs::read(project.join(path)).unwrap())
                .unwrap();
            path.clone()
        } else {
            let path = root.join("inputs").join(&a.file.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, &services.inputs[&a.file.path]).unwrap();
            format!("{destination}/{label}/inputs/{}", a.file.path)
        };
        index.push(json!({"artifact":a,"source":source}));
    }
    std::fs::write(
        root.join("input-index.json"),
        super::super::json(&index).unwrap(),
    )
    .unwrap();
    for (c, b) in services.writes.lock().unwrap().iter() {
        let path = root.join("candidates").join(&c.file.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b).unwrap();
    }
}
async fn fixture(dialogue: bool) -> (StageInput, OfflineServices) {
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let case = if dialogue { "dialogue" } else { "saved-audio" };
    let base = project.join("fixtures/generation/pilot-v1").join(case);
    let pilot_input: StageInput =
        parse(&std::fs::read(base.join("stage-input.json")).unwrap()).unwrap();
    let index: Vec<Value> = parse(&std::fs::read(base.join("input-index.json")).unwrap()).unwrap();
    let (mut input, mut services) = super::fixture();
    input.inputs = pilot_input.inputs.clone();
    input.library_revision = pilot_input.library_revision;
    services.inputs.clear();
    for row in index {
        let a: ArtifactRef = serde_json::from_value(row["artifact"].clone()).unwrap();
        services.inputs.insert(
            a.file.path,
            std::fs::read(project.join(row["source"].as_str().unwrap())).unwrap(),
        );
    }
    let plan: Value =
        parse(&std::fs::read(base.join("candidates/private/pilot-plan.json")).unwrap()).unwrap();
    let pilot = put(
        &mut input,
        &mut services,
        "private/pilot-plan.json",
        super::super::json(&plan).unwrap(),
    );
    let frozen = put(
        &mut input,
        &mut services,
        "private/speech-pilot-input.json",
        super::super::json(&pilot_input).unwrap(),
    );
    let direction: Value = parse(&services.inputs["private/direction-plan.json"]).unwrap();
    let catalog = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/voice-catalog.json")
        .unwrap()
        .clone();
    let entries=plan["performers"].as_array().unwrap().iter().map(|p|{let cast=direction["cast"].as_array().unwrap().iter().find(|c|c["proposal"]["id"]==p["speaker"]).unwrap();json!({"speaker":p["speaker"],"canonical_id":cast["proposal"]["canonical_id"],"provider":"speech-double","model":p["model"],"voice_id":p["voice_id"],"catalog":catalog,"language":p["language"],"accent":p["accent"],"baseline":p["baseline"],"reference":p["reference_audio"],"established":p["retained_binding"]["artifact"]})}).collect::<Vec<_>>();
    let bindings = put(
        &mut input,
        &mut services,
        "private/speech-bindings.json",
        super::super::json(
            &json!({"schema_version":1,"book_id":"test-book","revision":1,"entries":entries}),
        )
        .unwrap(),
    );
    let recipe = put(
        &mut input,
        &mut services,
        "private/speech-recipe.json",
        b"{\"schema_version\":1,\"kind\":\"synthetic-speech-v1\"}".to_vec(),
    );
    input.stage = Stage::Speech;
    input.mode = ExecutionMode::Fixture;
    input.pins.recipe_sha256 = recipe.file.sha256.clone();
    let ctx = json!({"schema_version":1,"book_id":"test-book","chapter_id":"chapter-1","pilot":pilot.file,"pilot_input":frozen.file,"bindings":bindings.file,"recipe":recipe.file,"speech_provider":"speech-double","transcription_provider":"transcription-double","transcription_model":"primary-transcription-double","estimate":{"requests":1,"input_tokens":100,"output_tokens":20,"money_microusd":0},"timeout_ms":30000,"limits":{"max_unit_text_bytes":32,"max_units":256,"transcription_max_wav_bytes":16777216,"transcription_max_seconds":60,"transcription_max_segments":16},"request_payloads":[],"observations":[]});
    put(
        &mut input,
        &mut services,
        "private/speech-context.json",
        super::super::json(&ctx).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    (input, services)
}
#[tokio::test]
async fn ordered_speech_units_reconstruct_all_narrator_prose_without_pilot_authorization() {
    let (input, services) = fixture(false).await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("plan", &input, &services, &out);
    let p = value(&services, "private/speech-plan.json");
    assert_eq!(p["source_complete"], true);
    assert_eq!(p["units"].as_array().unwrap().len(), 2);
    let text = p["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["text"].as_str().unwrap())
        .collect::<String>();
    assert_eq!(text, "Mira watched the rivière. The water was still.");
    assert_eq!(p["bulk_speech_approved"], false);
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "speech-bulk-authorization" && f.outcome == ReviewOutcome::Missing)
    );
    assert_eq!(
        value(&services, "private/speech/requests/u0000.json")["text"],
        "Mira watched the rivière."
    );
    assert!(out.library_updates.is_empty() && out.package_candidate.is_none());
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}
fn edit_context(
    input: &mut StageInput,
    services: &mut OfflineServices,
    f: impl FnOnce(&mut Value),
) {
    let mut c: Value = parse(&services.inputs["private/speech-context.json"]).unwrap();
    f(&mut c);
    put(
        input,
        services,
        "private/speech-context.json",
        super::super::json(&c).unwrap(),
    );
}
async fn commit_requests(input: &mut StageInput, services: &mut OfflineServices) {
    GenerationExecutor
        .execute(input.clone(), services)
        .await
        .unwrap();
    let writes = services.writes.lock().unwrap().clone();
    let mut refs = vec![];
    for (c, b) in writes {
        if c.file.path.starts_with("private/speech/requests/") {
            refs.push(put(input, services, &c.file.path, b).file);
        }
    }
    edit_context(input, services, |ctx| ctx["request_payloads"] = json!(refs));
    services.writes.lock().unwrap().clear();
}
#[tokio::test]
async fn committed_speech_payloads_propose_v2_content_but_live_mode_cannot_synthesize() {
    let (mut input, mut services) = fixture(false).await;
    commit_requests(&mut input, &mut services).await;
    input.mode = ExecutionMode::Live;
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let p: ProviderProposalV2 =
        serde_json::from_value(value(&services, "private/speech/proposals/u0000.json")).unwrap();
    p.validate().unwrap();
    assert_eq!(p.kind, OperationKind::Speech);
    assert_eq!(p.payload.file.path, "private/speech/requests/u0000.json");
    assert_eq!(
        value(&services, "private/speech-handoff.json")["dispatch_permitted"],
        false
    );
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "speech-bulk-authorization" && f.outcome == ReviewOutcome::Missing)
    );
}
fn wav(rate: u32, channels: u16, frames: u32) -> Vec<u8> {
    let block = channels * 2;
    let size = frames * block as u32;
    let mut b = vec![];
    b.extend(b"RIFF");
    b.extend((36 + size).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(channels.to_le_bytes());
    b.extend(rate.to_le_bytes());
    b.extend((rate * block as u32).to_le_bytes());
    b.extend(block.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(size.to_le_bytes());
    b.extend(vec![0; size as usize]);
    b
}
async fn saved_fixture(mixed: bool) -> (StageInput, OfflineServices) {
    let (mut input, mut services) = fixture(false).await;
    commit_requests(&mut input, &mut services).await;
    let mut refs = vec![];
    for i in 0..2 {
        let path = format!("private/speech/requests/u{i:04}.json");
        let request = input
            .inputs
            .iter()
            .find(|a| a.file.path == path)
            .unwrap()
            .clone();
        let req: Value = parse(&services.inputs[&path]).unwrap();
        let (rate, channels) = if mixed && i == 1 {
            (48000, 2)
        } else {
            (24000, 1)
        };
        let audio = put(
            &mut input,
            &mut services,
            &format!("private/speech-audio/u{i:04}.wav"),
            wav(rate, channels, 2400 + i),
        );
        let raw = json!({"schema_version":1,"provenance":"synthetic","request_sha256":request.file.sha256,"recipe_sha256":input.pins.recipe_sha256,"source_sha256":req["source_sha256"],"bindings_sha256":req["bindings"]["sha256"],"unit_id":req["unit"]["id"],"turn_id":req["unit"]["turn_id"],"span":req["unit"]["span"],"text_sha256":digest(req["text"].as_str().unwrap().as_bytes()),"speaker":req["speaker"],"model":req["model"],"voice_id":req["voice_id"],"audio":audio,"format":{"sample_rate":rate,"channels":channels,"bits_per_sample":16,"frames":2400+i}});
        let response = put(
            &mut input,
            &mut services,
            &format!("private/speech-raw/u{i:04}.json"),
            super::super::json(&raw).unwrap(),
        );
        let obs = json!({"schema_version":1,"provenance":"synthetic","request":request,"response":response,"issued":null,"result":null,"findings":[{"id":"retained-speech-adverse","field":"speech","outcome":"uncertain","message":"Synthetic silence does not prove speech fidelity.","evidence":[]}]});
        refs.push(
            put(
                &mut input,
                &mut services,
                &format!("private/speech-observations/u{i:04}.json"),
                super::super::json(&obs).unwrap(),
            )
            .file,
        );
    }
    edit_context(&mut input, &mut services, |c| {
        c["observations"] = json!(refs)
    });
    (input, services)
}
#[tokio::test]
async fn saved_native_takes_preserve_original_wavs_and_local_frame_domains() {
    let (input, services) = saved_fixture(false).await;
    let original = services.inputs.clone();
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("saved-takes", &input, &services, &out);
    let m = value(&services, "private/speech-takes.json");
    assert_eq!(m["takes"].as_array().unwrap().len(), 2);
    assert_eq!(m["takes"][0]["native"]["frames"], 2400);
    assert_eq!(m["takes"][1]["native_span"], json!([0, 2401]));
    assert_eq!(m["takes"][1]["global_narration_span"], Value::Null);
    assert_eq!(m["takes"][0]["speech_fidelity"], "missing");
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "retained-speech-adverse" && f.outcome == ReviewOutcome::Uncertain)
    );
    assert!(
        out.artifacts
            .iter()
            .all(|a| a.file.media_type == "application/json")
    );
    assert_eq!(services.inputs, original);
}
#[tokio::test]
async fn transcription_plans_use_actual_native_frames_without_expected_prose_hints() {
    let (input, services) = saved_fixture(false).await;
    GenerationExecutor.execute(input, &services).await.unwrap();
    let p = value(&services, "private/speech-transcription-plan.json");
    assert_eq!(p["batches"].as_array().unwrap().len(), 1);
    let b = &p["batches"][0];
    assert_eq!(b["frames"], 4801);
    assert_eq!(b["segments"][0]["native_span"], json!([0, 2400]));
    assert_eq!(b["segments"][1]["batch_span"], json!([2400, 4801]));
    assert_eq!(
        b["model_payload"],
        json!({"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":null})
    );
    assert_eq!(b["materialized_audio"], Value::Null);
    assert_eq!(b["proposal"], Value::Null);
    let model = super::super::json(&b["model_payload"]).unwrap();
    assert!(!String::from_utf8(model).unwrap().contains("Mira"));
    assert_eq!(p["measured_word_intervals"], Value::Null);
}
#[tokio::test]
async fn incompatible_native_formats_never_share_offsets_or_get_silently_resampled() {
    let (input, services) = saved_fixture(true).await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("mixed-native", &input, &services, &out);
    let p = value(&services, "private/speech-transcription-plan.json");
    let batches = p["batches"].as_array().unwrap();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0]["sample_rate"], 24000);
    assert_eq!(batches[1]["sample_rate"], 48000);
    assert_eq!(batches[1]["channels"], 2);
    assert_eq!(batches[1]["segments"][0]["batch_span"], json!([0, 2401]));
    assert_eq!(
        value(&services, "private/speech-takes.json")["takes"][1]["global_narration_span"],
        Value::Null
    );
}
#[tokio::test]
async fn genuine_dialogue_and_narrator_attribution_remain_in_exact_source_order() {
    let (input, services) = fixture(true).await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("dialogue", &input, &services, &out);
    let p = value(&services, "private/speech-plan.json");
    let units = p["units"].as_array().unwrap();
    assert_eq!(units.len(), 3);
    assert_eq!(
        units
            .iter()
            .map(|u| u["text"].as_str().unwrap())
            .collect::<String>(),
        "Mira said, “Open the gate.” Mira moved to the gate."
    );
    assert_eq!(units[0]["speaker"], "narrator");
    assert_eq!(units[1]["speaker"], "mira");
    assert_eq!(units[1]["kind"], "dialogue");
    assert_eq!(units[2]["speaker"], "narrator");
    assert_eq!(
        value(&services, "private/speech/requests/u0001.json")["text"],
        "“Open the gate.”"
    );
}
#[tokio::test]
async fn unsupported_byte_or_unit_limits_keep_every_source_character_unresolved() {
    for count in [false, true] {
        let (mut input, mut services) = fixture(false).await;
        edit_context(&mut input, &mut services, |c| {
            if count {
                c["limits"]["max_units"] = json!(1);
            } else {
                c["limits"]["max_unit_text_bytes"] = json!(2);
            }
        });
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        let p = value(&services, "private/speech-plan.json");
        let text = p["units"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u["text"].as_str().unwrap())
            .collect::<String>();
        assert_eq!(text, "Mira watched the rivière. The water was still.");
        assert_eq!(p["source_complete"], true);
        assert!(
            out.artifacts
                .iter()
                .all(|a| !a.file.path.starts_with("private/speech/requests/"))
        );
        assert!(
            out.findings
                .iter()
                .any(|f| f.id == "speech-unit-unresolved" || f.id == "speech-unit-limit")
        );
    }
}
fn edit_bindings(
    input: &mut StageInput,
    services: &mut OfflineServices,
    f: impl FnOnce(&mut Value),
) {
    let mut b: Value = parse(&services.inputs["private/speech-bindings.json"]).unwrap();
    f(&mut b);
    let a = put(
        input,
        services,
        "private/speech-bindings.json",
        super::super::json(&b).unwrap(),
    );
    edit_context(input, services, |c| c["bindings"] = json!(a.file));
}
#[tokio::test]
async fn unverified_or_mentioned_only_performers_cannot_be_recast_into_speech() {
    for field in [
        "speaker",
        "model",
        "voice_id",
        "catalog",
        "established",
        "baseline",
    ] {
        let (mut input, mut services) = fixture(false).await;
        edit_bindings(&mut input, &mut services, |b| match field {
            "catalog" => b["entries"][0]["catalog"]["file"]["sha256"] = json!("0".repeat(64)),
            "established" => b["entries"][0]["established"] = b["entries"][0]["catalog"].clone(),
            _ => {
                b["entries"][0][field] = json!(if field == "speaker" {
                    "mira"
                } else {
                    "unsupported"
                })
            }
        });
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(
            out.artifacts
                .iter()
                .all(|a| !a.file.path.starts_with("private/speech/requests/"))
        );
        assert!(
            out.findings
                .iter()
                .any(|f| f.id.starts_with("speech-voice-binding:"))
        );
    }
}
#[tokio::test]
async fn forged_pilot_approval_and_caller_authority_are_rejected() {
    let (mut input, mut services) = fixture(false).await;
    let mut p: Value = parse(&services.inputs["private/pilot-plan.json"]).unwrap();
    p["bulk_speech_approved"] = json!(true);
    let a = put(
        &mut input,
        &mut services,
        "private/pilot-plan.json",
        super::super::json(&p).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| c["pilot"] = json!(a.file));
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    assert!(services.writes.lock().unwrap().is_empty());
    for field in ["bulk_authorization", "run_id", "epoch", "expected_prose"] {
        let (mut input, mut services) = fixture(false).await;
        edit_context(&mut input, &mut services, |c| c[field] = json!(true));
        assert!(GenerationExecutor.execute(input, &services).await.is_err());
    }
}
fn mutate_raw(input: &mut StageInput, services: &mut OfflineServices, f: impl FnOnce(&mut Value)) {
    let mut raw: Value = parse(&services.inputs["private/speech-raw/u0000.json"]).unwrap();
    f(&mut raw);
    let a = put(
        input,
        services,
        "private/speech-raw/u0000.json",
        super::super::json(&raw).unwrap(),
    );
    let mut obs: Value = parse(&services.inputs["private/speech-observations/u0000.json"]).unwrap();
    obs["response"] = json!(a);
    let a = put(
        input,
        services,
        "private/speech-observations/u0000.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(input, services, |c| c["observations"][0] = json!(a.file));
}
#[tokio::test]
async fn exact_raw_source_turn_voice_recipe_request_and_native_frames_fail_closed() {
    for field in [
        "source_sha256",
        "recipe_sha256",
        "bindings_sha256",
        "request_sha256",
        "unit_id",
        "turn_id",
        "span",
        "text_sha256",
        "speaker",
        "model",
        "voice_id",
        "audio",
        "format",
        "unexpected",
    ] {
        let (mut input, mut services) = saved_fixture(false).await;
        mutate_raw(&mut input, &mut services, |r| match field {
            "audio" => r["audio"]["id"] = json!("wrong-audio"),
            "format" => r["format"]["frames"] = json!(1),
            "span" => r["span"] = json!([0, 1]),
            _ => {
                r[field] = json!(if field.ends_with("sha256") {
                    "0".repeat(64)
                } else {
                    "wrong".into()
                })
            }
        });
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{field}"
        );
        assert!(services.writes.lock().unwrap().is_empty(), "{field}");
    }
}
#[tokio::test]
async fn stale_source_catalog_recipe_and_committed_request_reject_without_repair_calls() {
    for path in [
        "raw-source.txt",
        "private/voice-catalog.json",
        "private/speech-recipe.json",
        "private/speech/requests/u0000.json",
        "private/speech-pilot-input.json",
    ] {
        let (mut input, mut services) = saved_fixture(false).await;
        let mut b = services.inputs[path].clone();
        b.push(b' ');
        put(&mut input, &mut services, path, b);
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{path}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn recorded_replay_preserves_all_compiled_bytes_raw_receipts_and_cache_keys() {
    let (mut input, mut services) = saved_fixture(false).await;
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let original_inputs = services.inputs.clone();
    let outputs = services.writes.lock().unwrap().clone();
    for (c, b) in &outputs {
        put(&mut input, &mut services, &c.file.path, b.clone());
    }
    input.mode = ExecutionMode::Replay;
    services.writes.lock().unwrap().clear();
    let second = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(first.cache_keys, second.cache_keys);
    for (c, b) in outputs {
        assert_eq!(
            services
                .writes
                .lock()
                .unwrap()
                .iter()
                .find(|(a, _)| a.file.path == c.file.path)
                .unwrap()
                .1,
            b,
            "{}",
            c.file.path
        );
    }
    for (p, b) in original_inputs {
        assert_eq!(services.inputs[&p], b, "{p}");
    }
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    assert!(second.library_updates.is_empty());
}
#[tokio::test]
async fn transcription_slices_respect_actual_pcm_budgets_and_preserve_cross_cut_uncertainty() {
    let (mut input, mut services) = saved_fixture(false).await;
    edit_context(&mut input, &mut services, |c| {
        c["limits"]["transcription_max_wav_bytes"] = json!(1000);
        c["limits"]["transcription_max_segments"] = json!(2);
    });
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("bounded-transcription", &input, &services, &out);
    let p = value(&services, "private/speech-transcription-plan.json");
    let mut ranges = BTreeMap::<String, Vec<(u64, u64)>>::new();
    for b in p["batches"].as_array().unwrap() {
        assert!(b["wav_bytes_if_materialized"].as_u64().unwrap() <= 1000);
        assert!(b["segments"].as_array().unwrap().len() <= 2);
        for s in b["segments"].as_array().unwrap() {
            ranges
                .entry(s["audio"]["file"]["path"].as_str().unwrap().into())
                .or_default()
                .push((
                    s["native_span"][0].as_u64().unwrap(),
                    s["native_span"][1].as_u64().unwrap(),
                ));
        }
    }
    for (path, r) in ranges {
        let mut end = 0;
        for (a, b) in r {
            assert_eq!(a, end);
            assert!(b > a);
            end = b;
        }
        assert_eq!(end, if path.contains("0000") { 2400 } else { 2401 });
    }
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "speech-transcription-cross-cut")
    );
    assert_eq!(p["measured_word_intervals"], Value::Null);
}
#[tokio::test]
async fn transcription_fanout_limits_keep_remaining_native_ranges_explicitly_unplanned() {
    let (mut input, mut services) = saved_fixture(false).await;
    edit_context(&mut input, &mut services, |c| {
        c["limits"]["transcription_max_wav_bytes"] = json!(46);
        c["limits"]["transcription_max_batches"] = json!(1);
    });
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("unplanned", &input, &services, &out);
    let p = value(&services, "private/speech-transcription-plan.json");
    assert_eq!(p["batches"].as_array().unwrap().len(), 1);
    assert_eq!(p["batches"][0]["frames"], 1);
    assert_eq!(p["unplanned_audio"].as_array().unwrap().len(), 2);
    assert_eq!(p["unplanned_audio"][0]["native_span"], json!([1, 2400]));
    assert_eq!(p["unplanned_audio"][1]["native_span"], json!([0, 2401]));
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "speech-transcription-plan-limit")
    );
}
fn issued_records(input: &StageInput, services: &mut OfflineServices, fault: &str) -> Value {
    let p: ProviderProposalV2 =
        serde_json::from_value(value(services, "private/speech/proposals/u0000.json")).unwrap();
    let rows: Value = parse(include_bytes!(
        "../../../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap();
    let mut op: ProviderOperationV2 = serde_json::from_value(
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "free-money-request")
            .unwrap()["wire"]
            .clone(),
    )
    .unwrap();
    op.context.provider = p.provider.clone();
    op.identity.run_id = input.run_id.clone();
    op.identity.epoch = input.epoch;
    op.identity.operation_id = "speech-saved-operation".into();
    for w in &mut op.quota_windows {
        match &mut w.scope {
            ProviderQuotaScope::Provider { provider }
            | ProviderQuotaScope::Project { provider, .. } => *provider = p.provider.clone(),
            ProviderQuotaScope::Model {
                provider, model, ..
            } => {
                *provider = p.provider.clone();
                *model = p.model.clone();
            }
        }
    }
    op.operation = ProviderOperation {
        attempt_id: "speech-saved-attempt".into(),
        unit_id: input.unit_id.clone(),
        kind: p.kind,
        provider: p.provider,
        model: p.model,
        recipe_sha256: p.recipe_sha256,
        request_sha256: p.payload.file.sha256.clone(),
        payload: p.payload,
        inputs: p.inputs,
        estimate: p.estimate,
        timeout_ms: p.timeout_ms,
    };
    let mut obs: Value = parse(&services.inputs["private/speech-observations/u0000.json"]).unwrap();
    let response: ArtifactRef = serde_json::from_value(obs["response"].clone()).unwrap();
    let raw: Value = parse(&services.inputs[&response.file.path]).unwrap();
    let audio: ArtifactRef = serde_json::from_value(raw["audio"].clone()).unwrap();
    let charge = QuotaAmounts {
        money_microusd: 0,
        requests: 1,
        tokens: 120,
        repairs: 0,
    };
    let mut r = ProviderResultV2 {
        schema_version: 2,
        result: ProviderResult {
            attempt_id: op.operation.attempt_id.clone(),
            completion: ProviderCompletion::Completed,
            request_sha256: op.operation.request_sha256.clone(),
            response_sha256: Some(response.file.sha256.clone()),
            raw_response: Some(response),
            outputs: vec![audio],
            usage: Some(op.operation.estimate.clone()),
            started_at: "synthetic-saved-speech".into(),
            elapsed_ms: 1,
            diagnostic: "Saved synthetic metadata; no transport or synthesis.".into(),
        },
        context: op.context.clone(),
        identity: op.identity.clone(),
        quota_windows: op.quota_windows.clone(),
        observed_usage: ObservedProviderUsage {
            requests: Some(1),
            tokens: Some(120),
            input_tokens: Some(100),
            output_tokens: Some(20),
            money_microusd: Some(0),
        },
        accounted_usage: charge.clone(),
        started_at_ms: 1000,
        recorded_at_ms: 1001,
        epoch_disposition: ProviderEpochDisposition::Current,
        retry: None,
        recovery: None,
    };
    for w in &mut r.quota_windows {
        w.accounted = charge.clone();
    }
    match fault {
        "issued-model" => op.operation.model = "wrong-model".into(),
        "issued-epoch" => op.identity.epoch += 1,
        "result-request" => r.result.request_sha256 = "0".repeat(64),
        "result-audio" => r.result.outputs.clear(),
        "result-raw" => r.result.response_sha256 = Some("0".repeat(64)),
        "ambiguous" => {
            r.result.completion = ProviderCompletion::Ambiguous;
            obs["response"] = Value::Null;
        }
        "obsolete" => {
            r.epoch_disposition = ProviderEpochDisposition::Obsolete;
            obs["response"] = Value::Null;
        }
        _ => {}
    }
    obs["issued"] = json!(op);
    obs["result"] = json!(r);
    obs
}
async fn recorded_fixture(fault: &str) -> (StageInput, OfflineServices) {
    let (mut input, mut services) = saved_fixture(false).await;
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let obs = issued_records(&input, &mut services, fault);
    let a = put(
        &mut input,
        &mut services,
        "private/speech-observations/u0000.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| {
        c["observations"][0] = json!(a.file)
    });
    services.writes.lock().unwrap().clear();
    (input, services)
}
#[tokio::test]
async fn saved_v2_result_identities_and_epoch_are_checked_without_submission() {
    for fault in [
        "issued-model",
        "issued-epoch",
        "result-request",
        "result-audio",
        "result-raw",
    ] {
        let (input, services) = recorded_fixture(fault).await;
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{fault}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
    let (input, services) = recorded_fixture("").await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("recorded-v2", &input, &services, &out);
    let m = value(&services, "private/speech-takes.json");
    assert_eq!(
        m["observations"][0]["record"]["result"]["accounted_usage"]["tokens"],
        120
    );
    assert_eq!(m["takes"].as_array().unwrap().len(), 2);
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
}
#[tokio::test]
async fn uncertain_saved_attempts_keep_original_charges_and_do_not_become_usable_takes() {
    for fault in ["ambiguous", "obsolete"] {
        let (input, services) = recorded_fixture(fault).await;
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        let m = value(&services, "private/speech-takes.json");
        assert_eq!(m["takes"].as_array().unwrap().len(), 1);
        assert_eq!(
            m["observations"][0]["record"]["result"]["accounted_usage"]["tokens"],
            120
        );
        assert!(
            out.findings
                .iter()
                .any(|f| f.id == "speech-saved-attempt-unresolved")
        );
    }
}
struct NoDispatch {
    base: OfflineServices,
    reads: bool,
}
#[async_trait]
impl RunServices for NoDispatch {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        assert!(self.reads, "oversized envelope read");
        self.base.read_artifact(a).await
    }
    async fn write_candidate(
        &self,
        r: CandidateWrite,
        b: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        self.base.write_candidate(r, b).await
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        self.base.check_cancelled().await
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        panic!("journal mutation")
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("legacy synthesis")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("helper dispatch")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("library query/mutation")
    }
    async fn issue_provider_v2(
        &self,
        _: ProviderProposalV2,
    ) -> Result<ProviderOperationV2, ServiceError> {
        panic!("new issuance")
    }
    async fn execute_provider_v2(
        &self,
        _: &ProviderOperationV2,
    ) -> Result<ProviderResultV2, ServiceError> {
        panic!("new synthesis")
    }
    async fn recover_provider_v2(
        &self,
        _: ProviderRecoveryRequestV2,
    ) -> Result<ProviderRecoveryResultV2, ServiceError> {
        panic!("recovery dispatch")
    }
}
#[tokio::test]
async fn fixture_replay_and_live_speech_never_dispatch_even_with_runtime_methods_available() {
    for mode in [
        ExecutionMode::Fixture,
        ExecutionMode::Replay,
        ExecutionMode::Live,
    ] {
        let (mut input, base) = recorded_fixture("").await;
        input.mode = mode;
        let out = GenerationExecutor
            .execute(input, &NoDispatch { base, reads: true })
            .await
            .unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(
            out.findings
                .iter()
                .any(|f| f.id == "speech-runtime-execution-unavailable")
        );
    }
}
#[tokio::test]
async fn oversized_metadata_wav_and_input_envelopes_fail_before_service_reads() {
    for case in ["metadata", "wav", "count", "aggregate"] {
        let (mut input, base) = fixture(false).await;
        match case {
            "metadata" => {
                let a = input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "private/speech-context.json")
                    .unwrap();
                a.file.media_type = "audio/wav".into();
                a.file.bytes = 1024 * 1024 + 1;
            }
            "wav" => {
                let a = &mut input.inputs[0];
                a.file.path = "private/large.wav".into();
                a.file.media_type = "audio/wav".into();
                a.file.bytes = 8 * 1024 * 1024 + 1;
            }
            "count" => {
                input.inputs = vec![input.inputs[0].clone(); 65];
            }
            _ => {
                input.inputs = (0..9)
                    .map(|i| {
                        let mut a = input.inputs[0].clone();
                        a.id = format!("large-{i}");
                        a.file.path = format!("private/large-{i}.wav");
                        a.file.media_type = "audio/wav".into();
                        a.file.bytes = 8 * 1024 * 1024;
                        a
                    })
                    .collect();
            }
        }
        assert!(
            GenerationExecutor
                .execute(input, &NoDispatch { base, reads: false })
                .await
                .is_err(),
            "{case}"
        );
    }
}
#[tokio::test]
async fn private_speech_records_and_transcription_plans_cannot_alias_audience_assets() {
    let (input, services) = saved_fixture(false).await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let (mut package, mut target) = super::fixture();
    let mut hashes = BTreeSet::new();
    for path in [
        "private/speech-context.json",
        "private/speech-bindings.json",
        "private/speech-raw/u0000.json",
        "private/speech-observations/u0000.json",
        "private/voice-catalog.json",
    ] {
        let a = put(
            &mut package,
            &mut target,
            path,
            services.inputs[path].clone(),
        );
        hashes.insert(a.file.sha256);
    }
    for (c, b) in services.writes.lock().unwrap().iter() {
        let a = put(&mut package, &mut target, &c.file.path, b.clone());
        hashes.insert(a.file.sha256);
    }
    let b =
        super::super::json(&value(&services, "private/speech-transcription-plan.json")).unwrap();
    let mut alias = put(&mut package, &mut target, "assets/speech-alias.svg", b);
    alias.file.media_type = "image/svg+xml".into();
    package
        .inputs
        .iter_mut()
        .find(|a| a.file.path == alias.file.path)
        .unwrap()
        .file = alias.file.clone();
    let mut doc: PlaybackDocument = parse(&target.inputs["fixture.json"]).unwrap();
    doc.manifest.assets.push(alias.file);
    put(
        &mut package,
        &mut target,
        "fixture.json",
        super::super::json(&doc).unwrap(),
    );
    let result = GenerationExecutor
        .execute(package, &target)
        .await
        .unwrap()
        .package_candidate
        .unwrap();
    assert!(
        out.artifacts
            .iter()
            .all(|a| a.file.path.starts_with("private/"))
    );
    assert!(
        result
            .payload_inventory
            .iter()
            .all(|f| !f.path.starts_with("private/") && !hashes.contains(&f.sha256))
    );
}
#[tokio::test]
async fn source_scoped_acting_splits_units_and_stays_outside_verbatim_payload_text() {
    let (mut input, mut services) = fixture(false).await;
    let mut original: StageInput =
        parse(&services.inputs["private/speech-pilot-input.json"]).unwrap();
    let mut selection: Value = parse(&services.inputs["private/pilot-selection.json"]).unwrap();
    let p = &mut selection["performers"][0]["passages"][1];
    p["acting"] = json!("still and measured");
    p["acting_support"] = json!({"span":p["span"],"quote":p["text"]});
    let a = put(
        &mut original,
        &mut services,
        "private/pilot-selection.json",
        super::super::json(&selection).unwrap(),
    );
    let mut ctx: Value = parse(&services.inputs["private/pilot-context.json"]).unwrap();
    ctx["selection"] = json!(a.file);
    ctx["request_payloads"] = json!([]);
    ctx["observations"] = json!([]);
    ctx["reviews"] = json!([]);
    put(
        &mut original,
        &mut services,
        "private/pilot-context.json",
        super::super::json(&ctx).unwrap(),
    );
    GenerationExecutor
        .execute(original.clone(), &services)
        .await
        .unwrap();
    let plan = value(&services, "private/pilot-plan.json");
    let recorded = services.writes.lock().unwrap().clone();
    for (c, b) in recorded {
        if c.file.path.starts_with("private/pilot/requests/") {
            put(&mut original, &mut services, &c.file.path, b);
        }
    }
    for a in &original.inputs {
        input.inputs.retain(|v| v.file.path != a.file.path);
        input.inputs.push(a.clone());
    }
    let pilot = put(
        &mut input,
        &mut services,
        "private/pilot-plan.json",
        super::super::json(&plan).unwrap(),
    );
    let saved = put(
        &mut input,
        &mut services,
        "private/speech-pilot-input.json",
        super::super::json(&original).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| {
        c["pilot"] = json!(pilot.file);
        c["pilot_input"] = json!(saved.file);
        c["limits"]["max_unit_text_bytes"] = json!(100);
    });
    services.writes.lock().unwrap().clear();
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("acting", &input, &services, &out);
    let p = value(&services, "private/speech-plan.json");
    assert_eq!(p["units"].as_array().unwrap().len(), 2);
    let req = value(&services, "private/speech/requests/u0001.json");
    assert_eq!(req["acting"], "still and measured");
    assert_eq!(req["text"], " The water was still.");
    assert_eq!(req["model"], "gemini-3.8-flash-tts");
    assert_eq!(req["voice_id"], "voice-n");
}
#[tokio::test]
async fn malformed_native_audio_and_missing_context_never_gain_execution_support() {
    let (mut input, mut services) = saved_fixture(false).await;
    let a = put(
        &mut input,
        &mut services,
        "private/speech-audio/u0000.wav",
        b"not PCM".to_vec(),
    );
    mutate_raw(&mut input, &mut services, |r| r["audio"] = json!(a));
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    assert!(services.writes.lock().unwrap().is_empty());
    let (mut input, services) = fixture(false).await;
    input
        .inputs
        .retain(|a| a.file.path != "private/speech-context.json");
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "speech-context-missing")
    );
}
