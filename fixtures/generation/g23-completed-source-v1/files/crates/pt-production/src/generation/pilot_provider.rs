//! Content-only v2 proposals and immutable saved observations; runtime owns all issuance.
use super::*;
use serde::Deserialize;
use serde_json::Value;
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    pub schema_version: u32,
    pub provenance: Provenance,
    pub request: ArtifactRef,
    pub response: Option<ArtifactRef>,
    pub issued: Option<ProviderOperationV2>,
    pub result: Option<ProviderResultV2>,
    pub findings: Vec<Finding>,
}
pub(super) type Saved = BTreeMap<String, (FileRef, Observation)>;
pub(super) struct Job {
    pub id: String,
    pub request: Value,
    pub kind: OperationKind,
    pub provider: Id,
    pub model: Id,
    pub inputs: Vec<ArtifactRef>,
    pub allow_dispatch: bool,
}
fn need(ok: bool, msg: &str) -> Result<(), StageError> {
    require(ok, "pilot-provider", "evidence", msg)?;
    Ok(())
}
pub(super) fn saved(files: &Files, refs: &[FileRef]) -> Result<Saved, StageError> {
    let mut out = BTreeMap::new();
    for f in refs {
        let (actual, b) = files
            .get(&f.path)
            .ok_or_else(|| StageError::Evidence("missing pilot observation".into()))?;
        need(actual == f, "stale observation reference")?;
        let obs: Observation = parse(b)?;
        need(
            obs.schema_version == 1
                && matches!(
                    obs.provenance,
                    Provenance::Synthetic | Provenance::Diagnostic | Provenance::Imported
                ),
            "pilot observation provenance/schema",
        )?;
        need(
            out.insert(obs.request.file.path.clone(), (f.clone(), obs))
                .is_none(),
            "duplicate pilot observation request",
        )?;
    }
    Ok(out)
}
pub(super) async fn collect(
    input: &StageInput,
    services: &dyn RunServices,
    files: &mut Files,
    saved: &mut Saved,
    jobs: &[Job],
    recipe: &FileRef,
    estimate: &Usage,
    timeout_ms: u64,
    payloads: &[FileRef],
    outputs: &mut Vec<(String, Value)>,
    findings: &mut Vec<Finding>,
) -> Result<Vec<(String, FileRef, Observation)>, StageError> {
    let observer = ProviderObserverV2 {
        schema_version: 2,
        run_id: input.run_id.clone(),
        epoch: input.epoch,
        unit_id: input.unit_id.clone(),
    };
    let mut observations = vec![];
    for job in jobs {
        let path = format!("private/pilot/requests/{}.json", job.id);
        let b = json(&job.request)?;
        need(b.len() <= 1024 * 1024, "pilot request limit")?;
        let expected = FileRef {
            path: path.clone(),
            sha256: digest(&b),
            bytes: b.len() as u64,
            media_type: "application/json".into(),
            dimensions: None,
        };
        outputs.push((path.clone(), job.request.clone()));
        let registered = payloads.iter().find(|f| f.path == path);
        let Some(payload) = input.inputs.iter().find(|a| registered == Some(&a.file)) else {
            need(
                !saved.contains_key(&path),
                "observation without committed request",
            )?;
            findings.push(missing("pilot-request-commit-pending","Runtime must commit exact request bytes before v2 issuance; candidates are not committed ArtifactRefs."));
            continue;
        };
        need(
            payload.file == expected,
            "committed pilot request differs from exact compilation",
        )?;
        let proposal = ProviderProposalV2 {
            schema_version: 2,
            kind: job.kind,
            provider: job.provider.clone(),
            model: job.model.clone(),
            recipe_sha256: recipe.sha256.clone(),
            payload: payload.clone(),
            inputs: job.inputs.clone(),
            estimate: estimate.clone(),
            timeout_ms,
        };
        proposal.validate()?;
        outputs.push((
            format!("private/pilot/proposals/{}.json", job.id),
            serde_json::to_value(&proposal).unwrap(),
        ));
        let mut fresh = false;
        let mut record = saved.remove(&path);
        if record.is_none() && input.mode == ExecutionMode::Live && job.allow_dispatch {
            match services.issue_provider_v2(proposal.clone()).await {
                Ok(issued) => {
                    issued.validate_for_proposal(&proposal, &observer)?;
                    services.check_cancelled().await?;
                    let mut obs = Observation {
                        schema_version: 1,
                        provenance: Provenance::Diagnostic,
                        request: payload.clone(),
                        response: None,
                        issued: Some(issued.clone()),
                        result: None,
                        findings: vec![],
                    };
                    match services.execute_provider_v2(&issued).await{
                        Ok(result)=>{result.validate_for_operation(&issued)?;if usable(&result){obs.response=result.result.raw_response.clone();}else{obs.findings.push(missing("pilot-provider-result-unresolved","Uncertain/obsolete/retry/recovery result retains original usage and grants no usable pilot evidence."));}obs.result=Some(result);},
                        Err(ServiceError::Cancelled)=>return Err(ServiceError::Cancelled.into()),
                        Err(_)=>obs.findings.push(missing("pilot-provider-v2-unavailable","Issued attempt unavailable; resolve original issuance without fallback or repurchase.")),
                    }
                    let value = serde_json::to_value(&obs).unwrap();
                    let b = json(&value)?;
                    let f = FileRef {
                        path: format!("private/pilot/observations/{}.json", job.id),
                        sha256: digest(&b),
                        bytes: b.len() as u64,
                        media_type: "application/json".into(),
                        dimensions: None,
                    };
                    outputs.push((f.path.clone(), value));
                    record = Some((f, obs));
                    fresh = true;
                }
                Err(ServiceError::Cancelled) => return Err(ServiceError::Cancelled.into()),
                Err(_) => findings.push(missing(
                    "pilot-provider-v2-unavailable",
                    "Runtime v2 methods unavailable/denied; no legacy/helper/model fallback.",
                )),
            }
        }
        if let Some((f, obs)) = record {
            need(
                serde_json::to_value(&obs.request).ok() == serde_json::to_value(payload).ok(),
                "saved pilot request ArtifactRef changed",
            )?;
            if let Some(issued) = &obs.issued {
                issued.validate_for_proposal(&proposal, &observer)?;
                if let Some(result) = &obs.result {
                    result.validate_for_operation(issued)?;
                    need(
                        obs.response.as_ref().is_none_or(|a| {
                            usable(result)
                                && result.result.raw_response.as_ref().is_some_and(|r| {
                                    serde_json::to_value(r).ok() == serde_json::to_value(a).ok()
                                })
                        }),
                        "pilot result/raw identity changed",
                    )?;
                } else {
                    need(
                        obs.response.is_none(),
                        "pilot response lacks completed result",
                    )?;
                }
            } else {
                need(obs.result.is_none(), "pilot result lacks original issuance")?;
            }
            if let Some(raw) = &obs.response {
                need(raw.file.bytes <= 1024 * 1024, "pilot raw response limit")?;
                if fresh {
                    load(services, files, raw, 1024 * 1024).await?;
                } else {
                    need(
                        input.inputs.iter().any(|a| {
                            serde_json::to_value(a).ok() == serde_json::to_value(raw).ok()
                        }),
                        "pilot raw ArtifactRef missing or changed",
                    )?;
                }
            }
            findings.extend(obs.findings.clone());
            observations.push((job.id.clone(), f, obs));
        } else {
            findings.push(missing(
                &format!("pilot-observation-missing:{}", job.id),
                "No usable saved result; fixture/replay never redispatch.",
            ));
        }
    }
    Ok(observations)
}
fn usable(r: &ProviderResultV2) -> bool {
    r.result.completion == ProviderCompletion::Completed
        && r.epoch_disposition == ProviderEpochDisposition::Current
        && r.retry.is_none()
        && r.recovery.is_none()
}
pub(super) async fn load(
    services: &dyn RunServices,
    files: &mut Files,
    a: &ArtifactRef,
    limit: u64,
) -> Result<(), StageError> {
    need(a.file.bytes <= limit, "pilot result artifact limit")?;
    a.file.validate()?;
    if let Some((old, _)) = files.get(&a.file.path) {
        need(
            *old == a.file,
            "pilot result path collides with different bytes",
        )?;
        return Ok(());
    }
    need(
        files.len() < 64
            && files
                .values()
                .map(|(f, _)| f.bytes)
                .sum::<u64>()
                .saturating_add(a.file.bytes)
                <= 64 * 1024 * 1024,
        "pilot result aggregate limit",
    )?;
    services.check_cancelled().await?;
    let b = services.read_artifact(a).await?;
    a.file.verify(&b)?;
    services.check_cancelled().await?;
    files.insert(a.file.path.clone(), (a.file.clone(), b));
    Ok(())
}
