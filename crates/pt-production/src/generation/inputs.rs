//! Bounded verified inputs; only the package-selected delivery master has a larger budget.
use super::*;

type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
const MAX_MASTER_BYTES: u64 = 256 * 1024 * 1024;
const MAX_OTHER_BYTES: u64 = 64 * 1024 * 1024;

pub(super) struct Loaded {
    pub files: Files,
    pub package: Option<PlaybackDocument>,
}
async fn read(input: &ArtifactRef, services: &dyn RunServices) -> Result<Vec<u8>, StageError> {
    services.check_cancelled().await?;
    let bytes = services.read_artifact(input).await?;
    services.check_cancelled().await?;
    input.file.verify(&bytes)?;
    Ok(bytes)
}
fn delivery(bytes: &[u8], doc: &PlaybackDocument) -> Result<(), StageError> {
    // Public, stateless R11 validation; no runtime storage/transport internals.
    crate::runtime::validate_artifact_bytes(bytes, "audio/wav")
        .map_err(|e| StageError::Evidence(format!("delivery WAV: {}", e.code)))?;
    // Structure/chunk extents were checked above. Bind physical frames to the package domain.
    let mut pos = 12;
    let mut format = None;
    let mut frames = None;
    while pos < bytes.len() {
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        match &bytes[pos..pos + 4] {
            b"fmt " => {
                let p = &bytes[pos + 8..];
                format = Some((
                    u16::from_le_bytes(p[2..4].try_into().unwrap()),
                    u32::from_le_bytes(p[4..8].try_into().unwrap()),
                ));
            }
            b"data" => frames = Some(size as u64 / 4),
            _ => {}
        }
        pos += 8 + size + size % 2;
    }
    require(
        format == Some((2, 48000)) && frames == Some(doc.manifest.sample_count),
        "generation_master_domain",
        &doc.manifest.entries.audio.path,
        "delivery must be PCM16 stereo 48 kHz with exactly the declared package frames",
    )?;
    Ok(())
}

pub(super) async fn load(
    input: &StageInput,
    services: &dyn RunServices,
) -> Result<Loaded, StageError> {
    require(
        input.inputs.len() <= 64,
        "generation_inputs",
        "inputs",
        "too many input artifacts",
    )?;
    let mut unique: BTreeMap<&str, &ArtifactRef> = BTreeMap::new();
    let mut ids: BTreeMap<&str, &ArtifactRef> = BTreeMap::new();
    for artifact in &input.inputs {
        for previous in [
            unique.get(artifact.file.path.as_str()),
            ids.get(artifact.id.as_str()),
        ]
        .into_iter()
        .flatten()
        {
            require(
                previous.id == artifact.id
                    && previous.file == artifact.file
                    && previous.provenance == artifact.provenance,
                "generation_duplicate",
                &artifact.id,
                "conflicting input path or artifact identity",
            )?;
        }
        unique.insert(&artifact.file.path, artifact);
        ids.insert(&artifact.id, artifact);
    }
    let mut files: Files = BTreeMap::new();
    let package = if input.stage == Stage::Package {
        let metadata = unique
            .get("fixture.json")
            .ok_or_else(|| StageError::Evidence("missing package input: fixture.json".into()))?;
        require(
            metadata.file.bytes <= MAX_ARTIFACT_BYTES
                && metadata.file.media_type == "application/json",
            "generation_size",
            "fixture.json",
            "bounded JSON package metadata required",
        )?;
        let bytes = read(metadata, services).await?;
        let doc: PlaybackDocument = parse(&bytes)?;
        validate_playback(&doc)?;
        let selected = &doc.manifest.entries.audio;
        require(
            selected.media_type == "audio/wav"
                && selected.dimensions.is_none()
                && unique
                    .get(selected.path.as_str())
                    .is_some_and(|a| a.file == *selected),
            "generation_master_identity",
            &selected.path,
            "selected delivery master must match an exact verified input reference",
        )?;
        files.insert(metadata.file.path.clone(), (metadata.file.clone(), bytes));
        Some(doc)
    } else {
        None
    };
    let selected = package.as_ref().map(|d| &d.manifest.entries.audio);
    let mut other = 0u64;
    let mut master = 0u64;
    for artifact in unique.values() {
        let is_master = selected == Some(&artifact.file);
        let limit = if is_master {
            MAX_MASTER_BYTES
        } else {
            MAX_ARTIFACT_BYTES
        };
        require(
            artifact.file.bytes <= limit,
            "generation_size",
            &artifact.id,
            "input exceeds bound",
        )?;
        if is_master {
            master = master.saturating_add(artifact.file.bytes);
        } else {
            other = other.saturating_add(artifact.file.bytes);
        }
        require(
            master <= MAX_MASTER_BYTES && other <= MAX_OTHER_BYTES,
            "generation_size",
            "inputs",
            "separate master/non-master aggregate exceeded",
        )?;
    }
    // Reserve the complete envelope before any payload read, independent of input order.
    for artifact in unique.values() {
        let is_master = selected == Some(&artifact.file);
        if let Some((old, _)) = files.get(&artifact.file.path) {
            require(
                old == &artifact.file,
                "generation_duplicate",
                &artifact.id,
                "conflicting artifact path",
            )?;
            continue;
        }
        let bytes = read(artifact, services).await?;
        if is_master {
            delivery(&bytes, package.as_ref().unwrap())?;
        }
        files.insert(artifact.file.path.clone(), (artifact.file.clone(), bytes));
    }
    Ok(Loaded { files, package })
}
