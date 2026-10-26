use super::*;
use async_trait::async_trait;
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

struct OfflineServices {
    substitute_path: bool,
    cancel_on_emit: bool,
    cancelled: AtomicBool,
    inputs: BTreeMap<String, Vec<u8>>,
    writes: Mutex<Vec<(CandidateFile, Vec<u8>)>>,
}
#[async_trait]
impl RunServices for OfflineServices {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        self.inputs
            .get(&a.file.path)
            .cloned()
            .ok_or_else(|| ServiceError::MissingArtifact(a.id.clone()))
    }
    async fn write_candidate(
        &self,
        r: CandidateWrite,
        bytes: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        assert!(bytes.len() as u64 <= r.max_bytes);
        let candidate = CandidateFile {
            candidate_id: r.candidate_id,
            provenance: r.provenance,
            file: FileRef {
                path: if self.substitute_path {
                    "substituted.json".into()
                } else {
                    r.relative_path
                },
                sha256: digest(&bytes),
                bytes: bytes.len() as u64,
                media_type: r.media_type,
                dimensions: None,
            },
        };
        self.writes.lock().unwrap().push((candidate.clone(), bytes));
        Ok(candidate)
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        if self.cancel_on_emit {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err(ServiceError::Cancelled)
        } else {
            Ok(())
        }
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("offline provider call")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("unregistered helper")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        Ok(vec![])
    }
}
fn fixture() -> (StageInput, OfflineServices) {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/shared/two-scene-v1");
    let doc: PlaybackDocument =
        serde_json::from_slice(&std::fs::read(root.join("fixture.json")).unwrap()).unwrap();
    let mut inputs = BTreeMap::new();
    let mut refs = vec![];
    let mut files: Vec<FileRef> = doc
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(doc.manifest.assets.iter())
        .cloned()
        .collect();
    for path in ["fixture.json", "profile.json"] {
        let bytes = std::fs::read(root.join(path)).unwrap();
        files.push(FileRef {
            path: path.into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        });
    }
    for file in files {
        inputs.insert(
            file.path.clone(),
            std::fs::read(root.join(&file.path)).unwrap(),
        );
        refs.push(ArtifactRef {
            id: format!("input:{}", file.path),
            file,
            provenance: Provenance::Imported,
        });
    }
    let profile_sha256 = digest(inputs.get("profile.json").unwrap());
    (
        StageInput {
            schema_version: 1,
            run_id: "offline-run".into(),
            stage: Stage::Package,
            unit_id: "saved-excerpt".into(),
            epoch: 1,
            revision: 1,
            inputs: refs,
            pins: Pins {
                recipe_sha256: digest(b"generation-diagnostic-v1"),
                profile_sha256,
                profile_id: doc.shell.profile.id.clone(),
            },
            mode: ExecutionMode::Replay,
            limits: Limits::default(),
            library_revision: 0,
        },
        OfflineServices {
            substitute_path: false,
            cancel_on_emit: false,
            cancelled: AtomicBool::new(false),
            inputs,
            writes: Mutex::new(vec![]),
        },
    )
}
#[tokio::test]
async fn diagnostic_candidate_uses_saved_payloads_and_exposes_missing_acceptance() {
    let (input, services) = fixture();
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    let package = output.package_candidate.unwrap();
    assert_eq!(package.provenance, Provenance::Diagnostic);
    assert!(package.findings.iter().any(|f|f.id=="independent-acoustic-evidence" && f.outcome==ReviewOutcome::Missing));
    let writes = services.writes.lock().unwrap();
    let get = |path: &str| writes.iter().find(|(a, _)| a.file.path == path).unwrap();
    let inventory = &get("inventory.json").1;
    assert_eq!(digest(inventory), package.inventory_sha256);
    let acceptance: Acceptance = serde_json::from_slice(&get("acceptance.json").1).unwrap();
    assert_eq!(acceptance.inventory_sha256, package.inventory_sha256);
    assert!(
        acceptance
            .attestations
            .iter()
            .filter(|a| a.kind == EvidenceKind::IndependentAnchor)
            .all(|a| a.provenance == Provenance::Synthetic)
    );
    let manifest: Manifest = serde_json::from_slice(&get("manifest.json").1).unwrap();
    assert_eq!(manifest.acceptance, get("acceptance.json").0.file);
    package
        .manifest
        .file
        .verify(&get("manifest.json").1)
        .unwrap();
    let doc: PlaybackDocument = serde_json::from_slice(&get("diagnostic-document.json").1).unwrap();
    validate_playback(&doc).unwrap();
    assert!(!production_blockers(&doc).is_empty());
    for f in &package.payload_inventory {
        f.verify(services.inputs.get(&f.path).unwrap()).unwrap();
    }
    if let Ok(path) = std::env::var("PT_GENERATION_EXPORT") {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        for f in &package.payload_inventory {
            let path = root.join(&f.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, services.inputs.get(&f.path).unwrap()).unwrap();
        }
        for (f, b) in writes.iter() {
            let path = root.join(&f.file.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b).unwrap();
        }
        std::fs::write(
            root.join("package-candidate.json"),
            serde_json::to_vec_pretty(&package).unwrap(),
        )
        .unwrap();
        verify_package_files(&doc, &root).unwrap();
    }
}

#[tokio::test]
async fn source_stage_freezes_exact_cached_prose_and_scalar_mapping_through_services() {
    for number in [55, 36] {
        let (mut input, mut services) = fixture();
        input.stage = Stage::Source;
        input.inputs.clear();
        services.inputs.clear();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/generation/source");
        for (suffix, path) in [
            ("frozen.json", "frozen-source.json"),
            ("raw.txt", "raw-source.txt"),
            ("clean.txt", "clean-source.txt"),
        ] {
            let bytes = std::fs::read(root.join(format!("pg{number}-{suffix}"))).unwrap();
            input.inputs.push(ArtifactRef {
                id: format!("source:{path}"),
                file: FileRef {
                    path: path.into(),
                    sha256: digest(&bytes),
                    bytes: bytes.len() as u64,
                    media_type: if path.ends_with("json") {
                        "application/json"
                    } else {
                        "text/plain"
                    }
                    .into(),
                    dimensions: None,
                },
                provenance: Provenance::Imported,
            });
            services.inputs.insert(path.into(), bytes);
        }
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(output.outcome, StageOutcome::Complete);
        let writes = services.writes.lock().unwrap();
        let source: serde_json::Value = serde_json::from_slice(&writes[0].1).unwrap();
        let frozen: serde_json::Value =
            serde_json::from_slice(services.inputs.get("frozen-source.json").unwrap()).unwrap();
        assert_eq!(source["source"], frozen);
        assert_eq!(source["inputs"].as_array().unwrap().len(), 3);
    }
}

#[tokio::test]
async fn existing_failures_uncertainty_and_extra_required_checks_remain_blocking() {
    let (mut input, mut services) = fixture();
    let mut doc: PlaybackDocument =
        serde_json::from_slice(services.inputs.get("fixture.json").unwrap()).unwrap();
    for finding in &mut doc.acceptance.findings {
        if finding.id == "narrator-continuity" {
            finding.outcome = ReviewOutcome::Fail;
            finding.message = "Measured continuity failed".into();
        }
        if finding.id == "story-coverage" {
            finding.outcome = ReviewOutcome::Uncertain;
        }
    }
    doc.acceptance
        .required_check_ids
        .push("additional-book-specific-check".into());
    let bytes = serde_json::to_vec(&doc).unwrap();
    let reference = input
        .inputs
        .iter_mut()
        .find(|a| a.file.path == "fixture.json")
        .unwrap();
    reference.file.sha256 = digest(&bytes);
    reference.file.bytes = bytes.len() as u64;
    services.inputs.insert("fixture.json".into(), bytes);
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(
        output
            .findings
            .iter()
            .find(|f| f.id == "narrator-continuity")
            .unwrap()
            .outcome,
        ReviewOutcome::Fail
    );
    assert_eq!(
        output
            .findings
            .iter()
            .find(|f| f.id == "story-coverage")
            .unwrap()
            .outcome,
        ReviewOutcome::Uncertain
    );
    assert!(
        output.findings.iter().any(
            |f| f.id == "additional-book-specific-check" && f.outcome == ReviewOutcome::Missing
        )
    );
}

#[tokio::test]
async fn rejects_stale_bytes_wrong_pins_substituted_candidates_and_late_cancellation() {
    let (input, mut services) = fixture();
    services.inputs.get_mut("audio/master.wav").unwrap()[100] ^= 1;
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    assert!(services.writes.lock().unwrap().is_empty());
    let (mut input, services) = fixture();
    input.pins.profile_sha256 = "0".repeat(64);
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    let (input, mut services) = fixture();
    services.substitute_path = true;
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
    let (input, mut services) = fixture();
    services.cancel_on_emit = true;
    assert!(matches!(
        GenerationExecutor.execute(input, &services).await,
        Err(StageError::Service(ServiceError::Cancelled))
    ));
}
