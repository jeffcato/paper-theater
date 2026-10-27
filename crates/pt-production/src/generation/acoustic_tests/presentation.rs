use super::*;

fn add(
    input: &mut StageInput,
    services: &mut ProtocolServices,
    path: &str,
    bytes: Vec<u8>,
    media: &str,
) -> FileRef {
    let file = FileRef {
        path: path.into(),
        sha256: digest(&bytes),
        bytes: bytes.len() as u64,
        media_type: media.into(),
        dimensions: None,
    };
    input.inputs.push(ArtifactRef {
        id: format!("presentation:{path}"),
        file: file.clone(),
        provenance: Provenance::Diagnostic,
    });
    services.inputs.insert(path.into(), bytes);
    file
}
async fn fixture() -> (StageInput, ProtocolServices) {
    let (mut acoustic, mut original) = protocol_fixture(ExecutionMode::Replay, "");
    attach_fact_inventory(&mut acoustic, &mut original);
    add_recorded_report(&mut acoustic, &mut original);
    let facts = GenerationExecutor
        .execute(acoustic, &original)
        .await
        .unwrap();
    let (mut input, mut services) = recorded_fact_inputs(&facts, &original, true);
    add_recorded_report(&mut input, &mut services);
    input.stage = Stage::Presentation;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("fixtures/generation/two-scene-readiness-v2");
    for path in [
        "fixture.json",
        "timeline.json",
        "read-along.json",
        "presentation-map.json",
        "audio/master.wav",
        "credits.json",
        "correction.json",
        "evidence/alignment.json",
    ] {
        add(
            &mut input,
            &mut services,
            &format!("delivery/{path}"),
            std::fs::read(source.join(path)).unwrap(),
            if path.ends_with(".wav") {
                "audio/wav"
            } else {
                "application/json"
            },
        );
    }
    add(
        &mut input,
        &mut services,
        "delivery/edit-recipe.txt",
        std::fs::read(root.join("tools/generation/readiness_correction.py")).unwrap(),
        "text/plain",
    );
    let f = |path: &str| {
        input
            .inputs
            .iter()
            .find(|a| a.file.path == path)
            .unwrap()
            .file
            .clone()
    };
    let recipe = serde_json::json!({"schema_version":1,"provenance":"diagnostic","kind":"pcm-reconstruction-recipe; not an original execution receipt",
        "algorithm":"rational-zero-order-hold-pcm16-v1","channel_map":"mono-duplicate-stereo",
        "narration_sha256":f("excerpt.wav").sha256,"delivery_sha256":f("delivery/audio/master.wav").sha256,
        "map_sha256":f("delivery/presentation-map.json").sha256,"source_frames":1451280,"delivery_frames":3262560,
        "source_omissions":[],"segments":[
            {"id":"speech-0","processing_delay_frames":0,"compensation_frames":0,"trim_start_frames":0,"trim_end_frames":0},
            {"id":"speech-1","processing_delay_frames":0,"compensation_frames":0,"trim_start_frames":0,"trim_end_frames":0}],
        "historical":{"description":f("delivery/credits.json"),"edit_record":f("delivery/correction.json"),"edit_implementation":f("delivery/edit-recipe.txt")}});
    add(
        &mut input,
        &mut services,
        "presentation-recipe.json",
        json(&recipe).unwrap(),
        "application/json",
    );
    (input, services)
}
#[tokio::test]
async fn saved_v2_pcm_and_fact_evidence_produce_verified_delivery_bounds() {
    let (input, services) = fixture().await;
    let output = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let report = artifact_json(&output, &services, "presentation-fact-evidence.json");
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    assert_eq!(
        report["placement"]["outcome"], "pass",
        "{}",
        report["placement"]
    );
    for (id, raw, mapped) in [
        ("cellar-instruction", 546720, 1189440),
        ("Dorothy-urged", 506640, 1109280),
        ("screaming", 506640, 1109280),
        ("urged-cellar", 546720, 1189440),
    ] {
        let row = report["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == id)
            .unwrap();
        assert_eq!(row["narration"]["not_before_sample"], raw);
        assert_eq!(row["delivery_not_before_sample"], mapped);
        assert_eq!(row["outcome"], "pass");
    }
    for (id, outcome) in [
        ("strange-event", "uncertain"),
        ("balloon-ascent", "uncertain"),
        ("unprovided-fact", "missing"),
    ] {
        let row = report["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == id)
            .unwrap();
        assert_eq!(row["outcome"], outcome);
        assert!(row["delivery_not_before_sample"].is_null());
    }
    assert_eq!(report["word_count"], 176);
    assert_eq!(report["highlight_group_count"], 170);
    assert_eq!(report["point_word_count"], 6);
    assert!(report["reveal_permissions"].as_array().unwrap().is_empty());
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    assert!(
        report["missing_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == "presentation-fact:strange-event")
    );
    assert_eq!(
        report["entry_comparisons"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["timing"] == "would_be_late")
            .count(),
        4
    );
    if let Ok(destination) = std::env::var("PT_PRESENTATION_EXPORT") {
        assert!(safe_path(&destination) && destination.starts_with("fixtures/generation/"));
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(&destination);
        std::fs::create_dir_all(&root).unwrap();
        for path in [
            "presentation-recipe.json",
            "fact-evidence.json",
            "acoustic-validation.json",
        ] {
            std::fs::write(root.join(path), &services.inputs[path]).unwrap();
        }
        std::fs::write(
            root.join("presentation-fact-evidence.json"),
            &services.candidates.lock().unwrap()[&output.artifacts[0].candidate_id].1,
        )
        .unwrap();
        let index = input
            .inputs
            .iter()
            .map(|a| {
                let path = &a.file.path;
                let source = if path == "acoustic-replay.json" {
                    "fixtures/generation/acoustic/replay.json".into()
                } else if path == "source-facts.json" {
                    "fixtures/generation/acoustic-facts-v1/source-facts.json".into()
                } else if path == "delivery/edit-recipe.txt" {
                    "tools/generation/readiness_correction.py".into()
                } else if let Some(p) = path.strip_prefix("delivery/") {
                    format!("fixtures/generation/two-scene-readiness-v2/{p}")
                } else if [
                    "presentation-recipe.json",
                    "fact-evidence.json",
                    "acoustic-validation.json",
                ]
                .contains(&path.as_str())
                {
                    format!("{destination}/{path}")
                } else {
                    format!("fixtures/generation/acoustic/{path}")
                };
                serde_json::json!({"artifact":a,"source":source})
            })
            .collect::<Vec<_>>();
        std::fs::write(root.join("input-index.json"), json(&index).unwrap()).unwrap();
    }
}

fn fork(input: &StageInput, services: &ProtocolServices) -> (StageInput, ProtocolServices) {
    (
        input.clone(),
        ProtocolServices {
            inputs: services.inputs.clone(),
            report: services.report.clone(),
            fault: "",
            candidates: Mutex::new(BTreeMap::new()),
            cancelled: AtomicBool::new(false),
            helper_calls: AtomicUsize::new(0),
            candidate_reads: AtomicUsize::new(0),
            emits: AtomicUsize::new(0),
        },
    )
}
fn no_bounds(report: &serde_json::Value) {
    assert_ne!(
        report["placement"]["outcome"], "pass",
        "{}",
        report["placement"]
    );
    assert!(
        report["facts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["delivery_not_before_sample"].is_null())
    );
}
#[tokio::test]
async fn missing_mapping_recipe_or_observations_leave_no_delivery_bounds() {
    let (base, original) = fixture().await;
    for path in [
        "delivery/presentation-map.json",
        "presentation-recipe.json",
        "delivery/evidence/alignment.json",
        "delivery/audio/master.wav",
        "delivery/edit-recipe.txt",
        "source-facts.json",
        "excerpt.wav",
    ] {
        let (mut input, services) = fork(&base, &original);
        input.inputs.retain(|a| a.file.path != path);
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
        no_bounds(&artifact_json(
            &output,
            &services,
            "presentation-fact-evidence.json",
        ));
        assert!(
            output
                .findings
                .iter()
                .any(|f| f.outcome != ReviewOutcome::Pass)
        );
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
        assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn stale_g16_inventory_normalization_map_recipe_and_delivery_bindings_fail_closed() {
    let (base, original) = fixture().await;
    for fault in [
        "inventory",
        "fact-result",
        "normalized",
        "map",
        "recipe",
        "timeline",
        "read-along",
        "alignment",
        "edit-recipe",
        "master-hash",
        "narration-hash",
    ] {
        let (mut input, mut services) = fork(&base, &original);
        let path = match fault {
            "inventory" => "source-facts.json",
            "fact-result" => "fact-evidence.json",
            "normalized" => "acoustic-replay.json",
            "map" => "delivery/presentation-map.json",
            "recipe" => "presentation-recipe.json",
            "timeline" => "delivery/timeline.json",
            "read-along" => "delivery/read-along.json",
            "alignment" => "delivery/evidence/alignment.json",
            "edit-recipe" => "delivery/edit-recipe.txt",
            "master-hash" => "delivery/audio/master.wav",
            _ => "excerpt.wav",
        };
        if matches!(fault, "master-hash" | "narration-hash") {
            services.inputs.get_mut(path).unwrap()[100] ^= 1;
            assert!(GenerationExecutor.execute(input, &services).await.is_err());
            assert!(services.candidates.lock().unwrap().is_empty());
            continue;
        }
        let mut bytes = services.inputs[path].clone();
        if fault == "edit-recipe" {
            bytes.push(b' ');
        } else {
            let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            match fault {
                "inventory" => value["facts"][0]["source_spans"] = serde_json::json!([[315, 335]]),
                "fact-result" => value["facts"][0]["not_before_sample"] = serde_json::json!(1),
                "normalized" => value["word_track"][0]["end_sample"] = serde_json::json!(1),
                "map" => value["segments"][1]["destination"][0] = serde_json::json!(1),
                "recipe" => value["algorithm"] = serde_json::json!("unverified-resampler"),
                "timeline" => value["presentation_map_sha256"] = serde_json::json!("0".repeat(64)),
                "read-along" => value["words"][0]["start_sample"] = serde_json::json!(0),
                "alignment" => value["words"][0]["end_sample"] = serde_json::json!(1),
                _ => unreachable!(),
            }
            bytes = json(&value).unwrap();
        }
        replace_input(&mut input, &mut services, path, bytes);
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        no_bounds(&artifact_json(
            &output,
            &services,
            "presentation-fact-evidence.json",
        ));
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn recorded_mapping_is_byte_identical_and_every_changed_dependency_invalidates_it() {
    let (mut input, services) = fixture().await;
    input.mode = ExecutionMode::Fixture;
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let candidate = &first.artifacts[0];
    let expected = services.candidates.lock().unwrap()[&candidate.candidate_id]
        .1
        .clone();
    input.mode = ExecutionMode::Replay;
    let (mut replay, mut recorded) = fork(&input, &services);
    add(
        &mut replay,
        &mut recorded,
        "presentation-fact-evidence.json",
        expected.clone(),
        "application/json",
    );
    let output = GenerationExecutor
        .execute(replay.clone(), &recorded)
        .await
        .unwrap();
    assert_eq!(
        recorded.candidates.lock().unwrap()[&output.artifacts[0].candidate_id].1,
        expected
    );
    assert_eq!(recorded.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(recorded.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(recorded.emits.load(Ordering::SeqCst), 0);
    for path in [
        "presentation-recipe.json",
        "delivery/presentation-map.json",
        "source-facts.json",
        "delivery/audio/master.wav",
        "excerpt.wav",
        "presentation-fact-evidence.json",
    ] {
        let (mut changed, mut services) = fork(&replay, &recorded);
        let mut bytes = services.inputs[path].clone();
        if path.ends_with(".wav") {
            bytes[100] ^= 1;
        } else if path == "presentation-fact-evidence.json" {
            let mut report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            report["implementation_sha256"] = serde_json::json!("0".repeat(64));
            bytes = json(&report).unwrap();
        } else {
            bytes.push(b' ');
        }
        replace_input(&mut changed, &mut services, path, bytes);
        assert!(
            GenerationExecutor
                .execute(changed, &services)
                .await
                .is_err(),
            "accepted stale {path}"
        );
        assert!(services.candidates.lock().unwrap().is_empty());
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
        assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn recorded_adverse_g16_findings_still_block_mapped_facts() {
    let (base, original) = fixture().await;
    for outcome in [ReviewOutcome::Fail, ReviewOutcome::Uncertain] {
        let (mut input, mut services) = fork(&base, &original);
        let mut summary: serde_json::Value =
            serde_json::from_slice(&services.inputs["acoustic-validation.json"]).unwrap();
        summary["findings"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["id"] == "acoustic-fact:cellar-instruction")
            .unwrap()["outcome"] = serde_json::to_value(outcome).unwrap();
        replace_input(
            &mut input,
            &mut services,
            "acoustic-validation.json",
            json(&summary).unwrap(),
        );
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        let report = artifact_json(&output, &services, "presentation-fact-evidence.json");
        let cellar = report["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == "cellar-instruction")
            .unwrap();
        assert!(cellar["delivery_not_before_sample"].is_null());
        assert_eq!(cellar["outcome"], serde_json::to_value(outcome).unwrap());
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    }
}

fn rel(mut f: FileRef) -> FileRef {
    f.path = f.path.strip_prefix("delivery/").unwrap().into();
    f
}
fn reference(input: &StageInput, path: &str) -> FileRef {
    input
        .inputs
        .iter()
        .find(|a| a.file.path == path)
        .unwrap()
        .file
        .clone()
}
fn rebind_delivery(input: &mut StageInput, services: &mut ProtocolServices) {
    let mut doc: PlaybackDocument =
        serde_json::from_slice(&services.inputs["delivery/fixture.json"]).unwrap();
    doc.manifest.entries.audio = rel(reference(input, "delivery/audio/master.wav"));
    doc.timeline.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    doc.read_along.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    // Synthetic negative media receives no copied reveal permission or action approval.
    for reveal in &mut doc.manifest.reveals {
        reveal.not_before_sample = None;
    }
    doc.timeline.cues.clear();
    replace_input(
        input,
        services,
        "delivery/timeline.json",
        json(&doc.timeline).unwrap(),
    );
    doc.manifest.entries.timeline = rel(reference(input, "delivery/timeline.json"));
    replace_input(
        input,
        services,
        "delivery/read-along.json",
        json(&doc.read_along).unwrap(),
    );
    doc.manifest.entries.read_along = rel(reference(input, "delivery/read-along.json"));
    replace_input(
        input,
        services,
        "delivery/fixture.json",
        json(&doc).unwrap(),
    );
    let mut edit: serde_json::Value =
        serde_json::from_slice(&services.inputs["delivery/correction.json"]).unwrap();
    edit["provenance"] = serde_json::json!("synthetic");
    edit["new_bindings"]["delivery_audio_sha256"] =
        serde_json::json!(doc.manifest.entries.audio.sha256);
    replace_input(
        input,
        services,
        "delivery/correction.json",
        json(&edit).unwrap(),
    );
    let mut recipe: serde_json::Value =
        serde_json::from_slice(&services.inputs["presentation-recipe.json"]).unwrap();
    recipe["delivery_sha256"] = serde_json::json!(doc.manifest.entries.audio.sha256);
    recipe["historical"]["edit_record"] =
        serde_json::to_value(reference(input, "delivery/correction.json")).unwrap();
    replace_input(
        input,
        services,
        "presentation-recipe.json",
        json(&recipe).unwrap(),
    );
}
#[tokio::test]
async fn changed_pcm_with_all_new_hashes_still_fails_physical_placement() {
    let (base, original) = fixture().await;
    for (frame, reason) in [
        (96000, "delivered_pcm_mismatch"),
        (0, "declared_gap_not_silent"),
    ] {
        let (mut input, mut services) = fork(&base, &original);
        let mut bytes = services.inputs["delivery/audio/master.wav"].clone();
        bytes[44 + frame * 4] ^= 1;
        replace_input(
            &mut input,
            &mut services,
            "delivery/audio/master.wav",
            bytes,
        );
        rebind_delivery(&mut input, &mut services);
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        let report = artifact_json(&output, &services, "presentation-fact-evidence.json");
        no_bounds(&report);
        assert_eq!(report["placement"]["reason"], reason);
    }
}
#[tokio::test]
async fn imported_recorded_mapping_replays_without_journal_accounting_or_catalog_mutation() {
    let (mut input, mut services) = fixture().await;
    let initial = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let expected = services.candidates.lock().unwrap()[&initial.artifacts[0].candidate_id]
        .1
        .clone();
    add(
        &mut input,
        &mut services,
        "presentation-fact-evidence.json",
        expected.clone(),
        "application/json",
    );
    let saved = tempfile::tempdir().unwrap();
    for (path, bytes) in &services.inputs {
        let p = saved.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(directory.path()).unwrap();
    let request = serde_json::json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":input.pins,"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}});
    let run = store
        .create("presentation-recorded-import", &request)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_acoustic_fixture(&run, epoch, saved.path())
        .unwrap();
    let snapshot = store.api_snapshot(&run).unwrap();
    input.inputs = snapshot
        .stages
        .iter()
        .find(|s| s.stage == Stage::Acoustic)
        .unwrap()
        .artifacts
        .clone();
    input.mode = ExecutionMode::Replay;
    services.inputs.clear();
    for a in &input.inputs {
        services
            .inputs
            .insert(a.file.path.clone(), store.artifact(&a.id).unwrap().0);
    }
    services.candidates.lock().unwrap().clear();
    let before = store.snapshot(&run).unwrap();
    let journal = store.journal(&run, 0, 500).unwrap();
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(
        services.candidates.lock().unwrap()[&output.artifacts[0].candidate_id].1,
        expected
    );
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert_eq!(store.journal(&run, 0, 500).unwrap(), journal);
    assert!(store.catalog("pg-55").unwrap().is_none());
    assert!(
        journal
            .iter()
            .all(|e| !e["type"].as_str().unwrap().starts_with("helper_")
                && !e["type"].as_str().unwrap().starts_with("provider_"))
    );
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn derived_saved_bundle_replays_from_its_exact_input_index() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle = root.join("fixtures/generation/presentation-evidence-v1");
    let index: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(bundle.join("input-index.json")).unwrap()).unwrap();
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    input.stage = Stage::Presentation;
    input.inputs.clear();
    services.inputs.clear();
    for entry in index {
        let artifact: ArtifactRef = serde_json::from_value(entry["artifact"].clone()).unwrap();
        let source = entry["source"].as_str().unwrap();
        assert!(safe_path(source));
        services.inputs.insert(
            artifact.file.path.clone(),
            std::fs::read(root.join(source)).unwrap(),
        );
        input.inputs.push(artifact);
    }
    let expected = std::fs::read(bundle.join("presentation-fact-evidence.json")).unwrap();
    add(
        &mut input,
        &mut services,
        "presentation-fact-evidence.json",
        expected.clone(),
        "application/json",
    );
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(
        services.candidates.lock().unwrap()[&output.artifacts[0].candidate_id].1,
        expected
    );
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}
