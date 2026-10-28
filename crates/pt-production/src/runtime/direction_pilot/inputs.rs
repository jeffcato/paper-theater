use super::*;
use std::io::Read;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub artifact: c::ArtifactRef,
    pub source: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Registration {
    pub schema_version: u32,
    pub recipe_sha256: String,
    pub book_id: String,
    pub chapter_id: String,
    pub profile: c::Pins,
    pub source_pins: c::Pins,
    pub library_revision: u64,
    pub files: Vec<Entry>,
    pub private_only: bool,
    pub bulk_speech_approved: bool,
    pub production_approved: bool,
}
pub(super) fn registration() -> Result<Registration> {
    Ok(serde_json::from_slice(include_bytes!(
        "../../../../../fixtures/runtime/direction-pilot-v1/registration.json"
    ))?)
}
pub(super) fn limit(f: &c::FileRef) -> u64 {
    if matches!(f.path.as_str(), "raw-source.txt" | "clean-source.txt") {
        16 * META
    } else if f.path.ends_with(".wav") && f.media_type == "audio/wav" {
        8 * META
    } else {
        META
    }
}
pub(super) fn bounds(refs: &[c::ArtifactRef]) -> Result<()> {
    if refs.len() > 64
        || refs
            .iter()
            .try_fold(0u64, |sum, a| sum.checked_add(a.file.bytes))
            .is_none_or(|n| n > 64 * META)
    {
        return fail(
            "saved_flow_bounds",
            "input inventory exceeds the private stage envelope",
        );
    }
    let mut ids = BTreeMap::new();
    let mut paths = BTreeMap::new();
    for a in refs {
        a.file
            .validate()
            .map_err(|e| Error::new("saved_flow_input", e.to_string()))?;
        if a.file.bytes > limit(&a.file)
            || !c::valid_id(&a.id)
            || ids.insert(&a.id, &a.file).is_some()
            || paths.insert(&a.file.path, &a.id).is_some()
        {
            return fail(
                "saved_flow_bounds",
                "invalid, conflicting or oversized private input",
            );
        }
    }
    Ok(())
}
pub(super) fn local(root: &Path, entry: &Entry) -> Result<Vec<u8>> {
    if !c::safe_path(&entry.source) {
        return fail("saved_flow_path", "unsafe source path");
    }
    if root.is_symlink() {
        return fail("saved_flow_path", "source root symlink rejected");
    }
    let root = root.canonicalize()?;
    let mut path = root.clone();
    for part in entry.source.split('/') {
        path.push(part);
        if path.is_symlink() {
            return fail("saved_flow_path", "source symlink rejected");
        }
    }
    if !path.canonicalize()?.starts_with(&root)
        || !path.metadata()?.is_file()
        || path.metadata()?.len() != entry.artifact.file.bytes
    {
        return fail("saved_flow_input", "source extent changed");
    }
    let mut b = Vec::new();
    std::fs::File::open(path)?
        .take(limit(&entry.artifact.file) + 1)
        .read_to_end(&mut b)?;
    entry
        .artifact
        .file
        .verify(&b)
        .map_err(|e| Error::new("saved_flow_input", e.to_string()))?;
    super::super::media::validate_artifact_bytes(&b, &entry.artifact.file.media_type)?;
    Ok(b)
}
pub(super) fn bytes(store: &Store, f: &c::FileRef) -> Result<Vec<u8>> {
    if f.bytes > limit(f) {
        return fail("saved_flow_bounds", "private input exceeds its envelope");
    }
    let b = store.read_object(&f.sha256, f.bytes as usize)?;
    f.verify(&b)
        .map_err(|e| Error::new("saved_flow_corrupt", e.to_string()))?;
    Ok(b)
}
pub(super) fn insert(
    tx: &rusqlite::Connection,
    run: &str,
    a: &c::ArtifactRef,
    origin: &str,
) -> Result<()> {
    let encoded = serde_json::to_string(a)?;
    let old: Option<String> = tx
        .query_row(
            "SELECT reference FROM saved_flow_artifacts WHERE run=?1 AND id=?2",
            params![run, a.id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(old) = old {
        if old != encoded {
            return fail(
                "saved_flow_conflict",
                "private artifact identity is immutable",
            );
        }
        return Ok(());
    }
    tx.execute(
        "INSERT INTO saved_flow_artifacts VALUES(?1,?2,?3,?4)",
        params![run, a.id, encoded, origin],
    )?;
    Ok(())
}
pub(super) fn verify_owned(store: &Store, run: &str, a: &c::ArtifactRef) -> Result<Vec<u8>> {
    let old: Option<String> = store
        .db()?
        .query_row(
            "SELECT reference FROM saved_flow_artifacts WHERE run=?1 AND id=?2",
            params![run, a.id],
            |r| r.get(0),
        )
        .optional()?;
    if old.as_deref() != Some(serde_json::to_string(a)?.as_str()) {
        return fail(
            "saved_flow_scope",
            "artifact is outside the frozen run scope",
        );
    }
    bytes(store, &a.file)
}
