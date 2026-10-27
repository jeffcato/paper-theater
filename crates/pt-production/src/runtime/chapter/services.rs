use super::*;
use async_trait::async_trait;
#[derive(Clone)]
pub(super) enum Source {
    Cas,
    Local(PathBuf, Unit),
}
#[derive(Clone)]
pub(super) struct Grant {
    pub config: ChapterHelper,
    pub unit_key: String,
    pub revision: u64,
    pub inputs: Vec<c::ArtifactRef>,
    pub timeout_ms: u64,
}
pub(super) struct ChapterServices {
    pub store: Store,
    pub run: String,
    pub epoch: Option<i64>,
    pub source: Source,
    pub grant: Option<Grant>,
    allowed: Mutex<BTreeMap<String, c::ArtifactRef>>,
    candidates: Mutex<BTreeMap<String, (c::CandidateFile, Vec<u8>)>>,
}
fn error(e: Error) -> c::ServiceError {
    if e.code == "stale_epoch" {
        c::ServiceError::Cancelled
    } else {
        c::ServiceError::Internal(e.to_string())
    }
}
fn denied() -> c::ServiceError {
    c::ServiceError::Unauthorized(
        "private chapter unit capability does not authorize this operation".into(),
    )
}
pub(super) fn inputs(run: &str, k: &str, unit: &Unit, record: bool) -> Vec<c::ArtifactRef> {
    unit.inputs
        .iter()
        .cloned()
        .chain([alias(&unit.metadata, "chapter-import-unit.json")])
        .chain(record.then(|| alias(&unit.record, "chapter-import-helper-record.json")))
        .map(|f| c::ArtifactRef {
            id: digest(format!("{run}:{k}:{}:{}", f.path, f.sha256).as_bytes()),
            file: f,
            provenance: c::Provenance::Diagnostic,
        })
        .collect()
}
impl ChapterServices {
    pub fn new(
        store: Store,
        run: &str,
        epoch: Option<i64>,
        inputs: Vec<c::ArtifactRef>,
        source: Source,
        grant: Option<Grant>,
    ) -> Self {
        Self {
            store,
            run: run.into(),
            epoch,
            source,
            grant,
            allowed: Mutex::new(inputs.into_iter().map(|a| (a.id.clone(), a)).collect()),
            candidates: Mutex::new(BTreeMap::new()),
        }
    }
    pub fn add_input(&self, a: c::ArtifactRef) {
        self.allowed.lock().unwrap().insert(a.id.clone(), a);
    }
    fn active(&self) -> std::result::Result<(), c::ServiceError> {
        if let Some(epoch) = self.epoch {
            active(&self.store, &self.run, epoch).map_err(error)?;
        }
        Ok(())
    }
    pub fn bytes(&self, f: &c::FileRef) -> Result<Vec<u8>> {
        if f.bytes > FILE {
            return fail("chapter_bounds", "unit file envelope exceeded");
        }
        let bytes = match &self.source {
            Source::Cas => verify::cas_bytes(&self.store, f, FILE)?,
            Source::Local(root, u) => {
                let name = match f.path.as_str() {
                    "chapter-import-unit.json" => &u.metadata.path,
                    "chapter-import-helper-record.json" => &u.record.path,
                    _ => &f.path,
                };
                let b = verify::local(root, name, f.bytes)?;
                f.verify(&b)
                    .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
                b
            }
        };
        Ok(bytes)
    }
}
#[async_trait]
impl c::RunServices for ChapterServices {
    async fn check_cancelled(&self) -> std::result::Result<(), c::ServiceError> {
        self.active()
    }
    async fn read_artifact(
        &self,
        a: &c::ArtifactRef,
    ) -> std::result::Result<Vec<u8>, c::ServiceError> {
        self.active()?;
        let allowed = self.allowed.lock().unwrap();
        if !allowed
            .get(&a.id)
            .is_some_and(|old| old.file == a.file && old.provenance == a.provenance)
        {
            return Err(denied());
        }
        drop(allowed);
        let b = self.bytes(&a.file).map_err(error)?;
        self.active()?;
        Ok(b)
    }
    async fn write_candidate(
        &self,
        r: c::CandidateWrite,
        b: Vec<u8>,
    ) -> std::result::Result<c::CandidateFile, c::ServiceError> {
        self.active()?;
        if !c::valid_id(&r.candidate_id)
            || !OUTPUTS.contains(&r.relative_path.as_str())
                && r.relative_path != "chapter-import-helper-record.json"
            || r.media_type != "application/json"
            || r.dimensions.is_some()
            || r.provenance != c::Provenance::Diagnostic
            || b.len() as u64 > META
            || b.len() as u64 > r.max_bytes
            || r.max_bytes > super::super::media::MAX_ARTIFACT_BYTES as u64
        {
            return Err(denied());
        }
        let result = c::CandidateFile {
            candidate_id: r.candidate_id,
            file: file(&r.relative_path, &b, "application/json"),
            provenance: r.provenance,
        };
        let mut candidates = self.candidates.lock().unwrap();
        if let Some((old, _)) = candidates.get(&result.candidate_id) {
            if old.file != result.file || old.provenance != result.provenance {
                return Err(denied());
            }
        } else if candidates.len() >= 4 {
            return Err(denied());
        }
        candidates.insert(result.candidate_id.clone(), (result.clone(), b));
        Ok(result)
    }
    async fn read_candidate(
        &self,
        a: &c::CandidateFile,
    ) -> std::result::Result<Vec<u8>, c::ServiceError> {
        self.active()?;
        let candidates = self.candidates.lock().unwrap();
        let (stored, b) = candidates.get(&a.candidate_id).ok_or_else(denied)?;
        if stored.file != a.file || stored.provenance != a.provenance {
            return Err(denied());
        }
        a.file.verify(b)?;
        let b = b.clone();
        drop(candidates);
        self.active()?;
        Ok(b)
    }
    async fn helper(
        &self,
        op: c::HelperOperation,
    ) -> std::result::Result<c::HelperResult, c::ServiceError> {
        self.active()?;
        let grant = self.grant.as_ref().ok_or_else(denied)?;
        let epoch = self.epoch.ok_or_else(denied)?;
        let equal = op.inputs.len() == grant.inputs.len()
            && op
                .inputs
                .iter()
                .zip(&grant.inputs)
                .all(|(a, b)| a.id == b.id && a.file == b.file && a.provenance == b.provenance);
        if op.id != format!("chapter-import:{}", grant.unit_key)
            || op.kind != c::HelperKind::AlignAudio
            || op.recipe_sha256 != RECIPE
            || !equal
            || op.timeout_ms == 0
            || op.timeout_ms > grant.timeout_ms
            || op.timeout_ms > 30000
            || op.max_output_bytes == 0
            || op.max_output_bytes > META
        {
            return Err(denied());
        }
        let b = worker::run(self, grant, &op, epoch).await.map_err(error)?;
        self.active()?;
        let value: Value =
            serde_json::from_slice(&b).map_err(|e| c::ServiceError::Internal(e.to_string()))?;
        if value["production_approved"] != false
            || value["provider_calls"] != 0
            || value["recipe_sha256"] != RECIPE
        {
            return Err(denied());
        }
        let candidate = self
            .write_candidate(
                c::CandidateWrite {
                    candidate_id: digest(format!("{}:{}", op.id, digest(&b)).as_bytes()),
                    relative_path: "chapter-import-helper-record.json".into(),
                    media_type: "application/json".into(),
                    dimensions: None,
                    max_bytes: META,
                    provenance: c::Provenance::Diagnostic,
                },
                b,
            )
            .await?;
        Ok(c::HelperResult {
            id: op.id,
            outputs: vec![candidate],
            findings: vec![],
        })
    }
    async fn provider(
        &self,
        _: c::ProviderOperation,
    ) -> std::result::Result<c::ProviderResult, c::ServiceError> {
        Err(denied())
    }
    async fn emit(&self, _: c::JournalProposal) -> std::result::Result<(), c::ServiceError> {
        Err(denied())
    }
    async fn library(
        &self,
        _: c::LibraryQuery,
    ) -> std::result::Result<Vec<c::LibraryBinding>, c::ServiceError> {
        Err(denied())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use c::RunServices;
    use std::io::Write;
    #[tokio::test]
    async fn exact_grants_reject_bad_envelopes_and_cross_unit_substitution_without_fallback() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let request = json!({"schema_version":1,"book_id":"pg-55","source_id":"pg-55-chapter-1","mode":"fixture","pins":{"recipe_sha256":RECIPE,"profile_sha256":digest(b"no-profile"),"profile_id":"diagnostic"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":30000}});
        let run = store.create("chapter-grants", &request).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, epoch) = store.claim().unwrap().unwrap();
        let code = tempfile::tempdir().unwrap();
        for name in CODE {
            std::fs::copy(
                root.join("tools/generation").join(name),
                code.path().join(name),
            )
            .unwrap();
        }
        // Extra files in the configured directory must never join the registered closure.
        std::fs::write(
            code.path().join("json.py"),
            b"raise RuntimeError('unregistered module was imported')\n",
        )
        .unwrap();
        let config =
            ChapterHelper::new(code.path().to_path_buf(), digest(b"trusted test stage")).unwrap();
        let rt = crate::runtime::Runtime::new(
            store.clone(),
            std::sync::Arc::new(crate::generation::GenerationExecutor),
        )
        .with_chapter_helper(config.clone());
        let original = root.join("fixtures/generation/chapter-import-v2/producer-inputs");
        rt.import_chapter_bundle(
            &run,
            epoch,
            &original,
            "chapter-evidence.json",
            &original.join("bundle"),
        )
        .await
        .unwrap();
        let (hash, plan) = load(&store, &run, Some(&config)).unwrap();
        let unit = &plan.units[0];
        let k = key(&hash, unit);
        let allowed = inputs(&run, &k, unit, false);
        let grant = Grant {
            config: config.clone(),
            unit_key: k.clone(),
            revision: 1,
            inputs: allowed.clone(),
            timeout_ms: 30000,
        };
        let service = ChapterServices::new(
            store.clone(),
            &run,
            Some(epoch),
            allowed.clone(),
            Source::Cas,
            Some(grant),
        );
        let base = c::HelperOperation {
            id: format!("chapter-import:{k}"),
            kind: c::HelperKind::AlignAudio,
            inputs: allowed.clone(),
            recipe_sha256: RECIPE.into(),
            timeout_ms: 30000,
            max_output_bytes: META,
        };
        let mut bads = Vec::new();
        let mut bad = base.clone();
        bad.timeout_ms = 0;
        bads.push(bad);
        let mut bad = base.clone();
        bad.timeout_ms = 30001;
        bads.push(bad);
        let mut bad = base.clone();
        bad.max_output_bytes = META + 1;
        bads.push(bad);
        for recipe in [
            "af53771eb6f8d5b807958416503fb525de6d408d605b7c86b705f084866fe647",
            "e21cd180f26d19b5669c97c8fe25f0b59a4266c781167161257f62ca1932dff4",
        ] {
            let mut bad = base.clone();
            bad.recipe_sha256 = recipe.into();
            bads.push(bad);
        }
        let other = inputs(&run, &key(&hash, &plan.units[1]), &plan.units[1], false);
        let mut bad = base.clone();
        bad.inputs = other.clone();
        bads.push(bad);
        let mut bad = base.clone();
        bad.id = "another-unit".into();
        bads.push(bad);
        let mut bad = base.clone();
        bad.inputs.push(other[0].clone());
        bads.push(bad);
        for bad in bads {
            assert!(service.helper(bad).await.is_err());
        }
        assert!(service.read_artifact(&other[0]).await.is_err());
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        rt.execute_chapter_unit(&run, epoch, &unit.label)
            .await
            .unwrap();
        let saved: String = store
            .db()
            .unwrap()
            .query_row("SELECT response FROM attempts WHERE run=?1", [&run], |r| {
                r.get(0)
            })
            .unwrap();
        store
            .db()
            .unwrap()
            .execute("UPDATE attempts SET response=NULL WHERE run=?1", [&run])
            .unwrap();
        assert!(
            service.helper(base.clone()).await.is_err(),
            "missing completed response must never authorize redispatch"
        );
        store
            .db()
            .unwrap()
            .execute(
                "UPDATE attempts SET response=?1 WHERE run=?2",
                params![saved, run],
            )
            .unwrap();
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        std::fs::OpenOptions::new()
            .append(true)
            .open(code.path().join("adapters.py"))
            .unwrap()
            .write_all(b"\n# changed closure\n")
            .unwrap();
        assert!(
            ChapterHelper::new(code.path().to_path_buf(), digest(b"trusted test stage")).is_err()
        );
        assert!(
            rt.execute_chapter_unit(&run, epoch, &unit.label)
                .await
                .is_err()
        );
        assert_eq!(
            store
                .db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
