use super::store::*;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Component, Path},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InventoryFile {
    pub path: String,
    pub artifact_id: String,
    pub sha256: String,
    pub bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Publication {
    pub book_id: String,
    pub manifest: Vec<u8>,
    pub manifest_hash: String,
    pub files: Vec<InventoryFile>,
    pub acceptance_artifact: String,
    pub evidence_artifacts: Vec<String>,
}
fn relative(s: &str) -> bool {
    !s.is_empty()
        && !s.contains('\\')
        && !s.contains(':')
        && s.len() < 256
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && !s.split('/').any(|c| c.is_empty() || c.starts_with('.'))
}
impl Store {
    fn verify_candidate(&self, p: &Publication) -> Result<()> {
        if p.files.is_empty()
            || p.files.len() > 256
            || digest(&p.manifest) != p.manifest_hash
            || p.manifest.len() > 1024 * 1024
        {
            return fail("invalid_package", "invalid manifest/inventory");
        }
        let manifest: Value = serde_json::from_slice(&p.manifest)?;
        let mut shared_document = None;
        for id in &p.evidence_artifacts {
            let bytes = self.artifact(id)?.0;
            if let Ok(document) = serde_json::from_slice::<pt_contracts::PlaybackDocument>(&bytes) {
                shared_document = Some(document);
                break;
            }
        }
        let document = shared_document.ok_or_else(|| {
            Error::new(
                "production_verifier_pending",
                "shared acceptance document is required",
            )
        })?;
        let blockers = pt_contracts::production_blockers(&document);
        if !blockers.is_empty() {
            return Err(Error::new(
                "production_verifier_pending",
                blockers.join(","),
            ));
        }
        if manifest["book_id"] != p.book_id
            || manifest["status"] != "accepted"
            || manifest["provenance"] != "live"
            || manifest["unresolved_findings"]
                .as_array()
                .is_none_or(|a| !a.is_empty())
        {
            return fail(
                "ineligible_package",
                "only a complete reviewed live candidate may publish",
            );
        }
        let acceptance = self.artifact(&p.acceptance_artifact)?;
        let review: Value = serde_json::from_slice(&acceptance.0)?;
        let mut files = p.files.clone();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let payload = digest(&serde_json::to_vec(&files)?);
        if review["status"] != "accepted"
            || review["payload_sha256"] != payload
            || manifest["payload_sha256"] != payload
            || manifest["acceptance_sha256"] != acceptance.2
            || review["unresolved_findings"]
                .as_array()
                .is_none_or(|a| !a.is_empty())
            || review["checker_revision"]
                .as_str()
                .is_none_or(|s| s.is_empty())
            || review["profile_revision"]
                .as_str()
                .is_none_or(|s| s.is_empty())
            || manifest["files"] != serde_json::to_value(&files)?
            || review["evidence_artifacts"] != serde_json::to_value(&p.evidence_artifacts)?
            || p.evidence_artifacts.is_empty()
        {
            return fail(
                "invalid_acceptance",
                "candidate acceptance is not bound to exact payload and evidence",
            );
        }
        let mut names = std::collections::HashSet::new();
        for f in &files {
            if !relative(&f.path)
                || f.path == "manifest.json"
                || f.path == "acceptance.json"
                || !names.insert(f.path.clone())
            {
                return fail("invalid_inventory", "unsafe or duplicate package path");
            }
            let (bytes, _, hash) = self.artifact(&f.artifact_id)?;
            if hash != f.sha256 || bytes.len() != f.bytes {
                return fail("invalid_inventory", "payload identity mismatch");
            }
        }
        for evidence in &p.evidence_artifacts {
            self.artifact(evidence)?;
        }
        Ok(())
    }
    #[allow(dead_code)] // Admission stays private until the shared production verifier is integrated.
    pub(crate) fn prepare_publication(
        &self,
        run: &str,
        epoch: i64,
        p: &Publication,
    ) -> Result<String> {
        self.verify_candidate(p)?;
        let id = p.manifest_hash.clone();
        self.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            let mode: String = tx.query_row(
                "SELECT json_extract(request,'$.mode') FROM runs WHERE id=?1",
                [run],
                |r| r.get(0),
            )?;
            if mode != "live" {
                return fail("ineligible_package", "fixture runs cannot publish");
            }
            tx.execute(
                "INSERT OR IGNORE INTO publications VALUES(?1,?2,?3,?4,?5,'pending')",
                params![id, run, epoch, p.book_id, serde_json::to_string(p)?],
            )?;
            let owner: (String, i64) = tx.query_row(
                "SELECT run,epoch FROM publications WHERE id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if owner != (run.to_string(), epoch) {
                return fail("publication_conflict", "candidate belongs to another epoch");
            }
            Store::emit(
                tx,
                run,
                "publication_prepared",
                &json!({"manifest_sha256":id}),
            )?;
            Ok(id)
        })
    }
    pub(crate) fn activate_publication(&self, id: &str) -> Result<()> {
        if !valid_hash(id) {
            return fail("invalid_package", "invalid publication identity");
        }
        let (run, epoch, encoded, state): (String, i64, String, String) = self.db()?.query_row(
            "SELECT run,epoch,record,state FROM publications WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        if state == "active" {
            return self.verify_release(id);
        }
        if state != "pending" {
            return fail("publication_aborted", "candidate epoch is no longer active");
        }
        Store::active(&self.db()?, &run, epoch)?;
        let p: Publication = serde_json::from_str(&encoded)?;
        self.verify_candidate(&p)?;
        let destination = self.root.join("releases").join(id);
        if !destination.exists() {
            let staging = tempfile::Builder::new()
                .prefix("pending-")
                .tempdir_in(self.root.join("releases"))?;
            for f in &p.files {
                write_file(staging.path(), &f.path, &self.artifact(&f.artifact_id)?.0)?;
            }
            write_file(
                staging.path(),
                "acceptance.json",
                &self.artifact(&p.acceptance_artifact)?.0,
            )?;
            write_file(staging.path(), "manifest.json", &p.manifest)?;
            std::fs::File::open(staging.path())?.sync_all()?;
            // Same process serializes activations. Existing immutable directories are never replaced.
            std::fs::rename(staging.path(), &destination)?;
            std::fs::File::open(self.root.join("releases"))?.sync_all()?;
        }
        self.verify_release(id)?;
        self.transaction(|tx| {
            Store::active(tx,&run,epoch)?;
            tx.execute("INSERT INTO catalog VALUES(?1,?2) ON CONFLICT(book) DO UPDATE SET publication=excluded.publication",params![p.book_id,id])?;
            tx.execute("UPDATE publications SET state='active' WHERE id=?1",[id])?;
            Store::emit(tx,&run,"catalog_activated",&json!({"book_id":p.book_id,"manifest_sha256":id}))?;Ok(())
        })
    }
    fn verify_release(&self, id: &str) -> Result<()> {
        let encoded: String =
            self.db()?
                .query_row("SELECT record FROM publications WHERE id=?1", [id], |r| {
                    r.get(0)
                })?;
        let p: Publication = serde_json::from_str(&encoded)?;
        self.verify_candidate(&p)?;
        let root = self.root.join("releases").join(id);
        let mut expected = std::collections::BTreeMap::new();
        expected.insert(
            "manifest.json".to_owned(),
            (p.manifest_hash, p.manifest.len()),
        );
        let acceptance = self.artifact(&p.acceptance_artifact)?;
        expected.insert(
            "acceptance.json".to_owned(),
            (acceptance.2, acceptance.0.len()),
        );
        for f in p.files {
            expected.insert(f.path, (f.sha256, f.bytes));
        }
        let mut seen = std::collections::BTreeSet::new();
        fn walk(
            root: &Path,
            dir: &Path,
            seen: &mut std::collections::BTreeSet<String>,
        ) -> Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                if kind.is_symlink() {
                    return fail("invalid_package", "symlink in release");
                }
                if kind.is_dir() {
                    walk(root, &entry.path(), seen)?;
                } else if kind.is_file() {
                    seen.insert(
                        entry
                            .path()
                            .strip_prefix(root)
                            .unwrap()
                            .to_str()
                            .ok_or_else(|| Error::new("invalid_path", "non-UTF8 path"))?
                            .to_owned(),
                    );
                } else {
                    return fail("invalid_package", "special file in release");
                }
            }
            Ok(())
        }
        if std::fs::symlink_metadata(&root)?.file_type().is_symlink() {
            return fail("invalid_package", "symlink release");
        }
        walk(&root, &root, &mut seen)?;
        if seen != expected.keys().cloned().collect() {
            return fail("invalid_inventory", "release inventory changed");
        }
        for (path, (hash, size)) in expected {
            let path = root.join(path);
            if std::fs::metadata(&path)?.len() != size as u64
                || digest(&std::fs::read(path)?) != hash
            {
                return fail("corrupt_artifact", "release bytes changed");
            }
        }
        Ok(())
    }
    pub(crate) fn reconcile_publications(&self) -> Result<()> {
        let pending = self
            .db()?
            .prepare("SELECT id,run,epoch FROM publications WHERE state='pending'")?
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (id, run, epoch) in pending {
            // Recovery advances epochs: interrupted activation keeps the previous catalog.
            if !self.is_active(&run, epoch)? {
                self.transaction(|tx| {
                    tx.execute("UPDATE publications SET state='aborted' WHERE id=?1", [&id])?;
                    Store::emit(
                        tx,
                        &run,
                        "publication_aborted",
                        &json!({"manifest_sha256":id,"reason":"epoch_superseded"}),
                    )?;
                    Ok(())
                })?;
            } else if self.activate_publication(&id).is_err() {
                self.transaction(|tx| {
                    tx.execute("UPDATE publications SET state='aborted' WHERE id=?1", [&id])?;
                    Store::emit(
                        tx,
                        &run,
                        "publication_aborted",
                        &json!({"manifest_sha256":id,"reason":"verification_failed"}),
                    )?;
                    Ok(())
                })?;
            }
        }
        Ok(())
    }
    pub fn catalog(&self, book: &str) -> Result<Option<String>> {
        Ok(self
            .db()?
            .query_row(
                "SELECT publication FROM catalog WHERE book=?1",
                [book],
                |r| r.get(0),
            )
            .optional()?)
    }
}
fn write_file(root: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    let dest = root.join(relative);
    let parent = dest.parent().unwrap();
    std::fs::create_dir_all(parent)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&dest)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    let mut dir = Some(parent);
    while let Some(p) = dir {
        std::fs::File::open(p)?.sync_all()?;
        if p == root {
            break;
        }
        dir = p.parent();
    }
    Ok(())
}
