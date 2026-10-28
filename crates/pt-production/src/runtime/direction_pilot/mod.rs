//! Private saved Source → Direction → Pilot diagnostic composition.
use super::store::*;
use pt_contracts as c;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
mod inputs;
mod services;
const META: u64 = 1024 * 1024;
const PHASES: [(&str, c::Stage); 6] = [
    ("source", c::Stage::Source),
    ("direction-request", c::Stage::Direction),
    ("direction-result", c::Stage::Direction),
    ("pilot-request", c::Stage::Pilot),
    ("pilot-takes", c::Stage::Pilot),
    ("pilot-review", c::Stage::Pilot),
];
#[derive(Clone, Serialize, Deserialize)]
struct Plan {
    run: String,
    request_sha256: String,
    implementation_sha256: String,
    registration_sha256: String,
    epoch: i64,
    registration: inputs::Registration,
}
#[derive(Clone, Serialize, Deserialize)]
struct Completed {
    output: c::StageOutput,
    artifacts: Vec<c::ArtifactRef>,
    aliases: Vec<(c::ArtifactRef, c::ArtifactRef)>,
}
#[derive(Clone, Serialize)]
pub struct SavedPhase {
    pub phase: String,
    pub stage: c::Stage,
    pub outcome: c::StageOutcome,
    pub inputs: Vec<c::ArtifactRef>,
    pub outputs: Vec<c::ArtifactRef>,
    pub original_mappings: Vec<(c::ArtifactRef, c::ArtifactRef)>,
    pub original_epoch: u64,
    pub committed_epoch: i64,
}
fn equal<A: Serialize, B: Serialize>(a: &A, b: &B) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}
fn file(path: &str, b: &[u8]) -> c::FileRef {
    c::FileRef {
        path: path.into(),
        sha256: digest(b),
        bytes: b.len() as u64,
        media_type: "application/json".into(),
        dimensions: None,
    }
}
fn artifact(run: &str, phase: &str, f: c::FileRef) -> c::ArtifactRef {
    c::ArtifactRef {
        id: digest(format!("{run}:{phase}:{}:{}", f.path, f.sha256).as_bytes()),
        file: f,
        provenance: c::Provenance::Diagnostic,
    }
}
fn revision() -> String {
    digest(
        &[
            include_bytes!("mod.rs").as_slice(),
            include_bytes!("inputs.rs"),
            include_bytes!("services.rs"),
            include_bytes!("../../generation/mod.rs"),
            include_bytes!("../../generation/source.rs"),
            include_bytes!("../../generation/inputs.rs"),
            include_bytes!("../../../../../fixtures/generation/direction-v1/response-schema.json"),
            include_bytes!("../../../../pt-contracts/src/services.rs"),
            include_bytes!("../../../../pt-contracts/src/provider_metadata.rs"),
            include_bytes!("../../../../pt-contracts/src/provider_service.rs"),
            include_bytes!("../../../../pt-contracts/src/source_slice.rs"),
            include_bytes!("../../../../pt-contracts/src/wav.rs"),
            include_bytes!("../../generation/direction.rs"),
            include_bytes!("../../generation/pilot.rs"),
            include_bytes!("../../generation/pilot_provider.rs"),
            include_bytes!("../../generation/pilot_direction.rs"),
            include_bytes!("../../generation/pilot_evidence.rs"),
            c::SERVICE_REVISION.as_bytes(),
            c::acceptance_binding_checker().sha256.as_bytes(),
        ]
        .concat(),
    )
}
fn request(store: &Store, run: &str) -> Result<c::RunRequest> {
    Ok(serde_json::from_value(
        store.snapshot(run)?["request"].clone(),
    )?)
}
fn scope(store: &Store, run: &str, epoch: i64) -> Result<()> {
    Store::active(&store.db()?, run, epoch)?;
    let r = request(store, run)?;
    let expected = inputs::registration()?;
    if r.mode != c::ExecutionMode::Fixture
        || r.pins.recipe_sha256 != RECIPE
        || r.book_id != expected.book_id
        || r.source_id != expected.chapter_id
        || r.pins.profile_sha256 != expected.profile.profile_sha256
        || r.pins.profile_id != expected.profile.profile_id
    {
        return fail(
            "saved_flow_scope",
            "trusted saved source/book/profile recipe required",
        );
    }
    Ok(())
}
fn template<'a>(plan: &'a Plan, path: &str) -> Result<&'a c::ArtifactRef> {
    plan.registration
        .files
        .iter()
        .find(|e| e.artifact.file.path == path)
        .map(|e| &e.artifact)
        .ok_or_else(|| Error::new("saved_flow_input", "registered saved role missing"))
}
fn load(store: &Store, run: &str) -> Result<Plan> {
    let encoded: String = store.db()?.query_row(
        "SELECT reference FROM saved_flow_plans WHERE run=?1",
        [run],
        |r| r.get(0),
    )?;
    let f: c::FileRef = serde_json::from_str(&encoded)?;
    let plan: Plan = serde_json::from_slice(&inputs::bytes(store, &f)?)?;
    if plan.run != run
        || plan.implementation_sha256 != revision()
        || plan.registration_sha256
            != digest(include_bytes!(
                "../../../../../fixtures/runtime/direction-pilot-v1/registration.json"
            ))
        || plan.request_sha256 != digest(&serde_json::to_vec(&request(store, run)?)?)
    {
        return fail(
            "saved_flow_stale",
            "request, compilation or registered original scope changed",
        );
    }
    if !equal(&plan.registration, &inputs::registration()?) {
        return fail("saved_flow_stale", "immutable registration content changed");
    }
    for e in &plan.registration.files {
        inputs::verify_owned(store, run, &e.artifact)?;
    }
    Ok(plan)
}
pub(super) fn freeze(store: &Store, run: &str, epoch: i64, root: &Path) -> Result<()> {
    scope(store, run, epoch)?;
    if store.db()?.query_row(
        "SELECT EXISTS(SELECT 1 FROM saved_flow_plans WHERE run=?1)",
        [run],
        |r| r.get::<_, bool>(0),
    )? {
        let p = load(store, run)?;
        if root.exists() {
            for e in &p.registration.files {
                inputs::local(root, e)?;
            }
        }
        return Ok(());
    }
    let registration = inputs::registration()?;
    inputs::bounds(
        &registration
            .files
            .iter()
            .map(|e| e.artifact.clone())
            .collect::<Vec<_>>(),
    )?;
    let mut frozen = Vec::new();
    for e in &registration.files {
        scope(store, run, epoch)?;
        let b = inputs::local(root, e)?;
        store.put(&b)?;
        frozen.push(e.artifact.clone());
    }
    let plan = Plan {
        run: run.into(),
        request_sha256: digest(&serde_json::to_vec(&request(store, run)?)?),
        implementation_sha256: revision(),
        registration_sha256: digest(include_bytes!(
            "../../../../../fixtures/runtime/direction-pilot-v1/registration.json"
        )),
        epoch,
        registration,
    };
    let b = serde_json::to_vec(&plan)?;
    store.put(&b)?;
    let f = file("private/saved-flow-plan.json", &b);
    store.transaction(|tx|{Store::active(tx,run,epoch)?;for a in &frozen{inputs::insert(tx,run,a,"immutable saved original; no provider authority")?;}tx.execute("INSERT INTO saved_flow_plans VALUES(?1,?2)",params![run,serde_json::to_string(&f)?])?;Store::emit(tx,run,"saved_flow_inputs_frozen",&json!({"source_sha256":template(&plan,"source.json")?.file.sha256,"registration_sha256":plan.registration_sha256,"recipe_sha256":RECIPE,"production_approved":false,"bulk_speech_approved":false}))?;Store::active(tx,run,epoch)?;Ok(())})
}
fn completed(
    store: &Store,
    run: &str,
    ordinal: usize,
) -> Result<Option<(c::StageInput, Completed, i64)>> {
    let row:Option<(String,String,i64,String,String)>=store.db()?.query_row("SELECT input,result,committed_epoch,input_sha256,result_sha256 FROM saved_flow_phases WHERE run=?1 AND phase=?2 AND result IS NOT NULL",params![run,PHASES[ordinal].0],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    row.map(|(input, result, epoch, ih, rh)| {
        let expected = digest(&serde_json::to_vec(&(run, ordinal, &input))?);
        if ih != expected
            || rh != digest(&serde_json::to_vec(&(run, ordinal, &ih, epoch, &result))?)
        {
            return fail("saved_flow_corrupt", "phase checkpoint changed");
        }
        let i: c::StageInput = serde_json::from_str(&input)?;
        let c: Completed = serde_json::from_str(&result)?;
        for a in i.inputs.iter().chain(&c.artifacts) {
            inputs::verify_owned(store, run, a)?;
        }
        Ok((i, c, epoch))
    })
    .transpose()
}
fn require_output(store: &Store, plan: &Plan, phase: usize, path: &str) -> Result<c::ArtifactRef> {
    let (_, done, _) = completed(store, &plan.run, phase)?
        .ok_or_else(|| Error::new("saved_flow_phase", "previous phase is not committed"))?;
    let a = done
        .artifacts
        .into_iter()
        .find(|a| a.file.path == path)
        .ok_or_else(|| Error::new("saved_flow_phase", "required private output missing"))?;
    if let Ok(original) = template(plan, path) {
        if original.file != a.file {
            return fail(
                "saved_flow_stale",
                "compiled bytes differ from the independently frozen saved identity",
            );
        }
        Ok(original.clone())
    } else {
        Ok(a)
    }
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
        return fail("saved_flow_bounds", "private context exceeds metadata cap");
    }
    store.put(&b)?;
    let a = artifact(run, phase, file(path, &b));
    store.transaction(|tx| {
        Store::active(tx, run, epoch)?;
        inputs::insert(tx, run, &a, "runtime-owned immutable phase context")?;
        Store::active(tx, run, epoch)
    })?;
    Ok(a)
}
fn prepare(store: &Store, plan: &Plan, epoch: i64, ordinal: usize) -> Result<c::StageInput> {
    let run = &plan.run;
    let name = PHASES[ordinal].0;
    if let Some((encoded, sha)) = store
        .db()?
        .query_row(
            "SELECT input,input_sha256 FROM saved_flow_phases WHERE run=?1 AND phase=?2",
            params![run, name],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?
    {
        if sha != digest(&serde_json::to_vec(&(run, ordinal, &encoded))?) {
            return fail("saved_flow_corrupt", "prepared scope changed");
        }
        let input: c::StageInput = serde_json::from_str(&encoded)?;
        for a in &input.inputs {
            inputs::verify_owned(store, run, a)?;
        }
        return Ok(input);
    }
    let get = |p: &str| template(plan, p).cloned();
    let mut refs = Vec::new();
    let mut pins = plan.registration.source_pins.clone();
    if ordinal == 0 {
        for p in ["raw-source.txt", "clean-source.txt", "frozen-source.json"] {
            refs.push(get(p)?);
        }
    } else {
        require_output(store, plan, 0, "source.json")?;
        for p in [
            "raw-source.txt",
            "clean-source.txt",
            "frozen-source.json",
            "source.json",
            "private/book-registry.json",
            "private/direction-recipe.json",
            "private/direction-context.json",
        ] {
            refs.push(get(p)?);
        }
        let ctx: Value = serde_json::from_slice(&inputs::bytes(
            store,
            &get("private/direction-context.json")?.file,
        )?)?;
        pins.recipe_sha256 = ctx["recipe"]["sha256"].as_str().unwrap().into();
        if ordinal >= 2 {
            require_output(store, plan, 1, "private/direction-request.json")?;
            for p in [
                "private/direction-request.json",
                "private/direction-response.json",
                "private/direction-observation.json",
            ] {
                refs.push(get(p)?);
            }
        }
        if ordinal >= 3 {
            refs.push(require_output(
                store,
                plan,
                2,
                "private/direction-plan.json",
            )?);
            let (direction, _, _) = completed(store, run, 2)?.unwrap();
            let original = metadata(
                store,
                run,
                epoch,
                name,
                "private/pilot-direction-input.json",
                &direction,
            )?;
            refs.push(original.clone());
            for p in [
                "private/voice-catalog.json",
                "private/pilot-selection.json",
                "private/pilot-recipe.json",
            ] {
                refs.push(get(p)?);
            }
            let mut ctx: Value = serde_json::from_slice(&inputs::bytes(
                store,
                &get("private/pilot-context.json")?.file,
            )?)?;
            ctx["direction_input"] = json!(original.file);
            ctx["request_payloads"] = json!([]);
            ctx["observations"] = json!([]);
            ctx["reviews"] = json!([]);
            pins.recipe_sha256 = ctx["recipe"]["sha256"].as_str().unwrap().into();
            if ordinal >= 4 {
                let mut payloads = Vec::new();
                for p in [
                    "private/pilot/requests/p000-t00.json",
                    "private/pilot/requests/p000-t01.json",
                ] {
                    let a = require_output(store, plan, 3, p)?;
                    payloads.push(a.file.clone());
                    refs.push(a);
                }
                let original_ctx: Value = serde_json::from_slice(&inputs::bytes(
                    store,
                    &get("private/pilot-context.json")?.file,
                )?)?;
                ctx["observations"] = original_ctx["observations"].clone();
                for e in &plan.registration.files {
                    if e.artifact.file.path.starts_with("private/pilot-audio/")
                        || e.artifact.file.path.starts_with("private/pilot-raw/t")
                        || e.artifact.file.path.starts_with("private/pilot-saved/t")
                    {
                        refs.push(e.artifact.clone());
                    }
                }
                if ordinal >= 5 {
                    let a =
                        require_output(store, plan, 4, "private/pilot/requests/p000-review.json")?;
                    payloads.push(a.file.clone());
                    refs.push(a);
                    ctx["reviews"] = original_ctx["reviews"].clone();
                    for e in &plan.registration.files {
                        if e.artifact.file.path == "private/pilot-raw/review.json"
                            || e.artifact.file.path == "private/pilot-saved/review.json"
                        {
                            refs.push(e.artifact.clone());
                        }
                    }
                }
                ctx["request_payloads"] = json!(payloads);
            }
            refs.push(metadata(
                store,
                run,
                epoch,
                name,
                "private/pilot-context.json",
                &ctx,
            )?);
        }
    }
    inputs::bounds(&refs)?;
    let stage = PHASES[ordinal].1;
    let input = c::StageInput {
        schema_version: 1,
        run_id: run.clone(),
        stage,
        unit_id: format!(
            "saved-flow-{}",
            serde_json::to_value(stage)?.as_str().unwrap()
        ),
        epoch: plan.epoch as u64,
        revision: ordinal as u64 + 1,
        inputs: refs,
        pins,
        mode: c::ExecutionMode::Replay,
        limits: request(store, run)?.limits,
        library_revision: plan.registration.library_revision,
    };
    input
        .validate()
        .map_err(|e| Error::new("saved_flow_input", e.to_string()))?;
    let encoded = serde_json::to_string(&input)?;
    let ih = digest(&serde_json::to_vec(&(run, ordinal, &encoded))?);
    store.transaction(|tx|{Store::active(tx,run,epoch)?;tx.execute("INSERT INTO saved_flow_phases(run,phase,ordinal,prepared_epoch,input,input_sha256) VALUES(?1,?2,?3,?4,?5,?6)",params![run,name,ordinal,epoch,encoded,ih])?;Store::emit(tx,run,"saved_flow_phase_prepared",&json!({"phase":name,"input_sha256":ih,"original_epoch":plan.epoch,"lease_epoch":epoch,"provider_authority":false}))?;Store::active(tx,run,epoch)?;Ok(())})?;
    Ok(input)
}
async fn execute(
    store: &Store,
    executor: &dyn c::StageExecutor,
    run: &str,
    lease: Option<i64>,
    input: c::StageInput,
) -> Result<(c::StageOutput, Vec<(c::CandidateFile, Vec<u8>)>)> {
    use c::RunServices;
    let services = services::Services {
        store: store.clone(),
        run: run.into(),
        lease,
        allowed: input.inputs.clone(),
        candidates: std::sync::Mutex::new(BTreeMap::new()),
    };
    let wall_time = input.limits.wall_time_ms.min(30000);
    let future = executor.execute(input, &services);
    tokio::pin!(future);
    let deadline = tokio::time::sleep(std::time::Duration::from_millis(wall_time));
    tokio::pin!(deadline);
    let output = loop {
        tokio::select! {out=&mut future=>break out.map_err(|e|Error::new("saved_flow_stage",e.to_string()))?,_=&mut deadline=>return fail("saved_flow_timeout","private phase exceeded deadline"),_=tokio::time::sleep(std::time::Duration::from_millis(25))=>if let Some(epoch)=lease{Store::active(&store.db()?,run,epoch)?;}}
    };
    if !output.decisions.is_empty()
        || !output.library_updates.is_empty()
        || output.package_candidate.is_some()
        || output.artifacts.is_empty()
        || output.artifacts.len() > 64
        || output.findings.len() > 4096
    {
        return fail(
            "saved_flow_output",
            "private planning cannot grant decisions/library/package authority",
        );
    }
    let mut candidates = Vec::new();
    for a in &output.artifacts {
        let b = services
            .read_candidate(a)
            .await
            .map_err(|e| Error::new("saved_flow_candidate", e.to_string()))?;
        candidates.push((a.clone(), b));
    }
    Ok((output, candidates))
}
pub(super) async fn run(
    store: &Store,
    executor: &dyn c::StageExecutor,
    run: &str,
    epoch: i64,
    until: usize,
) -> Result<()> {
    scope(store, run, epoch)?;
    let plan = load(store, run)?;
    for ordinal in 0..until.min(6) {
        let name = PHASES[ordinal].0;
        if completed(store, run, ordinal)?.is_some() {
            continue;
        }
        let input = prepare(store, &plan, epoch, ordinal)?;
        let (output, candidates) =
            execute(store, executor, run, Some(epoch), input.clone()).await?;
        if output.outcome
            != if ordinal == 0 {
                c::StageOutcome::Complete
            } else {
                c::StageOutcome::NeedsRepair
            }
        {
            return fail("saved_flow_output", "unexpected diagnostic phase outcome");
        }
        let mut artifacts = Vec::new();
        let mut aliases = Vec::new();
        for (candidate, b) in &candidates {
            store.put(b)?;
            let a = artifact(run, name, candidate.file.clone());
            if let Ok(original) = template(&plan, &candidate.file.path) {
                if original.file == candidate.file {
                    aliases.push((original.clone(), a.clone()));
                }
            }
            artifacts.push(a);
        }
        let done = Completed {
            output,
            artifacts,
            aliases,
        };
        let encoded = serde_json::to_string(&done)?;
        if encoded.len() > 8 * META as usize {
            return fail("saved_flow_bounds", "checkpoint metadata limit");
        }
        store.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            for a in &input.inputs {
                inputs::verify_owned(store, run, a)?;
            }
            for a in &done.artifacts {
                inputs::bytes(store, &a.file)?;
                inputs::insert(tx, run, a, "runtime-committed candidate; original provenance preserved separately")?;
            }
            let (prepared, ih): (String, String) = tx.query_row(
                "SELECT input,input_sha256 FROM saved_flow_phases WHERE run=?1 AND phase=?2",
                params![run, name], |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if prepared != serde_json::to_string(&input)?
                || ih != digest(&serde_json::to_vec(&(run, ordinal, &prepared))?) {
                return fail("saved_flow_corrupt", "consumed phase scope changed before commit");
            }
            let rh = digest(&serde_json::to_vec(&(run, ordinal, &ih, epoch, &encoded))?);
            if tx.execute(
                "UPDATE saved_flow_phases SET committed_epoch=?3,result=?4,result_sha256=?5 WHERE run=?1 AND phase=?2 AND result IS NULL",
                params![run, name, epoch, encoded, rh],
            )? != 1 {
                return fail("saved_flow_conflict", "phase was already committed");
            }
            for f in &done.output.findings {
                let mut public = f.clone();
                public.evidence.clear();
                public.message = format!("{name}: {}", f.message.chars().take(900).collect::<String>());
                Store::emit(tx, run, "check_completed", &serde_json::to_value(public)?)?;
            }
            let stage = serde_json::to_value(PHASES[ordinal].1)?.as_str().unwrap().to_owned();
            tx.execute("UPDATE stages SET state=?3 WHERE run=?1 AND name=?2",
                params![run, stage, if ordinal == 0 { "succeeded" } else { "repairing" }])?;
            Store::emit(tx, run, "saved_flow_phase_checkpoint", &json!({
                "phase": name, "source_revision": plan.registration_sha256,
                "outcome": done.output.outcome, "provider_calls": 0, "bulk_speech_approved": false
            }))?;
            Store::active(tx, run, epoch)
        })?;
    }
    Ok(())
}
pub(super) async fn replay(
    store: &Store,
    executor: &dyn c::StageExecutor,
    run: &str,
) -> Result<Vec<(String, Vec<(c::FileRef, Vec<u8>)>)>> {
    load(store, run)?;
    let mut results = Vec::new();
    for ordinal in 0..6 {
        let (input, done, _) = completed(store, run, ordinal)?
            .ok_or_else(|| Error::new("saved_flow_replay", "all phase checkpoints required"))?;
        let (output, candidates) = execute(store, executor, run, None, input).await?;
        if !equal(&output, &done.output) {
            return fail(
                "saved_flow_replay",
                "recorded phase output/cache/findings changed",
            );
        }
        results.push((
            PHASES[ordinal].0.into(),
            candidates.into_iter().map(|(a, b)| (a.file, b)).collect(),
        ));
    }
    Ok(results)
}
impl Store {
    pub fn direction_pilot_phases(&self, run: &str) -> Result<Vec<SavedPhase>> {
        let _ = load(self, run)?;
        let mut phases = Vec::new();
        for (i, (name, stage)) in PHASES.iter().enumerate() {
            if let Some((input, result, epoch)) = completed(self, run, i)? {
                phases.push(SavedPhase {
                    phase: (*name).into(),
                    stage: *stage,
                    outcome: result.output.outcome,
                    inputs: input.inputs,
                    outputs: result.artifacts,
                    original_mappings: result.aliases,
                    original_epoch: input.epoch,
                    committed_epoch: epoch,
                });
            }
        }
        Ok(phases)
    }
    pub fn direction_pilot_artifact(&self, run: &str, phase: &str, path: &str) -> Result<Vec<u8>> {
        let i = PHASES
            .iter()
            .position(|(n, _)| *n == phase)
            .ok_or_else(|| Error::new("not_found", "unknown phase"))?;
        let (_, out, _) =
            completed(self, run, i)?.ok_or_else(|| Error::new("not_found", "phase incomplete"))?;
        let a = out
            .artifacts
            .iter()
            .find(|a| a.file.path == path)
            .ok_or_else(|| Error::new("not_found", "private artifact missing"))?;
        inputs::verify_owned(self, run, a)
    }
}
pub(super) const RECIPE: &str = "b1def6b774001f5c5a991c7b4a4afefc628d9aa37d746239f14d7e567fcaf209";
pub(super) fn registered(stage: pt_contracts::Stage, recipe: &str) -> bool {
    use pt_contracts::Stage;
    matches!(
        (stage, recipe),
        (
            Stage::Source,
            "8b626e55693d774361f507bc2c45f40054e6855d71fbec78ad9072d8812f14b4"
        ) | (
            Stage::Direction,
            "12bb7c33aca1770192584551ab5551f76438db57ea5602d5225b809b32ea80c4"
        ) | (
            Stage::Pilot,
            "d0fd8d6a584cd79ddc95247a0a240bf984a78ca08302d21abcb61e990efa2ce6"
        )
    )
}

pub(super) fn verify_private_bytes(store: &Store, a: &c::ArtifactRef) -> Result<()> {
    let b = inputs::bytes(store, &a.file)?;
    super::media::validate_artifact_bytes(&b, &a.file.media_type)
}
