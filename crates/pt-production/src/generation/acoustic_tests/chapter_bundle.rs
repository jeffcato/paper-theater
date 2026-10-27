use super::*;

fn fixture() -> (StageInput, ProtocolServices, tempfile::TempDir) {
    fixture_size(false)
}
fn fixture_size(large: bool) -> (StageInput, ProtocolServices, tempfile::TempDir) {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("bundle");
    let mut narration = project.join("fixtures/generation/acoustic/excerpt.wav");
    if large {
        use std::io::{Read, Seek, SeekFrom, Write};
        let mut original = std::fs::File::open(&narration).unwrap();
        let mut head = [0; 44];
        original.read_exact(&mut head).unwrap();
        let bytes = 600u32 * 24000 * 2;
        head[4..8].copy_from_slice(&(bytes + 36).to_le_bytes());
        head[40..44].copy_from_slice(&bytes.to_le_bytes());
        narration = temp.path().join("chapter.wav");
        let mut f = std::fs::File::create(&narration).unwrap();
        f.write_all(&head).unwrap();
        f.set_len(bytes as u64 + 44).unwrap();
        f.seek(SeekFrom::Start(44 + 1000000 * 2)).unwrap();
        std::io::copy(&mut original, &mut f).unwrap();
    }
    let mut command = std::process::Command::new("/usr/bin/python3");
    command
        .arg("-B")
        .arg(project.join("tools/generation/chapter_acoustic.py"))
        .arg("produce")
        .arg(&root)
        .arg("--narration")
        .arg(&narration)
        .arg("--source")
        .arg(project.join("fixtures/generation/acoustic/source.txt"))
        .arg("--evidence-index")
        .arg(project.join("fixtures/generation/presentation-evidence-v1/input-index.json"));
    if large {
        command.arg("--evidence-offset").arg("1000000");
    } else {
        command.arg("--max-frames").arg("530000");
    }
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    input.inputs.clear();
    services.inputs.clear();
    let unit: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("units/chunk-0000.json")).unwrap())
            .unwrap();
    for value in unit["files"].as_array().unwrap() {
        let file: FileRef = serde_json::from_value(value.clone()).unwrap();
        services.inputs.insert(
            file.path.clone(),
            std::fs::read(root.join(&file.path)).unwrap(),
        );
        input.inputs.push(ArtifactRef {
            id: format!("bundle:{}", file.path),
            file,
            provenance: Provenance::Imported,
        });
    }
    for (alias, path) in [
        ("chapter-unit.json", "units/chunk-0000.json"),
        ("chapter-helper-record.json", "records/chunk-0000.json"),
    ] {
        let bytes = std::fs::read(root.join(path)).unwrap();
        input.inputs.push(ArtifactRef {
            id: format!("bundle:{alias}"),
            provenance: Provenance::Diagnostic,
            file: FileRef {
                path: alias.into(),
                sha256: digest(&bytes),
                bytes: bytes.len() as u64,
                media_type: "application/json".into(),
                dimensions: None,
            },
        });
        services.inputs.insert(alias.into(), bytes);
    }
    (input, services, temp)
}
#[tokio::test]
async fn bounded_chapter_unit_revalidates_global_facts_and_replays_without_dispatch() {
    let (mut input, mut services, _temp) = fixture();
    let output = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let report = artifact_json(&output, &services, "chapter-acoustic-validation.json");
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    assert_eq!(report["word_count"], 176);
    assert_eq!(report["highlight_group_count"], 170);
    assert_eq!(report["point_word_count"], 6);
    let cellar = report["facts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["original"]["id"] == "cellar-instruction")
        .unwrap();
    assert_eq!(cellar["chapter_not_before_sample"], 546720);
    assert_eq!(cellar["required_chunks"].as_array().unwrap().len(), 2);
    assert_eq!(
        report["facts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "pass")
            .count(),
        4
    );
    for id in ["strange-event", "balloon-ascent", "unprovided-fact"] {
        assert!(
            report["facts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["original"]["id"] == id)
                .unwrap()["chapter_not_before_sample"]
                .is_null()
        );
    }
    let candidate = &output.artifacts[0];
    let bytes = services.candidates.lock().unwrap()[&candidate.candidate_id]
        .1
        .clone();
    if let Ok(destination) = std::env::var("PT_CHAPTER_EXPORT") {
        assert!(safe_path(&destination) && destination.starts_with("fixtures/generation/"));
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(destination);
        std::fs::create_dir_all(&root).unwrap();
        for name in [
            "chapter-index.json",
            "chapter-unit.json",
            "chapter-helper-record.json",
        ] {
            std::fs::write(root.join(name), &services.inputs[name]).unwrap();
        }
        std::fs::write(root.join("chapter-acoustic-validation.json"), &bytes).unwrap();
        std::fs::copy(
            _temp.path().join("bundle/bundle-verification.json"),
            root.join("bundle-verification.json"),
        )
        .unwrap();
    }
    input.inputs.push(ArtifactRef {
        id: "recorded:chapter-unit".into(),
        file: candidate.file.clone(),
        provenance: Provenance::Diagnostic,
    });
    services
        .inputs
        .insert(candidate.file.path.clone(), bytes.clone());
    services.candidates.lock().unwrap().clear();
    let replay = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    assert_eq!(
        services.candidates.lock().unwrap()[&replay.artifacts[0].candidate_id].1,
        bytes
    );
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    let mut stale: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    stale["implementation_sha256"] = serde_json::json!("0".repeat(64));
    replace_input(
        &mut input,
        &mut services,
        "chapter-acoustic-validation.json",
        json(&stale).unwrap(),
    );
    services.candidates.lock().unwrap().clear();
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    assert!(services.candidates.lock().unwrap().is_empty());
}

#[tokio::test]
async fn missing_unregistered_record_preserves_original_uncertainty_without_dispatch() {
    let (mut input, services, _temp) = fixture();
    input.mode = ExecutionMode::Fixture;
    input
        .inputs
        .retain(|a| a.file.path != "chapter-helper-record.json");
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    assert!(output.findings.iter().any(|f|f.id=="acoustic-fact:strange-event" && f.outcome==ReviewOutcome::Uncertain));
    assert!(
        output
            .findings
            .iter()
            .any(|f| f.id == "chapter-helper-registration-pending"
                && f.outcome == ReviewOutcome::Missing)
    );
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn temporary_600_second_chapter_replays_a_maximum_sized_unit_with_unknown_global_evidence() {
    let (input, services, _temp) = fixture_size(true);
    assert!(
        input
            .inputs
            .iter()
            .all(|a| a.file.bytes <= 16 * 1024 * 1024)
    );
    assert!(!input.inputs.iter().any(|a| a.file.path == "chapter.wav"));
    assert!(
        input
            .inputs
            .iter()
            .any(|a| a.file.bytes == 16 * 1024 * 1024)
    );
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    let report = artifact_json(&output, &services, "chapter-acoustic-validation.json");
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    assert_eq!(report["word_count"], 176);
    assert_eq!(report["words"][0]["global_samples"][0], 1009600);
    assert!(
        report["facts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["chapter_not_before_sample"].is_null())
    );
    assert_eq!(
        report["facts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["original"]["outcome"] == "pass")
            .count(),
        4
    );
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn forged_records_changed_chunks_offsets_sources_and_oversize_inputs_are_rejected() {
    for fault in [
        "record",
        "chunk",
        "offset",
        "receipt",
        "source",
        "oversize",
        "aggregate",
    ] {
        let (mut input, mut services, _temp) = fixture();
        match fault {
            "chunk" => {
                services.inputs.get_mut("chunks/chunk-0000.wav").unwrap()[100] ^= 1;
            }
            "oversize" | "aggregate" => {
                let count = if fault == "aggregate" { 4 } else { 1 };
                for n in 0..count {
                    input.inputs.push(ArtifactRef {
                        id: format!("oversize:{n}"),
                        file: FileRef {
                            path: format!("extra-{n}.json"),
                            sha256: "0".repeat(64),
                            bytes: 16 * 1024 * 1024 + u64::from(fault == "oversize"),
                            media_type: "application/json".into(),
                            dimensions: None,
                        },
                        provenance: Provenance::Synthetic,
                    });
                }
            }
            _ => {
                let path = match fault {
                    "record" => "chapter-helper-record.json",
                    "source" => "chapter-source.txt",
                    _ => "chapter-index.json",
                };
                if fault == "source" {
                    replace_input(&mut input, &mut services, path, b"changed source".to_vec());
                } else {
                    let mut value: serde_json::Value =
                        serde_json::from_slice(&services.inputs[path]).unwrap();
                    match fault {
                        "record" => {
                            value["facts"][0]["chapter_not_before_sample"] = serde_json::json!(1)
                        }
                        "offset" => value["evidence_scope"][0] = serde_json::json!(1),
                        "receipt" => {
                            value["receipts"][0]["request_sha256"] =
                                serde_json::json!("0".repeat(64))
                        }
                        _ => unreachable!(),
                    };
                    replace_input(&mut input, &mut services, path, json(&value).unwrap());
                }
            }
        }
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "accepted {fault}"
        );
        assert!(services.candidates.lock().unwrap().is_empty());
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn a_valid_unit_without_a_cross_cut_dependency_keeps_that_fact_unresolved() {
    let (mut input, mut services, _temp) = fixture();
    let mut unit: serde_json::Value =
        serde_json::from_slice(&services.inputs["chapter-unit.json"]).unwrap();
    unit["included_chunks"] = serde_json::json!(["chunk-0000"]);
    unit["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["path"] != "chunks/chunk-0001.wav");
    replace_input(
        &mut input,
        &mut services,
        "chapter-unit.json",
        json(&unit).unwrap(),
    );
    input
        .inputs
        .retain(|a| a.file.path != "chunks/chunk-0001.wav");
    let mut record: serde_json::Value =
        serde_json::from_slice(&services.inputs["chapter-helper-record.json"]).unwrap();
    record["unit_sha256"] = serde_json::json!(digest(&services.inputs["chapter-unit.json"]));
    record["verified_chunks"] = serde_json::json!(["chunk-0000"]);
    record["checked_files"]
        .as_object_mut()
        .unwrap()
        .remove("chunks/chunk-0001.wav");
    for f in record["facts"].as_array_mut().unwrap() {
        if f["required_chunks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "chunk-0001")
        {
            f["present_chunks"] = serde_json::json!(["chunk-0000"]);
            f["outcome"] = serde_json::json!("missing");
            f["chapter_not_before_sample"] = serde_json::Value::Null;
            f["reason"] = serde_json::json!("cross_cut_evidence_missing");
        }
    }
    replace_input(
        &mut input,
        &mut services,
        "chapter-helper-record.json",
        json(&record).unwrap(),
    );
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    let report = artifact_json(&output, &services, "chapter-acoustic-validation.json");
    let cellar = report["facts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["original"]["id"] == "cellar-instruction")
        .unwrap();
    assert_eq!(cellar["original"]["outcome"], "pass");
    assert_eq!(cellar["outcome"], "missing");
    assert!(cellar["chapter_not_before_sample"].is_null());
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
}
fn ref_for(input: &StageInput, path: &str) -> FileRef {
    input
        .inputs
        .iter()
        .find(|a| a.file.path == path)
        .unwrap()
        .file
        .clone()
}
#[tokio::test]
async fn recorded_adverse_global_findings_survive_rebound_bundle_metadata() {
    let (mut input, mut services, _temp) = fixture();
    let path = "acoustic-validation.json";
    let mut summary: serde_json::Value = serde_json::from_slice(&services.inputs[path]).unwrap();
    summary["findings"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["id"] == "acoustic-fact:cellar-instruction")
        .unwrap()["outcome"] = serde_json::json!("fail");
    replace_input(&mut input, &mut services, path, json(&summary).unwrap());
    let changed = serde_json::to_value(ref_for(&input, path)).unwrap();
    let mut index: serde_json::Value =
        serde_json::from_slice(&services.inputs["chapter-index.json"]).unwrap();
    *index["context_files"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["path"] == path)
        .unwrap() = changed.clone();
    replace_input(
        &mut input,
        &mut services,
        "chapter-index.json",
        json(&index).unwrap(),
    );
    let mut unit: serde_json::Value =
        serde_json::from_slice(&services.inputs["chapter-unit.json"]).unwrap();
    unit["index"] = serde_json::to_value(ref_for(&input, "chapter-index.json")).unwrap();
    let new_index = unit["index"].clone();
    for f in unit["files"].as_array_mut().unwrap() {
        if f["path"] == path {
            *f = changed.clone();
        } else if f["path"] == "chapter-index.json" {
            *f = new_index.clone();
        }
    }
    replace_input(
        &mut input,
        &mut services,
        "chapter-unit.json",
        json(&unit).unwrap(),
    );
    let mut record: serde_json::Value =
        serde_json::from_slice(&services.inputs["chapter-helper-record.json"]).unwrap();
    record["index_sha256"] = serde_json::json!(ref_for(&input, "chapter-index.json").sha256);
    record["unit_sha256"] = serde_json::json!(ref_for(&input, "chapter-unit.json").sha256);
    record["checked_files"][path] = serde_json::json!(ref_for(&input, path).sha256);
    record["checked_files"]["chapter-index.json"] =
        serde_json::json!(ref_for(&input, "chapter-index.json").sha256);
    replace_input(
        &mut input,
        &mut services,
        "chapter-helper-record.json",
        json(&record).unwrap(),
    );
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    let report = artifact_json(&output, &services, "chapter-acoustic-validation.json");
    let cellar = report["facts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["original"]["id"] == "cellar-instruction")
        .unwrap();
    assert_eq!(cellar["outcome"], "fail");
    assert!(cellar["chapter_not_before_sample"].is_null());
}
