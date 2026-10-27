//! Private, sequential units inside the existing Acoustic stage. No publication authority.
use super::store::*;
use pt_contracts as c;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};
mod services;
mod verify;
mod worker;
pub const RECIPE: &str = "76e5cb432d7e3a1a91b1e59a6e27963e4bdf0c1e531f26d5368dda476132d598";
const META: u64 = 1024 * 1024;
const FILE: u64 = 16 * META;
const TOTAL: u64 = 64 * META;
const OUTPUTS: [&str; 3] = [
    "private/chapter-import-validation.json",
    "private/chapter-import-proposals.json",
    "private/chapter-import-evidence-inventory.json",
];
const CODE: [&str; 4] = [
    "chapter_import_helper.py",
    "chapter_import.py",
    "chapter_acoustic.py",
    "adapters.py",
];
#[derive(Clone)]
pub struct ChapterHelper {
    directory: PathBuf,
    stage_revision: String,
}
impl ChapterHelper {
    pub fn new(directory: PathBuf, stage_revision: String) -> Result<Self> {
        if !valid_hash(&stage_revision) {
            return fail(
                "chapter_registration",
                "trusted stage build digest required",
            );
        }
        let value = Self {
            directory,
            stage_revision,
        };
        value.code()?;
        Ok(value)
    }
    fn code(&self) -> Result<Vec<(&'static str, Vec<u8>)>> {
        let mut files = Vec::new();
        let mut all = Vec::new();
        for name in CODE {
            let bytes = verify::local(&self.directory, name, META)?;
            all.extend_from_slice(&bytes);
            files.push((name, bytes));
        }
        if digest(&all) != RECIPE {
            return fail(
                "chapter_registration",
                "G21 helper closure does not match the separately registered recipe",
            );
        }
        Ok(files)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    scope: String,
    file: c::FileRef,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Unit {
    label: String,
    metadata: c::FileRef,
    record: c::FileRef,
    inputs: Vec<c::FileRef>,
    revision: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema_version: u32,
    recipe: String,
    stage_revision: String,
    runtime_revision: String,
    request_sha256: String,
    spec: c::FileRef,
    source_origin: u64,
    pins: Vec<Pin>,
    units: Vec<Unit>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ChapterUnitState {
    pub label: String,
    pub unit_key: String,
    pub outcome: Option<c::StageOutcome>,
    pub outputs: Vec<c::ArtifactRef>,
}
fn runtime_revision() -> String {
    digest(
        &[
            include_bytes!("mod.rs").as_slice(),
            include_bytes!("verify.rs"),
            include_bytes!("services.rs"),
            include_bytes!("worker.rs"),
            include_bytes!("../helper.rs"),
            c::SERVICE_REVISION.as_bytes(),
        ]
        .concat(),
    )
}
fn file(path: &str, bytes: &[u8], mime: &str) -> c::FileRef {
    c::FileRef {
        path: path.into(),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
        media_type: mime.into(),
        dimensions: None,
    }
}
fn alias(f: &c::FileRef, path: &str) -> c::FileRef {
    let mut f = f.clone();
    f.path = path.into();
    f
}
fn key(plan_hash: &str, unit: &Unit) -> String {
    digest(&serde_json::to_vec(&(plan_hash, unit)).unwrap())
}
fn active(store: &Store, run: &str, epoch: i64) -> Result<()> {
    Store::active(&store.db()?, run, epoch)
}
async fn run_stage(
    executor: &dyn c::StageExecutor,
    input: c::StageInput,
    service: &services::ChapterServices,
) -> Result<c::StageOutput> {
    input
        .validate()
        .map_err(|e| Error::new("chapter_input", e.to_string()))?;
    let deadline = tokio::time::sleep(std::time::Duration::from_millis(
        input.limits.wall_time_ms.min(30000),
    ));
    tokio::pin!(deadline);
    let future = executor.execute(input, service);
    tokio::pin!(future);
    loop {
        tokio::select! {
            out=&mut future=>return out.map_err(|e|Error::new(if service.epoch.is_some_and(|e|!service.store.is_active(&service.run,e).unwrap_or(false)){"stale_epoch"}else{"chapter_stage"},e.to_string())),
            _=&mut deadline=>return fail("chapter_stage_timeout","bounded chapter stage exceeded its allowance"),
            _=tokio::time::sleep(std::time::Duration::from_millis(25))=>if let Some(e)=service.epoch {active(&service.store,&service.run,e)?;},
        }
    }
}
fn scope(store: &Store, run: &str, epoch: i64) -> Result<c::RunRequest> {
    active(store, run, epoch)?;
    let request: c::RunRequest = serde_json::from_value(store.snapshot(run)?["request"].clone())?;
    if request.mode != c::ExecutionMode::Fixture || request.pins.recipe_sha256 != RECIPE {
        return fail(
            "chapter_forbidden",
            "registered chapter execution is fixture-only",
        );
    }
    Ok(request)
}
fn load(store: &Store, run: &str, config: Option<&ChapterHelper>) -> Result<(String, Plan)> {
    let (hash, encoded): (String, String) = store
        .db()?
        .query_row(
            "SELECT sha256,reference FROM chapter_plans WHERE run=?1",
            [run],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| Error::new("chapter_plan_missing", "frozen chapter inputs required"))?;
    let reference: c::FileRef = serde_json::from_str(&encoded)?;
    if hash != reference.sha256 {
        return fail(
            "chapter_plan_stale",
            "plan row and immutable reference differ",
        );
    }
    let bytes = verify::cas_bytes(store, &reference, super::media::MAX_ARTIFACT_BYTES as u64)?;
    let plan: Plan = serde_json::from_slice(&bytes)?;
    if plan.schema_version != 1
        || plan.recipe != RECIPE
        || plan.runtime_revision != runtime_revision()
        || config.is_some_and(|c| c.stage_revision != plan.stage_revision)
    {
        return fail(
            "chapter_plan_stale",
            "recipe/runtime/stage compilation changed",
        );
    }
    if plan.request_sha256 != digest(&serde_json::to_vec(&store.snapshot(run)?["request"])?) {
        return fail("chapter_plan_stale", "intended run inputs changed");
    }
    verify::whole(&verify::Tree::Cas(store), &plan, None)?;
    Ok((reference.sha256, plan))
}
impl Store {
    pub fn chapter_units(&self, run: &str) -> Result<Vec<ChapterUnitState>> {
        let (hash, plan) = load(self, run, None)?;
        plan.units
            .iter()
            .map(|u| {
                let k = key(&hash, u);
                let saved: Option<String> = self
                    .db()?
                    .query_row(
                        "SELECT outputs FROM chapter_units WHERE run=?1 AND unit_key=?2",
                        params![run, k],
                        |r| r.get(0),
                    )
                    .optional()?;
                Ok(ChapterUnitState {
                    label: u.label.clone(),
                    unit_key: k,
                    outcome: saved.as_ref().map(|_| c::StageOutcome::NeedsRepair),
                    outputs: saved
                        .map(|s| serde_json::from_str(&s))
                        .transpose()?
                        .unwrap_or_default(),
                })
            })
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn chapter_artifact(&self, run: &str, label: &str, path: &str) -> Result<Vec<u8>> {
        let encoded: String = self.db()?.query_row(
            "SELECT outputs FROM chapter_units WHERE run=?1 AND label=?2",
            params![run, label],
            |r| r.get(0),
        )?;
        let outputs: Vec<c::ArtifactRef> = serde_json::from_str(&encoded)?;
        let a = outputs
            .iter()
            .find(|a| a.file.path == path)
            .ok_or_else(|| Error::new("not_found", "private chapter artifact missing"))?;
        verify::cas_bytes(self, &a.file, META)
    }
}
fn checkpoint(
    store: &Store,
    run: &str,
    label: &str,
    k: &str,
    hash: &str,
    unit: &Unit,
) -> Result<Option<(c::FileRef, Vec<c::ArtifactRef>)>> {
    let row:Option<(String,i64,u64,String,String,String,String)>=store.db()?.query_row("SELECT input_sha256,epoch,revision,record,outputs,findings,checkpoint_sha256 FROM chapter_units WHERE run=?1 AND unit_key=?2 AND status='needs_repair'",params![run,k],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
    row.map(
        |(input, epoch, revision, record, outputs, findings, checksum)| {
            let record: c::FileRef = serde_json::from_str(&record)?;
            let outputs: Vec<c::ArtifactRef> = serde_json::from_str(&outputs)?;
            let findings: Vec<c::Finding> = serde_json::from_str(&findings)?;
            let actual = digest(&serde_json::to_vec(&(
                run, k, label, &input, epoch, revision, &record, &outputs, &findings,
            ))?);
            if input != hash
                || revision != unit.revision
                || actual != checksum
                || record != alias(&unit.record, "chapter-import-helper-record.json")
                || outputs.len() != 3
                || OUTPUTS
                    .iter()
                    .any(|p| outputs.iter().filter(|a| a.file.path == *p).count() != 1)
            {
                return fail(
                    "chapter_checkpoint_corrupt",
                    "unit checkpoint identities changed",
                );
            }
            verify::cas_bytes(store, &record, META)?;
            for a in &outputs {
                if a.provenance != c::Provenance::Diagnostic
                    || a.file.media_type != "application/json"
                {
                    return fail(
                        "chapter_checkpoint_corrupt",
                        "private output identity changed",
                    );
                }
                verify::cas_bytes(store, &a.file, META)?;
            }
            Ok((record, outputs))
        },
    )
    .transpose()
}
pub(super) async fn import(
    store: &Store,
    executor: &dyn c::StageExecutor,
    config: &ChapterHelper,
    run: &str,
    epoch: i64,
    original: &Path,
    evidence: &str,
    bundle: &Path,
) -> Result<()> {
    let request = scope(store, run, epoch)?;
    config.code()?;
    if store.db()?.query_row(
        "SELECT EXISTS(SELECT 1 FROM chapter_plans WHERE run=?1)",
        [run],
        |r| r.get::<_, bool>(0),
    )? {
        let (_, plan) = load(store, run, Some(config))?;
        if original.join(&plan.spec.path).is_file()
            && bundle.join("chapter-import-index.json").is_file()
        {
            if evidence != plan.spec.path {
                return fail(
                    "chapter_plan_stale",
                    "replacement source inventory requires a new frozen run",
                );
            }
            verify::whole(
                &verify::Tree::Roots(original, bundle),
                &plan,
                Some((store, run, epoch)),
            )?;
        }
        return Ok(());
    }
    let snapshot = tempfile::Builder::new()
        .prefix("chapter-freeze-")
        .tempdir_in(store.root.join("scratch"))?;
    let plan = verify::snapshot(
        store,
        run,
        epoch,
        original,
        evidence,
        bundle,
        snapshot.path(),
        &request,
        &config.stage_revision,
    )?;
    let tree = verify::Tree::Local(snapshot.path());
    verify::whole(&tree, &plan, Some((store, run, epoch)))?;
    // Reconstruct every producer record through the supplied real StageExecutor before import.
    for unit in &plan.units {
        let inputs = services::inputs(run, "preflight", unit, true);
        let service = services::ChapterServices::new(
            store.clone(),
            run,
            Some(epoch),
            inputs.clone(),
            services::Source::Local(snapshot.path().join("bundle"), unit.clone()),
            None,
        );
        let input = stage_input(
            &request,
            run,
            epoch,
            unit,
            "preflight",
            inputs,
            c::ExecutionMode::Replay,
        );
        let output = run_stage(executor, input, &service).await?;
        checked_output(&service, output).await?;
    }
    let bytes = serde_json::to_vec(&plan)?;
    if bytes.len() > super::media::MAX_ARTIFACT_BYTES {
        return fail("chapter_bounds", "frozen plan exceeds JSON envelope");
    }
    for pin in &plan.pins {
        active(store, run, epoch)?;
        verify::freeze(store, &tree, pin)?;
    }
    store.put(&bytes)?;
    let reference = file("private/chapter-plan.json", &bytes, "application/json");
    store.transaction(|tx| {
        Store::active(tx,run,epoch)?;
        tx.execute("INSERT INTO chapter_plans VALUES(?1,?2,?3)",params![run,reference.sha256,serde_json::to_string(&reference)?])?;
        Store::emit(tx,run,"chapter_inputs_frozen",&json!({"plan_sha256":reference.sha256,"units":plan.units.len(),"recipe_sha256":RECIPE,"production_approved":false}))?;Ok(())
    })
}
fn stage_input(
    request: &c::RunRequest,
    run: &str,
    epoch: i64,
    unit: &Unit,
    k: &str,
    inputs: Vec<c::ArtifactRef>,
    mode: c::ExecutionMode,
) -> c::StageInput {
    c::StageInput {
        schema_version: 1,
        run_id: run.into(),
        stage: c::Stage::Acoustic,
        unit_id: k.into(),
        epoch: epoch as u64,
        revision: unit.revision,
        inputs,
        pins: request.pins.clone(),
        mode,
        limits: request.limits.clone(),
        library_revision: 0,
    }
}
async fn checked_output(
    service: &services::ChapterServices,
    output: c::StageOutput,
) -> Result<(Vec<(c::CandidateFile, Vec<u8>)>, Vec<c::Finding>)> {
    use c::RunServices;
    if output.outcome != c::StageOutcome::NeedsRepair
        || output.artifacts.len() != 3
        || !output.decisions.is_empty()
        || !output.library_updates.is_empty()
        || output.package_candidate.is_some()
        || output.findings.len() > 2051
    {
        return fail(
            "chapter_output",
            "only bounded private NeedsRepair output is allowed",
        );
    }
    let mut outputs = Vec::new();
    for path in OUTPUTS {
        let matches = output
            .artifacts
            .iter()
            .filter(|a| a.file.path == path)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return fail(
                "chapter_output",
                "all three distinct private outputs required",
            );
        }
        let a = matches[0];
        let bytes = service
            .read_candidate(a)
            .await
            .map_err(|e| Error::new("chapter_candidate", e.to_string()))?;
        if a.file.bytes > META
            || a.file.media_type != "application/json"
            || a.provenance != c::Provenance::Diagnostic
        {
            return fail("chapter_output", "private metadata identity rejected");
        }
        outputs.push((a.clone(), bytes));
    }
    Ok((outputs, output.findings))
}
pub(super) async fn execute_unit(
    store: &Store,
    executor: &dyn c::StageExecutor,
    config: &ChapterHelper,
    run: &str,
    epoch: i64,
    label: &str,
) -> Result<c::StageOutcome> {
    use c::RunServices;
    let request = scope(store, run, epoch)?;
    config.code()?;
    let (hash, plan) = load(store, run, Some(config))?;
    let unit = plan
        .units
        .iter()
        .find(|u| u.label == label)
        .ok_or_else(|| Error::new("chapter_unit_missing", "unit is not in the frozen plan"))?;
    let k = key(&hash, unit);
    if checkpoint(store, run, label, &k, &hash, unit)?.is_some() {
        store.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            Store::emit(
                tx,
                run,
                "chapter_unit_reused",
                &json!({"unit_key":k,"unit":label,"outcome":"needs_repair"}),
            )?;
            Ok(())
        })?;
        return Ok(c::StageOutcome::NeedsRepair);
    }
    store.transaction(|tx| {
        Store::active(tx, run, epoch)?;
        tx.execute(
            "UPDATE stages SET state='running' WHERE run=?1 AND name='acoustic'",
            [run],
        )?;
        Store::emit(
            tx,
            run,
            "chapter_unit_started",
            &json!({"unit":label,"unit_key":k,"revision":unit.revision}),
        )?;
        Ok(())
    })?;
    let mut inputs = services::inputs(run, &k, unit, false);
    let grant = services::Grant {
        config: config.clone(),
        unit_key: k.clone(),
        revision: unit.revision,
        inputs: inputs.clone(),
        timeout_ms: request.limits.wall_time_ms.min(30000),
    };
    let service = services::ChapterServices::new(
        store.clone(),
        run,
        Some(epoch),
        inputs.clone(),
        services::Source::Cas,
        Some(grant.clone()),
    );
    let operation = c::HelperOperation {
        id: format!("chapter-import:{k}"),
        kind: c::HelperKind::AlignAudio,
        recipe_sha256: RECIPE.into(),
        inputs: inputs.clone(),
        timeout_ms: grant.timeout_ms,
        max_output_bytes: META,
    };
    let result = service.helper(operation.clone()).await.map_err(|e| {
        Error::new(
            if !store.is_active(run, epoch).unwrap_or(false) {
                "stale_epoch"
            } else {
                "chapter_helper"
            },
            e.to_string(),
        )
    })?;
    if result.id != operation.id || result.outputs.len() != 1 {
        return fail("chapter_helper", "helper returned a different operation");
    }
    let candidate = &result.outputs[0];
    let bytes = service
        .read_candidate(candidate)
        .await
        .map_err(|e| Error::new("chapter_candidate", e.to_string()))?;
    candidate
        .file
        .verify(&bytes)
        .map_err(|e| Error::new("chapter_candidate", e.to_string()))?;
    if candidate.file != alias(&unit.record, "chapter-import-helper-record.json") {
        return fail(
            "chapter_helper",
            "registered helper disagrees with independently reconstructed record",
        );
    }
    let record = c::ArtifactRef {
        id: digest(format!("{run}:{k}:record:{}", candidate.file.sha256).as_bytes()),
        file: candidate.file.clone(),
        provenance: c::Provenance::Diagnostic,
    };
    service.add_input(record.clone());
    inputs.push(record.clone());
    let input = stage_input(
        &request,
        run,
        epoch,
        unit,
        &k,
        inputs,
        c::ExecutionMode::Fixture,
    );
    let output = run_stage(executor, input, &service).await?;
    let (outputs, mut findings) = checked_output(&service, output).await?;
    if plan.source_origin > 0 {
        findings.push(c::Finding{id:"missing_source_mapping".into(),field:"acoustic.chapter.import".into(),outcome:c::ReviewOutcome::Missing,message:"Nonzero-origin source needs an independently pinned full-source/scalar mapping before playback acceptance.".into(),evidence:vec![]});
    }
    let mut references = Vec::new();
    for (candidate, bytes) in outputs {
        store.put(&bytes)?;
        references.push(c::ArtifactRef {
            id: digest(
                format!(
                    "{run}:{k}:{}:{}",
                    candidate.file.path, candidate.file.sha256
                )
                .as_bytes(),
            ),
            file: candidate.file,
            provenance: c::Provenance::Diagnostic,
        });
    }
    let encoded = serde_json::to_string(&findings)?;
    if encoded.len() > super::media::MAX_ARTIFACT_BYTES {
        return fail("chapter_bounds", "private findings envelope exceeded");
    }
    store.transaction(|tx| {
        Store::active(tx,run,epoch)?;
        let current:String=tx.query_row("SELECT sha256 FROM chapter_plans WHERE run=?1",[run],|r|r.get(0))?;
        if current!=hash {return fail("chapter_plan_stale","frozen inputs changed before checkpoint");}
        for f in unit.inputs.iter().chain([&unit.metadata]) {verify::cas_bytes(store,f,FILE)?;}
        for a in references.iter().chain([&record]) {verify::cas_bytes(store,&a.file,META)?;}
        let checksum=digest(&serde_json::to_vec(&(run,&k,label,&hash,epoch,unit.revision,&record.file,&references,&findings))?);
        tx.execute("INSERT INTO chapter_units VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'needs_repair',?10)",params![run,k,label,hash,epoch,unit.revision,serde_json::to_string(&record.file)?,serde_json::to_string(&references)?,encoded,checksum])?;
        for f in &findings {
            let summary=format!("{label}: {}",f.message).chars().take(1000).collect::<String>();
            Store::emit(tx,run,"check_completed",&json!({"id":f.id,"field":f.field,"outcome":f.outcome,"message":summary,"evidence":[]}))?;
        }
        tx.execute("UPDATE stages SET state='repairing' WHERE run=?1 AND name='acoustic'",[run])?;
        Store::emit(tx,run,"chapter_unit_checkpoint",&json!({"unit":label,"unit_key":k,"revision":unit.revision,"outputs":references.iter().map(|a|&a.file.sha256).collect::<Vec<_>>(),"outcome":"needs_repair"}))?;Ok(())
    })?;
    Ok(c::StageOutcome::NeedsRepair)
}
pub(super) async fn replay(
    store: &Store,
    executor: &dyn c::StageExecutor,
    config: &ChapterHelper,
    run: &str,
    label: &str,
) -> Result<Vec<(c::FileRef, Vec<u8>)>> {
    let (hash, plan) = load(store, run, Some(config))?;
    let unit = plan
        .units
        .iter()
        .find(|u| u.label == label)
        .ok_or_else(|| Error::new("chapter_unit_missing", "unknown frozen unit"))?;
    let k = key(&hash, unit);
    let (record, expected) = checkpoint(store, run, label, &k, &hash, unit)?.ok_or_else(|| {
        Error::new(
            "chapter_replay_missing",
            "all three saved private outputs required",
        )
    })?;
    let snapshot = store.snapshot(run)?;
    let request: c::RunRequest = serde_json::from_value(snapshot["request"].clone())?;
    let mut inputs = services::inputs(run, &k, unit, false);
    inputs.push(c::ArtifactRef {
        id: digest(format!("{run}:{k}:record:{}", record.sha256).as_bytes()),
        file: record,
        provenance: c::Provenance::Diagnostic,
    });
    inputs.extend(expected.iter().cloned());
    let service = services::ChapterServices::new(
        store.clone(),
        run,
        None,
        inputs.clone(),
        services::Source::Cas,
        None,
    );
    let input = stage_input(
        &request,
        run,
        snapshot["epoch"].as_i64().unwrap(),
        unit,
        &k,
        inputs,
        c::ExecutionMode::Replay,
    );
    let output = run_stage(executor, input, &service).await?;
    let (actual, _) = checked_output(&service, output).await?;
    actual
        .into_iter()
        .map(|(a, b)| {
            let old = expected
                .iter()
                .find(|e| e.file.path == a.file.path)
                .unwrap();
            if old.file != a.file || old.provenance != a.provenance {
                return fail("chapter_replay_changed", "private replay identity changed");
            }
            old.file
                .verify(&b)
                .map_err(|e| Error::new("chapter_replay_changed", e.to_string()))?;
            Ok((a.file, b))
        })
        .collect()
}
