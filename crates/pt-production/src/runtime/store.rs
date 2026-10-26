use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const STAGES: &[&str] = &[
    "source",
    "direction",
    "pilot",
    "speech",
    "acoustic",
    "visual_preflight",
    "visuals",
    "presentation",
    "music_mix",
    "choreography",
    "review",
    "package",
];
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl Error {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
macro_rules! convert {
    ($t:ty, $c:literal) => {
        impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Self::new($c, e.to_string())
            }
        }
    };
}
convert!(rusqlite::Error, "database_error");
convert!(std::io::Error, "io_error");
convert!(serde_json::Error, "invalid_json");
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
pub(crate) fn fail<T>(code: &'static str, msg: &str) -> Result<T> {
    Err(Error::new(code, msg))
}
pub(crate) fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Clone)]
pub struct Store {
    pub(crate) root: Arc<PathBuf>,
    pub(crate) wake: Arc<tokio::sync::Notify>,
}
impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        let root = root.canonicalize()?;
        for p in ["objects", "releases", "scratch"] {
            std::fs::create_dir_all(root.join(p))?;
        }
        let store = Self {
            root: Arc::new(root),
            wake: Arc::new(tokio::sync::Notify::new()),
        };
        store.db()?.execute_batch(include_str!("schema.sql"))?;
        Ok(store)
    }
    pub(crate) fn db(&self) -> Result<Connection> {
        let db = Connection::open(self.root.join("runtime.sqlite3"))?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        Ok(db)
    }
    pub(crate) fn transaction<T>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = f(&tx)?;
        tx.commit()?;
        self.wake.notify_waiters();
        Ok(result)
    }
    pub(crate) fn active(tx: &Connection, run: &str, epoch: i64) -> Result<()> {
        let active = tx
            .query_row(
                "SELECT epoch=?2 AND state IN ('running','waiting_provider') FROM runs WHERE id=?1",
                params![run, epoch],
                |r| r.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false);
        if !active {
            return fail("stale_epoch", "worker is cancelled, terminal or superseded");
        }
        Ok(())
    }
    pub fn is_active(&self, run: &str, epoch: i64) -> Result<bool> {
        Ok(Self::active(&self.db()?, run, epoch).is_ok())
    }
    pub(crate) fn emit(tx: &Transaction<'_>, run: &str, kind: &str, data: &Value) -> Result<i64> {
        let encoded = serde_json::to_string(data)?;
        if encoded.len() > 64 * 1024 {
            return fail("event_too_large", "use an artifact reference");
        }
        tx.execute("INSERT INTO events(run,seq,epoch,created,kind,data) SELECT ?1,COALESCE(MAX(seq),0)+1,(SELECT epoch FROM runs WHERE id=?1),?2,?3,?4 FROM events WHERE run=?1", params![run,now(),kind,encoded])?;
        Ok(
            tx.query_row("SELECT MAX(seq) FROM events WHERE run=?1", [run], |r| {
                r.get(0)
            })?,
        )
    }
    pub fn create(&self, key: &str, request: &Value) -> Result<Value> {
        if key.is_empty() || key.len() > 128 {
            return fail("invalid_idempotency_key", "key must be 1..128 bytes");
        }
        if request["mode"] != "fixture" && request["mode"] != "live" {
            return fail("invalid_mode", "recorded replay uses read endpoints");
        }
        if !matches!(
            (request["book_id"].as_str(), request["source_id"].as_str()),
            (Some("pg-55"), Some("pg-55-chapter-1"))
                | (Some("pg-36"), Some("pg-36-book-1-chapter-1"))
        ) {
            return fail("invalid_source", "select a configured first chapter");
        }
        if request["mode"] == "live" {
            return fail(
                "live_disabled",
                "live generation requires integrated executor and explicit authorization",
            );
        }
        let req: pt_contracts::RunRequest = serde_json::from_value(request.clone())?;
        let input = pt_contracts::StageInput {
            schema_version: req.schema_version,
            run_id: "pending".into(),
            stage: pt_contracts::Stage::Source,
            unit_id: "pending".into(),
            epoch: 0,
            revision: 0,
            inputs: vec![],
            pins: req.pins,
            mode: req.mode,
            limits: req.limits,
            library_revision: 0,
        };
        input
            .validate()
            .map_err(|e| Error::new("invalid_request", e.to_string()))?;
        if input.limits.wall_time_ms == 0 || input.limits.wall_time_ms > 300_000 {
            return fail("invalid_request", "wall time must be 1..300000 ms");
        }
        let body = serde_json::to_string(request)?;
        if body.len() > 64 * 1024 {
            return fail("request_too_large", "request limit exceeded");
        }
        let id = self.transaction(|tx| {
            if let Some((id, saved)) = tx.query_row("SELECT id,request FROM runs WHERE idempotency=?1",[key], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()? {
                if saved != body { return fail("idempotency_conflict", "key already belongs to a different request"); } return Ok(id);
            }
            let id = digest(format!("{key}:{body}").as_bytes());
            tx.execute("INSERT INTO runs(id,idempotency,request,state,epoch,created) VALUES(?1,?2,?3,'queued',0,?4)",params![id,key,body,now()])?;
            for stage in STAGES { tx.execute("INSERT INTO stages(run,name,state) VALUES(?1,?2,'pending')",params![id,stage])?; }
            Self::emit(tx,&id,"run_queued",&json!({"provenance":"synthetic","mode":"fixture"}))?;
            Ok(id)
        })?;
        self.snapshot(&id)
    }
    pub fn claim(&self) -> Result<Option<(String, i64)>> {
        self.transaction(|tx| {
            let busy: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE state IN ('running','waiting_provider'))",[],|r|r.get(0))?;
            if busy { return Ok(None); }
            let next = tx.query_row("SELECT id,epoch FROM runs WHERE state IN ('queued','recovering') ORDER BY created,id LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?))).optional()?;
            if let Some((id,epoch)) = &next {
                tx.execute("UPDATE runs SET state='running' WHERE id=?1",[id])?;
                Self::emit(tx,id,"run_started",&json!({"epoch":epoch}))?;
            } Ok(next)
        })
    }
    /// Called once by the server process, before starting its worker.
    pub fn recover(&self) -> Result<()> {
        self.transaction(|tx| {
            tx.execute(
                "UPDATE attempts SET state='uncertain' WHERE state IN ('reserved','dispatched')",
                [],
            )?;
            let ids = tx
                .prepare("SELECT id FROM runs WHERE state IN ('running','waiting_provider')")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for id in ids {
                tx.execute(
                    "UPDATE runs SET state=CASE WHEN state='waiting_provider' AND retry_at>?2 THEN 'waiting_provider' ELSE 'recovering' END,epoch=epoch+1 WHERE id=?1",
                    params![id,now()],
                )?;
                tx.execute(
                    "UPDATE stages SET state='pending' WHERE run=?1 AND state='running'",
                    [&id],
                )?;
                Self::emit(
                    tx,
                    &id,
                    "run_recovering",
                    &json!({"uncertain_attempts_retained":true}),
                )?;
            }
            Ok(())
        })?;
        self.reconcile_publications()
    }
    pub fn cancel(&self, run: &str) -> Result<Value> {
        self.transaction(|tx| {
            let state: Option<String> = tx.query_row("SELECT state FROM runs WHERE id=?1",[run],|r|r.get(0)).optional()?;
            let state = state.ok_or_else(||Error::new("not_found","run not found"))?;
            if !matches!(state.as_str(),"succeeded"|"failed"|"cancelled") {
                tx.execute("UPDATE runs SET state='cancelled',epoch=epoch+1 WHERE id=?1",[run])?;
                tx.execute("UPDATE stages SET state='cancelled' WHERE run=?1 AND state NOT IN ('succeeded','failed')",[run])?;
                Self::emit(tx,run,"run_terminal",&json!({"state":"cancelled"}))?;
            } Ok(())
        })?;
        self.snapshot(run)
    }
    pub(crate) fn finish(
        &self,
        run: &str,
        epoch: i64,
        state: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        self.transaction(|tx| {
            Self::active(tx, run, epoch)?;
            tx.execute(
                "UPDATE runs SET state=?3,error=?4 WHERE id=?1 AND epoch=?2",
                params![run, epoch, state, reason],
            )?;
            if state == "failed" {
                tx.execute(
                    "UPDATE stages SET state='failed' WHERE run=?1 AND state='running'",
                    [run],
                )?;
            }
            Self::emit(
                tx,
                run,
                "run_terminal",
                &json!({"state":state,"diagnosis":reason}),
            )?;
            Ok(())
        })
    }
    pub fn snapshot(&self, run: &str) -> Result<Value> {
        let mut db = self.db()?;
        let tx = db.transaction()?;
        let row = tx
            .query_row(
                "SELECT request,state,epoch,created,error FROM runs WHERE id=?1",
                [run],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| Error::new("not_found", "run not found"))?;
        let stages = tx
            .prepare("SELECT name,state FROM stages WHERE run=?1 ORDER BY rowid")?
            .query_map([run], |r| {
                Ok(json!({"name":r.get::<_,String>(0)?,"state":r.get::<_,String>(1)?}))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let artifacts = tx.prepare("SELECT id,name,sha256,bytes,media_type,provenance FROM artifacts WHERE run=?1 ORDER BY rowid")?.query_map([run],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"sha256":r.get::<_,String>(2)?,"bytes":r.get::<_,i64>(3)?,"media_type":r.get::<_,String>(4)?,"provenance":r.get::<_,String>(5)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut public_stages = Vec::new();
        for stage in &stages {
            let name = stage["name"].as_str().unwrap();
            let saved:Option<(String,String)>=tx.query_row("SELECT unit_key,output FROM units WHERE run=?1 AND stage=?2 ORDER BY rowid DESC LIMIT 1",params![run,name],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            let (key, mut artifacts) = match saved {
                Some((key, output)) => (key, serde_json::from_str::<Value>(&output)?),
                None => (format!("pending-{name}"), json!([])),
            };
            if matches!(name, "source" | "acoustic") {
                let refs = tx
                    .prepare("SELECT i.reference FROM run_inputs i WHERE i.run=?1 AND (EXISTS(SELECT 1 FROM run_input_scopes s WHERE s.run=i.run AND s.artifact=i.artifact AND s.scope=?2) OR (?2='saved' AND NOT EXISTS(SELECT 1 FROM run_input_scopes s WHERE s.run=i.run))) ORDER BY i.rowid")?
                    .query_map(params![run,if name=="acoustic"{"acoustic"}else{"saved"}], |r| r.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                for a in refs {
                    artifacts
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::from_str(&a)?);
                }
            }
            let outcome = match stage["state"].as_str() {
                Some("succeeded") => Some("complete"),
                Some("failed") => Some("failed"),
                Some("cancelled") => Some("cancelled"),
                Some("repairing") => Some("needs_repair"),
                _ => None,
            };
            public_stages
                .push(json!({"stage":name,"unit_id":key,"outcome":outcome,"artifacts":artifacts}));
        }
        let mut findings = std::collections::BTreeMap::new();
        let records = tx
            .prepare(
                "SELECT data FROM events WHERE run=?1 AND kind='check_completed' ORDER BY seq",
            )?
            .query_map([run], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for record in records {
            if let Ok(finding) = serde_json::from_str::<pt_contracts::Finding>(&record) {
                findings.insert((finding.field.clone(), finding.id.clone()), finding);
            }
        }
        let findings = findings.into_values().collect::<Vec<_>>();
        let seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(seq),0) FROM events WHERE run=?1",
            [run],
            |r| r.get(0),
        )?;
        Ok(
            json!({"schema_version":1,"id":run,"request":serde_json::from_str::<Value>(&row.0)?,"state":row.1,"epoch":row.2,"created":row.3,"error":row.4,"stages":stages,"artifacts":artifacts,"last_event_seq":seq,"provenance":"synthetic","stage_snapshots":public_stages,"findings":findings}),
        )
    }
    pub fn history(&self, before: i64, limit: usize) -> Result<Vec<Value>> {
        let db = self.db()?;
        let ids = db
            .prepare("SELECT id FROM runs WHERE rowid<?1 ORDER BY rowid DESC LIMIT ?2")?
            .query_map(params![before, limit.clamp(1, 100)], |r| {
                r.get::<_, String>(0)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter().map(|id| self.snapshot(id)).collect()
    }
    pub fn journal(&self, run: &str, after: i64, limit: usize) -> Result<Vec<Value>> {
        let db = self.db()?;
        let max: Option<i64> = db.query_row("SELECT (SELECT COALESCE(MAX(seq),0) FROM events WHERE run=?1) FROM runs WHERE id=?1",[run],|r|r.get(0)).optional()?;
        let max = max.ok_or_else(|| Error::new("not_found", "run not found"))?;
        if after < 0 || after > max {
            return fail("snapshot_required", "invalid event cursor");
        }
        let mut stmt = db.prepare("SELECT seq,epoch,created,kind,data FROM events WHERE run=?1 AND seq>?2 ORDER BY seq LIMIT ?3")?;
        let rows = stmt.query_map(params![run, after, limit.clamp(1, 500)], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        rows.map(|row| {let(s,e,t,k,d)=row?; Ok(json!({"schema_version":1,"run_id":run,"seq":s,"epoch":e,"timestamp":t,"type":k,"data":serde_json::from_str::<Value>(&d)?}))}).collect()
    }
    pub(crate) fn object(&self, hash: &str) -> Result<PathBuf> {
        if !valid_hash(hash) {
            return fail("invalid_digest", "expected SHA-256");
        }
        let path = self.root.join("objects").join(hash);
        if path.exists()
            && (!path.canonicalize()?.starts_with(self.root.join("objects"))
                || std::fs::symlink_metadata(&path)?.file_type().is_symlink())
        {
            return fail("invalid_path", "object escaped storage");
        }
        Ok(path)
    }
    /// Bytes and their directory entry reach stable storage before any DB reference.
    pub(crate) fn put(&self, bytes: &[u8]) -> Result<String> {
        self.put_typed(bytes, "application/octet-stream")
    }
    pub(crate) fn put_typed(&self, bytes: &[u8], media_type: &str) -> Result<String> {
        super::media::validate_artifact_bytes(bytes, media_type)?;
        let hash = digest(bytes);
        let path = self.object(&hash)?;
        if path.exists() {
            super::media::VerifiedArtifact::checked(
                std::fs::File::open(&path)?,
                bytes.len() as u64,
                media_type.into(),
                hash.clone(),
            )?;
            return Ok(hash);
        }
        let mut file = tempfile::NamedTempFile::new_in(self.root.join("objects"))?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        if let Err(e) = file.persist_noclobber(&path) {
            if e.error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(e.error.into());
            }
        }
        std::fs::File::open(self.root.join("objects"))?.sync_all()?;
        super::media::VerifiedArtifact::checked(
            std::fs::File::open(path)?,
            bytes.len() as u64,
            media_type.into(),
            hash.clone(),
        )?;
        Ok(hash)
    }
    pub fn open_artifact(&self, id: &str) -> Result<super::media::VerifiedArtifact> {
        self.open_artifact_bounded(id, super::media::MAX_MASTER_BYTES)
    }
    pub(crate) fn open_artifact_bounded(
        &self,
        id: &str,
        limit: usize,
    ) -> Result<super::media::VerifiedArtifact> {
        let (hash, size, mime): (String, u64, String) = self
            .db()?
            .query_row(
                "SELECT sha256,bytes,media_type FROM artifacts WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| Error::new("not_found", "artifact not found"))?;
        if size > limit as u64 {
            return fail("artifact_too_large", "artifact exceeds the caller envelope");
        }
        let path = self.object(&hash)?;
        if !std::fs::symlink_metadata(&path)
            .map_err(|_| Error::new("corrupt_artifact", "artifact missing"))?
            .is_file()
        {
            return fail("corrupt_artifact", "artifact is not a regular file");
        }
        super::media::VerifiedArtifact::checked(std::fs::File::open(path)?, size, mime, hash)
    }
    pub fn artifact(&self, id: &str) -> Result<(Vec<u8>, String, String)> {
        self.artifact_bounded(id, super::media::MAX_MASTER_BYTES)
    }
    pub(crate) fn artifact_bounded(
        &self,
        id: &str,
        limit: usize,
    ) -> Result<(Vec<u8>, String, String)> {
        use std::io::Read;
        let verified = self.open_artifact_bounded(id, limit)?;
        let mut bytes = Vec::with_capacity(verified.bytes as usize);
        verified
            .file
            .take(verified.bytes + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != verified.bytes || digest(&bytes) != verified.sha256 {
            return fail("corrupt_artifact", "artifact changed during reading");
        }
        Ok((bytes, verified.media_type, verified.sha256))
    }
    pub(crate) fn read_object(&self, hash: &str, size: usize) -> Result<Vec<u8>> {
        self.read_object_typed(hash, size, "application/octet-stream")
    }
    fn read_object_typed(&self, hash: &str, size: usize, media_type: &str) -> Result<Vec<u8>> {
        let p = self.object(hash)?;
        if size > super::media::artifact_limit(media_type)
            || std::fs::metadata(&p)
                .map_err(|_| Error::new("corrupt_artifact", "artifact missing or unreadable"))?
                .len()
                != size as u64
        {
            return fail("corrupt_artifact", "artifact size changed");
        }
        let bytes = std::fs::read(p)?;
        if digest(&bytes) != hash {
            return fail("corrupt_artifact", "artifact digest changed");
        }
        super::media::validate_artifact_bytes(&bytes, media_type)
            .map_err(|_| Error::new("corrupt_artifact", "stored media validation failed"))?;
        Ok(bytes)
    }
    pub async fn changed(&self) {
        self.wake.notified().await;
    }
}

impl Store {
    pub fn api_snapshot(&self, run: &str) -> Result<pt_contracts::RunSnapshot> {
        let v = self.snapshot(run)?;
        Ok(pt_contracts::RunSnapshot {
            schema_version: 1,
            run_id: run.to_owned(),
            request_sha256: digest(&serde_json::to_vec(&v["request"])?),
            epoch: v["epoch"].as_u64().unwrap(),
            revision: 0,
            state: serde_json::from_value(v["state"].clone())?,
            last_event_seq: v["last_event_seq"].as_u64().unwrap(),
            stages: serde_json::from_value(v["stage_snapshots"].clone())?,
            findings: serde_json::from_value(v["findings"].clone())?,
        })
    }
    pub fn api_journal(
        &self,
        run: &str,
        after: i64,
        limit: usize,
    ) -> Result<Vec<pt_contracts::RunEvent>> {
        use pt_contracts::{EventKind, JournalProposal, RunEvent};
        self.journal(run, after, limit)?
            .into_iter()
            .map(|v| {
                let data = &v["data"];
                let kind = match v["type"].as_str().unwrap_or("") {
                    "task_completed" => EventKind::TaskCompleted,
                    "director_decision" => EventKind::DirectorDecision,
                    "artifact_ready" => EventKind::ArtifactReady,
                    "check_completed" => EventKind::CheckCompleted,
                    "cache_reused" => EventKind::CacheReused,
                    "provider_wait" => EventKind::ProviderWait,
                    "run_terminal" => EventKind::RunTerminal,
                    "budget_reserved" | "provider_result" => EventKind::BudgetUpdate,
                    _ => EventKind::TaskStarted,
                };
                let entry = if v["type"] == "journal_entry" {
                    serde_json::from_value(data.clone())?
                } else {
                    let mut ids: Vec<String> = data["artifact_ids"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default();
                    if let Some(id) = data["artifact"]["id"].as_str() {
                        ids.push(id.to_owned());
                    }
                    JournalProposal {
                        kind,
                        task_id: data["unit_key"].as_str().map(str::to_owned),
                        decision_id: if kind == EventKind::DirectorDecision {
                            data["id"].as_str().map(str::to_owned)
                        } else {
                            None
                        },
                        artifact_ids: ids,
                        summary: serde_json::to_string(&json!({"type":v["type"],"record":data}))?,
                    }
                };
                Ok(RunEvent {
                    schema_version: 1,
                    run_id: run.to_owned(),
                    seq: v["seq"].as_u64().unwrap(),
                    epoch: v["epoch"].as_u64().unwrap(),
                    revision: 0,
                    timestamp: v["timestamp"].to_string(),
                    entry,
                })
            })
            .collect()
    }
}
impl Store {
    pub fn api_history(&self, before: i64, limit: usize) -> Result<Value> {
        let db = self.db()?;
        let rows = db
            .prepare("SELECT rowid,id FROM runs WHERE rowid<?1 ORDER BY rowid DESC LIMIT ?2")?
            .query_map(params![before, limit.clamp(1, 100)], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let runs = rows
            .iter()
            .map(|(_, id)| self.api_snapshot(id))
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"runs":runs,"next_before":rows.last().map(|r|r.0)}))
    }
}
