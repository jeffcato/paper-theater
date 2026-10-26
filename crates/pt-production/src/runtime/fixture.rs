use super::store::*;
use pt_contracts as c;
use rusqlite::{OptionalExtension, params};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};
impl Store {
    /// Trusted local fixture configuration only; no request-supplied filesystem paths.
    pub fn seed_fixture(
        &self,
        run: &str,
        epoch: i64,
        root: &Path,
        sources: Option<&Path>,
    ) -> Result<()> {
        Store::active(&self.db()?, run, epoch)?;
        self.scope_legacy_inputs(run, epoch)?;
        let existing = self.seed_inputs(run)?;
        if !existing.is_empty() {
            for a in existing {
                a.file
                    .verify(&self.artifact(&a.id)?.0)
                    .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
            }
            return Ok(());
        }
        let raw = bounded_file(root, "fixture.json")?;
        let doc: c::PlaybackDocument = serde_json::from_slice(&raw)?;
        c::validate_playback(&doc).map_err(|e| Error::new("invalid_fixture", e.to_string()))?;
        let manifest: c::Manifest = serde_json::from_slice(&bounded_file(root, "manifest.json")?)?;
        if serde_json::to_value(&manifest)? != serde_json::to_value(&doc.manifest)? {
            return fail(
                "invalid_fixture",
                "fixture manifest envelope differs from bytes",
            );
        }
        let mut files = BTreeMap::new();
        for f in doc
            .manifest
            .entries
            .files()
            .into_iter()
            .chain(doc.manifest.assets.iter())
            .chain([&doc.manifest.acceptance])
        {
            let verified = super::media::VerifiedArtifact::open(root, f)?;
            use std::io::Read;
            let mut bytes = Vec::with_capacity(verified.bytes as usize);
            verified
                .file
                .take(verified.bytes + 1)
                .read_to_end(&mut bytes)?;
            f.verify(&bytes)
                .map_err(|e| Error::new("invalid_fixture", e.to_string()))?;
            files.insert(
                f.path.clone(),
                (f.clone(), bytes, c::Provenance::Diagnostic),
            );
        }
        for name in [
            "fixture.json",
            "profile.json",
            "inventory.json",
            "manifest.json",
            "source.txt",
            "presentation-map.json",
        ] {
            if files.contains_key(name) {
                continue;
            }
            let bytes = bounded_file(root, name)?;
            let f = c::FileRef {
                path: name.into(),
                sha256: digest(&bytes),
                bytes: bytes.len() as u64,
                media_type: if name.ends_with(".txt") {
                    "text/plain"
                } else {
                    "application/json"
                }
                .into(),
                dimensions: None,
            };
            files.insert(name.into(), (f, bytes, c::Provenance::Diagnostic));
        }
        verify_fixture_bindings(&doc, &files)?;
        if let Some(sources) = sources {
            let request = self.snapshot(run)?;
            let prefix = match request["request"]["book_id"].as_str() {
                Some("pg-55") => "pg55",
                Some("pg-36") => "pg36",
                _ => return fail("invalid_source", "unknown fixture source"),
            };
            for (suffix, name, mime) in [
                ("raw.txt", "raw-source.txt", "text/plain"),
                ("clean.txt", "clean-source.txt", "text/plain"),
                ("frozen.json", "frozen-source.json", "application/json"),
            ] {
                let bytes = bounded_file(sources, &format!("{prefix}-{suffix}"))?;
                let file = c::FileRef {
                    path: name.into(),
                    sha256: digest(&bytes),
                    bytes: bytes.len() as u64,
                    media_type: mime.into(),
                    dimensions: None,
                };
                files.insert(name.into(), (file, bytes, c::Provenance::Imported));
            }
        }
        let mut references = Vec::new();
        for (_, (file, bytes, provenance)) in files {
            self.put_typed(&bytes, &file.media_type)?;
            references.push(c::ArtifactRef {
                id: digest(format!("{run}:seed:{}:{}", file.path, file.sha256).as_bytes()),
                file,
                provenance,
            });
        }
        self.transaction(|tx|{Store::active(tx,run,epoch)?;
            for reference in &references{let encoded=serde_json::to_string(reference)?;
                tx.execute("INSERT OR IGNORE INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![reference.id,run,reference.file.path,reference.file.sha256,reference.file.bytes,reference.file.media_type,serde_json::to_value(reference.provenance)?.as_str(),encoded])?;
                tx.execute("INSERT OR IGNORE INTO run_inputs VALUES(?1,?2,?3)",params![run,reference.id,encoded])?;
                tx.execute("INSERT OR IGNORE INTO run_input_scopes VALUES(?1,?2,'saved')",params![run,reference.id])?;
            }
            Store::emit(tx,run,"fixture_inputs_imported",&json!({"provenance":"diagnostic","artifact_ids":references.iter().map(|a|&a.id).collect::<Vec<_>>(),"source_coverage":"Saved package is partial; frozen chapter text is a separate input."}))?;Ok(())
        })
    }
    fn scope_legacy_inputs(&self, run: &str, epoch: i64) -> Result<()> {
        self.transaction(|tx|{Store::active(tx,run,epoch)?;
            let rows=tx.prepare("SELECT i.artifact,i.reference FROM run_inputs i WHERE i.run=?1 AND NOT EXISTS(SELECT 1 FROM run_input_scopes s WHERE s.run=i.run AND s.artifact=i.artifact)")?.query_map([run],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
            let count=rows.len();for (id,encoded) in rows{let a:c::ArtifactRef=serde_json::from_str(&encoded)?;let scope=if id==digest(format!("{run}:seed:{}:{}",a.file.path,a.file.sha256).as_bytes()){"saved"}else if id==digest(format!("{run}:acoustic:{}:{}",a.file.path,a.file.sha256).as_bytes()){"acoustic"}else{return fail("input_scope_unknown","legacy artifact lacks a known immutable bundle identity")};tx.execute("INSERT INTO run_input_scopes VALUES(?1,?2,?3)",params![run,id,scope])?;}
            if count>0{Store::emit(tx,run,"input_scopes_migrated",&json!({"inputs":count,"artifact_bytes_changed":false}))?;}Ok(())
        })
    }
    pub(crate) fn seed_inputs(&self, run: &str) -> Result<Vec<c::ArtifactRef>> {
        self.seed_inputs_for_stage(run, c::Stage::Package)
    }
    pub(crate) fn seed_inputs_for_stage(
        &self,
        run: &str,
        stage: c::Stage,
    ) -> Result<Vec<c::ArtifactRef>> {
        let scope = if stage == c::Stage::Acoustic {
            "acoustic"
        } else {
            "saved"
        };
        self.db()?.prepare("SELECT i.reference FROM run_inputs i WHERE i.run=?1 AND (EXISTS(SELECT 1 FROM run_input_scopes s WHERE s.run=i.run AND s.artifact=i.artifact AND s.scope=?2) OR NOT EXISTS(SELECT 1 FROM run_input_scopes s WHERE s.run=i.run)) ORDER BY i.rowid")?.query_map(params![run,scope],|r|r.get::<_,String>(0))?.map(|s|Ok(serde_json::from_str(&s?)?)).collect()
    }
    pub fn diagnostic_ref(&self, run: &str) -> Result<Option<c::ReleaseRef>> {
        Ok(self
            .db()?
            .query_row(
                "SELECT manifest_hash FROM diagnostic_packages WHERE run=?1",
                [run],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .map(|manifest_sha256| c::ReleaseRef {
                manifest_path: format!("/api/v1/runs/{run}/package/manifest.json"),
                manifest_sha256,
            }))
    }
    pub fn package_artifact(&self, run: &str, path: &str) -> Result<(Vec<u8>, String, String)> {
        self.artifact(&self.package_artifact_id(run, path)?)
    }
    pub fn open_package_artifact(
        &self,
        run: &str,
        path: &str,
    ) -> Result<super::media::VerifiedArtifact> {
        self.open_artifact(&self.package_artifact_id(run, path)?)
    }
    fn package_artifact_id(&self, run: &str, path: &str) -> Result<String> {
        if !c::safe_path(path) {
            return fail("not_found", "package path denied");
        }
        let encoded: String = self
            .db()?
            .query_row(
                "SELECT mapping FROM diagnostic_packages WHERE run=?1",
                [run],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::new("not_found", "diagnostic package missing"))?;
        let map: BTreeMap<String, String> = serde_json::from_str(&encoded)?;
        let id = map
            .get(path)
            .ok_or_else(|| Error::new("not_found", "path not inventoried"))?;
        Ok(id.clone())
    }
}
fn bounded_file(root: &Path, path: &str) -> Result<Vec<u8>> {
    if !c::safe_path(path) {
        return fail("invalid_path", "unsafe fixture path");
    }
    let canonical = root.canonicalize()?;
    let mut p = root.to_path_buf();
    for part in path.split('/') {
        p.push(part);
        if p.is_symlink() {
            return fail("invalid_path", "symlink fixture input");
        }
    }
    if !p.canonicalize()?.starts_with(&canonical) || p.metadata()?.len() > 32 * 1024 * 1024 {
        return fail("invalid_fixture", "fixture file escaped or exceeded bounds");
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(p)?
        .take(super::media::MAX_ARTIFACT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    super::media::validate_artifact_size(bytes.len() as u64, "application/octet-stream")?;
    Ok(bytes)
}
fn verify_fixture_bindings(
    doc: &c::PlaybackDocument,
    files: &BTreeMap<String, (c::FileRef, Vec<u8>, c::Provenance)>,
) -> Result<()> {
    let bytes = |path: &str| {
        files
            .get(path)
            .map(|(_, b, _)| b)
            .ok_or_else(|| Error::new("invalid_fixture", "bound file missing"))
    };
    let inventory = bytes("inventory.json")?;
    let mut expected = doc
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(doc.manifest.assets.iter())
        .collect::<Vec<_>>();
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    let actual: Vec<c::FileRef> = serde_json::from_slice(inventory)?;
    if digest(inventory) != doc.acceptance.inventory_sha256
        || actual != expected.into_iter().cloned().collect::<Vec<_>>()
        || !actual.windows(2).all(|w| w[0].path < w[1].path)
    {
        return fail("invalid_fixture", "exact sorted inventory differs");
    }
    macro_rules! bound {
        ($path:expr, $value:expr, $ty:ty) => {
            let disk: $ty = serde_json::from_slice(bytes($path)?)?;
            if serde_json::to_value(disk)? != serde_json::to_value($value)? {
                return fail("invalid_fixture", "load envelope differs from bound file");
            }
        };
    }
    bound!(
        &doc.manifest.entries.timeline.path,
        &doc.timeline,
        c::Timeline
    );
    bound!(
        &doc.manifest.entries.actors.path,
        &doc.actors,
        Vec<c::RegisteredCard>
    );
    bound!(&doc.manifest.entries.shell.path, &doc.shell, c::Shell);
    bound!(
        &doc.manifest.entries.read_along.path,
        &doc.read_along,
        c::ReadAlong
    );
    bound!(
        &doc.manifest.acceptance.path,
        &doc.acceptance,
        c::Acceptance
    );
    Ok(())
}

pub(crate) fn candidate_mapping(
    output: &c::StageOutput,
    inputs: &[c::ArtifactRef],
    artifacts: &[c::ArtifactRef],
    candidates: &BTreeMap<String, Vec<u8>>,
    pins: &c::Pins,
) -> Result<Option<(String, BTreeMap<String, String>)>> {
    let Some(package) = &output.package_candidate else {
        return Ok(None);
    };
    if package.provenance != c::Provenance::Diagnostic
        || output.outcome != c::StageOutcome::NeedsRepair
        || serde_json::to_value(&package.pins)? != serde_json::to_value(pins)?
    {
        return fail(
            "ineligible_package",
            "diagnostic boundary never grants production acceptance",
        );
    }
    let mut by_path = BTreeMap::new();
    for a in inputs.iter().chain(artifacts) {
        by_path.insert(a.file.path.clone(), a);
    }
    let inventory = candidates
        .get("inventory.json")
        .ok_or_else(|| Error::new("invalid_package", "inventory bytes unavailable"))?;
    let files: Vec<c::FileRef> = serde_json::from_slice(inventory)?;
    if digest(inventory) != package.inventory_sha256
        || files != package.payload_inventory
        || !files.windows(2).all(|w| w[0].path < w[1].path)
    {
        return fail("invalid_package", "exact sorted inventory differs");
    }
    let manifest_bytes = candidates
        .get(&package.manifest.file.path)
        .ok_or_else(|| Error::new("invalid_package", "manifest candidate unavailable"))?;
    package
        .manifest
        .file
        .verify(manifest_bytes)
        .map_err(|e| Error::new("invalid_package", e.to_string()))?;
    let manifest: c::Manifest = serde_json::from_slice(manifest_bytes)?;
    let acceptance_bytes = candidates
        .get(&package.acceptance.file.path)
        .ok_or_else(|| Error::new("invalid_package", "acceptance candidate unavailable"))?;
    let acceptance: c::Acceptance = serde_json::from_slice(acceptance_bytes)?;
    let mut expected = manifest
        .entries
        .files()
        .into_iter()
        .chain(manifest.assets.iter())
        .cloned()
        .collect::<Vec<_>>();
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    if expected != files
        || manifest.provenance != c::Provenance::Diagnostic
        || acceptance.provenance != c::Provenance::Diagnostic
        || acceptance.inventory_sha256 != package.inventory_sha256
        || acceptance.recipe_sha256 != pins.recipe_sha256
        || acceptance.profile_sha256 != pins.profile_sha256
        || manifest.acceptance != package.acceptance.file
    {
        return fail("invalid_package", "diagnostic package binding mismatch");
    }
    let mut mapping = BTreeMap::new();
    for f in files
        .iter()
        .chain([&package.manifest.file, &package.acceptance.file])
    {
        let a = by_path
            .get(&f.path)
            .ok_or_else(|| Error::new("invalid_package", "payload missing"))?;
        if a.file != *f {
            return fail("invalid_package", "payload exact reference differs");
        }
        mapping.insert(f.path.clone(), a.id.clone());
    }
    let a = by_path
        .get("inventory.json")
        .ok_or_else(|| Error::new("invalid_package", "inventory missing"))?;
    mapping.insert("inventory.json".into(), a.id.clone());
    Ok(Some((package.manifest.file.sha256.clone(), mapping)))
}
impl Store {
    pub fn seed_acoustic_fixture(&self, run: &str, epoch: i64, root: &Path) -> Result<()> {
        fn inventory(root: &Path, dir: &Path, files: &mut Vec<String>) -> Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let e = entry?;
                if e.file_type()?.is_symlink() {
                    return fail("invalid_fixture", "helper fixture symlink");
                }
                if e.file_type()?.is_dir() {
                    inventory(root, &e.path(), files)?;
                } else if e.file_type()?.is_file() {
                    files.push(
                        e.path()
                            .strip_prefix(root)
                            .unwrap()
                            .to_str()
                            .ok_or_else(|| Error::new("invalid_path", "UTF-8 required"))?
                            .into(),
                    );
                }
                if files.len() > 128 {
                    return fail("invalid_fixture", "helper input count exceeded");
                }
            }
            Ok(())
        }
        Store::active(&self.db()?, run, epoch)?;
        self.scope_legacy_inputs(run, epoch)?;
        let existing = self.seed_inputs_for_stage(run, c::Stage::Acoustic)?;
        if !existing.is_empty() {
            for a in existing {
                a.file
                    .verify(&self.artifact(&a.id)?.0)
                    .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
            }
            return Ok(());
        }
        let mut paths = Vec::new();
        inventory(root, root, &mut paths)?;
        paths.sort();
        let mut refs = Vec::new();
        let mut total = 0;
        for path in paths {
            let bytes = bounded_file(root, &path)?;
            total += bytes.len();
            if total > 64 * 1024 * 1024 {
                return fail("invalid_fixture", "helper fixture size exceeded");
            }
            let mime = if path.ends_with(".json") {
                "application/json"
            } else if path.ends_with(".wav") {
                "audio/wav"
            } else if path.ends_with(".txt") {
                "text/plain"
            } else {
                return fail("invalid_fixture", "helper input type rejected");
            };
            let file = c::FileRef {
                path,
                sha256: self.put(&bytes)?,
                bytes: bytes.len() as u64,
                media_type: mime.into(),
                dimensions: None,
            };
            refs.push(c::ArtifactRef {
                id: digest(format!("{run}:acoustic:{}:{}", file.path, file.sha256).as_bytes()),
                file,
                provenance: c::Provenance::Imported,
            });
        }
        self.transaction(|tx|{Store::active(tx,run,epoch)?;for a in &refs{let reference=serde_json::to_string(a)?;tx.execute("INSERT OR IGNORE INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,'imported',?7)",params![a.id,run,a.file.path,a.file.sha256,a.file.bytes,a.file.media_type,reference])?;tx.execute("INSERT OR IGNORE INTO run_inputs VALUES(?1,?2,?3)",params![run,a.id,reference])?;tx.execute("INSERT OR IGNORE INTO run_input_scopes VALUES(?1,?2,'acoustic')",params![run,a.id])?;}Store::emit(tx,run,"acoustic_inputs_imported",&json!({"artifact_ids":refs.iter().map(|a|&a.id).collect::<Vec<_>>(),"provenance":"imported"}))?;Ok(())})
    }
}
