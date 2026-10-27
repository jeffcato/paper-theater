//! Private generation declarations; runtime independently pins policy and executes acceptance.
use super::*;
use serde_json::Value;
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
fn rank(scope: AcceptanceScope) -> u8 {
    match scope {
        AcceptanceScope::Diagnostic => 0,
        AcceptanceScope::AutonomousExcerpt => 1,
        AcceptanceScope::ProductionChapter => 2,
    }
}
pub(super) fn scope(files: &Files, findings: &mut Vec<Finding>) -> AcceptanceScope {
    let parsed = files
        .get("acceptance-context.json")
        .filter(|(f, _)| f.bytes <= META)
        .and_then(|(_, b)| serde_json::from_slice::<Context>(b).ok());
    if let Some(ctx) = parsed
        && ctx.schema_version == 1
    {
        let (requested, minimum, purpose) = (ctx.scope, ctx.minimum_scope, ctx.purpose);
        let selected = [requested, minimum, purpose]
            .into_iter()
            .max_by_key(|s| rank(*s))
            .unwrap();
        if selected != requested {
            findings.push(Finding{id:"acceptance-scope".into(),field:"acceptance.context".into(),outcome:ReviewOutcome::Fail,message:"Requested acceptance scope would reduce the supplied minimum or source purpose.".into(),evidence:vec![]});
        }
        return selected;
    }
    findings.push(missing("acceptance-context","Explicit pinned scope/binding context is missing or malformed; retain the strongest shared requirements."));
    AcceptanceScope::ProductionChapter
}

pub(super) fn public_assets(
    doc: &mut PlaybackDocument,
    input: &StageInput,
    files: &Files,
) -> Result<(), StageError> {
    let mut protected = [
        doc.manifest.source_sha256.clone(),
        doc.read_along.narration_sha256.clone(),
        doc.read_along.alignment_sha256.clone(),
        input.pins.recipe_sha256.clone(),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let authored = doc
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(doc.manifest.assets.iter())
        .collect::<Vec<_>>();
    protected.extend(
        files
            .values()
            .filter(|(f, _)| !authored.contains(&f))
            .map(|(f, _)| f.sha256.clone()),
    );
    protected.extend(
        files
            .iter()
            .filter(|(p, _)| {
                p.starts_with("private/")
                    || p.starts_with("evidence/")
                    || p.starts_with("meta/")
                    || p.starts_with("transcribe/")
            })
            .map(|(_, (f, _))| f.sha256.clone()),
    );
    if let Some((f, b)) = files.get("acceptance-context.json")
        && f.bytes <= META
        && let Ok(ctx) = serde_json::from_slice::<Context>(b)
    {
        protected.extend(
            ctx.inputs
                .iter()
                .filter(|i| {
                    !matches!(
                        i.role,
                        AcceptanceInputRole::PresentationMap | AcceptanceInputRole::Profile
                    )
                })
                .map(|i| i.file.sha256.clone()),
        );
        protected.extend(
            ctx.reports
                .iter()
                .chain(ctx.checker_identity.iter())
                .chain(ctx.inherited_inventory.iter())
                .map(|f| f.sha256.clone()),
        );
        protected.extend(
            ctx.observations
                .iter()
                .map(|o| o.raw_evidence.sha256.clone()),
        );
        if let Some(f) = ctx.inherited_inventory
            && let Ok(b) = bytes(files, &f)
            && let Ok(v) = serde_json::from_slice::<AcceptanceEvidenceInventory>(b)
        {
            protected.extend(v.proposals.iter().map(|p| p.raw_evidence.sha256.clone()));
        }
    }
    require(
        !doc.manifest
            .entries
            .files()
            .iter()
            .any(|f| protected.contains(&f.sha256)),
        "public_export",
        "entries",
        "a private record cannot alias a required public entry",
    )?;
    doc.manifest.assets.retain(|f| {
        !f.path.starts_with("private/")
            && !protected.contains(&f.sha256)
            && (f.media_type.starts_with("image/")
                || f.media_type.starts_with("font/")
                || (f.path == "presentation-map.json"
                    && f.sha256 == doc.timeline.presentation_map_sha256)
                || (f.path == "profile.json" && f.sha256 == input.pins.profile_sha256))
    });
    for actor in &doc.actors {
        require(
            doc.manifest.assets.contains(&actor.image),
            "public_export",
            "actors",
            "a private or unlisted file cannot become a public actor",
        )?;
    }
    Ok(())
}

use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;
const META: u64 = 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    id: Id,
    check_id: Id,
    target: AcceptanceTarget,
    outcome: ReviewOutcome,
    provenance: Provenance,
    raw_evidence: FileRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    scope: AcceptanceScope,
    minimum_scope: AcceptanceScope,
    purpose: AcceptanceScope,
    source_scope_id: Id,
    source_span: SourceSpan,
    narration_sample_rate: u32,
    narration_sample_count: u64,
    player_version: String,
    candidate_checker_version: String,
    device_id: Option<Id>,
    inputs: Vec<AcceptanceInput>,
    checker_identity: Option<FileRef>,
    additional_required_check_ids: Vec<Id>,
    inherited_inventory: Option<FileRef>,
    observations: Vec<Observation>,
    reports: Vec<FileRef>,
}
fn finding(id: impl Into<Id>, outcome: ReviewOutcome, message: &str) -> Finding {
    Finding {
        id: id.into(),
        field: "acceptance.proposals".into(),
        outcome,
        message: message.into(),
        evidence: vec![],
    }
}
fn bytes<'a>(files: &'a Files, f: &FileRef) -> Result<&'a [u8], String> {
    f.validate().map_err(|e| e.to_string())?;
    let (actual, b) = files
        .get(&f.path)
        .ok_or_else(|| format!("missing pinned input: {}", f.path))?;
    if actual != f {
        return Err(format!("stale input identity: {}", f.path));
    }
    Ok(b)
}
fn wave(bytes: &[u8]) -> Result<(u32, u64), String> {
    crate::runtime::validate_artifact_bytes(bytes, "audio/wav").map_err(|e| e.to_string())?;
    let (mut p, mut rate, mut block, mut frames) = (12, 0, 0, 0);
    while p < bytes.len() {
        let n = u32::from_le_bytes(bytes[p + 4..p + 8].try_into().unwrap()) as usize;
        let b = &bytes[p + 8..p + 8 + n];
        match &bytes[p..p + 4] {
            b"fmt " => {
                rate = u32::from_le_bytes(b[4..8].try_into().unwrap());
                block = u16::from_le_bytes(b[12..14].try_into().unwrap());
            }
            b"data" => frames = n as u64 / block as u64,
            _ => {}
        }
        p += 8 + n + n % 2;
    }
    Ok((rate, frames))
}
fn fact_inventory(
    ctx: &Context,
    doc: &PlaybackDocument,
    files: &Files,
    findings: &mut Vec<Finding>,
) -> AcceptanceFactInventory {
    let mut facts = vec![];
    for g in &doc.manifest.reveals {
        for (n, f) in g.facts.iter().enumerate() {
            let id = format!("{}-fact-{n}", g.id);
            let association = doc
                .acceptance
                .attestations
                .iter()
                .find(|a| {
                    a.id == f.image_attestation_id
                        && a.source_sha256 == doc.manifest.source_sha256
                        && a.kind == EvidenceKind::ImageScope
                        && a.source_span == f.source_span
                })
                .and_then(|a| {
                    doc.timeline
                        .scenes
                        .iter()
                        .flat_map(|s| &s.entry_snapshot.instances)
                        .filter(|i| g.contributor_ids.contains(&i.id))
                        .filter_map(|i| doc.actors.iter().find(|c| c.id == i.asset_id))
                        .find(|c| c.image.sha256 == a.target_sha256)
                });
            if let Some(card) = association
                && valid_id(&id)
                && bytes(files, &card.image).is_ok()
                && f.source_span[0] >= ctx.source_span[0]
                && f.source_span[0] < f.source_span[1]
                && f.source_span[1] <= ctx.source_span[1]
            {
                facts.push(AcceptanceFact {
                    id,
                    reveal_id: g.id.clone(),
                    source_span: f.source_span,
                    asset_sha256: card.image.sha256.clone(),
                });
            } else {
                findings.push(finding(format!("unbound-authored-fact:{id}"),ReviewOutcome::Missing,"Authored source/reveal/image association is missing or ambiguous; no dummy fact or asset identity was emitted."));
            }
        }
    }
    AcceptanceFactInventory {
        schema_version: 1,
        source_scope_id: ctx.source_scope_id.clone(),
        source_sha256: doc.manifest.source_sha256.clone(),
        source_span: ctx.source_span,
        facts,
        required_cue_ids: doc.timeline.cues.iter().map(|c| c.id.clone()).collect(),
    }
}
fn binding(
    ctx: &Context,
    doc: &PlaybackDocument,
    input: &StageInput,
    files: &Files,
    inventory: &FileRef,
    facts: &FileRef,
) -> Result<AcceptanceBinding, String> {
    let roles = [
        AcceptanceInputRole::Source,
        AcceptanceInputRole::Narration,
        AcceptanceInputRole::Alignment,
        AcceptanceInputRole::PresentationMap,
        AcceptanceInputRole::Recipe,
        AcceptanceInputRole::Profile,
        AcceptanceInputRole::Player,
    ];
    if ctx.inputs.len() != roles.len()
        || roles
            .iter()
            .any(|r| ctx.inputs.iter().filter(|i| i.role == *r).count() != 1)
    {
        return Err(
            "Seven unique pinned source/media/recipe/profile/player roles are required.".into(),
        );
    }
    let get = |r| &ctx.inputs.iter().find(|i| i.role == r).unwrap().file;
    for i in &ctx.inputs {
        bytes(files, &i.file)?;
    }
    let checker_file = ctx
        .checker_identity
        .as_ref()
        .ok_or("Pinned checker identity input is missing.")?;
    let checker: AcceptanceCheckerIdentity =
        serde_json::from_slice(bytes(files, checker_file)?).map_err(|e| e.to_string())?;
    if checker != acceptance_binding_checker() {
        return Err("Checker implementation/version identity is stale.".into());
    }
    let profile: RenderingProfile =
        serde_json::from_slice(bytes(files, get(AcceptanceInputRole::Profile))?)
            .map_err(|e| e.to_string())?;
    let map: PresentationMap =
        serde_json::from_slice(bytes(files, get(AcceptanceInputRole::PresentationMap))?)
            .map_err(|e| e.to_string())?;
    let alignment: Value =
        serde_json::from_slice(bytes(files, get(AcceptanceInputRole::Alignment))?)
            .map_err(|e| e.to_string())?;
    if bytes(files, get(AcceptanceInputRole::Source))? != doc.source_text.as_bytes()
        || get(AcceptanceInputRole::Narration).sha256 != doc.read_along.narration_sha256
        || wave(bytes(files, get(AcceptanceInputRole::Narration))?)?
            != (ctx.narration_sample_rate, ctx.narration_sample_count)
        || get(AcceptanceInputRole::Alignment).sha256 != doc.read_along.alignment_sha256
        || alignment["source_sha256"] != doc.manifest.source_sha256
        || alignment["narration_sha256"] != doc.read_along.narration_sha256
        || get(AcceptanceInputRole::PresentationMap).sha256 != doc.timeline.presentation_map_sha256
        || serde_json::to_value(&map).unwrap()
            != serde_json::to_value(&doc.presentation_map).unwrap()
        || map.source_audio_sha256 != doc.read_along.narration_sha256
        || map.source_sample_rate != ctx.narration_sample_rate
        || map.delivery_sample_rate != doc.manifest.sample_rate
        || map
            .segments
            .iter()
            .any(|s| s.source[1] > ctx.narration_sample_count)
        || get(AcceptanceInputRole::Recipe).sha256 != input.pins.recipe_sha256
        || doc.acceptance.recipe_sha256 != input.pins.recipe_sha256
        || get(AcceptanceInputRole::Profile).sha256 != input.pins.profile_sha256
        || serde_json::to_value(profile).unwrap()
            != serde_json::to_value(&doc.shell.profile).unwrap()
        || ctx.player_version != doc.acceptance.player_version
        || ctx.candidate_checker_version != doc.acceptance.checker_version
        || !valid_id(&ctx.player_version)
        || !valid_id(&ctx.candidate_checker_version)
    {
        return Err("Source/media/map/recipe/profile/version binding mismatch.".into());
    }
    let assets = doc
        .manifest
        .assets
        .iter()
        .map(|f| f.sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(AcceptanceBinding {
        inventory_sha256: inventory.sha256.clone(),
        source_sha256: get(AcceptanceInputRole::Source).sha256.clone(),
        narration_sha256: get(AcceptanceInputRole::Narration).sha256.clone(),
        alignment_sha256: get(AcceptanceInputRole::Alignment).sha256.clone(),
        presentation_map_sha256: get(AcceptanceInputRole::PresentationMap).sha256.clone(),
        master_sha256: doc.manifest.entries.audio.sha256.clone(),
        recipe_sha256: get(AcceptanceInputRole::Recipe).sha256.clone(),
        profile_sha256: get(AcceptanceInputRole::Profile).sha256.clone(),
        player_sha256: get(AcceptanceInputRole::Player).sha256.clone(),
        checker_sha256: checker.sha256,
        fact_inventory_sha256: facts.sha256.clone(),
        asset_sha256s: assets,
        source_span: ctx.source_span,
        source_scope_id: ctx.source_scope_id.clone(),
        player_version: ctx.player_version.clone(),
        candidate_checker_version: ctx.candidate_checker_version.clone(),
        device_id: ctx.device_id.clone(),
        narration_sample_rate: ctx.narration_sample_rate,
        narration_sample_count: ctx.narration_sample_count,
        delivery_sample_rate: doc.manifest.sample_rate,
        delivery_sample_count: doc.manifest.sample_count,
    })
}
fn target_valid(
    target: &AcceptanceTarget,
    check: &str,
    b: &AcceptanceBinding,
    facts: &AcceptanceFactInventory,
    doc: &PlaybackDocument,
) -> bool {
    let class = acceptance_check_matrix()
        .checks
        .into_iter()
        .find(|c| c.id == check)
        .map(|c| c.target)
        .unwrap_or_else(|| "candidate".into());
    match target {
        AcceptanceTarget::Candidate => class == "candidate",
        AcceptanceTarget::Source { span } => class == "source" && *span == b.source_span,
        AcceptanceTarget::Asset { sha256 } => {
            class == "asset"
                && doc
                    .manifest
                    .assets
                    .iter()
                    .any(|f| f.sha256 == *sha256 && f.media_type.starts_with("image/"))
        }
        AcceptanceTarget::Audio { domain, span } => {
            let count = match domain {
                AcceptanceSampleDomain::Narration if class == "narration" => {
                    b.narration_sample_count
                }
                AcceptanceSampleDomain::Delivery if class == "delivery" => b.delivery_sample_count,
                _ => return false,
            };
            span[0] < span[1] && span[1] <= count
        }
        AcceptanceTarget::Fact { fact, domain } => {
            facts.facts.contains(fact)
                && matches!(
                    (domain, class.as_str()),
                    (AcceptanceSampleDomain::Source, "fact_source")
                        | (AcceptanceSampleDomain::Narration, "fact_narration")
                        | (AcceptanceSampleDomain::Delivery, "fact_delivery")
                )
        }
    }
}
fn severity(o: ReviewOutcome) -> u8 {
    match o {
        ReviewOutcome::Pass => 0,
        ReviewOutcome::Missing => 1,
        ReviewOutcome::Uncertain => 2,
        ReviewOutcome::Fail => 3,
    }
}
pub(super) fn public_findings(findings: &[Finding]) -> Vec<Finding> {
    let mut rows: BTreeMap<Id, ReviewOutcome> = BTreeMap::new();
    for f in findings {
        rows.entry(f.id.clone())
            .and_modify(|v| {
                if severity(f.outcome) > severity(*v) {
                    *v = f.outcome
                }
            })
            .or_insert(f.outcome);
    }
    rows.into_iter()
        .map(|(id, outcome)| Finding {
            id,
            field: "acceptance".into(),
            outcome,
            message:
                "Diagnostic observations only; required production verification remains unresolved."
                    .into(),
            evidence: vec![],
        })
        .collect()
}
async fn private<T: Serialize>(
    services: &dyn RunServices,
    input: &StageInput,
    path: &str,
    value: &T,
) -> Result<CandidateFile, StageError> {
    require(
        json(value)?.len() as u64 <= META,
        "acceptance_bounds",
        path,
        "private proposal metadata exceeds one MiB",
    )?;
    write(services, input, path, value).await
}
pub(super) struct Prepared {
    pub artifacts: Vec<CandidateFile>,
    pub findings: Vec<Finding>,
    pub required: Vec<Id>,
}
pub(super) async fn prepare(
    input: &StageInput,
    services: &dyn RunServices,
    files: &Files,
    doc: &PlaybackDocument,
    inventory: &FileRef,
) -> Result<Prepared, StageError> {
    let mut findings = doc.acceptance.findings.clone();
    let selected = scope(files, &mut findings);
    let mut required = doc
        .acceptance
        .required_check_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    required.extend(
        required_acceptance_checks(selected)
            .into_iter()
            .map(|c| c.id),
    );
    let mut artifacts = vec![
        private(
            services,
            input,
            "private/authored-acceptance.json",
            &doc.acceptance,
        )
        .await?,
    ];
    let mut parsed = None;
    let mut eligible = false;
    if let Some((f, b)) = files.get("acceptance-context.json") {
        if f.bytes <= META {
            match serde_json::from_slice::<Context>(b) {
                Ok(ctx)
                    if ctx.schema_version == 1
                        && ctx.inputs.len() <= 7
                        && ctx.observations.len() <= 1024
                        && ctx.reports.len() <= 64
                        && ctx.additional_required_check_ids.len() <= 128 =>
                {
                    eligible = ctx.scope == selected
                        && ctx.scope == ctx.purpose
                        && rank(ctx.scope) >= rank(ctx.minimum_scope)
                        && valid_id(&ctx.source_scope_id)
                        && ctx.source_span == [0, doc.source_text.chars().count() as u64]
                        && ctx
                            .additional_required_check_ids
                            .iter()
                            .all(|s| valid_id(s));
                    if !eligible {
                        findings.push(finding("acceptance-context-binding", ReviewOutcome::Fail, "Context scope/source binding is invalid; retain inherited observations without proposing a complete binding."));
                    }
                    parsed = Some(ctx);
                }
                _ => findings.push(finding(
                    "acceptance-context-binding",
                    ReviewOutcome::Fail,
                    "Malformed/oversized context; no complete binding is proposed.",
                )),
            }
        } else {
            findings.push(finding(
                "acceptance-context-binding",
                ReviewOutcome::Fail,
                "Private context exceeds its metadata bound.",
            ));
        }
    }
    let mut proposals = vec![];
    let mut generated = vec![];
    let mut fact_ref = None;
    let mut bound = None;
    let mut report_refs = vec![];
    if let Some(ctx) = parsed {
        required.extend(
            ctx.additional_required_check_ids
                .iter()
                .filter(|id| valid_id(id))
                .cloned(),
        );
        for r in &ctx.reports {
            match bytes(files, r) {
                Ok(b) => {
                    report_refs.push(r.clone());
                    if let Ok(v) = serde_json::from_slice::<Value>(b) {
                        if let Some(a) = v["findings"].as_array() {
                            for f in a {
                                if let Ok(f) = serde_json::from_value::<Finding>(f.clone()) {
                                    findings.push(f);
                                }
                            }
                        }
                        if let Some(a) = v["facts"].as_array() {
                            for row in a {
                                let original = if row.get("original").is_some() {
                                    &row["original"]
                                } else if row.get("narration").is_some() {
                                    &row["narration"]
                                } else {
                                    row
                                };
                                if let (Some(id), Ok(outcome)) = (
                                    original["id"].as_str(),
                                    serde_json::from_value::<ReviewOutcome>(row["outcome"].clone()),
                                ) {
                                    findings.push(finding(format!("observed:{}:{id}",&r.sha256[..12]),outcome,"Original private observation retained; no reveal/asset applicability is inferred."));
                                }
                            }
                        }
                    }
                }
                Err(e) => findings.push(finding(
                    "private-report-binding",
                    ReviewOutcome::Missing,
                    &e,
                )),
            }
        }
        let facts = fact_inventory(&ctx, doc, files, &mut findings);
        if valid_id(&ctx.source_scope_id)
            && ctx.source_span == [0, doc.source_text.chars().count() as u64]
        {
            let candidate =
                private(services, input, "private/acceptance-facts.json", &facts).await?;
            fact_ref = Some(candidate.file.clone());
            if eligible {
                match binding(&ctx, doc, input, files, inventory, &candidate.file) {
                    Ok(b) => bound = Some(b),
                    Err(e) => findings.push(finding(
                        "acceptance-input-bindings",
                        ReviewOutcome::Missing,
                        &e,
                    )),
                };
            }
            artifacts.push(candidate);
        }
        if let Some(f) = ctx.inherited_inventory {
            match bytes(files, &f).and_then(|b| {
                serde_json::from_slice::<AcceptanceEvidenceInventory>(b).map_err(|e| e.to_string())
            }) {
                Ok(v) if v.schema_version == 1 && v.proposals.len() <= 1024 => proposals.extend(v.proposals),
                _ => findings.push(finding("inherited-evidence-inventory", ReviewOutcome::Fail,
                    "Inherited private evidence inventory is missing, malformed or oversized; its input remains private.")),
            }
        }
        for observation in ctx.observations {
            if valid_id(&observation.check_id) {
                required.insert(observation.check_id.clone());
            }
            let valid = bound.as_ref().is_some_and(|b| {
                target_valid(&observation.target, &observation.check_id, b, &facts, doc)
            }) && valid_id(&observation.id)
                && valid_id(&observation.check_id)
                && bytes(files, &observation.raw_evidence).is_ok()
                && observation.raw_evidence.bytes <= 16 * 1024 * 1024
                && !inventory_contains(doc, &observation.raw_evidence.sha256);
            if let Some(b) = &bound
                && valid
            {
                let p = AcceptanceEvidenceProposal {
                    schema_version: 1,
                    id: observation.id,
                    check_id: observation.check_id,
                    target: observation.target,
                    binding: b.clone(),
                    outcome: observation.outcome,
                    provenance: observation.provenance,
                    raw_evidence: observation.raw_evidence,
                };
                require(
                    json(&p)?.len() <= 64 * 1024,
                    "acceptance_bounds",
                    "proposal",
                    "proposal exceeds shared metadata limit",
                )?;
                generated.push(p.clone());
                proposals.push(p);
            } else {
                if valid_id(&observation.check_id) && observation.outcome != ReviewOutcome::Pass {
                    findings.push(finding(
                        observation.check_id.clone(), observation.outcome,
                        "Original adverse observation retained despite its unresolved binding; no verifier execution is inferred.",
                    ));
                }
                findings.push(finding(format!("unbound-proposal:{}",observation.id),ReviewOutcome::Missing,"Pinned bytes, target/association, domain or implementation identity is unavailable or inconsistent."));
            }
        }
        let mut ids = BTreeSet::new();
        let mut coverage = BTreeSet::new();
        for p in &proposals {
            if valid_id(&p.check_id) {
                required.insert(p.check_id.clone());
            }
            if !ids.insert(p.id.clone()) || !coverage.insert((p.check_id.clone(), json(&p.target)?))
            {
                findings.push(finding(
                    format!("proposal-conflict:{}", p.id),
                    ReviewOutcome::Fail,
                    "Duplicate/conflicting evidence is preserved for independent checking.",
                ));
            }
            let valid = p.schema_version == 1
                && valid_id(&p.id)
                && valid_id(&p.check_id)
                && bound.as_ref() == Some(&p.binding)
                && target_valid(&p.target, &p.check_id, &p.binding, &facts, doc)
                && bytes(files, &p.raw_evidence).is_ok()
                && p.raw_evidence.bytes <= 16 * 1024 * 1024
                && !inventory_contains(doc, &p.raw_evidence.sha256);
            if !valid {
                findings.push(finding(format!("proposal-binding:{}",p.id),ReviewOutcome::Fail,"Inherited/claimed evidence has stale or missing bindings/bytes; the record is retained."));
            }
            if p.outcome != ReviewOutcome::Pass {
                findings.push(finding(
                    p.check_id.clone(),
                    p.outcome,
                    "Adverse private evidence retained; a later passing claim cannot erase it.",
                ));
            }
        }
    }
    required.extend(
        findings
            .iter()
            .filter(|f| f.outcome != ReviewOutcome::Pass)
            .map(|f| f.id.clone()),
    );
    for id in &required {
        if !findings
            .iter()
            .any(|f| f.id == *id && f.outcome != ReviewOutcome::Pass)
        {
            findings.push(missing(id,"A declaration/proposal does not execute the required independent production verifier."));
        }
    }
    findings.push(missing("production-verifier-pending","Runtime must freeze its own policy, execute the independent checker and retain all missing semantic verifiers."));
    required.insert("production-verifier-pending".into());
    let additions = private(
        services,
        input,
        "private/acceptance-proposals.json",
        &generated,
    )
    .await?;
    let ledger = private(
        services,
        input,
        "private/acceptance-evidence-inventory.json",
        &AcceptanceEvidenceInventory {
            schema_version: 1,
            proposals,
        },
    )
    .await?;
    let status = json!({
        "schema_version":1,"scope":selected,
        "context":files.get("acceptance-context.json").map(|(f,_)|f),
        "provenance":"diagnostic","production_approved":false,"provider_calls":0,"reveal_permissions":[],
        "payload_inventory":inventory,"fact_inventory":fact_ref,"binding":bound,
        "proposals":additions.file,"evidence_inventory":ledger.file,"original_reports":report_refs,
        "required_check_ids":required,
        "observations":findings.iter().map(|f|json!({"id":f.id,"field":f.field,"outcome":f.outcome})).collect::<Vec<_>>(),
        "runtime_policy_authority":false
    });
    artifacts.push(additions);
    artifacts.push(ledger);
    artifacts.push(
        private(
            services,
            input,
            "private/acceptance-proposal-status.json",
            &status,
        )
        .await?,
    );
    if input.mode == ExecutionMode::Replay {
        for candidate in &artifacts {
            if let Some((previous, _)) = files.get(&candidate.file.path) {
                require(
                    previous == &candidate.file,
                    "acceptance_replay_binding",
                    &candidate.file.path,
                    "private proposal inputs or identities changed",
                )?;
            }
        }
    }
    Ok(Prepared {
        artifacts,
        findings,
        required: required.into_iter().collect(),
    })
}
fn inventory_contains(doc: &PlaybackDocument, sha: &str) -> bool {
    doc.manifest
        .entries
        .files()
        .into_iter()
        .chain(doc.manifest.assets.iter())
        .any(|f| f.sha256 == sha)
}
