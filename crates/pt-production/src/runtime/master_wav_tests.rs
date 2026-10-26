use super::*;
use async_trait::async_trait;
use pt_contracts as c;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[path = "../../../../fixtures/runtime/chapter_wav.rs"]
mod wav;
fn request() -> serde_json::Value {
    json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":digest(b"chapter-master-v1"),"profile_sha256":digest(b"profile"),"profile_id":"diagnostic"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}})
}
struct Master(Arc<AtomicUsize>);
#[async_trait]
impl c::StageExecutor for Master {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let bytes = wav::chapter();
        let file = services
            .write_candidate(
                c::CandidateWrite {
                    candidate_id: input.unit_id,
                    relative_path: "audio/master.wav".into(),
                    media_type: "audio/wav".into(),
                    max_bytes: 256 * 1024 * 1024,
                    dimensions: None,
                    provenance: c::Provenance::Synthetic,
                },
                bytes,
            )
            .await?;
        file.file.verify(&services.read_candidate(&file).await?)?;
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![file],
            decisions: vec![],
            findings: vec![],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
#[tokio::test]
async fn chapter_master_roundtrips_and_restarts_without_redispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let id = store.create("chapter-master", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = Runtime::new(store.clone(), Arc::new(Master(calls.clone())));
    let a = runtime
        .execute_stage(&id, epoch, c::Stage::MusicMix)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(a.file.bytes, 34_560_044);
    a.file.verify(&store.artifact(&a.id).unwrap().0).unwrap();
    drop(runtime);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let reused = Runtime::new(store.clone(), Arc::new(Master(calls.clone())))
        .execute_stage(&id, epoch, c::Stage::MusicMix)
        .await
        .unwrap();
    assert_eq!(reused[0].file, a.file);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        store
            .journal(&id, 0, 500)
            .unwrap()
            .iter()
            .any(|e| e["type"] == "cache_reused")
    );
    use std::io::{Seek, SeekFrom, Write};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(store.object(&a.file.sha256).unwrap())
        .unwrap();
    file.seek(SeekFrom::End(-1)).unwrap();
    file.write_all(&[9]).unwrap();
    for size in [34_560_044, 44, MAX_MASTER_BYTES as u64 + 4] {
        file.set_len(size).unwrap();
        assert_eq!(
            Runtime::new(store.clone(), Arc::new(Master(calls.clone())))
                .execute_stage(&id, epoch, c::Stage::MusicMix)
                .await
                .unwrap_err()
                .code,
            "corrupt_artifact"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(store.catalog("pg-55").unwrap().is_none());
}

#[test]
fn chapter_wav_envelope_rejects_malformed_and_oversized_files() {
    let valid = [wav::header(48).as_slice(), &[0; 4]].concat();
    validate_artifact_bytes(&valid, "audio/wav").unwrap();
    for (offset, byte) in [
        (0, b'X'),
        (4, 0),
        (20, 3),
        (22, 3),
        (24, 0),
        (28, 1),
        (32, 1),
        (34, 8),
        (40, 0),
    ] {
        let mut bad = valid.clone();
        bad[offset] = byte;
        assert!(
            validate_artifact_bytes(&bad, "audio/wav").is_err(),
            "offset {offset}"
        );
    }
    assert!(validate_artifact_bytes(&valid[..47], "audio/wav").is_err());
    let mut trailing = valid.clone();
    trailing.push(0);
    assert!(validate_artifact_bytes(&trailing, "audio/wav").is_err());
    assert!(validate_artifact_size(MAX_ARTIFACT_BYTES as u64 + 1, "application/json").is_err());
    validate_artifact_size(MAX_MASTER_BYTES as u64, "audio/wav").unwrap();
    assert!(validate_artifact_size(MAX_MASTER_BYTES as u64 + 1, "audio/wav").is_err());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("oversized.wav");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_MASTER_BYTES as u64 + 4).unwrap();
    let reference = c::FileRef {
        path: "oversized.wav".into(),
        sha256: digest(b"unused"),
        bytes: MAX_MASTER_BYTES as u64 + 4,
        media_type: "audio/wav".into(),
        dimensions: None,
    };
    assert_eq!(
        VerifiedArtifact::open(temp.path(), &reference)
            .err()
            .unwrap()
            .code,
        "artifact_too_large"
    );
}

#[test]
fn existing_native_acoustic_pcm_remains_valid_inside_the_small_envelope() {
    let mut bytes = [wav::header(48).as_slice(), &[0; 4]].concat();
    bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
    bytes[24..28].copy_from_slice(&24000u32.to_le_bytes());
    bytes[28..32].copy_from_slice(&48000u32.to_le_bytes());
    bytes[32..34].copy_from_slice(&2u16.to_le_bytes());
    validate_artifact_bytes(&bytes, "audio/wav").unwrap();
}

#[test]
fn chapter_master_accepts_exact_256_mib_without_a_large_read_buffer() {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    let temp = tempfile::tempdir().unwrap();
    let mut file = std::fs::File::create(temp.path().join("boundary.wav")).unwrap();
    file.write_all(&wav::header(MAX_MASTER_BYTES as u64))
        .unwrap();
    file.set_len(MAX_MASTER_BYTES as u64).unwrap();
    let mut hash = Sha256::new();
    hash.update(wav::header(MAX_MASTER_BYTES as u64));
    let zeros = [0u8; 65536];
    let mut remaining = MAX_MASTER_BYTES - 44;
    while remaining > 0 {
        let n = remaining.min(zeros.len());
        hash.update(&zeros[..n]);
        remaining -= n;
    }
    let reference = c::FileRef {
        path: "boundary.wav".into(),
        sha256: format!("{:x}", hash.finalize()),
        bytes: MAX_MASTER_BYTES as u64,
        media_type: "audio/wav".into(),
        dimensions: None,
    };
    let verified = VerifiedArtifact::open(temp.path(), &reference).unwrap();
    assert_eq!(verified.bytes, 268_435_456);
}

struct TightEnvelopes(String);
#[async_trait]
impl c::StageExecutor for TightEnvelopes {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let result = services
            .write_candidate(
                c::CandidateWrite {
                    candidate_id: "oversized-json".into(),
                    relative_path: "report.json".into(),
                    media_type: "application/json".into(),
                    max_bytes: MAX_ARTIFACT_BYTES as u64 + 1,
                    dimensions: None,
                    provenance: c::Provenance::Synthetic,
                },
                b"{}".to_vec(),
            )
            .await;
        assert!(matches!(result, Err(c::ServiceError::Unauthorized(_))));
        let master = input
            .inputs
            .into_iter()
            .find(|a| a.file.media_type == "audio/wav")
            .unwrap();
        let result = services
            .helper(c::HelperOperation {
                id: "oversized-input".into(),
                kind: c::HelperKind::AlignAudio,
                inputs: vec![master],
                recipe_sha256: self.0.clone(),
                timeout_ms: 30000,
                max_output_bytes: 16 * 1024 * 1024,
            })
            .await;
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("artifact_too_large"),
            "storage allowance must not enlarge helper inputs"
        );
        let result = services
            .helper(c::HelperOperation {
                id: "oversized-output".into(),
                kind: c::HelperKind::AlignAudio,
                inputs: vec![],
                recipe_sha256: self.0.clone(),
                timeout_ms: 30000,
                max_output_bytes: 16 * 1024 * 1024 + 1,
            })
            .await;
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("helper_forbidden")
        );
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![],
            decisions: vec![],
            findings: vec![],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
#[tokio::test]
async fn chapter_storage_does_not_expand_json_or_acoustic_helper_envelopes() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let id = store.create("master-bounds", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    Runtime::new(
        store.clone(),
        Arc::new(Master(Arc::new(AtomicUsize::new(0)))),
    )
    .execute_stage(&id, epoch, c::Stage::MusicMix)
    .await
    .unwrap();
    let recipe = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/generation");
    let config = super::helper::OfflineAcoustic::new(recipe.clone()).unwrap();
    Runtime::new(store.clone(), Arc::new(TightEnvelopes(config.sha256)))
        .with_offline_acoustic(recipe)
        .unwrap()
        .execute_stage(&id, epoch, c::Stage::Choreography)
        .await
        .unwrap();
    assert!(
        !store.journal(&id, 0, 500).unwrap().iter().any(|e| matches!(
            e["type"].as_str(),
            Some("helper_completed" | "helper_failed" | "provider_dispatched")
        ))
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
}

struct LateMaster {
    store: Store,
    run: String,
    cancel: bool,
}

struct AggregateBounds;
#[async_trait]
impl c::StageExecutor for AggregateBounds {
    async fn execute(
        &self,
        _: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let bytes = wav::chapter();
        for index in 0..8 {
            let result = services
                .write_candidate(
                    c::CandidateWrite {
                        candidate_id: format!("audio-{index}"),
                        relative_path: format!("audio/{index}.wav"),
                        media_type: "audio/wav".into(),
                        max_bytes: MAX_MASTER_BYTES as u64,
                        dimensions: None,
                        provenance: c::Provenance::Synthetic,
                    },
                    bytes.clone(),
                )
                .await;
            if index < 7 {
                result?;
            } else {
                assert!(
                    matches!(result, Err(c::ServiceError::Limit(_))),
                    "eight individually valid WAVs exceed the aggregate envelope"
                );
            }
        }
        let mut json = vec![b' '; 17 * 1024 * 1024];
        json.extend(b"{}");
        for index in 0..2 {
            let result = services
                .write_candidate(
                    c::CandidateWrite {
                        candidate_id: format!("json-{index}"),
                        relative_path: format!("report-{index}.json"),
                        media_type: "application/json".into(),
                        max_bytes: MAX_ARTIFACT_BYTES as u64,
                        dimensions: None,
                        provenance: c::Provenance::Synthetic,
                    },
                    json.clone(),
                )
                .await;
            if index == 0 {
                result?;
            } else {
                assert!(
                    matches!(result, Err(c::ServiceError::Limit(_))),
                    "non-WAV aggregate stays at 32 MiB"
                );
            }
        }
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![],
            decisions: vec![],
            findings: vec![],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
#[tokio::test]
async fn chapter_candidate_aggregate_allows_a_master_without_unbounding_other_files() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let id = store.create("master-aggregate", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    Runtime::new(store.clone(), Arc::new(AggregateBounds))
        .execute_stage(&id, epoch, c::Stage::MusicMix)
        .await
        .unwrap();
    assert!(store.catalog("pg-55").unwrap().is_none());
}
#[async_trait]
impl c::StageExecutor for LateMaster {
    async fn execute(
        &self,
        input: c::StageInput,
        services: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        let mut output = Master(Arc::new(AtomicUsize::new(0)))
            .execute(input, services)
            .await?;
        if self.cancel {
            self.store.cancel(&self.run).unwrap();
        } else {
            output.decisions.push(c::Decision {
                id: "invalid-decision".into(),
                summary: "invalid proposal forces rollback after object persistence".into(),
                inputs: vec![],
                selected_ids: vec![],
                rejected_ids: vec![],
                supersedes: None,
            });
        }
        Ok(output)
    }
}
#[tokio::test]
async fn chapter_master_cancel_and_failed_commit_never_expose_partial_results() {
    for cancel in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let id = store.create("master-atomic", &request()).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, epoch) = store.claim().unwrap().unwrap();
        let error = Runtime::new(
            store.clone(),
            Arc::new(LateMaster {
                store: store.clone(),
                run: id.clone(),
                cancel,
            }),
        )
        .execute_stage(&id, epoch, c::Stage::MusicMix)
        .await
        .unwrap_err();
        assert_eq!(
            error.code,
            if cancel {
                "stale_epoch"
            } else {
                "invalid_decision"
            }
        );
        let snapshot = store.api_snapshot(&id).unwrap();
        assert!(snapshot.stages.iter().all(|s| s.artifacts.is_empty()));
        assert!(
            !store.journal(&id, 0, 500).unwrap().iter().any(|e| matches!(
                e["type"].as_str(),
                Some(
                    "artifact_ready"
                        | "task_completed"
                        | "library_bound"
                        | "diagnostic_package_ready"
                )
            ))
        );
        assert!(store.catalog("pg-55").unwrap().is_none());
        assert_eq!(
            std::fs::read_dir(temp.path().join("objects"))
                .unwrap()
                .count(),
            1,
            "only an unreferenced immutable object may remain"
        );
    }
}

#[tokio::test]
async fn chapter_master_fixture_import_preserves_inventory_and_private_package_reads() {
    fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy_tree(&e.path(), &to.join(e.file_name()));
            } else {
                std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("fixture");
    copy_tree(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/shared/two-scene-v1"),
        &fixture,
    );
    let mut doc: c::PlaybackDocument =
        serde_json::from_slice(&std::fs::read(fixture.join("fixture.json")).unwrap()).unwrap();
    let bytes = wav::chapter();
    std::fs::write(fixture.join(&doc.manifest.entries.audio.path), &bytes).unwrap();
    doc.manifest.entries.audio.bytes = bytes.len() as u64;
    doc.manifest.entries.audio.sha256 = digest(&bytes);
    // Replacement audio invalidates reveal evidence; retain it as ineligible.
    for reveal in &mut doc.manifest.reveals {
        reveal.not_before_sample = None;
    }
    doc.timeline.cues.clear();
    doc.manifest.sample_count = 8_640_000;
    doc.timeline.sample_count = 8_640_000;
    doc.timeline.closing.terminal_sample = 8_640_000;
    doc.timeline.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    doc.read_along.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    for (file, bytes) in [
        (
            &mut doc.manifest.entries.timeline,
            serde_json::to_vec(&doc.timeline).unwrap(),
        ),
        (
            &mut doc.manifest.entries.read_along,
            serde_json::to_vec(&doc.read_along).unwrap(),
        ),
    ] {
        std::fs::write(fixture.join(&file.path), &bytes).unwrap();
        file.sha256 = digest(&bytes);
        file.bytes = bytes.len() as u64;
    }
    let mut inventory = doc
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(doc.manifest.assets.iter())
        .cloned()
        .collect::<Vec<_>>();
    inventory.sort_by(|a, b| a.path.cmp(&b.path));
    let inventory = serde_json::to_vec(&inventory).unwrap();
    std::fs::write(fixture.join("inventory.json"), &inventory).unwrap();
    doc.acceptance.inventory_sha256 = digest(&inventory);
    let acceptance = serde_json::to_vec(&doc.acceptance).unwrap();
    std::fs::write(fixture.join(&doc.manifest.acceptance.path), &acceptance).unwrap();
    doc.manifest.acceptance.sha256 = digest(&acceptance);
    doc.manifest.acceptance.bytes = acceptance.len() as u64;
    std::fs::write(
        fixture.join("manifest.json"),
        serde_json::to_vec(&doc.manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.join("fixture.json"),
        serde_json::to_vec(&doc).unwrap(),
    )
    .unwrap();
    let store = Store::open(&temp.path().join("store")).unwrap();
    let id = store.create("master-fixture", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store.seed_fixture(&id, epoch, &fixture, None).unwrap();
    let a = store
        .seed_inputs(&id)
        .unwrap()
        .into_iter()
        .find(|a| a.file.path == "audio/master.wav")
        .unwrap();
    assert_eq!(a.file, doc.manifest.entries.audio);
    a.file.verify(&store.artifact(&a.id).unwrap().0).unwrap();
    // An immutable diagnostic mapping may expose bytes, never publish the fixture.
    store
        .transaction(|tx| {
            tx.execute(
                "INSERT INTO diagnostic_packages VALUES(?1,?2,?3,?4,'{}')",
                rusqlite::params![
                    id,
                    epoch,
                    digest(b"manifest"),
                    serde_json::to_string(&std::collections::BTreeMap::from([(
                        "audio/master.wav",
                        &a.id
                    )]))?
                ],
            )?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        store
            .open_package_artifact(&id, "audio/master.wav")
            .unwrap()
            .sha256,
        a.file.sha256
    );
    assert!(store.open_package_artifact(&id, "audio/other.wav").is_err());
    assert!(store.catalog("pg-55").unwrap().is_none());
    assert!(!c::production_blockers(&doc).is_empty());
}
