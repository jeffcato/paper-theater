use super::*;
use serde_json::{Value, json};
fn put(
    input: &mut StageInput,
    services: &mut OfflineServices,
    path: &str,
    b: Vec<u8>,
) -> ArtifactRef {
    let a = ArtifactRef {
        id: format!("committed:primary:{}", path.replace('/', ":")),
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
    let Ok(destination) = std::env::var("PT_PRIMARY_EXPORT") else {
        return;
    };
    assert!(safe_path(&destination) && destination.starts_with("fixtures/generation/"));
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = project.join(&destination).join(label);
    std::fs::create_dir_all(&root).unwrap();
    let mut borrowed = BTreeMap::new();
    for case in ["saved-takes", "recorded-v2"] {
        let base = project.join("fixtures/generation/speech-v1").join(case);
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
        let p = root.join("candidates").join(&c.file.path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b).unwrap();
    }
}
fn wav(pcm: &[u8], rate: u32) -> Vec<u8> {
    let n = pcm.len() as u32;
    let mut b = vec![];
    b.extend(b"RIFF");
    b.extend((36 + n).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(rate.to_le_bytes());
    b.extend((rate * 2).to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(n.to_le_bytes());
    b.extend(pcm);
    b
}
async fn fixture() -> (StageInput, OfflineServices) {
    fixture_options(false, false).await
}
async fn fixture_options(cut: bool, large: bool) -> (StageInput, OfflineServices) {
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = project.join("fixtures/generation/speech-v1/saved-takes");
    let mut speech: StageInput =
        parse(&std::fs::read(root.join("stage-input.json")).unwrap()).unwrap();
    let index: Vec<Value> = parse(&std::fs::read(root.join("input-index.json")).unwrap()).unwrap();
    let (_, mut services) = super::fixture();
    services.inputs.clear();
    for row in index {
        let a: ArtifactRef = serde_json::from_value(row["artifact"].clone()).unwrap();
        services.inputs.insert(
            a.file.path,
            std::fs::read(project.join(row["source"].as_str().unwrap())).unwrap(),
        );
    }
    let mut observations = vec![];
    let rate = if large { 192000 } else { 24000 };
    for u in 0..2 {
        let path = format!("private/speech-audio/u{u:04}.wav");
        let frames = if large { 2250000 + u } else { 2400 + u };
        let pcm = (0..frames)
            .flat_map(|i| {
                (((i * 13 + u * 1709) % 60000) as i16)
                    .wrapping_sub(30000)
                    .to_le_bytes()
            })
            .collect::<Vec<_>>();
        let a = put(&mut speech, &mut services, &path, wav(&pcm, rate));
        let path = format!("private/speech-raw/u{u:04}.json");
        let mut raw: Value = parse(&services.inputs[&path]).unwrap();
        raw["audio"] = json!(a);
        raw["format"]["frames"] = json!(frames);
        raw["format"]["sample_rate"] = json!(rate);
        let a = put(
            &mut speech,
            &mut services,
            &path,
            super::super::json(&raw).unwrap(),
        );
        let path = format!("private/speech-observations/u{u:04}.json");
        let mut obs: Value = parse(&services.inputs[&path]).unwrap();
        obs["response"] = json!(a);
        let a = put(
            &mut speech,
            &mut services,
            &path,
            super::super::json(&obs).unwrap(),
        );
        observations.push(a.file);
    }
    let mut sctx: Value = parse(&services.inputs["private/speech-context.json"]).unwrap();
    sctx["observations"] = json!(observations);
    if cut {
        sctx["limits"]["transcription_max_wav_bytes"] = json!(1000);
    }
    let speech_context = put(
        &mut speech,
        &mut services,
        "private/speech-context.json",
        super::super::json(&sctx).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    GenerationExecutor
        .execute(speech.clone(), &services)
        .await
        .unwrap();
    let plan = value(&services, "private/speech-plan.json");
    let manifest = value(&services, "private/speech-takes.json");
    let copies = value(&services, "private/speech-transcription-plan.json");
    let mut input = speech.clone();
    let speech_input = put(
        &mut input,
        &mut services,
        "private/primary-speech-input.json",
        super::super::json(&speech).unwrap(),
    );
    let plan_ref = put(
        &mut input,
        &mut services,
        "private/speech-plan.json",
        super::super::json(&plan).unwrap(),
    );
    let manifest_ref = put(
        &mut input,
        &mut services,
        "private/speech-takes.json",
        super::super::json(&manifest).unwrap(),
    );
    let copies_ref = put(
        &mut input,
        &mut services,
        "private/speech-transcription-plan.json",
        super::super::json(&copies).unwrap(),
    );
    let batch = &copies["batches"][0];
    let mut pcm = services.inputs["private/speech-audio/u0000.wav"][44..].to_vec();
    pcm.extend(&services.inputs["private/speech-audio/u0001.wav"][44..]);
    if cut {
        pcm.truncate(batch["frames"].as_u64().unwrap() as usize * 2);
    }
    let audio = put(
        &mut input,
        &mut services,
        "private/primary-batch.wav",
        wav(&pcm, rate),
    );
    let recipe_sha = digest(b"synthetic-reference-copy-v1");
    let receipt = json!({"schema_version":1,"provenance":"synthetic","recipe":"native-pcm16-frame-copy-v1","recipe_sha256":recipe_sha,"take_manifest":manifest_ref,"copy_plan":copies_ref,"source_sha256":plan["source_sha256"],"batch_id":batch["id"],"format":{"sample_rate":batch["sample_rate"],"channels":batch["channels"],"bits_per_sample":16,"frames":batch["frames"]},"segments":batch["segments"],"unplanned_audio":copies["unplanned_audio"],"audio":audio,"sample_origin":"materialized-batch-local","narration_assembly":null,"measured_word_intervals":null,"fact_bounds":null,"model_payload":{"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":audio},"dispatch_permitted":false,"bulk_speech_approved":false,"production_approved":false});
    let receipt = put(
        &mut input,
        &mut services,
        "private/primary-copy-receipt.json",
        super::super::json(&receipt).unwrap(),
    );
    let recipe = put(
        &mut input,
        &mut services,
        "private/primary-recipe.json",
        b"{\"schema_version\":1,\"kind\":\"synthetic-unhinted-primary-v1\"}".to_vec(),
    );
    input.stage = Stage::Acoustic;
    input.mode = ExecutionMode::Fixture;
    input.pins.recipe_sha256 = recipe.file.sha256.clone();
    let context = json!({"schema_version":1,"speech_input":speech_input.file,"speech_context":speech_context.file,"speech_plan":plan_ref.file,"take_manifest":manifest_ref.file,"copy_plan":copies_ref.file,"batch_id":batch["id"],"batch_audio":audio,"copy_receipt":receipt.file,"copy_recipe_sha256":recipe_sha,"recipe":recipe.file,"provider":copies["provider"],"model":copies["model"],"estimate":{"requests":1,"input_tokens":100,"output_tokens":20,"money_microusd":0},"timeout_ms":30000,"request_payload":null,"observation":null,"limits":{"max_words":2048,"max_source_tokens":8192,"max_match_cells":2000000,"max_candidate_edges":16384}});
    put(
        &mut input,
        &mut services,
        "private/primary-transcription-context.json",
        super::super::json(&context).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    (input, services)
}
#[tokio::test]
async fn verified_distinguishable_batch_compiles_audio_only_request_without_dispatch() {
    let (input, services) = fixture().await;
    let ctx: Value = parse(&services.inputs["private/primary-transcription-context.json"]).unwrap();
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("request-phase", &input, &services, &out);
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert_eq!(
        value(&services, "private/primary/request.json"),
        json!({"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":ctx["batch_audio"]})
    );
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "primary-request-commit-pending")
    );
    assert_eq!(
        value(&services, "private/primary/normalized.json")["reveal_permissions"],
        json!([])
    );
    assert!(out.library_updates.is_empty() && out.package_candidate.is_none());
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}
fn edit_context(
    input: &mut StageInput,
    services: &mut OfflineServices,
    f: impl FnOnce(&mut Value),
) {
    let mut c: Value =
        parse(&services.inputs["private/primary-transcription-context.json"]).unwrap();
    f(&mut c);
    put(
        input,
        services,
        "private/primary-transcription-context.json",
        super::super::json(&c).unwrap(),
    );
}
async fn saved_fixture() -> (StageInput, OfflineServices) {
    let (mut input, mut services) = fixture().await;
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let req = value(&services, "private/primary/request.json");
    let request = put(
        &mut input,
        &mut services,
        "private/primary/request.json",
        super::super::json(&req).unwrap(),
    );
    let c: Value = parse(&services.inputs["private/primary-transcription-context.json"]).unwrap();
    let words = [
        ("Mira", "0s", "0s"),
        ("watched", "0s", "0.020s"),
        ("the", "0.020s", "0.040s"),
        ("rivière.", "0.040s", "0.080s"),
        ("The", "0.110s", "0.130s"),
        ("water", "0.130s", "0.150s"),
        ("was", "0.150s", "0.170s"),
        ("still.", "0.170s", "0.200s"),
    ]
    .iter()
    .map(|(w, a, b)| json!({"word":w,"startOffset":a,"endOffset":b}))
    .collect::<Vec<_>>();
    let raw = json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"audioTranscription":{"text":"Synthetic recognition fixture, not listening evidence.","words":words}}]}}],"modelVersion":c["model"]});
    let response = put(
        &mut input,
        &mut services,
        "private/primary/raw.json",
        super::super::json(&raw).unwrap(),
    );
    let obs = json!({"schema_version":1,"provenance":"synthetic","provider":c["provider"],"model":c["model"],"request":request,"audio":c["batch_audio"],"copy_recipe_sha256":c["copy_recipe_sha256"],"recipe_sha256":c["recipe"]["sha256"],"response":response,"issued":null,"result":null,"findings":[{"id":"retained-primary-adverse","field":"primary","outcome":"uncertain","message":"Synthetic observations do not prove speech fidelity.","evidence":[]}]});
    let observation = put(
        &mut input,
        &mut services,
        "private/primary/observation.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| {
        c["request_payload"] = json!(request.file);
        c["observation"] = json!(observation.file);
    });
    services.writes.lock().unwrap().clear();
    (input, services)
}
#[tokio::test]
async fn saved_raw_points_and_intervals_map_to_original_native_frames_and_source() {
    let (input, services) = saved_fixture().await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("measured", &input, &services, &out);
    let p = value(&services, "private/primary/normalized.json");
    assert_eq!(p["words"].as_array().unwrap().len(), 8);
    assert_eq!(p["words"][0]["timing_kind"], "point");
    assert_eq!(p["words"][0]["batch_samples"], json!([0, 0]));
    assert_eq!(p["words"][0]["native_samples"], json!([0, 0]));
    assert_eq!(p["words"][0]["source_span"], json!([0, 4]));
    assert_eq!(p["words"][4]["batch_samples"], json!([2640, 3120]));
    assert_eq!(p["words"][4]["native_samples"], json!([240, 720]));
    assert_eq!(p["highlight_groups"][0]["word_indices"], json!([0, 1]));
    assert_eq!(p["highlight_groups"][0]["native_samples"], json!([0, 480]));
    assert_eq!(p["reveal_permissions"], json!([]));
    assert_eq!(p["global_narration"], Value::Null);
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "retained-primary-adverse" && f.outcome == ReviewOutcome::Uncertain)
    );
    let proposal: ProviderProposalV2 =
        serde_json::from_value(value(&services, "private/primary/proposal.json")).unwrap();
    proposal.validate().unwrap();
    assert_eq!(proposal.kind, OperationKind::Transcription);
    assert_eq!(proposal.inputs.len(), 1);
    assert_eq!(proposal.inputs[0].file.path, "private/primary-batch.wav");
}
fn edit_raw(input: &mut StageInput, services: &mut OfflineServices, f: impl FnOnce(&mut Value)) {
    let mut raw: Value = parse(&services.inputs["private/primary/raw.json"]).unwrap();
    f(&mut raw);
    let a = put(
        input,
        services,
        "private/primary/raw.json",
        super::super::json(&raw).unwrap(),
    );
    let mut obs: Value = parse(&services.inputs["private/primary/observation.json"]).unwrap();
    obs["response"] = json!(a);
    let a = put(
        input,
        services,
        "private/primary/observation.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(input, services, |c| c["observation"] = json!(a.file));
}
#[tokio::test]
async fn repeated_words_remain_globally_ambiguous_even_if_one_occurrence_is_in_this_take() {
    let (mut input, mut services) = saved_fixture().await;
    edit_raw(&mut input, &mut services, |r| {
        r["candidates"][0]["content"]["parts"][0]["audioTranscription"]["words"] =
            json!([{"word":"the","startOffset":"0s","endOffset":"0.01s"}])
    });
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("ambiguity", &input, &services, &out);
    let p = value(&services, "private/primary/normalized.json");
    assert_eq!(
        p["words"][0]["global_source_candidates"],
        json!([[13, 16], [26, 29]])
    );
    assert_eq!(p["words"][0]["source_span"], Value::Null);
    assert_eq!(p["words"][0]["native_samples"], json!([0, 240]));
    assert!(p["highlight_groups"].as_array().unwrap().is_empty());
}
#[tokio::test]
async fn decimal_ceiling_points_and_subsample_intervals_keep_original_measurements() {
    let (mut input, mut services) = saved_fixture().await;
    edit_raw(&mut input, &mut services, |r| {
        r["candidates"][0]["content"]["parts"][0]["audioTranscription"]["words"] = json!([{"word":"Mira","startOffset":"0.000000001s","endOffset":"0.000000001s"},{"word":"watched","startOffset":"0.000000002s","endOffset":"0.020000001s"}])
    });
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("decimal-points", &input, &services, &out);
    let p = value(&services, "private/primary/normalized.json");
    assert_eq!(p["words"][0]["batch_samples"], json!([1, 1]));
    assert_eq!(p["words"][0]["original_offsets"]["start"], "0.000000001s");
    assert_eq!(p["words"][1]["batch_samples"], json!([1, 481]));
    assert_eq!(p["highlight_groups"][0]["word_indices"], json!([1]));
    assert_eq!(p["unresolved_points"], json!([0]));
    let (mut input, mut services) = saved_fixture().await;
    edit_raw(&mut input, &mut services, |r| {
        r["candidates"][0]["content"]["parts"][0]["audioTranscription"]["words"] =
            json!([{"word":"Mira","startOffset":"0.000000001s","endOffset":"0.000000002s"}])
    });
    GenerationExecutor.execute(input, &services).await.unwrap();
    let p = value(&services, "private/primary/normalized.json");
    assert_eq!(p["words"][0]["timing_kind"], "interval");
    assert_eq!(p["words"][0]["batch_samples"], json!([1, 1]));
    assert_eq!(p["words"][0]["native_samples"], Value::Null);
    assert!(p["highlight_groups"].as_array().unwrap().is_empty());
}
#[tokio::test]
async fn missing_and_substituted_unicode_recognition_never_creates_source_timing() {
    for missing in [false, true] {
        let (mut input, mut services) = saved_fixture().await;
        edit_raw(&mut input, &mut services, |r| {
            let w = r["candidates"][0]["content"]["parts"][0]["audioTranscription"]["words"]
                .as_array_mut()
                .unwrap();
            if missing {
                w.remove(3);
            } else {
                w[3]["word"] = json!("河");
            }
        });
        GenerationExecutor.execute(input, &services).await.unwrap();
        let p = value(&services, "private/primary/normalized.json");
        assert!(
            p["unmatched_source"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["word"] == "rivière" && s["outcome"] == "missing")
        );
        if !missing {
            assert_eq!(p["words"][3]["word"], "河");
            assert_eq!(p["words"][3]["source_span"], Value::Null);
        }
        assert_eq!(p["reveal_permissions"], json!([]));
    }
}
#[tokio::test]
async fn words_and_points_on_artificial_joins_are_not_split_or_assigned_a_native_domain() {
    for point in [false, true] {
        let (mut input, mut services) = saved_fixture().await;
        edit_raw(&mut input, &mut services, |r| {
            r["candidates"][0]["content"]["parts"][0]["audioTranscription"]["words"] = json!([{"word":"Mira","startOffset":if point{"0.1s"}else{"0.09s"},"endOffset":"0.11s"}])
        });
        if point {
            edit_raw(&mut input, &mut services, |r| {
                r["candidates"][0]["content"]["parts"][0]["audioTranscription"]["words"][0]["endOffset"] =
                    json!("0.1s")
            });
        }
        let out = GenerationExecutor
            .execute(input.clone(), &services)
            .await
            .unwrap();
        export_case("join", &input, &services, &out);
        let p = value(&services, "private/primary/normalized.json");
        assert_eq!(p["words"].as_array().unwrap().len(), 1);
        assert_eq!(p["words"][0]["native_samples"], Value::Null);
        assert_eq!(p["words"][0]["source_span"], Value::Null);
        assert!(p["highlight_groups"].as_array().unwrap().is_empty());
    }
}
fn edit_batch(
    input: &mut StageInput,
    services: &mut OfflineServices,
    f: impl FnOnce(&mut Vec<u8>),
) {
    let mut b = services.inputs["private/primary-batch.wav"].clone();
    f(&mut b);
    let a = put(input, services, "private/primary-batch.wav", b);
    let mut receipt: Value = parse(&services.inputs["private/primary-copy-receipt.json"]).unwrap();
    receipt["audio"] = json!(a);
    receipt["model_payload"]["audio"] = json!(a);
    let receipt = put(
        input,
        services,
        "private/primary-copy-receipt.json",
        super::super::json(&receipt).unwrap(),
    );
    edit_context(input, services, |c| {
        c["batch_audio"] = json!(a);
        c["copy_receipt"] = json!(receipt.file);
    });
}
#[tokio::test]
async fn distinguishable_pcm_rejects_reordered_repeated_skipped_or_wrong_format_frames() {
    for case in ["order", "repeat", "skip", "rate"] {
        let (mut input, mut services) = fixture().await;
        edit_batch(&mut input, &mut services, |b| match case {
            "order" => {
                let x = b[44..46].to_vec();
                let y = b[46..48].to_vec();
                b[44..46].copy_from_slice(&y);
                b[46..48].copy_from_slice(&x);
            }
            "repeat" => {
                let x = b[44..46].to_vec();
                b[46..48].copy_from_slice(&x);
            }
            "skip" => {
                let x = b[48..50].to_vec();
                b[46..48].copy_from_slice(&x);
            }
            _ => {
                b[24..28].copy_from_slice(&48000u32.to_le_bytes());
                b[28..32].copy_from_slice(&96000u32.to_le_bytes());
            }
        });
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{case}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn stale_copy_mappings_and_serialized_success_cannot_replace_revalidation() {
    for case in ["receipt", "plan", "manifest", "recipe", "hint"] {
        let (mut input, mut services) = fixture().await;
        let path = match case {
            "plan" => "private/speech-transcription-plan.json",
            "manifest" => "private/speech-takes.json",
            _ => "private/primary-copy-receipt.json",
        };
        let mut v: Value = parse(&services.inputs[path]).unwrap();
        match case {
            "receipt" => v["segments"].as_array_mut().unwrap().reverse(),
            "plan" => v["batches"][0]["segments"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            "manifest" => v["takes"][0]["native"]["frames"] = json!(1),
            "recipe" => v["recipe_sha256"] = json!("0".repeat(64)),
            _ => v["model_payload"]["expected_text"] = json!("Mira"),
        };
        let a = put(
            &mut input,
            &mut services,
            path,
            super::super::json(&v).unwrap(),
        );
        edit_context(&mut input, &mut services, |c| {
            c[match case {
                "plan" => "copy_plan",
                "manifest" => "take_manifest",
                _ => "copy_receipt",
            }] = json!(a.file)
        });
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{case}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn interior_native_cuts_preserve_measured_samples_but_do_not_claim_source_certainty() {
    let (mut input, mut services) = fixture_options(true, false).await;
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let request_bytes =
        super::super::json(&value(&services, "private/primary/request.json")).unwrap();
    let request = put(
        &mut input,
        &mut services,
        "private/primary/request.json",
        request_bytes,
    );
    let c: Value = parse(&services.inputs["private/primary-transcription-context.json"]).unwrap();
    let raw = json!({"modelVersion":c["model"],"candidates":[{"finishReason":"STOP","content":{"parts":[{"audioTranscription":{"words":[{"word":"Mira","startOffset":"0s","endOffset":"0.01s"}]}}]}}]});
    let response = put(
        &mut input,
        &mut services,
        "private/primary/raw.json",
        super::super::json(&raw).unwrap(),
    );
    let obs = json!({"schema_version":1,"provenance":"synthetic","provider":c["provider"],"model":c["model"],"request":request,"audio":c["batch_audio"],"copy_recipe_sha256":c["copy_recipe_sha256"],"recipe_sha256":c["recipe"]["sha256"],"response":response,"issued":null,"result":null,"findings":[]});
    let a = put(
        &mut input,
        &mut services,
        "private/primary/observation.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| {
        c["request_payload"] = json!(request.file);
        c["observation"] = json!(a.file);
    });
    services.writes.lock().unwrap().clear();
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("cut", &input, &services, &out);
    let p = value(&services, "private/primary/normalized.json");
    assert_eq!(p["words"][0]["native_samples"], json!([0, 240]));
    assert_eq!(p["words"][0]["source_span"], Value::Null);
    assert!(
        p["words"][0]["uncertainties"]
            .as_array()
            .unwrap()
            .contains(&json!("native-interior-cut-uncertainty"))
    );
}
#[tokio::test]
async fn only_the_explicit_verified_batch_role_can_exceed_eight_mib() {
    let (input, services) = fixture_options(false, true).await;
    let c: Value = parse(&services.inputs["private/primary-transcription-context.json"]).unwrap();
    assert!(c["batch_audio"]["file"]["bytes"].as_u64().unwrap() > 8 * 1024 * 1024);
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert_eq!(
        value(&services, "private/primary/normalized.json")["copy_verified"],
        true
    );
}
#[tokio::test]
async fn consumes_actual_r19_committed_batch_and_receipt_through_public_backend_apis() {
    use crate::runtime::{FixturePipeline, Runtime, Store};
    use std::sync::Arc;
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registration: Value = parse(
        &std::fs::read(project.join("fixtures/runtime/speech-v1/registration.json")).unwrap(),
    )
    .unwrap();
    let variant = registration["variants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["case"] == "recorded-v2")
        .unwrap();
    let mut pins = variant["original_input"]["pins"].clone();
    pins["recipe_sha256"] = variant["recipe_sha256"].clone();
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let request = json!({"schema_version":1,"book_id":"test-book","source_id":"chapter-1","mode":"fixture","pins":pins,"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}});
    let run = store.create("g25-r19-copy-consumption", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let runtime = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(GenerationExecutor))),
    )
    .with_saved_speech_inputs(project.clone());
    assert_eq!(
        runtime.work_once().await.unwrap_err().code,
        "stage_needs_repair"
    );
    let phases = store.speech_phases(&run).unwrap();
    let phase = phases.last().unwrap();
    let batch = store.speech_batches(&run).unwrap().remove(0);
    let rows: Vec<Value> = parse(
        &std::fs::read(project.join("fixtures/generation/speech-v1/recorded-v2/input-index.json"))
            .unwrap(),
    )
    .unwrap();
    let (_, mut services) = super::fixture();
    services.inputs.clear();
    for row in rows {
        let a: ArtifactRef = serde_json::from_value(row["artifact"].clone()).unwrap();
        services.inputs.insert(
            a.file.path,
            std::fs::read(project.join(row["source"].as_str().unwrap())).unwrap(),
        );
    }
    let original = phase.input.clone();
    for a in &original.inputs {
        a.file.verify(&services.inputs[&a.file.path]).unwrap();
    }
    let mut input = original.clone();
    let speech_context = original
        .inputs
        .iter()
        .find(|a| a.file.path == "private/speech-context.json")
        .unwrap()
        .file
        .clone();
    let speech_input = put(
        &mut input,
        &mut services,
        "private/primary-speech-input.json",
        super::super::json(&original).unwrap(),
    );
    for path in [
        "private/speech-plan.json",
        "private/speech-takes.json",
        "private/speech-transcription-plan.json",
    ] {
        let a = phase
            .artifacts
            .iter()
            .find(|a| a.file.path == path)
            .unwrap()
            .clone();
        let b = store.speech_artifact(&run, "takes", path).unwrap();
        a.file.verify(&b).unwrap();
        input.inputs.retain(|old| old.file.path != path);
        input.inputs.push(a);
        services.inputs.insert(path.into(), b);
    }
    let speech_plan = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/speech-plan.json")
        .unwrap()
        .file
        .clone();
    let take_manifest = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/speech-takes.json")
        .unwrap()
        .file
        .clone();
    let copy_plan = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/speech-transcription-plan.json")
        .unwrap()
        .file
        .clone();
    let plan: Value = parse(&services.inputs[&copy_plan.path]).unwrap();
    services.inputs.insert(
        batch.audio.file.path.clone(),
        store.speech_batch_audio(&run, &batch.id).unwrap(),
    );
    input.inputs.push(batch.audio.clone());
    let receipt_bytes = super::super::json(&batch.receipt).unwrap();
    batch.receipt_file.file.verify(&receipt_bytes).unwrap();
    services
        .inputs
        .insert(batch.receipt_file.file.path.clone(), receipt_bytes);
    input.inputs.push(batch.receipt_file.clone());
    let recipe = put(
        &mut input,
        &mut services,
        "private/primary-recipe.json",
        b"{\"schema_version\":1,\"kind\":\"synthetic-unhinted-primary-v1\"}".to_vec(),
    );
    input.stage = Stage::Acoustic;
    input.mode = ExecutionMode::Fixture;
    input.pins.recipe_sha256 = recipe.file.sha256.clone();
    let context = json!({"schema_version":1,"speech_input":speech_input.file,"speech_context":speech_context,"speech_plan":speech_plan,"take_manifest":take_manifest,"copy_plan":copy_plan,"batch_id":batch.id,"batch_audio":batch.audio,"copy_receipt":batch.receipt_file.file,"copy_recipe_sha256":batch.receipt["recipe_sha256"],"recipe":recipe.file,"provider":plan["provider"],"model":plan["model"],"estimate":{"requests":1,"input_tokens":100,"output_tokens":20,"money_microusd":0},"timeout_ms":30000,"request_payload":null,"observation":null,"limits":{"max_words":2048,"max_source_tokens":8192,"max_match_cells":2000000,"max_candidate_edges":16384}});
    put(
        &mut input,
        &mut services,
        "private/primary-transcription-context.json",
        super::super::json(&context).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("runtime-r19", &input, &services, &out);
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert_eq!(
        value(&services, "private/primary/normalized.json")["copy_verified"],
        true
    );
    assert_eq!(
        value(&services, "private/primary/request.json"),
        batch.receipt["model_payload"]
    );
    assert!(store.artifact(&batch.audio.id).is_err());
    assert!(store.catalog("test-book").unwrap().is_none());
}
#[tokio::test]
async fn refused_incomplete_empty_and_unmeasured_raw_responses_preserve_uncertainty() {
    for case in [
        "refused",
        "truncated",
        "candidates",
        "missing",
        "empty",
        "negative",
        "nan",
        "exponent",
        "precision",
        "order",
        "duration",
        "multiple-words",
    ] {
        let (mut input, mut services) = saved_fixture().await;
        edit_raw(&mut input, &mut services, |r| {
            let w = &mut r["candidates"][0]["content"]["parts"][0]["audioTranscription"]["words"];
            match case {
                "empty" => *w = json!([]),
                "negative" => w[0]["startOffset"] = json!("-0.1s"),
                "nan" => w[0]["startOffset"] = json!("NaNs"),
                "exponent" => w[0]["endOffset"] = json!("1e-2s"),
                "precision" => w[0]["endOffset"] = json!("0.0000000001s"),
                "order" => w[1]["startOffset"] = json!("0.19s"),
                "duration" => w[7]["endOffset"] = json!("0.201s"),
                "multiple-words" => w[0]["word"] = json!("Mira watched"),
                "refused" => r["candidates"][0]["finishReason"] = json!("SAFETY"),
                "truncated" => r["candidates"][0]["finishReason"] = json!("MAX_TOKENS"),
                "candidates" => {
                    let c = r["candidates"][0].clone();
                    r["candidates"].as_array_mut().unwrap().push(c);
                }
                _ => {
                    r["candidates"][0]["content"]["parts"] =
                        json!([{"text":"Pass: Mira watched the rivière."}])
                }
            }
        });
        let original = services.inputs["private/primary/raw.json"].clone();
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        let p = value(&services, "private/primary/normalized.json");
        assert!(p["words"].as_array().unwrap().is_empty(), "{case}");
        assert_ne!(p["parser_status"], "completed", "{case}");
        assert!(
            out.findings
                .iter()
                .any(|f| f.id == "primary-response-unresolved")
        );
        assert_eq!(services.inputs["private/primary/raw.json"], original);
        assert_eq!(p["reveal_permissions"], json!([]));
    }
}
#[tokio::test]
async fn malformed_original_json_is_retained_without_a_parser_or_model_fallback() {
    let (mut input, mut services) = saved_fixture().await;
    let raw = put(
        &mut input,
        &mut services,
        "private/primary/raw.json",
        b"{invalid original response".to_vec(),
    );
    let mut obs: Value = parse(&services.inputs["private/primary/observation.json"]).unwrap();
    obs["response"] = json!(raw);
    let a = put(
        &mut input,
        &mut services,
        "private/primary/observation.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| {
        c["observation"] = json!(a.file)
    });
    GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(
        value(&services, "private/primary/normalized.json")["parser_status"],
        "malformed-json"
    );
}
#[tokio::test]
async fn word_token_cell_and_occurrence_work_limits_reject_without_truncating_evidence() {
    for limit in [
        "max_words",
        "max_source_tokens",
        "max_match_cells",
        "max_candidate_edges",
    ] {
        let (mut input, mut services) = saved_fixture().await;
        edit_context(&mut input, &mut services, |c| c["limits"][limit] = json!(1));
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{limit}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn only_committed_exact_requests_and_bound_original_raw_audio_are_usable() {
    for field in [
        "request_payload",
        "provider",
        "model",
        "audio",
        "copy_recipe_sha256",
        "recipe_sha256",
        "request",
        "response",
    ] {
        let (mut input, mut services) = saved_fixture().await;
        if field == "request_payload" {
            edit_context(&mut input, &mut services, |c| {
                c["request_payload"] = Value::Null
            });
        } else {
            let mut obs: Value =
                parse(&services.inputs["private/primary/observation.json"]).unwrap();
            match field {
                "audio" | "request" | "response" => obs[field]["id"] = json!("foreign-artifact"),
                "provider" | "model" => obs[field] = json!("foreign"),
                _ => obs[field] = json!("0".repeat(64)),
            };
            let a = put(
                &mut input,
                &mut services,
                "private/primary/observation.json",
                super::super::json(&obs).unwrap(),
            );
            edit_context(&mut input, &mut services, |c| {
                c["observation"] = json!(a.file)
            });
        }
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{field}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
    let (mut input, mut services) = saved_fixture().await;
    let mut req: Value = parse(&services.inputs["private/primary/request.json"]).unwrap();
    req["expected_text"] = json!("Mira");
    let a = put(
        &mut input,
        &mut services,
        "private/primary/request.json",
        super::super::json(&req).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| {
        c["request_payload"] = json!(a.file)
    });
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
}
#[tokio::test]
async fn recorded_replay_is_byte_identical_and_never_rewrites_raw_or_receipt_inputs() {
    let (mut input, mut services) = saved_fixture().await;
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let original = services.inputs.clone();
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
    for (p, b) in original {
        assert_eq!(services.inputs[&p], b, "{p}");
    }
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    assert!(second.library_updates.is_empty());
}
fn saved_v2(input: &StageInput, services: &OfflineServices, fault: &str) -> Value {
    let p: ProviderProposalV2 =
        serde_json::from_value(value(services, "private/primary/proposal.json")).unwrap();
    let fixtures: Value = parse(include_bytes!(
        "../../../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap();
    let mut issued: ProviderOperationV2 = serde_json::from_value(
        fixtures
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == "free-money-request")
            .unwrap()["wire"]
            .clone(),
    )
    .unwrap();
    issued.context.provider = p.provider.clone();
    issued.identity.run_id = input.run_id.clone();
    issued.identity.epoch = input.epoch;
    issued.identity.operation_id = "saved-primary-operation".into();
    for w in &mut issued.quota_windows {
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
    issued.operation = ProviderOperation {
        attempt_id: "saved-primary-attempt".into(),
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
    let mut obs: Value = parse(&services.inputs["private/primary/observation.json"]).unwrap();
    let raw: ArtifactRef = serde_json::from_value(obs["response"].clone()).unwrap();
    let charge = QuotaAmounts {
        money_microusd: 0,
        requests: 1,
        tokens: 120,
        repairs: 0,
    };
    let mut result = ProviderResultV2 {
        schema_version: 2,
        result: ProviderResult {
            attempt_id: issued.operation.attempt_id.clone(),
            completion: ProviderCompletion::Completed,
            request_sha256: issued.operation.request_sha256.clone(),
            response_sha256: Some(raw.file.sha256.clone()),
            raw_response: Some(raw),
            outputs: vec![],
            usage: Some(issued.operation.estimate.clone()),
            started_at: "synthetic-original-primary".into(),
            elapsed_ms: 1,
            diagnostic: "Saved synthetic transcription metadata; no provider call.".into(),
        },
        context: issued.context.clone(),
        identity: issued.identity.clone(),
        quota_windows: issued.quota_windows.clone(),
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
    for w in &mut result.quota_windows {
        w.accounted = charge.clone();
    }
    match fault {
        "issued-model" => issued.operation.model = "foreign-model".into(),
        "issued-epoch" => issued.identity.epoch += 1,
        "issued-payload" => issued.operation.payload.id = "foreign-payload".into(),
        "result-request" => result.result.request_sha256 = "0".repeat(64),
        "result-raw" => result.result.response_sha256 = Some("0".repeat(64)),
        "result-context" => result.context.project_id = "foreign-project".into(),
        "ambiguous" => {
            result.result.completion = ProviderCompletion::Ambiguous;
            obs["response"] = Value::Null;
        }
        "obsolete" => {
            result.epoch_disposition = ProviderEpochDisposition::Obsolete;
            obs["response"] = Value::Null;
        }
        "unresolved" => {
            obs["response"] = Value::Null;
        }
        _ => {}
    }
    obs["issued"] = json!(issued);
    obs["result"] = if fault == "unresolved" {
        Value::Null
    } else {
        json!(result)
    };
    obs
}
async fn recorded_fixture(fault: &str) -> (StageInput, OfflineServices) {
    let (mut input, mut services) = saved_fixture().await;
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let obs = saved_v2(&input, &services, fault);
    let a = put(
        &mut input,
        &mut services,
        "private/primary/observation.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(&mut input, &mut services, |c| {
        c["observation"] = json!(a.file)
    });
    services.writes.lock().unwrap().clear();
    (input, services)
}
#[tokio::test]
async fn original_issued_result_scope_and_accounting_are_verified_without_new_dispatch() {
    for fault in [
        "issued-model",
        "issued-epoch",
        "issued-payload",
        "result-request",
        "result-raw",
        "result-context",
    ] {
        let (input, services) = recorded_fixture(fault).await;
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{fault}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
    let (input, services) = recorded_fixture("").await;
    GenerationExecutor.execute(input, &services).await.unwrap();
    let p = value(&services, "private/primary/normalized.json");
    assert_eq!(p["words"].as_array().unwrap().len(), 8);
    assert_eq!(p["observation"]["result"]["accounted_usage"]["tokens"], 120);
    for fault in ["ambiguous", "obsolete", "unresolved"] {
        let (input, services) = recorded_fixture(fault).await;
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        let p = value(&services, "private/primary/normalized.json");
        assert!(p["words"].as_array().unwrap().is_empty());
        assert!(!p["observation"]["issued"].is_null());
        if fault != "unresolved" {
            assert_eq!(p["observation"]["result"]["accounted_usage"]["tokens"], 120);
        }
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    }
}
struct NoDispatch {
    base: OfflineServices,
    allow_metadata_only: bool,
}
#[async_trait]
impl RunServices for NoDispatch {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        assert!(
            !self.allow_metadata_only
                || [
                    "private/primary-transcription-context.json",
                    "private/speech-context.json"
                ]
                .contains(&a.file.path.as_str()),
            "oversized payload was read"
        );
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
        panic!("legacy dispatch")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("helper dispatch")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("library mutation/query")
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
        panic!("new provider execution")
    }
    async fn recover_provider_v2(
        &self,
        _: ProviderRecoveryRequestV2,
    ) -> Result<ProviderRecoveryResultV2, ServiceError> {
        panic!("recovery dispatch")
    }
}
#[tokio::test]
async fn fixture_replay_and_live_modes_never_dispatch_even_with_available_methods() {
    for mode in [
        ExecutionMode::Fixture,
        ExecutionMode::Replay,
        ExecutionMode::Live,
    ] {
        let (mut input, base) = recorded_fixture("").await;
        input.mode = mode;
        let out = GenerationExecutor
            .execute(
                input,
                &NoDispatch {
                    base,
                    allow_metadata_only: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(out.library_updates.is_empty());
    }
}
#[tokio::test]
async fn metadata_native_batch_and_aggregate_caps_are_checked_before_payload_reads() {
    for case in [
        "metadata",
        "native",
        "batch",
        "aggregate",
        "count",
        "configured",
    ] {
        let (mut input, mut base) = fixture().await;
        match case {
            "metadata" => {
                let a = input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "private/primary-copy-receipt.json")
                    .unwrap();
                a.file.bytes = 1024 * 1024 + 1;
                a.file.media_type = "audio/wav".into();
            }
            "native" => {
                input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "private/speech-audio/u0000.wav")
                    .unwrap()
                    .file
                    .bytes = 8 * 1024 * 1024 + 1;
            }
            "batch" => {
                input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "private/primary-batch.wav")
                    .unwrap()
                    .file
                    .bytes = 16 * 1024 * 1024 + 1;
            }
            "count" => {
                input.inputs = vec![input.inputs[0].clone(); 65];
            }
            "aggregate" => {
                input.inputs = (0..9)
                    .map(|i| {
                        let mut a = input.inputs[0].clone();
                        a.id = format!("too-big-{i}");
                        a.file.path = format!("private/unselected-{i}.wav");
                        a.file.media_type = "audio/wav".into();
                        a.file.bytes = 8 * 1024 * 1024;
                        a
                    })
                    .collect();
            }
            _ => {
                let mut c: Value = parse(&base.inputs["private/speech-context.json"]).unwrap();
                c["limits"]["transcription_max_wav_bytes"] = json!(1000);
                let a = put(
                    &mut input,
                    &mut base,
                    "private/speech-context.json",
                    super::super::json(&c).unwrap(),
                );
                edit_context(&mut input, &mut base, |c| {
                    c["speech_context"] = json!(a.file)
                });
            }
        }
        assert!(
            GenerationExecutor
                .execute(
                    input,
                    &NoDispatch {
                        base,
                        allow_metadata_only: true
                    }
                )
                .await
                .is_err(),
            "{case}"
        );
    }
}
#[tokio::test]
async fn private_primary_requests_raw_and_mappings_cannot_alias_audience_assets() {
    use std::collections::BTreeSet;
    let (input, services) = saved_fixture().await;
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let (mut package, mut target) = super::fixture();
    let mut hashes = BTreeSet::new();
    for path in [
        "private/primary/raw.json",
        "private/primary/observation.json",
        "private/primary-transcription-context.json",
        "private/primary-copy-receipt.json",
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
    let b = super::super::json(&value(&services, "private/primary/normalized.json")).unwrap();
    let mut alias = put(&mut package, &mut target, "assets/primary.svg", b);
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
