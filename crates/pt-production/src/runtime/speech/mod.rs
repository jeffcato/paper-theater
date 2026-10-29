//! Offline saved Speech. Original observer identities are evidence, never authority.
use super::store::*;
use pt_contracts as c;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
mod batches;
mod pcm;
mod storage;
pub use batches::SpeechBatch;
mod services;
const META: u64 = 1024 * 1024;
const REGISTRATION: &[u8] =
    include_bytes!("../../../../../fixtures/runtime/speech-v1/registration.json");
const PHASES: [&str; 3] = ["requests", "registered", "takes"];
#[derive(Serialize, Deserialize)]
struct Plan {
    run: String,
    request_sha256: String,
    compiler_sha256: String,
    registration_sha256: String,
    original_epoch: i64,
    variant: storage::Variant,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SpeechPhase {
    pub phase: String,
    pub input: c::StageInput,
    pub output: c::StageOutput,
    pub artifacts: Vec<c::ArtifactRef>,
    pub original_mappings: Vec<(c::ArtifactRef, c::ArtifactRef)>,
    pub committed_epoch: i64,
}
fn contract(e: c::ContractError) -> Error {
    Error::new("speech_input", e.to_string())
}
fn equal<T: Serialize, U: Serialize>(a: &T, b: &U) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}
fn file(path: &str, b: &[u8], mime: &str) -> c::FileRef {
    c::FileRef {
        path: path.into(),
        sha256: digest(b),
        bytes: b.len() as u64,
        media_type: mime.into(),
        dimensions: None,
    }
}
fn artifact(run: &str, phase: &str, f: c::FileRef) -> c::ArtifactRef {
    c::ArtifactRef {
        id: digest(&serde_json::to_vec(&(run, phase, &f)).unwrap()),
        file: f,
        provenance: c::Provenance::Diagnostic,
    }
}
fn revision() -> String {
    digest(
        &[
            include_bytes!("mod.rs").as_slice(),
            include_bytes!("storage.rs"),
            include_bytes!("services.rs"),
            include_bytes!("../../generation/mod.rs"),
            include_bytes!("../../generation/inputs.rs"),
            include_bytes!("../../generation/source.rs"),
            include_bytes!("../../generation/direction.rs"),
            include_bytes!("../../generation/pilot.rs"),
            include_bytes!("../../generation/pilot_direction.rs"),
            include_bytes!("../../generation/pilot_provider.rs"),
            include_bytes!("../../generation/pilot_evidence.rs"),
            include_bytes!("../../generation/speech.rs"),
            include_bytes!("../../generation/speech_verify.rs"),
            include_bytes!("../../generation/speech_units.rs"),
            include_bytes!("../../generation/speech_takes.rs"),
            include_bytes!("../../generation/speech_transcription.rs"),
            c::SERVICE_REVISION.as_bytes(),
            c::acceptance_binding_checker().sha256.as_bytes(),
        ]
        .concat(),
    )
}
pub(super) fn is_recipe(recipe: &str) -> bool {
    static RECIPES: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    RECIPES
        .get_or_init(|| {
            let r: Value =
                serde_json::from_slice(REGISTRATION).expect("compiled Speech registration");
            r["variants"]
                .as_array()
                .expect("compiled variant list")
                .iter()
                .map(|v| {
                    v["recipe_sha256"]
                        .as_str()
                        .expect("compiled recipe")
                        .to_owned()
                })
                .collect()
        })
        .contains(recipe)
}
fn request(store: &Store, run: &str) -> Result<c::RunRequest> {
    Ok(serde_json::from_value(
        store.snapshot(run)?["request"].clone(),
    )?)
}
fn scope(store: &Store, run: &str, epoch: i64) -> Result<storage::Variant> {
    Store::active(&store.db()?, run, epoch)?;
    let r = request(store, run)?;
    let v = storage::variant(&r.pins.recipe_sha256)?;
    if r.mode != c::ExecutionMode::Fixture
        || r.book_id != "test-book"
        || r.source_id != "chapter-1"
        || r.pins.profile_sha256 != v.original_input.pins.profile_sha256
        || r.pins.profile_id != v.original_input.pins.profile_id
    {
        return fail(
            "speech_scope",
            "compiled source/profile/diagnostic scope required",
        );
    }
    Ok(v)
}
fn load(store: &Store, run: &str) -> Result<Plan> {
    let b: String = store.db()?.query_row(
        "SELECT reference FROM speech_flows WHERE run=?1",
        [run],
        |r| r.get(0),
    )?;
    let f: c::FileRef = serde_json::from_str(&b)?;
    let plan: Plan = serde_json::from_slice(&storage::bytes(store, &f)?)?;
    if plan.run != run
        || plan.compiler_sha256 != revision()
        || plan.registration_sha256 != digest(REGISTRATION)
        || plan.request_sha256 != digest(&serde_json::to_vec(&request(store, run)?)?)
        || !equal(
            &plan.variant,
            &storage::variant(&request(store, run)?.pins.recipe_sha256)?,
        )
    {
        return fail("speech_stale", "immutable scope or compiler changed");
    }
    for e in plan
        .variant
        .files
        .iter()
        .chain(std::iter::once(&plan.variant.original_file))
    {
        storage::verify_owned(store, run, &e.artifact)?;
    }
    Ok(plan)
}
pub(super) fn freeze(store: &Store, run: &str, epoch: i64, root: &Path) -> Result<()> {
    let variant = scope(store, run, epoch)?;
    if store.db()?.query_row(
        "SELECT EXISTS(SELECT 1 FROM speech_flows WHERE run=?1)",
        [run],
        |r| r.get::<_, bool>(0),
    )? {
        let p = load(store, run)?;
        if root.exists() {
            for e in p
                .variant
                .files
                .iter()
                .chain(std::iter::once(&p.variant.original_file))
            {
                storage::local(root, e)?;
            }
        }
        return Ok(());
    }
    let refs = variant
        .files
        .iter()
        .chain(std::iter::once(&variant.original_file))
        .map(|e| e.artifact.clone())
        .collect::<Vec<_>>();
    storage::bounds(&refs)?;
    for e in variant
        .files
        .iter()
        .chain(std::iter::once(&variant.original_file))
    {
        scope(store, run, epoch)?;
        let b = storage::local(root, e)?;
        if e.artifact.id == variant.original_file.artifact.id {
            let original: c::StageInput = serde_json::from_slice(&b)?;
            if !equal(&original, &variant.original_input)
                || digest(&b) != variant.original_input_sha256
            {
                return fail("speech_input", "original observer changed");
            }
        }
        store.put(&b)?;
    }
    let p = Plan {
        run: run.into(),
        request_sha256: digest(&serde_json::to_vec(&request(store, run)?)?),
        compiler_sha256: revision(),
        registration_sha256: digest(REGISTRATION),
        original_epoch: epoch,
        variant,
    };
    let b = serde_json::to_vec(&p)?;
    store.put(&b)?;
    let f = file("private/speech-flow.json", &b, "application/json");
    store.transaction(|tx|{Store::active(tx,run,epoch)?;for a in &refs{storage::insert(tx,run,a,"registered immutable original; no provider/bulk authority")?;}tx.execute("INSERT INTO speech_flows VALUES(?1,?2)",params![run,serde_json::to_string(&f)?])?;Store::emit(tx,run,"speech_inputs_frozen",&json!({"registration_sha256":p.registration_sha256,"original_observer":{"run_id":p.variant.original_input.run_id,"epoch":p.variant.original_input.epoch,"unit_id":p.variant.original_input.unit_id},"bulk_speech_approved":false}))?;Store::active(tx,run,epoch)})
}
fn done(store: &Store, run: &str, ordinal: usize) -> Result<Option<SpeechPhase>> {
    let row:Option<(String,String,String,String,i64)>=store.db()?.query_row("SELECT result,result_sha256,input,input_sha256,committed_epoch FROM speech_phases WHERE run=?1 AND ordinal=?2 AND result IS NOT NULL",params![run,ordinal],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    row.map(|(b, h, input, ih, epoch)| {
        if digest(&serde_json::to_vec(&(run, ordinal, &b))?) != h
            || digest(&serde_json::to_vec(&(run, ordinal, &input))?) != ih
        {
            return fail("speech_corrupt", "checkpoint/prepared input changed");
        }
        let r: SpeechPhase = serde_json::from_str(&b)?;
        if r.phase != PHASES[ordinal]
            || serde_json::to_string(&r.input)? != input
            || r.committed_epoch != epoch
        {
            return fail("speech_corrupt", "phase identity/scope/epoch changed");
        }
        for a in r.input.inputs.iter().chain(&r.artifacts) {
            storage::verify_owned(store, run, a)?;
        }
        Ok(r)
    })
    .transpose()
}
fn original<'a>(p: &'a Plan, path: &str) -> Result<&'a c::ArtifactRef> {
    p.variant
        .files
        .iter()
        .find(|e| e.artifact.file.path == path)
        .map(|e| &e.artifact)
        .ok_or_else(|| Error::new("speech_input", "original role missing"))
}
fn get_output(store: &Store, run: &str, phase: usize, path: &str) -> Result<c::ArtifactRef> {
    done(store, run, phase)?
        .and_then(|d| d.artifacts.into_iter().find(|a| a.file.path == path))
        .ok_or_else(|| Error::new("speech_phase", "required committed output missing"))
}
fn metadata(
    store: &Store,
    run: &str,
    epoch: i64,
    phase: &str,
    path: &str,
    value: &impl Serialize,
) -> Result<c::ArtifactRef> {
    let b = serde_json::to_vec_pretty(value)?;
    if b.len() as u64 > META {
        return fail("speech_bounds", "context cap");
    }
    store.put(&b)?;
    let a = artifact(run, phase, file(path, &b, "application/json"));
    store.transaction(|tx| {
        Store::active(tx, run, epoch)?;
        storage::insert(
            tx,
            run,
            &a,
            "runtime-owned derivative context; original retained separately",
        )?;
        Store::active(tx, run, epoch)
    })?;
    Ok(a)
}
fn prepare(store: &Store, p: &Plan, epoch: i64, ordinal: usize) -> Result<c::StageInput> {
    let run = &p.run;
    let row: Option<(String, String)> = store
        .db()?
        .query_row(
            "SELECT input,input_sha256 FROM speech_phases WHERE run=?1 AND ordinal=?2",
            params![run, ordinal],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((b, h)) = row {
        if h != digest(&serde_json::to_vec(&(run, ordinal, &b))?) {
            return fail("speech_corrupt", "prepared scope changed");
        }
        let i: c::StageInput = serde_json::from_str(&b)?;
        for a in &i.inputs {
            storage::verify_owned(store, run, a)?;
        }
        return Ok(i);
    }
    let mut i = p.variant.original_input.clone();
    i.mode = c::ExecutionMode::Replay;
    let context = original(p, "private/speech-context.json")?;
    let mut ctx: Value = serde_json::from_slice(&storage::bytes(store, &context.file)?)?;
    let registered = ctx["request_payloads"]
        .as_array()
        .ok_or_else(|| Error::new("speech_input", "payload list missing"))?
        .clone();
    let observations = ctx["observations"].clone();
    ctx["request_payloads"] = json!([]);
    ctx["observations"] = json!([]);
    i.inputs.retain(|a| {
        !(a.file.path.starts_with("private/speech/requests/")
            || a.file.path.starts_with("private/speech-observations/")
            || a.file.path.starts_with("private/speech-raw/")
            || a.file.path.starts_with("private/speech-audio/")
            || a.file.path == "private/speech-context.json")
    });
    if ordinal >= 1 {
        if registered.len() > 32 {
            return fail("speech_bounds", "registration fanout");
        }
        let mut payloads = Vec::new();
        for f in &registered {
            let f: c::FileRef = serde_json::from_value(f.clone())?;
            let a = get_output(store, run, 0, &f.path)?;
            let orig = original(p, &f.path)?;
            if a.file != f || orig.file != f {
                return fail("speech_stale", "request differs from frozen original");
            }
            i.inputs.push(orig.clone());
            payloads.push(f);
        }
        ctx["request_payloads"] = json!(payloads);
    }
    if ordinal >= 2 {
        let count = observations
            .as_array()
            .ok_or_else(|| Error::new("speech_input", "observation list missing"))?
            .len();
        if count > 32 {
            return fail("speech_bounds", "observation fanout");
        }
        for e in &p.variant.files {
            if e.artifact
                .file
                .path
                .starts_with("private/speech-observations/")
                || e.artifact.file.path.starts_with("private/speech-raw/")
                || e.artifact.file.path.starts_with("private/speech-audio/")
            {
                i.inputs.push(e.artifact.clone());
            }
        }
        ctx["observations"] = observations;
    }
    i.inputs.push(metadata(
        store,
        run,
        epoch,
        PHASES[ordinal],
        "private/speech-context.json",
        &ctx,
    )?);
    storage::bounds(&i.inputs)?;
    i.validate().map_err(contract)?;
    let b = serde_json::to_string(&i)?;
    let h = digest(&serde_json::to_vec(&(run, ordinal, &b))?);
    store.transaction(|tx|{Store::active(tx,run,epoch)?;tx.execute("INSERT INTO speech_phases(run,ordinal,input,input_sha256,prepared_epoch) VALUES(?1,?2,?3,?4,?5)",params![run,ordinal,b,h,epoch])?;Store::emit(tx,run,"speech_phase_prepared",&json!({"phase":PHASES[ordinal],"input_sha256":h,"original_epoch":i.epoch,"lease_epoch":epoch,"provider_authority":false}))?;Store::active(tx,run,epoch)})?;
    Ok(i)
}
async fn execute(
    store: &Store,
    executor: &dyn c::StageExecutor,
    run: &str,
    epoch: Option<i64>,
    input: c::StageInput,
) -> Result<(c::StageOutput, Vec<(c::CandidateFile, Vec<u8>)>)> {
    use c::RunServices;
    let s = services::Services {
        store: store.clone(),
        run: run.into(),
        lease: epoch,
        allowed: input.inputs.clone(),
        candidates: std::sync::Mutex::new(BTreeMap::new()),
    };
    let future = executor.execute(input, &s);
    tokio::pin!(future);
    let timer = tokio::time::sleep(std::time::Duration::from_millis(
        request(store, run)?.limits.wall_time_ms.min(30000),
    ));
    tokio::pin!(timer);
    let out = loop {
        tokio::select! {r=&mut future=>break r.map_err(|e|Error::new("speech_stage",e.to_string()))?,_=&mut timer=>return fail("speech_timeout","stage deadline"),_=tokio::time::sleep(std::time::Duration::from_millis(25))=>{if let Some(e)=epoch{Store::active(&store.db()?,run,e)?;}}}
    };
    if out.outcome != c::StageOutcome::NeedsRepair
        || !out.decisions.is_empty()
        || !out.library_updates.is_empty()
        || out.package_candidate.is_some()
        || out.artifacts.is_empty()
        || out.artifacts.len() > 64
        || out.findings.len() > 4096
    {
        return fail("speech_output", "private diagnostic output required");
    }
    let mut candidates = Vec::new();
    let mut paths = BTreeSet::new();
    for a in &out.artifacts {
        if !paths.insert(&a.file.path) {
            return fail("speech_output", "duplicate output path");
        }
        let b = s
            .read_candidate(a)
            .await
            .map_err(|e| Error::new("speech_candidate", e.to_string()))?;
        candidates.push((a.clone(), b));
    }
    Ok((out, candidates))
}
pub(super) async fn run(
    store: &Store,
    executor: &dyn c::StageExecutor,
    run: &str,
    epoch: i64,
    until: usize,
) -> Result<()> {
    scope(store, run, epoch)?;
    let p = load(store, run)?;
    for ordinal in 0..until.min(3) {
        if done(store, run, ordinal)?.is_some() {
            continue;
        }
        let input = prepare(store, &p, epoch, ordinal)?;
        let (output, bytes) = execute(store, executor, run, Some(epoch), input.clone()).await?;
        let mut artifacts = Vec::new();
        let mut mappings = Vec::new();
        for (candidate, b) in bytes {
            store.put(&b)?;
            let a = artifact(run, PHASES[ordinal], candidate.file.clone());
            if let Ok(orig) = original(&p, &a.file.path) {
                if orig.file == a.file {
                    mappings.push((orig.clone(), a.clone()));
                }
            }
            artifacts.push(a);
        }
        let completed = SpeechPhase {
            phase: PHASES[ordinal].into(),
            input,
            output,
            artifacts,
            original_mappings: mappings,
            committed_epoch: epoch,
        };
        let b = serde_json::to_string(&completed)?;
        if b.len() as u64 > 8 * META {
            return fail("speech_bounds", "checkpoint metadata cap");
        }
        let h = digest(&serde_json::to_vec(&(run, ordinal, &b))?);
        store.transaction(|tx|{Store::active(tx,run,epoch)?;for a in &completed.input.inputs{storage::verify_owned(store,run,a)?;}for a in &completed.artifacts{storage::bytes(store,&a.file)?;storage::insert(tx,run,a,"committed candidate; original remains immutable")?;}let (ib,ih):(String,String)=tx.query_row("SELECT input,input_sha256 FROM speech_phases WHERE run=?1 AND ordinal=?2",params![run,ordinal],|r|Ok((r.get(0)?,r.get(1)?)))?;if ib!=serde_json::to_string(&completed.input)?||ih!=digest(&serde_json::to_vec(&(run,ordinal,&ib))?){return fail("speech_corrupt","consumed scope changed");}if tx.execute("UPDATE speech_phases SET result=?3,result_sha256=?4,committed_epoch=?5 WHERE run=?1 AND ordinal=?2 AND result IS NULL",params![run,ordinal,b,h,epoch])?!=1{return fail("speech_conflict","phase already committed");}for f in &completed.output.findings{let mut public=f.clone();public.evidence.clear();Store::emit(tx,run,"check_completed",&serde_json::to_value(public)?)?;}tx.execute("UPDATE stages SET state='repairing' WHERE run=?1 AND name='speech'",[run])?;Store::emit(tx,run,"speech_phase_checkpoint",&json!({"phase":PHASES[ordinal],"registration_sha256":p.registration_sha256,"source_sha256":original(&p,"source.json")?.file.sha256,"compiler_sha256":p.compiler_sha256,"original_epoch":completed.input.epoch,"bulk_speech_approved":false}))?;Store::active(tx,run,epoch)})?;
    }
    Ok(())
}
pub(super) async fn replay(
    store: &Store,
    executor: &dyn c::StageExecutor,
    run: &str,
) -> Result<Vec<(String, Vec<(c::FileRef, Vec<u8>)>)>> {
    load(store, run)?;
    let mut all = Vec::new();
    for ordinal in 0..3 {
        let p = done(store, run, ordinal)?
            .ok_or_else(|| Error::new("speech_phase", "incomplete replay"))?;
        let (output, candidates) = execute(store, executor, run, None, p.input).await?;
        if !equal(&output, &p.output) {
            return fail("speech_replay", "output/findings/cache changed");
        }
        all.push((
            PHASES[ordinal].into(),
            candidates.into_iter().map(|(a, b)| (a.file, b)).collect(),
        ));
    }
    all.push(("pcm".into(), batches::replay(store, run).await?));
    Ok(all)
}
impl Store {
    pub fn speech_phases(&self, run: &str) -> Result<Vec<SpeechPhase>> {
        load(self, run)?;
        (0..3)
            .filter_map(|n| done(self, run, n).transpose())
            .collect()
    }
    pub fn speech_artifact(&self, run: &str, phase: &str, path: &str) -> Result<Vec<u8>> {
        let n = PHASES
            .iter()
            .position(|p| *p == phase)
            .ok_or_else(|| Error::new("not_found", "unknown Speech phase"))?;
        let a = get_output(self, run, n, path)?;
        storage::verify_owned(self, run, &a)
    }
}

pub(super) async fn materialize(store: &Store, run: &str, epoch: i64) -> Result<()> {
    batches::materialize(store, run, epoch).await
}
