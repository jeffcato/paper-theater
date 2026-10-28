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
pub(super) struct Variant {
    pub case: String,
    pub recipe_sha256: String,
    pub original_input: c::StageInput,
    pub original_input_sha256: String,
    pub original_file: Entry,
    pub files: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: u32,
    book_id: String,
    source_id: String,
    variants: Vec<Variant>,
    bulk_speech_approved: bool,
    production_approved: bool,
}
pub(super) fn variant(recipe: &str) -> Result<Variant> {
    let r: Registry = serde_json::from_slice(REGISTRATION)?;
    if r.schema_version != 1
        || r.book_id != "test-book"
        || r.source_id != "chapter-1"
        || r.bulk_speech_approved
        || r.production_approved
    {
        return fail("speech_registration", "invalid compiled registry");
    }
    r.variants
        .into_iter()
        .find(|v| v.recipe_sha256 == recipe)
        .ok_or_else(|| Error::new("speech_scope", "unregistered saved recipe"))
}
pub(super) fn limit(f: &c::FileRef) -> u64 {
    if ["raw-source.txt", "clean-source.txt"].contains(&f.path.as_str()) {
        16 * META
    } else if f.path.ends_with(".wav") && f.media_type == "audio/wav" {
        8 * META
    } else {
        META
    }
}
pub(super) fn bounds(refs: &[c::ArtifactRef]) -> Result<()> {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut total = 0u64;
    for a in refs {
        a.file.validate().map_err(contract)?;
        total = total
            .checked_add(a.file.bytes)
            .ok_or_else(|| Error::new("speech_bounds", "overflow"))?;
        if !c::valid_id(&a.id)
            || !ids.insert(&a.id)
            || !paths.insert(&a.file.path)
            || a.file.bytes > limit(&a.file)
        {
            return fail("speech_bounds", "invalid/duplicate/oversized input");
        }
    }
    if refs.len() > 64 || total > 64 * META {
        return fail("speech_bounds", "input count/aggregate cap");
    }
    Ok(())
}
pub(super) fn local(root: &Path, e: &Entry) -> Result<Vec<u8>> {
    if root.is_symlink() || !c::safe_path(&e.source) {
        return fail("speech_path", "unsafe producer path");
    }
    let root = root.canonicalize()?;
    let mut path = root.clone();
    for part in e.source.split('/') {
        path.push(part);
        if path.is_symlink() {
            return fail("speech_path", "symlink producer path");
        }
    }
    let mut f = std::fs::File::open(&path)?;
    if !f.metadata()?.is_file()
        || f.metadata()?.len() != e.artifact.file.bytes
        || !path.canonicalize()?.starts_with(root)
    {
        return fail("speech_input", "producer extent changed");
    }
    let mut b = Vec::new();
    f.by_ref()
        .take(limit(&e.artifact.file) + 1)
        .read_to_end(&mut b)?;
    e.artifact.file.verify(&b).map_err(contract)?;
    super::super::media::validate_artifact_bytes(&b, &e.artifact.file.media_type)?;
    Ok(b)
}
pub(super) fn bytes(store: &Store, f: &c::FileRef) -> Result<Vec<u8>> {
    if f.bytes > limit(f) {
        return fail("speech_bounds", "private input cap");
    }
    let b = store.read_object(&f.sha256, f.bytes as usize)?;
    f.verify(&b).map_err(contract)?;
    Ok(b)
}
pub(super) fn insert(
    tx: &rusqlite::Connection,
    run: &str,
    a: &c::ArtifactRef,
    origin: &str,
) -> Result<()> {
    let b = serde_json::to_string(a)?;
    let old: Option<String> = tx
        .query_row(
            "SELECT reference FROM speech_refs WHERE run=?1 AND id=?2",
            params![run, a.id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(old) = old {
        if old != b {
            return fail("speech_conflict", "immutable reference changed");
        }
        return Ok(());
    }
    tx.execute(
        "INSERT INTO speech_refs VALUES(?1,?2,?3,?4)",
        params![run, a.id, b, origin],
    )?;
    Ok(())
}
pub(super) fn verify_owned(store: &Store, run: &str, a: &c::ArtifactRef) -> Result<Vec<u8>> {
    let b: Option<String> = store
        .db()?
        .query_row(
            "SELECT reference FROM speech_refs WHERE run=?1 AND id=?2",
            params![run, a.id],
            |r| r.get(0),
        )
        .optional()?;
    if b.as_deref() != Some(serde_json::to_string(a)?.as_str()) {
        return fail("speech_scope", "reference outside private run scope");
    }
    bytes(store, &a.file)
}
