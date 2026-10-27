use super::*;
use serde_json::{Value, json};

fn put(
    input: &mut StageInput,
    services: &mut OfflineServices,
    path: &str,
    bytes: Vec<u8>,
) -> ArtifactRef {
    let a = ArtifactRef {
        id: format!("committed:pilot:{}", path.replace('/', ":")),
        file: FileRef {
            path: path.into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
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
    services.inputs.insert(path.into(), bytes);
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
    let Ok(destination) = std::env::var("PT_PILOT_EXPORT") else {
        return;
    };
    assert!(safe_path(&destination) && destination.starts_with("fixtures/generation/"));
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = project.join(&destination).join(label);
    std::fs::create_dir_all(&root).unwrap();
    for (name, b) in [
        ("stage-input.json", super::super::json(input).unwrap()),
        ("stage-output.json", super::super::json(out).unwrap()),
    ] {
        std::fs::write(root.join(name), b).unwrap();
    }
    let mut borrowed = BTreeMap::new();
    for name in ["dialogue", "narration-only"] {
        let dir = project.join("fixtures/generation/direction-v1").join(name);
        let index: Vec<Value> =
            parse(&std::fs::read(dir.join("input-index.json")).unwrap()).unwrap();
        for row in index {
            borrowed.insert(
                row["artifact"]["file"]["sha256"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                row["source"].as_str().unwrap().to_string(),
            );
        }
        let path = format!(
            "fixtures/generation/direction-v1/{name}/candidates/private/direction-plan.json"
        );
        let b = std::fs::read(project.join(&path)).unwrap();
        borrowed.insert(digest(&b), path);
    }
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
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let case = if dialogue {
        "dialogue"
    } else {
        "narration-only"
    };
    let base = root.join("fixtures/generation/direction-v1").join(case);
    let direction_input: StageInput =
        parse(&std::fs::read(base.join("stage-input.json")).unwrap()).unwrap();
    let index: Vec<Value> = parse(&std::fs::read(base.join("input-index.json")).unwrap()).unwrap();
    let (mut input, mut services) = super::fixture();
    input.inputs = direction_input.inputs.clone();
    services.inputs.clear();
    for row in index {
        let a: ArtifactRef = serde_json::from_value(row["artifact"].clone()).unwrap();
        services.inputs.insert(
            a.file.path,
            std::fs::read(root.join(row["source"].as_str().unwrap())).unwrap(),
        );
    }
    input.stage = Stage::Pilot;
    input.mode = ExecutionMode::Fixture;
    input.library_revision = direction_input.library_revision;
    let direction = put(
        &mut input,
        &mut services,
        "private/direction-plan.json",
        std::fs::read(base.join("candidates/private/direction-plan.json")).unwrap(),
    );
    let frozen = put(
        &mut input,
        &mut services,
        "private/pilot-direction-input.json",
        super::super::json(&direction_input).unwrap(),
    );
    let catalog=put(&mut input,&mut services,"private/voice-catalog.json",super::super::json(&json!({"schema_version":1,"provenance":"synthetic","provider":"speech-double","models":[{"id":"gemini-3.8-flash-tts","voices":[{"id":"voice-n","language":"en","accent":"catalog-default","reference":null}]}]})).unwrap());
    let plan: Value = parse(&services.inputs["private/direction-plan.json"]).unwrap();
    let text = plan["source_text"].as_str().unwrap();
    let n = text.chars().count();
    let cut = text.chars().position(|c| c == '.').unwrap() + 1;
    let chars = text.chars().collect::<Vec<_>>();
    let passages = if dialogue {
        json!([])
    } else {
        json!([{"turn_id":"t0","span":[0,cut],"text":chars[..cut].iter().collect::<String>(),"acting":"","acting_support":null},{"turn_id":"t0","span":[cut,n],"text":chars[cut..].iter().collect::<String>(),"acting":"","acting_support":null}])
    };
    let registry = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/book-registry.json")
        .unwrap()
        .file
        .clone();
    let selection=put(&mut input,&mut services,"private/pilot-selection.json",super::super::json(&json!({"schema_version":1,"provenance":"synthetic","source_sha256":plan["source_sha256"],"direction_sha256":direction.file.sha256,"catalog_sha256":catalog.file.sha256,"registry_sha256":registry.sha256,"performers":[{"speaker":"narrator","model":"gemini-3.8-flash-tts","voice_id":"voice-n","language":"en","accent":"catalog-default","baseline":"","passages":passages}]})).unwrap());
    let recipe = put(
        &mut input,
        &mut services,
        "private/pilot-recipe.json",
        b"{\"schema_version\":1,\"kind\":\"synthetic-pilot-v1\"}".to_vec(),
    );
    input.pins.recipe_sha256 = recipe.file.sha256.clone();
    let source = input
        .inputs
        .iter()
        .find(|a| a.file.path == "source.json")
        .unwrap()
        .file
        .clone();
    let ctx = json!({"schema_version":1,"book_id":"test-book","chapter_id":"chapter-1","source":source,"direction":direction.file,"direction_input":frozen.file,"registry":registry,"catalog":catalog.file,"selection":selection.file,"recipe":recipe.file,"first_audition_model":"gemini-3.8-flash-tts","speech_provider":"speech-double","review_provider":"review-double","review_model":"independent-review-double","estimate":{"requests":1,"input_tokens":100,"output_tokens":20,"money_microusd":0},"timeout_ms":30000,"observations":[],"reviews":[]});
    put(
        &mut input,
        &mut services,
        "private/pilot-context.json",
        super::super::json(&ctx).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    (input, services)
}
#[tokio::test]
async fn adjacent_verbatim_narrator_auditions_remain_private_and_unaccepted() {
    let (input, services) = fixture(false).await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("request-phase", &input, &services, &out);
    let plan = value(&services, "private/pilot-plan.json");
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert_eq!(plan["performers"][0]["speaker"], "narrator");
    assert_eq!(plan["performers"][0]["model"], "gemini-3.8-flash-tts");
    assert_eq!(
        plan["performers"][0]["passages"][0]["text"],
        "Mira watched the rivière."
    );
    assert_eq!(plan["production_approved"], false);
    assert_eq!(plan["bulk_speech_approved"], false);
    assert!(out.library_updates.is_empty() && out.package_candidate.is_none());
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "pilot-listening" && f.outcome == ReviewOutcome::Missing)
    );
    let request = value(&services, "private/pilot/requests/p000-t00.json");
    assert_eq!(request["text"], "Mira watched the rivière.");
    assert_eq!(request["style"], "");
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "pilot-request-commit-pending")
    );
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}
async fn commit_requests(input: &mut StageInput, services: &mut OfflineServices) {
    GenerationExecutor
        .execute(input.clone(), services)
        .await
        .unwrap();
    let writes = services.writes.lock().unwrap().clone();
    let mut payloads = vec![];
    for (c, b) in writes {
        if c.file.path.starts_with("private/pilot/requests/") {
            payloads.push(put(input, services, &c.file.path, b).file);
        }
    }
    edit_context(input, services, |ctx| {
        ctx["request_payloads"] = json!(payloads)
    });
    services.writes.lock().unwrap().clear();
}
#[tokio::test]
async fn committed_requests_use_v2_and_default_unavailable_retains_needs_repair() {
    let (mut input, mut services) = fixture(false).await;
    commit_requests(&mut input, &mut services).await;
    input.mode = ExecutionMode::Live;
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "pilot-provider-v2-unavailable")
    );
    assert!(
        !out.findings
            .iter()
            .any(|f| f.id == "pilot-request-commit-pending")
    );
    let p: ProviderProposalV2 =
        serde_json::from_value(value(&services, "private/pilot/proposals/p000-t00.json")).unwrap();
    p.validate().unwrap();
    assert_eq!(p.kind, OperationKind::Speech);
    assert_eq!(p.payload.file.path, "private/pilot/requests/p000-t00.json");
    let handoff = value(&services, "private/pilot-handoff.json");
    assert_eq!(handoff["caller_authority"], false);
}
fn wav(frames: u32) -> Vec<u8> {
    let mut b = vec![];
    b.extend(b"RIFF");
    b.extend((36 + frames * 2).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(24000u32.to_le_bytes());
    b.extend(48000u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend((frames * 2).to_le_bytes());
    b.extend(vec![0; frames as usize * 2]);
    b
}
fn edit_context(
    input: &mut StageInput,
    services: &mut OfflineServices,
    f: impl FnOnce(&mut Value),
) {
    let mut ctx: Value = parse(&services.inputs["private/pilot-context.json"]).unwrap();
    f(&mut ctx);
    put(
        input,
        services,
        "private/pilot-context.json",
        super::super::json(&ctx).unwrap(),
    );
}
async fn saved_audio_fixture() -> (StageInput, OfflineServices) {
    let (mut input, mut services) = fixture(false).await;
    commit_requests(&mut input, &mut services).await;
    add_saved_audio(&mut input, &mut services);
    (input, services)
}
fn add_saved_audio(mut input: &mut StageInput, mut services: &mut OfflineServices) {
    let mut observations = vec![];
    for ti in 0..2 {
        let path = format!("private/pilot/requests/p000-t{ti:02}.json");
        let request = input
            .inputs
            .iter()
            .find(|a| a.file.path == path)
            .unwrap()
            .clone();
        let req: Value = parse(&services.inputs[&path]).unwrap();
        let audio = put(
            &mut input,
            &mut services,
            &format!("private/pilot-audio/t{ti}.wav"),
            wav(2400 + ti),
        );
        let raw = json!({"schema_version":1,"provenance":"synthetic","request_sha256":request.file.sha256,"recipe_sha256":input.pins.recipe_sha256,"source_sha256":req["source_sha256"],"direction_sha256":req["direction"]["sha256"],"catalog_sha256":req["catalog"]["sha256"],"speaker":req["speaker"],"model":req["model"],"voice_id":req["voice_id"],"passage":req["passage"],"audio":audio,"format":{"sample_rate":24000,"channels":1,"bits_per_sample":16,"frames":2400+ti}});
        let response = put(
            &mut input,
            &mut services,
            &format!("private/pilot-raw/t{ti}.json"),
            super::super::json(&raw).unwrap(),
        );
        let obs = json!({"schema_version":1,"provenance":"synthetic","request":request,"response":response,"issued":null,"result":null,"findings":[{"id":"retained-pilot-adverse","field":"pilot","outcome":"uncertain","message":"Synthetic takes are not listening evidence.","evidence":[]}]});
        let o = put(
            &mut input,
            &mut services,
            &format!("private/pilot-saved/t{ti}.json"),
            super::super::json(&obs).unwrap(),
        );
        observations.push(o.file);
    }
    edit_context(&mut input, &mut services, |ctx| {
        ctx["observations"] = json!(observations)
    });
}
#[tokio::test]
async fn saved_synthetic_audio_binds_frames_and_both_neighbors_without_listening_approval() {
    let (input, services) = saved_audio_fixture().await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("saved-audio", &input, &services, &out);
    let plan = value(&services, "private/pilot-plan.json");
    assert_eq!(plan["takes"].as_array().unwrap().len(), 2);
    assert_eq!(plan["takes"][0]["format"]["frames"], 2400);
    assert_eq!(plan["takes"][0]["speech_fidelity"], "missing");
    let review = value(&services, "private/pilot/requests/p000-review.json");
    assert_eq!(review["takes"].as_array().unwrap().len(), 2);
    assert_eq!(
        review["expected_context"],
        "Mira watched the rivière. The water was still."
    );
    assert_eq!(review["reference_audio"], Value::Null);
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "retained-pilot-adverse" && f.outcome == ReviewOutcome::Uncertain)
    );
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "pilot-listening" && f.outcome == ReviewOutcome::Missing)
    );
}
fn update_selection(
    input: &mut StageInput,
    services: &mut OfflineServices,
    f: impl FnOnce(&mut Value),
) {
    let mut s: Value = parse(&services.inputs["private/pilot-selection.json"]).unwrap();
    f(&mut s);
    let a = put(
        input,
        services,
        "private/pilot-selection.json",
        super::super::json(&s).unwrap(),
    );
    edit_context(input, services, |ctx| ctx["selection"] = json!(a.file));
}
async fn establish_voice(input: &mut StageInput, services: &mut OfflineServices) -> ArtifactRef {
    let mut ctx: Value = parse(&services.inputs["private/pilot-context.json"]).unwrap();
    let catalog: ArtifactRef = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/voice-catalog.json")
        .unwrap()
        .clone();
    let reference = put(
        input,
        services,
        "private/established-reference.wav",
        wav(2400),
    );
    let binding=put(input,services,"private/established-voice.json",super::super::json(&json!({"schema_version":1,"book_id":"test-book","canonical_id":"test-book:narrator","provider":"speech-double","model":"gemini-3.8-flash-lite-tts","voice_id":"old-n","catalog":catalog,"language":"en","accent":"catalog-default","baseline":"steady","reference":reference,"recordings":[]})).unwrap());
    let mut original: StageInput =
        parse(&services.inputs["private/pilot-direction-input.json"]).unwrap();
    let mut registry: Value = parse(&services.inputs["private/book-registry.json"]).unwrap();
    registry["entities"][0]["voice"] = json!(binding);
    let reg = put(
        &mut original,
        services,
        "private/book-registry.json",
        super::super::json(&registry).unwrap(),
    );
    let mut dctx: Value = parse(&services.inputs["private/direction-context.json"]).unwrap();
    dctx["registry"] = json!(reg.file);
    put(
        &mut original,
        services,
        "private/direction-context.json",
        super::super::json(&dctx).unwrap(),
    );
    let mut req: Value = parse(&services.inputs["private/direction-request.json"]).unwrap();
    req["registry"] = json!({"file":reg.file,"snapshot":registry});
    let request = put(
        &mut original,
        services,
        "private/direction-request.json",
        super::super::json(&req).unwrap(),
    );
    let mut response: Value = parse(&services.inputs["private/direction-response.json"]).unwrap();
    response["request_sha256"] = json!(request.file.sha256);
    let response = put(
        &mut original,
        services,
        "private/direction-response.json",
        super::super::json(&response).unwrap(),
    );
    let mut obs: Value = parse(&services.inputs["private/direction-observation.json"]).unwrap();
    obs["request"] = json!(request);
    obs["response"] = json!(response);
    put(
        &mut original,
        services,
        "private/direction-observation.json",
        super::super::json(&obs).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    GenerationExecutor
        .execute(original.clone(), services)
        .await
        .unwrap();
    let plan = super::super::json(&value(services, "private/direction-plan.json")).unwrap();
    for a in &original.inputs {
        input.inputs.retain(|v| v.file.path != a.file.path);
        input.inputs.push(a.clone());
    }
    let direction = put(input, services, "private/direction-plan.json", plan);
    let saved = put(
        input,
        services,
        "private/pilot-direction-input.json",
        super::super::json(&original).unwrap(),
    );
    ctx["registry"] = json!(reg.file);
    ctx["direction"] = json!(direction.file);
    ctx["direction_input"] = json!(saved.file);
    let mut selection: Value = parse(&services.inputs["private/pilot-selection.json"]).unwrap();
    selection["direction_sha256"] = json!(direction.file.sha256);
    selection["registry_sha256"] = json!(reg.file.sha256);
    let sel = put(
        input,
        services,
        "private/pilot-selection.json",
        super::super::json(&selection).unwrap(),
    );
    ctx["selection"] = json!(sel.file);
    put(
        input,
        services,
        "private/pilot-context.json",
        super::super::json(&ctx).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    binding
}
#[tokio::test]
async fn established_lite_voice_cannot_be_upgraded_or_recast_by_a_pilot_selection() {
    let (mut input, mut services) = fixture(false).await;
    let binding = establish_voice(&mut input, &mut services).await;
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "pilot-established-identity:narrator")
    );
    let p = value(&services, "private/pilot-plan.json");
    assert_eq!(
        p["performers"][0]["retained_binding"]["artifact"],
        json!(binding)
    );
    assert_eq!(p["performers"][0]["model"], "gemini-3.8-flash-lite-tts");
    assert_eq!(p["performers"][0]["voice_id"], "old-n");
    assert!(
        out.artifacts
            .iter()
            .all(|a| !a.file.path.starts_with("private/pilot/requests/"))
    );
}
#[tokio::test]
async fn source_substrings_cannot_cut_words_or_sentences_to_create_two_auditions() {
    let (mut input, mut services) = fixture(false).await;
    update_selection(&mut input, &mut services, |s| {
        let full = "Mira watched the rivière. The water was still.";
        let chars = full.chars().collect::<Vec<_>>();
        s["performers"][0]["passages"][0]["span"] = json!([0, 2]);
        s["performers"][0]["passages"][0]["text"] = json!("Mi");
        s["performers"][0]["passages"][1]["span"] = json!([2, chars.len()]);
        s["performers"][0]["passages"][1]["text"] = json!(chars[2..].iter().collect::<String>());
    });
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert!(
        out.findings
            .iter()
            .any(|f| f.id.starts_with("pilot-passage-boundary:"))
    );
    assert!(
        out.artifacts
            .iter()
            .all(|a| !a.file.path.starts_with("private/pilot/requests/"))
    );
}
fn update_catalog(
    input: &mut StageInput,
    services: &mut OfflineServices,
    f: impl FnOnce(&mut Value),
) {
    let mut c: Value = parse(&services.inputs["private/voice-catalog.json"]).unwrap();
    f(&mut c);
    let a = put(
        input,
        services,
        "private/voice-catalog.json",
        super::super::json(&c).unwrap(),
    );
    edit_context(input, services, |ctx| ctx["catalog"] = json!(a.file));
    update_selection(input, services, |s| {
        s["catalog_sha256"] = json!(a.file.sha256)
    });
}
#[tokio::test]
async fn duplicate_catalog_models_and_voices_are_ambiguous_not_first_match_wins() {
    for models in [true, false] {
        let (mut input, mut services) = fixture(false).await;
        update_catalog(&mut input, &mut services, |c| {
            if models {
                let duplicate = c["models"][0].clone();
                c["models"].as_array_mut().unwrap().push(duplicate);
            } else {
                let duplicate = c["models"][0]["voices"][0].clone();
                c["models"][0]["voices"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
        });
        assert!(GenerationExecutor.execute(input, &services).await.is_err());
        assert!(services.writes.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn unknown_voices_and_mentioned_characters_receive_no_fallback_auditions() {
    for field in ["model", "voice_id", "speaker"] {
        let (mut input, mut services) = fixture(false).await;
        update_selection(&mut input, &mut services, |s| {
            s["performers"][0][field] = json!(if field == "speaker" {
                "mira"
            } else {
                "unknown"
            })
        });
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(
            out.artifacts
                .iter()
                .all(|a| !a.file.path.starts_with("private/pilot/requests/"))
        );
        assert!(
            out.findings
                .iter()
                .any(|f| f.id.starts_with("pilot-catalog-identity:")
                    || f.id.starts_with("pilot-unsupported-speaker:"))
        );
    }
}
#[tokio::test]
async fn genuine_dialogue_gets_its_own_role_without_inventing_another_performance() {
    let (mut input, mut services) = fixture(true).await;
    let plan: Value = parse(&services.inputs["private/direction-plan.json"]).unwrap();
    let turn = plan["turns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["proposal"]["speaker"] == "mira")
        .unwrap()["proposal"]
        .clone();
    update_selection(&mut input, &mut services, |s| {
        let mut p = s["performers"][0].clone();
        p["speaker"] = json!("mira");
        p["passages"] = json!([{"turn_id":turn["id"],"span":turn["span"],"text":turn["text"],"acting":"","acting_support":null}]);
        s["performers"].as_array_mut().unwrap().push(p);
    });
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("dialogue", &input, &services, &out);
    let req = value(&services, "private/pilot/requests/p000-t00.json");
    assert_eq!(req["speaker"], "mira");
    assert_eq!(req["text"], "“Open the gate.”");
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "pilot-adjacent-takes:mira")
    );
    assert_eq!(
        out.artifacts
            .iter()
            .filter(|a| a.file.path.starts_with("private/pilot/requests/"))
            .count(),
        1
    );
}
fn mutate_raw(input: &mut StageInput, services: &mut OfflineServices, f: impl FnOnce(&mut Value)) {
    let mut raw: Value = parse(&services.inputs["private/pilot-raw/t0.json"]).unwrap();
    f(&mut raw);
    let a = put(
        input,
        services,
        "private/pilot-raw/t0.json",
        super::super::json(&raw).unwrap(),
    );
    let mut obs: Value = parse(&services.inputs["private/pilot-saved/t0.json"]).unwrap();
    obs["response"] = json!(a);
    let a = put(
        input,
        services,
        "private/pilot-saved/t0.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(input, services, |ctx| {
        ctx["observations"][0] = json!(a.file)
    });
}
#[tokio::test]
async fn source_request_voice_recipe_passage_and_actual_audio_bindings_fail_closed() {
    for field in [
        "source_sha256",
        "direction_sha256",
        "catalog_sha256",
        "request_sha256",
        "recipe_sha256",
        "speaker",
        "model",
        "voice_id",
        "passage",
        "format",
        "audio",
    ] {
        let (mut input, mut services) = saved_audio_fixture().await;
        mutate_raw(&mut input, &mut services, |raw| match field {
            "passage" => raw["passage"]["span"] = json!([1, 24]),
            "format" => raw["format"]["frames"] = json!(1),
            "audio" => raw["audio"]["id"] = json!("foreign-audio"),
            _ => {
                raw[field] = json!(if field.ends_with("sha256") {
                    "0".repeat(64)
                } else {
                    "foreign".into()
                })
            }
        });
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{field}"
        );
        assert!(services.writes.lock().unwrap().is_empty(), "{field}");
    }
    let (mut input, mut services) = saved_audio_fixture().await;
    let a = put(
        &mut input,
        &mut services,
        "private/pilot-audio/t0.wav",
        b"malformed WAV".to_vec(),
    );
    mutate_raw(&mut input, &mut services, |r| r["audio"] = json!(a));
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
}
#[tokio::test]
async fn stale_direction_catalog_recipe_and_request_candidates_are_rejected() {
    for path in [
        "private/direction-plan.json",
        "private/voice-catalog.json",
        "private/pilot-recipe.json",
        "private/pilot/requests/p000-t00.json",
    ] {
        let (mut input, mut services) = saved_audio_fixture().await;
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
async fn saved_audio_replay_is_byte_identical_and_keeps_original_receipts_and_findings() {
    let (mut input, mut services) = saved_audio_fixture().await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let originals = services.writes.lock().unwrap().clone();
    let original_inputs = services.inputs.clone();
    for (c, b) in &originals {
        put(&mut input, &mut services, &c.file.path, b.clone());
    }
    input.mode = ExecutionMode::Replay;
    services.writes.lock().unwrap().clear();
    let replay = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(out.cache_keys, replay.cache_keys);
    for (c, b) in originals {
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
        if !p.starts_with("private/pilot/requests/") {
            assert_eq!(services.inputs[&p], b, "{p}");
        }
    }
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    assert!(replay.library_updates.is_empty());
}
struct ProviderDouble {
    base: OfflineServices,
    observer: ProviderObserverV2,
    issues: AtomicUsize,
    executions: AtomicUsize,
    fault: &'static str,
}
#[async_trait]
impl RunServices for ProviderDouble {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
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
        panic!("pilot journal effect")
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("legacy provider")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("helper redispatch")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("catalog mutation/query")
    }
    async fn issue_provider_v2(
        &self,
        p: ProviderProposalV2,
    ) -> Result<ProviderOperationV2, ServiceError> {
        let n = self.issues.fetch_add(1, Ordering::SeqCst) + 1;
        p.validate().unwrap();
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
        op.identity.run_id = self.observer.run_id.clone();
        op.identity.epoch = self.observer.epoch;
        op.identity.operation_id = format!("pilot-operation-{n}");
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
            attempt_id: format!("pilot-attempt-{n}"),
            unit_id: self.observer.unit_id.clone(),
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
        match self.fault {
            "issued-model" => op.operation.model = "wrong-model".into(),
            "issued-epoch" => op.identity.epoch += 1,
            "issued-payload" => op.operation.payload.id = "wrong-payload".into(),
            _ => {}
        }
        Ok(op)
    }
    async fn execute_provider_v2(
        &self,
        op: &ProviderOperationV2,
    ) -> Result<ProviderResultV2, ServiceError> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        op.validate().unwrap();
        if self.fault == "unavailable-execution" {
            return Err(ServiceError::ProviderV2Unavailable);
        }
        let ti = if op.operation.payload.file.path.contains("t00") {
            0
        } else {
            1
        };
        let path = format!("private/pilot-raw/t{ti}.json");
        let raw: Value = parse(&self.base.inputs[&path]).unwrap();
        let response = ArtifactRef {
            id: format!("committed:pilot:{}", path.replace('/', ":")),
            file: FileRef {
                path: path.clone(),
                sha256: digest(&self.base.inputs[&path]),
                bytes: self.base.inputs[&path].len() as u64,
                media_type: "application/json".into(),
                dimensions: None,
            },
            provenance: Provenance::Synthetic,
        };
        let audio: ArtifactRef = serde_json::from_value(raw["audio"].clone()).unwrap();
        let usage = QuotaAmounts {
            money_microusd: 0,
            requests: 1,
            tokens: 120,
            repairs: 0,
        };
        let mut result = ProviderResultV2 {
            schema_version: 2,
            result: ProviderResult {
                attempt_id: op.operation.attempt_id.clone(),
                completion: if self.fault == "ambiguous" {
                    ProviderCompletion::Ambiguous
                } else {
                    ProviderCompletion::Completed
                },
                request_sha256: op.operation.request_sha256.clone(),
                response_sha256: Some(response.file.sha256.clone()),
                raw_response: Some(response),
                outputs: vec![audio],
                usage: Some(op.operation.estimate.clone()),
                started_at: "synthetic-pilot-start".into(),
                elapsed_ms: 1,
                diagnostic: "Injected fake PCM; no real provider transport.".into(),
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
            accounted_usage: usage.clone(),
            started_at_ms: 1000,
            recorded_at_ms: 1001,
            epoch_disposition: if self.fault == "obsolete" {
                ProviderEpochDisposition::Obsolete
            } else {
                ProviderEpochDisposition::Current
            },
            retry: None,
            recovery: None,
        };
        for w in &mut result.quota_windows {
            w.accounted = usage.clone();
        }
        match self.fault {
            "result-request" => result.result.request_sha256 = "0".repeat(64),
            "result-raw" => result.result.response_sha256 = Some("0".repeat(64)),
            "missing-output" => result.result.outputs.clear(),
            _ => {}
        }
        Ok(result)
    }
}
async fn provider_fixture(fault: &'static str) -> (StageInput, ProviderDouble) {
    let (mut input, mut base) = saved_audio_fixture().await;
    edit_context(&mut input, &mut base, |ctx| ctx["observations"] = json!([]));
    input.mode = ExecutionMode::Live;
    let observer = ProviderObserverV2 {
        schema_version: 2,
        run_id: input.run_id.clone(),
        epoch: input.epoch,
        unit_id: input.unit_id.clone(),
    };
    (
        input,
        ProviderDouble {
            base,
            observer,
            issues: AtomicUsize::new(0),
            executions: AtomicUsize::new(0),
            fault,
        },
    )
}
#[tokio::test]
async fn provider_v2_doubles_replay_exact_recorded_audio_and_usage_without_redispatch() {
    let (mut input, mut services) = provider_fixture("").await;
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("provider-double", &input, &services.base, &first);
    assert_eq!(services.issues.load(Ordering::SeqCst), 2);
    assert_eq!(services.executions.load(Ordering::SeqCst), 2);
    let originals = services.base.writes.lock().unwrap().clone();
    let mut saved = vec![];
    for (c, b) in &originals {
        let a = put(&mut input, &mut services.base, &c.file.path, b.clone());
        if c.file.path.starts_with("private/pilot/observations/") {
            saved.push(a.file);
        }
    }
    edit_context(&mut input, &mut services.base, |ctx| {
        ctx["observations"] = json!(saved)
    });
    input.mode = ExecutionMode::Replay;
    services.base.writes.lock().unwrap().clear();
    let again = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(first.cache_keys, again.cache_keys);
    for (c, b) in originals
        .iter()
        .filter(|(c, _)| !c.file.path.starts_with("private/pilot/observations/"))
    {
        assert_eq!(
            &services
                .base
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
    let p = value(&services.base, "private/pilot-plan.json");
    assert_eq!(
        p["observations"][0][2]["result"]["accounted_usage"]["tokens"],
        120
    );
    assert_eq!(services.issues.load(Ordering::SeqCst), 2);
    assert_eq!(services.executions.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn issued_and_result_mismatches_reject_and_uncertain_attempts_stay_recorded() {
    for fault in [
        "issued-model",
        "issued-epoch",
        "issued-payload",
        "result-request",
        "result-raw",
        "missing-output",
    ] {
        let (input, services) = provider_fixture(fault).await;
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{fault}"
        );
        assert!(services.base.writes.lock().unwrap().is_empty());
    }
    for fault in ["ambiguous", "obsolete", "unavailable-execution"] {
        let (mut input, mut services) = provider_fixture(fault).await;
        let out = GenerationExecutor
            .execute(input.clone(), &services)
            .await
            .unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert_eq!(
            value(&services.base, "private/pilot-plan.json")["takes"],
            json!([])
        );
        let saved = services.base.writes.lock().unwrap().clone();
        let mut refs = vec![];
        for (c, b) in saved {
            if c.file.path.starts_with("private/pilot/observations/") {
                refs.push(put(&mut input, &mut services.base, &c.file.path, b).file);
            }
        }
        edit_context(&mut input, &mut services.base, |ctx| {
            ctx["observations"] = json!(refs)
        });
        services.base.writes.lock().unwrap().clear();
        GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(services.issues.load(Ordering::SeqCst), 2, "{fault}");
        assert_eq!(services.executions.load(Ordering::SeqCst), 2, "{fault}");
    }
}
#[tokio::test]
async fn compatible_established_lite_recordings_and_reference_are_reused_without_audition() {
    let (mut input, mut services) = fixture(false).await;
    update_catalog(&mut input, &mut services, |c| {
        c["models"].as_array_mut().unwrap().push(json!({"id":"gemini-3.8-flash-lite-tts","voices":[{"id":"old-n","language":"en","accent":"catalog-default","reference":null}]}))
    });
    update_selection(&mut input, &mut services, |s| {
        s["performers"][0]["model"] = json!("gemini-3.8-flash-lite-tts");
        s["performers"][0]["voice_id"] = json!("old-n");
        s["performers"][0]["baseline"] = json!("steady");
    });
    let binding = establish_voice(&mut input, &mut services).await;
    commit_requests(&mut input, &mut services).await;
    add_saved_audio(&mut input, &mut services);
    input.mode = ExecutionMode::Live;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("established-reuse", &input, &services, &out);
    let plan = value(&services, "private/pilot-plan.json");
    assert_eq!(plan["performers"][0]["decision"], "reuse-established");
    assert_eq!(
        plan["performers"][0]["retained_binding"]["artifact"],
        json!(binding)
    );
    assert_eq!(plan["takes"].as_array().unwrap().len(), 2);
    assert!(
        !out.findings
            .iter()
            .any(|f| f.id == "pilot-provider-v2-unavailable")
    );
    assert_eq!(
        value(&services, "private/pilot/requests/p000-review.json")["reference_audio"]["file"]["path"],
        "private/established-reference.wav"
    );
    assert!(out.library_updates.is_empty());
}
async fn add_review(input: &mut StageInput, services: &mut OfflineServices, one_sided: bool) {
    GenerationExecutor
        .execute(input.clone(), services)
        .await
        .unwrap();
    let req = value(services, "private/pilot/requests/p000-review.json");
    services.writes.lock().unwrap().clear();
    let request = put(
        input,
        services,
        "private/pilot/requests/p000-review.json",
        super::super::json(&req).unwrap(),
    );
    edit_context(input, services, |ctx| {
        ctx["request_payloads"]
            .as_array_mut()
            .unwrap()
            .push(json!(request.file))
    });
    let mut targets = req["takes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["audio"].clone())
        .collect::<Vec<_>>();
    if one_sided {
        targets.pop();
    }
    let raw = json!({"schema_version":1,"provenance":"synthetic","request_sha256":request.file.sha256,"recipe_sha256":input.pins.recipe_sha256,"speaker":"narrator","targets":targets,"reference_audio":null,"criteria":[{"id":"continuity","outcome":"fail","message":"Original adverse voice drift observation."},{"id":"listening","outcome":"pass","message":"A fake claim cannot authorize bulk speech."}]});
    let response = put(
        input,
        services,
        "private/pilot-raw/review.json",
        super::super::json(&raw).unwrap(),
    );
    let obs = json!({"schema_version":1,"provenance":"synthetic","request":request,"response":response,"issued":null,"result":null,"findings":[]});
    let a = put(
        input,
        services,
        "private/pilot-saved/review.json",
        super::super::json(&obs).unwrap(),
    );
    edit_context(input, services, |ctx| ctx["reviews"] = json!([a.file]));
}
#[tokio::test]
async fn review_requires_both_neighbors_and_preserves_failures_without_promoting_claimed_passes() {
    let (mut input, mut services) = saved_audio_fixture().await;
    add_review(&mut input, &mut services, false).await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("review", &input, &services, &out);
    assert!(out.findings.iter().any(|f|f.id=="pilot-review:narrator:continuity"&&f.outcome==ReviewOutcome::Fail));
    assert!(out.findings.iter().any(
        |f| f.id == "pilot-review:narrator:listening" && f.outcome == ReviewOutcome::Uncertain
    ));
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    let (mut input, mut services) = saved_audio_fixture().await;
    add_review(&mut input, &mut services, true).await;
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    assert!(services.writes.lock().unwrap().is_empty());
}
#[tokio::test]
async fn new_voice_first_audition_has_empty_style_and_keeps_acting_outside_prose() {
    let (mut input, mut services) = fixture(false).await;
    update_selection(&mut input, &mut services, |s| {
        s["performers"][0]["baseline"] = json!("steady");
        let p = &mut s["performers"][0]["passages"][0];
        p["acting"] = json!("quietly");
        p["acting_support"] = json!({"span":p["span"],"quote":p["text"]});
    });
    GenerationExecutor.execute(input, &services).await.unwrap();
    let request = value(&services, "private/pilot/requests/p000-t00.json");
    assert_eq!(request["style"], "");
    assert_eq!(request["baseline"], "steady");
    assert_eq!(request["text"], "Mira watched the rivière.");
}
struct NoReads;
#[async_trait]
impl RunServices for NoReads {
    async fn read_artifact(&self, _: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        panic!("oversized pilot input was read")
    }
    async fn write_candidate(
        &self,
        _: CandidateWrite,
        _: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        panic!("unexpected write")
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        panic!("unexpected journal")
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        Ok(())
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("unexpected provider")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("unexpected helper")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("unexpected library")
    }
}
#[tokio::test]
async fn metadata_mime_labels_cannot_bypass_pilot_limits_before_reads() {
    let (mut input, _) = fixture(false).await;
    let a = input
        .inputs
        .iter_mut()
        .find(|a| a.file.path == "private/pilot-context.json")
        .unwrap();
    a.file.bytes = 1024 * 1024 + 1;
    a.file.media_type = "audio/wav".into();
    assert!(GenerationExecutor.execute(input, &NoReads).await.is_err());
}
#[tokio::test]
async fn malformed_raw_observations_and_oversized_audio_or_envelopes_reject() {
    let (mut input, mut services) = saved_audio_fixture().await;
    mutate_raw(&mut input, &mut services, |raw| {
        raw["unexpected"] = json!(true)
    });
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    for case in ["audio", "count", "aggregate"] {
        let (mut input, _) = fixture(false).await;
        match case {
            "audio" => {
                let a = &mut input.inputs[0];
                a.file.path = "private/big.wav".into();
                a.file.media_type = "audio/wav".into();
                a.file.bytes = 8 * 1024 * 1024 + 1;
            }
            "count" => {
                let a = input.inputs[0].clone();
                input.inputs = vec![a; 65];
            }
            _ => {
                let mut refs = vec![];
                for i in 0..9 {
                    let mut a = input.inputs[0].clone();
                    a.id = format!("large-{i}");
                    a.file.path = format!("private/large-{i}.wav");
                    a.file.media_type = "audio/wav".into();
                    a.file.bytes = 8 * 1024 * 1024;
                    refs.push(a);
                }
                input.inputs = refs;
            }
        }
        assert!(
            GenerationExecutor.execute(input, &NoReads).await.is_err(),
            "{case}"
        );
    }
}
#[tokio::test]
async fn pilot_records_and_audio_reviews_remain_outside_audience_exports_even_with_aliases() {
    let (input, services) = saved_audio_fixture().await;
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let (mut package, mut target) = super::fixture();
    let mut protected = BTreeMap::new();
    for a in &input.inputs {
        if a.file.path.starts_with("private/") {
            let a = put(
                &mut package,
                &mut target,
                &a.file.path,
                services.inputs[&a.file.path].clone(),
            );
            protected.insert(a.file.sha256, true);
        }
    }
    for (c, b) in services.writes.lock().unwrap().iter() {
        let a = put(&mut package, &mut target, &c.file.path, b.clone());
        protected.insert(a.file.sha256, true);
    }
    let p = services.inputs["private/voice-catalog.json"].clone();
    let mut alias = put(&mut package, &mut target, "assets/pilot-alias.svg", p);
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
            .all(|f| !f.path.starts_with("private/") && !protected.contains_key(&f.sha256))
    );
}
