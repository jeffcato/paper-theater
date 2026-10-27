use super::*;
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

const MASTER_CAP: u64 = 256 * 1024 * 1024;
const CHAPTER_BYTES: u64 = 34_560_044;

struct CapacityServices {
    base: OfflineServices,
    master_path: PathBuf,
    reads: Mutex<Vec<String>>,
    stop_master: bool,
    cancel_read: bool,
    cancel_write: bool,
}
#[async_trait]
impl RunServices for CapacityServices {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        self.reads.lock().unwrap().push(a.file.path.clone());
        let bytes = if a.file.path == "audio/master.wav" {
            if self.stop_master {
                return Err(ServiceError::Cancelled);
            }
            std::fs::read(&self.master_path).unwrap()
        } else {
            self.base.read_artifact(a).await?
        };
        if self.cancel_read && a.file.path == "audio/master.wav" {
            self.base.cancelled.store(true, Ordering::SeqCst);
        }
        Ok(bytes)
    }
    async fn write_candidate(
        &self,
        r: CandidateWrite,
        bytes: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        let candidate = self.base.write_candidate(r, bytes).await?;
        if self.cancel_write {
            self.base.cancelled.store(true, Ordering::SeqCst);
        }
        Ok(candidate)
    }
    async fn emit(&self, p: JournalProposal) -> Result<(), ServiceError> {
        self.base.emit(p).await
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        self.base.check_cancelled().await
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("offline capacity provider call")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("capacity must not dispatch a helper")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("capacity must not access the library")
    }
}
fn wav_header(total: u64) -> [u8; 44] {
    let mut h = [0; 44];
    h[..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&((total - 8) as u32).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&2u16.to_le_bytes());
    h[24..28].copy_from_slice(&48000u32.to_le_bytes());
    h[28..32].copy_from_slice(&192000u32.to_le_bytes());
    h[32..34].copy_from_slice(&4u16.to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&((total - 44) as u32).to_le_bytes());
    h
}
fn file_digest(path: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    let mut f = std::fs::File::open(path).unwrap();
    let mut b = [0; 65536];
    loop {
        let n = f.read(&mut b).unwrap();
        if n == 0 {
            break;
        }
        hash.update(&b[..n]);
    }
    format!("{:x}", hash.finalize())
}
fn replace(
    input: &mut StageInput,
    services: &mut OfflineServices,
    path: &str,
    bytes: Vec<u8>,
) -> FileRef {
    let file = FileRef {
        path: path.into(),
        sha256: digest(&bytes),
        bytes: bytes.len() as u64,
        media_type: "application/json".into(),
        dimensions: None,
    };
    for a in input.inputs.iter_mut().filter(|a| a.file.path == path) {
        a.file = file.clone();
    }
    services.inputs.insert(path.into(), bytes);
    file
}
fn bind_document(
    input: &mut StageInput,
    services: &mut OfflineServices,
    mut doc: PlaybackDocument,
) {
    doc.manifest.entries.timeline = replace(
        input,
        services,
        "timeline.json",
        json(&doc.timeline).unwrap(),
    );
    doc.manifest.entries.read_along = replace(
        input,
        services,
        "read-along.json",
        json(&doc.read_along).unwrap(),
    );
    replace(input, services, "fixture.json", json(&doc).unwrap());
}
fn chapter_fixture() -> (StageInput, CapacityServices, tempfile::TempDir) {
    let (mut input, mut base) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("chapter.wav");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(&wav_header(CHAPTER_BYTES)).unwrap();
    f.set_len(CHAPTER_BYTES).unwrap();
    f.seek(SeekFrom::End(-4)).unwrap();
    f.write_all(&[1, 0, 2, 0]).unwrap();
    drop(f);
    let mut doc: PlaybackDocument = parse(base.inputs.get("fixture.json").unwrap()).unwrap();
    doc.manifest.entries.audio.sha256 = file_digest(&path);
    doc.manifest.entries.audio.bytes = CHAPTER_BYTES;
    let frames = (CHAPTER_BYTES - 44) / 4;
    doc.manifest.sample_count = frames;
    doc.timeline.sample_count = frames;
    doc.timeline.closing.terminal_sample = frames;
    doc.timeline.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    doc.read_along.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    for reveal in &mut doc.manifest.reveals {
        reveal.not_before_sample = None;
    }
    doc.timeline.cues.clear();
    doc.acceptance.findings.push(missing("capacity-synthetic-master", "Synthetic capacity media has no verified narration-to-presentation mapping or production reviews."));
    for a in input
        .inputs
        .iter_mut()
        .filter(|a| a.file.path == "audio/master.wav")
    {
        a.file = doc.manifest.entries.audio.clone();
        a.provenance = Provenance::Synthetic;
    }
    base.inputs.remove("audio/master.wav");
    bind_document(&mut input, &mut base, doc);
    (
        input,
        CapacityServices {
            base,
            master_path: path,
            reads: Mutex::new(vec![]),
            stop_master: false,
            cancel_read: false,
            cancel_write: false,
        },
        temp,
    )
}
fn output_doc(services: &CapacityServices) -> PlaybackDocument {
    let writes = services.base.writes.lock().unwrap();
    parse(
        &writes
            .iter()
            .find(|(c, _)| c.file.path == "diagnostic-document.json")
            .unwrap()
            .1,
    )
    .unwrap()
}
#[tokio::test]
async fn temporary_180_second_master_survives_generation_package_and_recorded_replay() {
    let (input, services, _temp) = chapter_fixture();
    let expected = file_digest(&services.master_path);
    let original: ReadAlong = parse(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/shared/two-scene-v1/read-along.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for mode in [ExecutionMode::Fixture, ExecutionMode::Replay] {
        let mut input = input.clone();
        input.mode = mode;
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
        let package = output.package_candidate.unwrap();
        let doc = output_doc(&services);
        assert_eq!(doc.manifest.entries.audio.bytes, CHAPTER_BYTES);
        assert_eq!(doc.manifest.entries.audio.sha256, expected);
        assert_eq!(doc.manifest.sample_count, 8_640_000);
        assert_eq!(doc.timeline.sample_count / 48000, 180);
        assert_eq!(doc.read_along.words.len(), 176);
        assert_eq!(doc.read_along.groups.len(), 170);
        assert_eq!(
            json(&doc.read_along.words).unwrap(),
            json(&original.words).unwrap()
        );
        assert_eq!(
            json(&doc.read_along.groups).unwrap(),
            json(&original.groups).unwrap()
        );
        assert!(
            doc.manifest
                .reveals
                .iter()
                .all(|r| r.not_before_sample.is_none())
        );
        assert!(!production_blockers(&doc).is_empty());
        assert_eq!(
            package
                .payload_inventory
                .iter()
                .find(|f| f.path == "audio/master.wav")
                .unwrap(),
            &doc.manifest.entries.audio
        );
        assert_eq!(file_digest(&services.master_path), expected);
        let exported = tempfile::tempdir().unwrap();
        for file in &package.payload_inventory {
            let path = exported.path().join(&file.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            if file.path == "audio/master.wav" {
                std::fs::copy(&services.master_path, path).unwrap();
            } else {
                std::fs::write(path, &services.base.inputs[&file.path]).unwrap();
            }
        }
        for (candidate, bytes) in services.base.writes.lock().unwrap().iter() {
            if ["inventory.json", "acceptance.json", "manifest.json"]
                .contains(&candidate.file.path.as_str())
            {
                std::fs::write(exported.path().join(&candidate.file.path), bytes).unwrap();
            }
        }
        assert!(!exported.path().join("private").exists());
        assert!(!exported.path().join("source.txt").exists());
        verify_package_files(&doc, exported.path()).unwrap();
    }
    assert_eq!(
        services
            .reads
            .lock()
            .unwrap()
            .iter()
            .filter(|p| *p == "audio/master.wav")
            .count(),
        2
    );
}

fn claim_master(input: &mut StageInput, services: &mut CapacityServices, size: u64) {
    let mut doc: PlaybackDocument =
        parse(services.base.inputs.get("fixture.json").unwrap()).unwrap();
    doc.manifest.entries.audio.bytes = size;
    for a in input
        .inputs
        .iter_mut()
        .filter(|a| a.file.path == "audio/master.wav")
    {
        a.file = doc.manifest.entries.audio.clone();
    }
    bind_document(input, &mut services.base, doc);
}
fn declared(path: &str, bytes: u64, media_type: &str) -> ArtifactRef {
    ArtifactRef {
        id: format!("input:{path}"),
        provenance: Provenance::Synthetic,
        file: FileRef {
            path: path.into(),
            bytes,
            sha256: "0".repeat(64),
            media_type: media_type.into(),
            dimensions: None,
        },
    }
}
#[tokio::test]
async fn aggregate_reservations_reject_before_master_reads_in_any_input_order() {
    for master_first in [true, false] {
        let (mut input, mut services, _temp) = chapter_fixture();
        services.stop_master = true;
        input.inputs.extend([
            declared("extra-a.bin", 32 * 1024 * 1024, "application/octet-stream"),
            declared("extra-b.bin", 32 * 1024 * 1024, "application/octet-stream"),
        ]);
        input
            .inputs
            .sort_by_key(|a| (a.file.path == "audio/master.wav") != master_first);
        assert!(
            matches!(GenerationExecutor.execute(input,&services).await,Err(StageError::Invalid(e)) if e.code=="generation_size")
        );
        assert!(
            !services
                .reads
                .lock()
                .unwrap()
                .iter()
                .any(|p| p == "audio/master.wav")
        );
        assert!(services.base.writes.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn exact_master_cap_is_admitted_once_and_over_cap_is_rejected_before_read() {
    for bytes in [MASTER_CAP, MASTER_CAP + 1] {
        let (mut input, mut services, _temp) = chapter_fixture();
        claim_master(&mut input, &mut services, bytes);
        if bytes == MASTER_CAP {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&services.master_path)
                .unwrap();
            file.write_all(&wav_header(MASTER_CAP)).unwrap();
            file.set_len(MASTER_CAP).unwrap();
            drop(file);
            rebind_wave(&mut input, &mut services);
            let mut doc: PlaybackDocument =
                parse(services.base.inputs.get("fixture.json").unwrap()).unwrap();
            doc.manifest.sample_count = (MASTER_CAP - 44) / 4;
            doc.timeline.sample_count = doc.manifest.sample_count;
            doc.timeline.closing.terminal_sample = doc.manifest.sample_count;
            bind_document(&mut input, &mut services.base, doc);
            let mut reference = input
                .inputs
                .iter()
                .find(|a| a.file.path == "audio/master.wav")
                .unwrap()
                .file
                .clone();
            reference.path = "chapter.wav".into();
            let verified =
                crate::runtime::VerifiedArtifact::open(_temp.path(), &reference).unwrap();
            assert_eq!(verified.bytes, MASTER_CAP);
        }
        services.stop_master = true;
        let master = input
            .inputs
            .iter()
            .find(|a| a.file.path == "audio/master.wav")
            .unwrap()
            .clone();
        input.inputs.insert(0, master);
        let result = GenerationExecutor.execute(input, &services).await;
        if bytes == MASTER_CAP {
            assert!(
                matches!(result, Err(StageError::Service(ServiceError::Cancelled))),
                "exact cap must reach the read service without allocating a cap-sized test buffer"
            );
            assert_eq!(
                services
                    .reads
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|p| *p == "audio/master.wav")
                    .count(),
                1
            );
        } else {
            assert!(matches!(result,Err(StageError::Invalid(e)) if e.code=="generation_size"));
            assert!(
                !services
                    .reads
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|p| p == "audio/master.wav")
            );
        }
    }
}

#[tokio::test]
async fn only_verified_package_selection_can_claim_the_master_budget() {
    for fault in [
        "other-audio",
        "other-json",
        "other-image",
        "mime-only",
        "wrong-master-ref",
        "wrong-mime",
        "metadata-hash",
        "metadata-size",
        "acoustic-stage",
        "input-count",
    ] {
        let (mut input, mut services, _temp) = chapter_fixture();
        services.stop_master = true;
        match fault {
            "other-audio" | "other-json" | "other-image" => {
                let (path, mime) = match fault {
                    "other-audio" => ("audio/other.wav", "audio/wav"),
                    "other-json" => ("other.json", "application/json"),
                    _ => ("other.png", "image/png"),
                };
                let mut a = declared(path, 32 * 1024 * 1024 + 1, mime);
                if fault == "other-image" {
                    a.file.dimensions = Some([1, 1]);
                }
                input.inputs.push(a);
            }
            "mime-only" => {
                input.inputs.retain(|a| a.file.path != "fixture.json");
            }
            "wrong-master-ref" => {
                input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "audio/master.wav")
                    .unwrap()
                    .file
                    .sha256 = "0".repeat(64);
            }
            "wrong-mime" => {
                let mut doc: PlaybackDocument =
                    parse(services.base.inputs.get("fixture.json").unwrap()).unwrap();
                doc.manifest.entries.audio.media_type = "application/octet-stream".into();
                input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "audio/master.wav")
                    .unwrap()
                    .file = doc.manifest.entries.audio.clone();
                bind_document(&mut input, &mut services.base, doc);
            }
            "metadata-hash" => {
                services.base.inputs.get_mut("fixture.json").unwrap()[0] = b' ';
            }
            "metadata-size" => {
                input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "fixture.json")
                    .unwrap()
                    .file
                    .bytes = 32 * 1024 * 1024 + 1;
            }
            "acoustic-stage" => {
                input.stage = Stage::Acoustic;
            }
            "input-count" => {
                while input.inputs.len() < 65 {
                    input.inputs.push(input.inputs[0].clone());
                }
            }
            _ => unreachable!(),
        }
        let result = GenerationExecutor.execute(input, &services).await;
        assert!(result.is_err(), "accepted {fault}");
        assert!(
            !matches!(result, Err(StageError::Service(ServiceError::Cancelled))),
            "attempted the master read for {fault}"
        );
        assert!(
            !services
                .reads
                .lock()
                .unwrap()
                .iter()
                .any(|p| p == "audio/master.wav"),
            "read master for {fault}"
        );
        assert!(services.base.writes.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn duplicate_conflicts_fail_before_reads_and_identical_inputs_read_once() {
    for conflict in ["path-id", "path-hash", "id-path", "provenance"] {
        let (mut input, mut services, _temp) = chapter_fixture();
        services.stop_master = true;
        let mut duplicate = input
            .inputs
            .iter()
            .find(|a| a.file.path == "audio/master.wav")
            .unwrap()
            .clone();
        match conflict {
            "path-id" => duplicate.id = "different-id".into(),
            "path-hash" => duplicate.file.sha256 = "0".repeat(64),
            "id-path" => duplicate.file.path = "different.wav".into(),
            "provenance" => duplicate.provenance = Provenance::Imported,
            _ => unreachable!(),
        }
        input.inputs.push(duplicate);
        assert!(
            matches!(GenerationExecutor.execute(input,&services).await,Err(StageError::Invalid(e)) if e.code=="generation_duplicate"),
            "accepted {conflict}"
        );
        assert!(services.reads.lock().unwrap().is_empty());
    }
    let (input, services, _temp) = chapter_fixture();
    GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(
        services
            .reads
            .lock()
            .unwrap()
            .iter()
            .filter(|p| *p == "profile.json")
            .count(),
        1
    );
}

fn rebind_wave(input: &mut StageInput, services: &mut CapacityServices) {
    let mut doc: PlaybackDocument =
        parse(services.base.inputs.get("fixture.json").unwrap()).unwrap();
    doc.manifest.entries.audio.sha256 = file_digest(&services.master_path);
    doc.manifest.entries.audio.bytes = std::fs::metadata(&services.master_path).unwrap().len();
    doc.timeline.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    doc.read_along.delivery_audio_sha256 = doc.manifest.entries.audio.sha256.clone();
    for a in input
        .inputs
        .iter_mut()
        .filter(|a| a.file.path == "audio/master.wav")
    {
        a.file = doc.manifest.entries.audio.clone();
    }
    bind_document(input, &mut services.base, doc);
}
#[tokio::test]
async fn delivery_wav_requires_complete_pcm_structure_hash_length_and_exact_frames() {
    for fault in [
        "riff",
        "truncated",
        "chunk-extent",
        "float",
        "extensible",
        "mono",
        "rate",
        "duration",
        "wrong-hash",
        "wrong-length",
        "fake-wav",
    ] {
        let (mut input, mut services, _temp) = chapter_fixture();
        let mut h = wav_header(CHAPTER_BYTES);
        match fault {
            "riff" => h[..4].copy_from_slice(b"RF64"),
            "truncated" | "wrong-length" => {
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&services.master_path)
                    .unwrap()
                    .set_len(CHAPTER_BYTES - 4)
                    .unwrap();
            }
            "chunk-extent" => h[40..44].copy_from_slice(&(CHAPTER_BYTES as u32).to_le_bytes()),
            "float" => h[20..22].copy_from_slice(&3u16.to_le_bytes()),
            "extensible" => h[20..22].copy_from_slice(&65534u16.to_le_bytes()),
            "mono" => {
                h[22..24].copy_from_slice(&1u16.to_le_bytes());
                h[28..32].copy_from_slice(&96000u32.to_le_bytes());
                h[32..34].copy_from_slice(&2u16.to_le_bytes());
            }
            "rate" => {
                h[24..28].copy_from_slice(&44100u32.to_le_bytes());
                h[28..32].copy_from_slice(&176400u32.to_le_bytes());
            }
            "duration" => {
                let mut doc: PlaybackDocument =
                    parse(services.base.inputs.get("fixture.json").unwrap()).unwrap();
                doc.manifest.sample_count += 1;
                doc.timeline.sample_count += 1;
                doc.timeline.closing.terminal_sample += 1;
                bind_document(&mut input, &mut services.base, doc);
            }
            "wrong-hash" => {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(&services.master_path)
                    .unwrap();
                file.seek(SeekFrom::End(-1)).unwrap();
                file.write_all(&[1]).unwrap();
            }
            "fake-wav" => h[..12].copy_from_slice(b"{\"not\":true}"),
            _ => unreachable!(),
        }
        std::fs::OpenOptions::new()
            .write(true)
            .open(&services.master_path)
            .unwrap()
            .write_all(&h)
            .unwrap();
        if !matches!(fault, "wrong-hash" | "wrong-length") {
            rebind_wave(&mut input, &mut services);
        }
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "accepted {fault}"
        );
        assert!(
            services.base.writes.lock().unwrap().is_empty(),
            "proposed a package for {fault}"
        );
    }
}

#[tokio::test]
async fn cancellation_stops_before_after_reads_and_after_candidate_write() {
    for fault in ["before-read", "after-read", "after-write"] {
        let (input, mut services, _temp) = chapter_fixture();
        match fault {
            "before-read" => services.base.cancelled.store(true, Ordering::SeqCst),
            "after-read" => services.cancel_read = true,
            "after-write" => services.cancel_write = true,
            _ => unreachable!(),
        }
        assert!(matches!(
            GenerationExecutor.execute(input, &services).await,
            Err(StageError::Service(ServiceError::Cancelled))
        ));
        if fault == "before-read" {
            assert!(services.reads.lock().unwrap().is_empty());
        }
        assert_eq!(
            services.base.writes.lock().unwrap().len(),
            usize::from(fault == "after-write")
        );
    }
}

#[tokio::test]
async fn master_and_other_aggregate_budgets_have_separate_exact_boundaries() {
    for excess in [0, 1] {
        let (mut input, mut services, _temp) = chapter_fixture();
        claim_master(&mut input, &mut services, MASTER_CAP);
        services.stop_master = true;
        let unique = input
            .inputs
            .iter()
            .map(|a| (a.file.path.clone(), a.file.bytes))
            .collect::<BTreeMap<_, _>>();
        let other = unique
            .iter()
            .filter(|(p, _)| p.as_str() != "audio/master.wav")
            .map(|(_, b)| *b)
            .sum::<u64>();
        let remaining = 64 * 1024 * 1024 - other;
        input.inputs.extend([
            declared("extras/a.bin", 32 * 1024 * 1024, "application/octet-stream"),
            declared(
                "extras/b.bin",
                remaining - 32 * 1024 * 1024 + excess,
                "application/octet-stream",
            ),
        ]);
        let result = GenerationExecutor.execute(input, &services).await;
        if excess == 0 {
            assert!(
                matches!(result, Err(StageError::Service(ServiceError::Cancelled))),
                "256 MiB master + exactly 64 MiB other data must reach the selected read"
            );
        } else {
            assert!(matches!(result,Err(StageError::Invalid(e)) if e.code=="generation_size"));
            assert!(
                !services
                    .reads
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|p| p == "audio/master.wav")
            );
        }
    }
}
