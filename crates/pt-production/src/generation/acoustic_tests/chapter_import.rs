use super::*;

fn fixture() -> (StageInput, ProtocolServices, tempfile::TempDir) {
    fixture_variant("")
}
fn fixture_variant(variant: &str) -> (StageInput, ProtocolServices, tempfile::TempDir) {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = tempfile::tempdir().unwrap();
    let result = std::process::Command::new("/usr/bin/python3")
        .args(["-B", "-c", "import sys;from pathlib import Path;sys.path.insert(0,sys.argv[1]);from test_chapter_import import build;build(Path(sys.argv[2]),sys.argv[3])"])
        .arg(project.join("tools/generation"))
        .arg(temp.path())
        .arg(variant)
        .output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let root = temp.path().join("bundle");
    let unit: serde_json::Value =
        parse(&std::fs::read(root.join("units/chunk-0000.json")).unwrap()).unwrap();
    let (mut input, mut services) = protocol_fixture(ExecutionMode::Replay, "");
    input.inputs.clear();
    services.inputs.clear();
    for value in unit["files"].as_array().unwrap() {
        let file: FileRef = serde_json::from_value(value.clone()).unwrap();
        services.inputs.insert(
            file.path.clone(),
            std::fs::read(root.join(&file.path)).unwrap(),
        );
        input.inputs.push(ArtifactRef {
            id: format!("import:{}", file.path),
            file,
            provenance: Provenance::Synthetic,
        });
    }
    for (alias, path) in [
        ("chapter-import-unit.json", "units/chunk-0000.json"),
        (
            "chapter-import-helper-record.json",
            "records/chunk-0000.json",
        ),
    ] {
        let bytes = std::fs::read(root.join(path)).unwrap();
        input.inputs.push(ArtifactRef {
            id: format!("import:{alias}"),
            file: FileRef {
                path: alias.into(),
                sha256: digest(&bytes),
                bytes: bytes.len() as u64,
                media_type: "application/json".into(),
                dimensions: None,
            },
            provenance: Provenance::Diagnostic,
        });
        services.inputs.insert(alias.into(), bytes);
    }
    (input, services, temp)
}

#[tokio::test]
async fn generic_complete_chapter_replays_with_global_facts_and_zero_dispatch() {
    let (mut input, mut services, _temp) = fixture();
    let out = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    let report = artifact_json(&out, &services, "private/chapter-import-validation.json");
    assert_eq!(report["verified_import"]["complete_source_coverage"], true);
    assert_eq!(report["verified_import"]["word_count"], 16);
    assert_eq!(
        report["verified_import"]["facts"][0]["not_before_sample"],
        8000
    );
    assert!(out.package_candidate.is_none());
    assert!(out.library_updates.is_empty());
    if let Ok(destination) = std::env::var("PT_CHAPTER_IMPORT_EXPORT") {
        assert!(safe_path(&destination) && destination.starts_with("fixtures/generation/"));
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(destination);
        fn copy_tree(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let target = to.join(entry.file_name());
                if path.is_dir() {
                    copy_tree(&path, &target);
                } else {
                    assert!(entry.metadata().unwrap().len() <= 1024 * 1024);
                    std::fs::copy(path, target).unwrap();
                }
            }
        }
        copy_tree(_temp.path(), &root.join("producer-inputs"));
        std::fs::write(root.join("stage-input.json"), json(&input).unwrap()).unwrap();
        std::fs::write(root.join("stage-output.json"), json(&out).unwrap()).unwrap();
        for a in &input.inputs {
            let p = root.join("stage").join(&a.file.path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, &services.inputs[&a.file.path]).unwrap();
        }
        for c in &out.artifacts {
            let p = root.join("stage").join(&c.file.path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, &services.candidates.lock().unwrap()[&c.candidate_id].1).unwrap();
        }
    }
    let evidence: AcceptanceEvidenceInventory = serde_json::from_slice(
        &services.candidates.lock().unwrap()[&out
            .artifacts
            .iter()
            .find(|c| c.file.path == "private/chapter-import-evidence-inventory.json")
            .unwrap()
            .candidate_id]
            .1,
    )
    .unwrap();
    assert!(evidence.proposals.is_empty());
    let original = out
        .artifacts
        .iter()
        .map(|c| {
            (
                c.clone(),
                services.candidates.lock().unwrap()[&c.candidate_id]
                    .1
                    .clone(),
            )
        })
        .collect::<Vec<_>>();
    for (c, b) in &original {
        input.inputs.push(ArtifactRef {
            id: format!("saved:{}", c.file.path),
            file: c.file.clone(),
            provenance: c.provenance,
        });
        services.inputs.insert(c.file.path.clone(), b.clone());
    }
    services.candidates.lock().unwrap().clear();
    let again = GenerationExecutor.execute(input, &services).await.unwrap();
    for (c, b) in original {
        assert_eq!(
            services.candidates.lock().unwrap()[&again
                .artifacts
                .iter()
                .find(|a| a.file.path == c.file.path)
                .unwrap()
                .candidate_id]
                .1,
            b
        );
    }
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.candidate_reads.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn generic_replay_retains_missing_dependencies_global_ambiguity_and_original_adverse_findings()
 {
    let mut cache_keys = std::collections::BTreeSet::new();
    for (variant, outcome, bound) in [
        ("missing", "missing", None),
        ("cut", "missing", None),
        ("ambiguous", "uncertain", None),
        ("adverse", "fail", None),
        ("precision", "pass", Some(2001)),
        ("extra", "uncertain", None),
        ("tolerance-equal", "pass", Some(8000)),
        ("tolerance-outside", "uncertain", None),
    ] {
        let (input, services, _temp) = fixture_variant(variant);
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert!(
            cache_keys.insert(out.cache_keys.clone()),
            "distinct source/evidence identities shared a cache key"
        );
        let report = artifact_json(&out, &services, "private/chapter-import-validation.json");
        assert_eq!(
            report["verified_import"]["facts"][0]["outcome"], outcome,
            "{variant}"
        );
        assert_eq!(
            report["verified_import"]["facts"][0]["not_before_sample"],
            serde_json::json!(bound),
            "{variant}"
        );
        if variant == "adverse" {
            assert!(out.findings.iter().any(|f| f.id == "acoustic-fact:gate"
                && f.outcome == ReviewOutcome::Fail
                && f.message == "retained original failure"));
        }
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn generic_1200_second_native_master_is_verified_through_bounded_unit_replay() {
    let (input, services, temp) = fixture_variant("large");
    assert!(
        std::fs::metadata(temp.path().join("chapter.wav"))
            .unwrap()
            .len()
            > 16 * 1024 * 1024
    );
    assert!(
        input.inputs.len() <= 64
            && input
                .inputs
                .iter()
                .all(|a| a.file.bytes <= 16 * 1024 * 1024)
    );
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let report = artifact_json(&out, &services, "private/chapter-import-validation.json");
    assert_eq!(report["verified_import"]["word_count"], 16);
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn generic_changed_records_and_stale_private_replay_are_rejected_before_output() {
    for fault in ["record", "recipe", "raw-bytes", "source", "pcm", "replay"] {
        let (mut input, mut services, _temp) = fixture();
        match fault {
            "record" => {
                let mut v: serde_json::Value =
                    parse(&services.inputs["chapter-import-helper-record.json"]).unwrap();
                v["facts"][0]["not_before_sample"] = serde_json::json!(1);
                replace_input(
                    &mut input,
                    &mut services,
                    "chapter-import-helper-record.json",
                    json(&v).unwrap(),
                );
            }
            "recipe" => {
                let mut v: serde_json::Value =
                    parse(&services.inputs["chapter-import-index.json"]).unwrap();
                v["recipe_sha256"] = serde_json::json!("0".repeat(64));
                replace_input(
                    &mut input,
                    &mut services,
                    "chapter-import-index.json",
                    json(&v).unwrap(),
                );
            }
            "raw-bytes" => {
                let mut v: serde_json::Value =
                    parse(&services.inputs["private/chapter-context.json"]).unwrap();
                v["raw"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                    .next()
                    .map(|v| *v = serde_json::json!("AAAA"));
                replace_input(
                    &mut input,
                    &mut services,
                    "private/chapter-context.json",
                    json(&v).unwrap(),
                );
            }
            "source" => {
                services.inputs.get_mut("private/source.txt").unwrap()[0] ^= 1;
            }
            "pcm" => {
                services
                    .inputs
                    .get_mut("private/chunks/chunk-0000.wav")
                    .unwrap()[44] ^= 1;
            }
            "replay" => {
                let out = GenerationExecutor
                    .execute(input.clone(), &services)
                    .await
                    .unwrap();
                let c = out.artifacts[0].clone();
                let mut v: serde_json::Value =
                    parse(&services.candidates.lock().unwrap()[&c.candidate_id].1).unwrap();
                v["implementation_sha256"] = serde_json::json!("0".repeat(64));
                input.inputs.push(ArtifactRef {
                    id: "stale:import".into(),
                    file: c.file,
                    provenance: c.provenance,
                });
                services.inputs.insert(
                    "private/chapter-import-validation.json".into(),
                    json(&v).unwrap(),
                );
                let b = services.inputs["private/chapter-import-validation.json"].clone();
                replace_input(
                    &mut input,
                    &mut services,
                    "private/chapter-import-validation.json",
                    b,
                );
            }
            _ => unreachable!(),
        }
        services.candidates.lock().unwrap().clear();
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{fault}"
        );
        assert!(services.candidates.lock().unwrap().is_empty());
        assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn missing_unregistered_import_record_cannot_trigger_dispatch() {
    let (mut input, mut services, _temp) = fixture();
    input
        .inputs
        .retain(|a| a.file.path != "chapter-import-helper-record.json");
    services.inputs.remove("chapter-import-helper-record.json");
    for mode in [
        ExecutionMode::Replay,
        ExecutionMode::Fixture,
        ExecutionMode::Live,
    ] {
        input.mode = mode;
        let out = GenerationExecutor
            .execute(input.clone(), &services)
            .await
            .unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(out.artifacts.is_empty());
    }
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}

struct NoReads;
#[tokio::test]
async fn imported_unit_rejects_inputs_outside_its_exact_private_allowlist() {
    let (mut input, mut services, _temp) = fixture();
    let bytes = b"{}".to_vec();
    input.inputs.push(ArtifactRef {
        id: "extra:unlisted".into(),
        file: FileRef {
            path: "private/unlisted.json".into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        },
        provenance: Provenance::Synthetic,
    });
    services
        .inputs
        .insert("private/unlisted.json".into(), bytes);
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    assert!(services.candidates.lock().unwrap().is_empty());
}
#[tokio::test]
async fn many_segments_reserve_all_private_replay_references_and_metadata() {
    let (mut input, mut services, _temp) = fixture_variant("many");
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    for c in &first.artifacts {
        let b = services.candidates.lock().unwrap()[&c.candidate_id]
            .1
            .clone();
        input.inputs.push(ArtifactRef {
            id: format!("saved:{}", c.file.path),
            file: c.file.clone(),
            provenance: c.provenance,
        });
        services.inputs.insert(c.file.path.clone(), b);
    }
    assert!(
        input.inputs.len() <= 64,
        "producer did not reserve all replay slots"
    );
    services.candidates.lock().unwrap().clear();
    let again = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(again.outcome, StageOutcome::NeedsRepair);
    assert_eq!(services.helper_calls.load(Ordering::SeqCst), 0);
}
#[async_trait]
impl RunServices for NoReads {
    async fn read_artifact(&self, _: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        panic!("oversized envelope performed a payload read")
    }
    async fn write_candidate(
        &self,
        _: CandidateWrite,
        _: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        panic!("oversized write")
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        Ok(())
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        panic!("oversized emit")
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("provider")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("helper")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("library")
    }
}
#[tokio::test]
async fn generic_limits_reject_before_any_service_read_even_with_forged_metadata_mime() {
    for fault in ["metadata", "replay-metadata", "file", "aggregate", "count"] {
        let (mut input, _) = protocol_fixture(ExecutionMode::Replay, "");
        input.inputs.clear();
        let artifact = |path: &str, bytes: u64, media: &str| ArtifactRef {
            id: format!("limit:{path}"),
            file: FileRef {
                path: path.into(),
                sha256: "0".repeat(64),
                bytes,
                media_type: media.into(),
                dimensions: None,
            },
            provenance: Provenance::Synthetic,
        };
        input
            .inputs
            .push(artifact("chapter-import-unit.json", 1, "application/json"));
        match fault {
            "metadata" => input.inputs.push(artifact(
                "private/chapter-context.json",
                1024 * 1024 + 1,
                "text/plain",
            )),
            "replay-metadata" => input.inputs.push(artifact(
                "private/chapter-import-proposals.json",
                1024 * 1024 + 1,
                "text/plain",
            )),
            "file" => input.inputs.push(artifact(
                "private/oversized.wav",
                16 * 1024 * 1024 + 1,
                "audio/wav",
            )),
            "aggregate" => {
                for i in 0..5 {
                    input.inputs.push(artifact(
                        &format!("private/{i}.wav"),
                        16 * 1024 * 1024,
                        "audio/wav",
                    ));
                }
            }
            "count" => {
                for i in 0..64 {
                    input.inputs.push(artifact(
                        &format!("private/{i}.json"),
                        1,
                        "application/json",
                    ));
                }
            }
            _ => unreachable!(),
        }
        assert!(
            GenerationExecutor.execute(input, &NoReads).await.is_err(),
            "{fault}"
        );
    }
}
