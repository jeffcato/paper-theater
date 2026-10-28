//! Private recorded Pilot evidence. Byte/target binding cannot authenticate listening.
use crate::acceptance::Reads;
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
pub const PILOT_REVIEW_CRITERIA: [&str; 8] = [
    "listening",
    "identity",
    "continuity",
    "intelligibility",
    "speech-fidelity",
    "room-tone",
    "safe-joins",
    "music-context",
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotTakeEvidenceV1 {
    pub request: ArtifactRef,
    pub response: ArtifactRef,
    pub observation: FileRef,
    pub audio: ArtifactRef,
    pub source_span: SourceSpan,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotRecordedReviewV1 {
    pub request: ArtifactRef,
    pub response: ArtifactRef,
    pub observation: FileRef,
    pub reviewer: ProviderObserverV2,
    pub provider: Id,
    pub model: Id,
    pub reference_audio: Option<ArtifactRef>,
    pub assembled_audio: Option<ArtifactRef>,
    pub join_recipe: Option<FileRef>,
    pub music_context: Option<FileRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotPerformerPolicyV1 {
    pub speaker: Id,
    pub canonical_id: Id,
    pub model: Id,
    pub voice_id: Id,
    pub takes: Vec<PilotTakeEvidenceV1>,
    pub review: Option<PilotRecordedReviewV1>,
}
/// Expected references come from runtime authority, never a candidate plan or a submitted Pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotReviewPolicyV1 {
    pub schema_version: u32,
    pub plan: FileRef,
    pub source: FileRef,
    pub source_record: FileRef,
    pub direction: FileRef,
    pub registry: FileRef,
    pub catalog: FileRef,
    pub recipe: FileRef,
    pub performers: Vec<PilotPerformerPolicyV1>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotCriterionObservationV1 {
    pub speaker: Id,
    pub id: Id,
    pub claimed_outcome: ReviewOutcome,
    pub effective_outcome: ReviewOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotReviewObservationV1 {
    pub schema_version: u32,
    pub policy_sha256: Digest,
    pub plan_sha256: Digest,
    pub source_sha256: Digest,
    pub binding_outcome: ReviewOutcome,
    pub review_outcome: ReviewOutcome,
    pub recording_count: u32,
    pub review_count: u32,
    pub production_review_executed: bool,
    pub criteria: Vec<PilotCriterionObservationV1>,
    pub reasons: Vec<AcceptanceReason>,
    pub consumed_sha256s: Vec<Digest>,
}
fn need(ok: bool) -> Result<(), AcceptanceReason> {
    if ok {
        Ok(())
    } else {
        Err(AcceptanceReason::InvalidPilotEvidence)
    }
}
fn same(a: &impl Serialize, b: &impl Serialize) -> bool {
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
fn rank(o: ReviewOutcome) -> u8 {
    match o {
        ReviewOutcome::Pass => 0,
        ReviewOutcome::Missing => 1,
        ReviewOutcome::Uncertain => 2,
        ReviewOutcome::Fail => 3,
    }
}
fn worse(a: ReviewOutcome, b: ReviewOutcome) -> ReviewOutcome {
    if rank(a) >= rank(b) { a } else { b }
}
impl PilotReviewPolicyV1 {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 1 && (1..=16).contains(&self.performers.len()),
            "pilot_review",
            "policy",
            "invalid schema/performer bound",
        )?;
        let mut speakers = BTreeSet::new();
        let mut canonicals = BTreeSet::new();
        let mut refs = vec![
            &self.plan,
            &self.source,
            &self.source_record,
            &self.direction,
            &self.registry,
            &self.catalog,
            &self.recipe,
        ];
        for p in &self.performers {
            require(
                [&p.speaker, &p.canonical_id, &p.model, &p.voice_id]
                    .iter()
                    .all(|x| valid_id(x))
                    && speakers.insert(&p.speaker)
                    && canonicals.insert(&p.canonical_id)
                    && (2..=4).contains(&p.takes.len()),
                "pilot_review",
                "performers",
                "unique performers and at least two expected adjacent takes required",
            )?;
            let mut requests = BTreeSet::new();
            let mut end = 0;
            for t in &p.takes {
                require(
                    t.source_span[0] >= end
                        && t.source_span[0] < t.source_span[1]
                        && t.source_span[1] <= MAX_SAFE_INTEGER
                        && requests.insert(&t.request.file.sha256),
                    "pilot_review",
                    "takes",
                    "duplicate/omitted or unordered take",
                )?;
                end = t.source_span[1];
                for a in [&t.request, &t.response, &t.audio] {
                    require(
                        valid_id(&a.id),
                        "pilot_review",
                        "artifact",
                        "invalid artifact ID",
                    )?;
                    refs.push(&a.file);
                }
                refs.push(&t.observation);
                require(
                    t.audio.file.media_type == "audio/wav" && t.audio.file.bytes <= 8 * 1024 * 1024,
                    "pilot_review",
                    "audio",
                    "bounded native WAV required",
                )?;
            }
            if let Some(r) = &p.review {
                r.reviewer.validate()?;
                require(
                    valid_id(&r.provider) && valid_id(&r.model),
                    "pilot_review",
                    "reviewer",
                    "invalid expected reviewer",
                )?;
                for a in [&r.request, &r.response].into_iter().chain(
                    [r.reference_audio.as_ref(), r.assembled_audio.as_ref()]
                        .into_iter()
                        .flatten(),
                ) {
                    require(
                        valid_id(&a.id),
                        "pilot_review",
                        "artifact",
                        "invalid artifact ID",
                    )?;
                    refs.push(&a.file);
                }
                refs.push(&r.observation);
                refs.extend(r.join_recipe.iter());
                refs.extend(r.music_context.iter());
            }
        }
        require(
            refs.len() <= 256
                && refs
                    .iter()
                    .try_fold(0u64, |n, f| n.checked_add(f.bytes))
                    .is_some_and(|n| n <= 96 * 1024 * 1024),
            "pilot_review",
            "bounds",
            "Pilot aggregate exceeds 96 MiB/256 refs",
        )?;
        let mut paths = std::collections::BTreeMap::new();
        for f in refs {
            f.validate()?;
            require(
                f.dimensions.is_none()
                    && f.bytes
                        <= if f.media_type == "audio/wav" {
                            8 * 1024 * 1024
                        } else if f == &self.source {
                            32 * 1024 * 1024
                        } else {
                            1024 * 1024
                        },
                "pilot_review",
                "bounds",
                "invalid private file bound",
            )?;
            if let Some(old) = paths.insert(&f.path, f) {
                require(old == f, "pilot_review", "path", "conflicting private path")?;
            }
        }
        Ok(())
    }
}
impl PilotReviewObservationV1 {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 1
                && [&self.policy_sha256, &self.plan_sha256, &self.source_sha256]
                    .iter()
                    .all(|h| valid_digest(h))
                && !self.production_review_executed
                && self.recording_count <= 64
                && self.review_count <= 16
                && self.criteria.len() <= 128
                && self.consumed_sha256s.len() <= 256
                && self.reasons.len() <= 32
                && self.review_outcome != ReviewOutcome::Pass,
            "pilot_review",
            "observation",
            "invalid or falsely authoritative observation",
        )?;
        require(
            self.reasons.iter().all(|r| {
                matches!(
                    r,
                    AcceptanceReason::UnverifiedPilotReviewer
                        | AcceptanceReason::AdverseEvidence
                        | AcceptanceReason::InvalidPilotEvidence
                        | AcceptanceReason::IncompletePilotEvidence
                        | AcceptanceReason::InvalidBinding
                        | AcceptanceReason::MissingBytes
                        | AcceptanceReason::ChangedBytes
                        | AcceptanceReason::InvalidAudioFormat
                        | AcceptanceReason::PrivateEvidenceExported
                )
            }),
            "pilot_review",
            "reasons",
            "unsupported Pilot reason",
        )?;
        require(
            self.consumed_sha256s.iter().all(|h| valid_digest(h))
                && self.consumed_sha256s.windows(2).all(|p| p[0] < p[1])
                && self
                    .reasons
                    .contains(&AcceptanceReason::UnverifiedPilotReviewer),
            "pilot_review",
            "observation",
            "missing pending reviewer or invalid consumption",
        )?;
        let mut ids = BTreeSet::new();
        let mut outcome = ReviewOutcome::Missing;
        for c in &self.criteria {
            let effective = if c.claimed_outcome == ReviewOutcome::Pass {
                ReviewOutcome::Uncertain
            } else {
                c.claimed_outcome
            };
            require(
                valid_id(&c.speaker)
                    && PILOT_REVIEW_CRITERIA.contains(&c.id.as_str())
                    && ids.insert((&c.speaker, &c.id))
                    && c.effective_outcome == effective,
                "pilot_review",
                "criteria",
                "invalid/duplicate or promoted criterion",
            )?;
            outcome = worse(outcome, effective);
        }
        require(
            rank(self.review_outcome) >= rank(outcome),
            "pilot_review",
            "review_outcome",
            "adverse criterion omitted",
        )?;
        if self.binding_outcome == ReviewOutcome::Pass {
            require(
                self.recording_count >= 2
                    && self.review_count > 0
                    && self.criteria.len() == self.review_count as usize * 8
                    && self.consumed_sha256s.contains(&self.plan_sha256)
                    && self.consumed_sha256s.contains(&self.source_sha256),
                "pilot_review",
                "binding",
                "incomplete recorded observation",
            )?;
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    schema_version: u32,
    provenance: Provenance,
    request: ArtifactRef,
    response: Option<ArtifactRef>,
    issued: Option<ProviderOperationV2>,
    result: Option<ProviderResultV2>,
    findings: Vec<Finding>,
}
struct ObservationScope<'a> {
    kind: OperationKind,
    recipe: &'a Digest,
    model: &'a str,
    inputs: Vec<&'a FileRef>,
    artifacts: Vec<&'a ArtifactRef>,
    output: Option<&'a ArtifactRef>,
    reviewer: Option<&'a PilotRecordedReviewV1>,
}
struct Inspect<'a> {
    reads: &'a mut Reads,
    root: &'a Path,
    hashes: BTreeSet<Digest>,
    values: BTreeMap<(String, Digest), Result<Value, AcceptanceReason>>,
}
impl Inspect<'_> {
    fn read(&mut self, f: &FileRef) -> Result<Vec<u8>, AcceptanceReason> {
        let b = self.reads.read(
            self.root,
            f,
            "pilot-recorded-input",
            if f.media_type == "audio/wav" {
                8 * 1024 * 1024
            } else {
                32 * 1024 * 1024
            },
        )?;
        self.hashes.insert(f.sha256.clone());
        Ok(b)
    }
    fn value(&mut self, f: &FileRef) -> Result<Value, AcceptanceReason> {
        let key = (f.path.clone(), f.sha256.clone());
        if let Some(v) = self.values.get(&key) {
            return v.clone();
        }
        let result = (|| {
            need(f.media_type == "application/json")?;
            serde_json::from_slice(&self.read(f)?)
                .map_err(|_| AcceptanceReason::InvalidPilotEvidence)
        })();
        self.values.insert(key, result.clone());
        result
    }
    fn audio(&mut self, a: &ArtifactRef) -> Result<Pcm16WavInfo, AcceptanceReason> {
        need(a.file.media_type == "audio/wav")?;
        let b = self.read(&a.file)?;
        inspect_pcm16_wav(&mut std::io::Cursor::new(b.as_slice()), b.len() as u64)
            .map_err(|_| AcceptanceReason::InvalidAudioFormat)
    }
    fn observation(
        &mut self,
        f: &FileRef,
        request: &ArtifactRef,
        response: &ArtifactRef,
        scope: ObservationScope<'_>,
    ) -> Result<Vec<Finding>, AcceptanceReason> {
        let o: Observation = serde_json::from_value(self.value(f)?)
            .map_err(|_| AcceptanceReason::InvalidPilotEvidence)?;
        need(
            o.schema_version == 1
                && same(&o.request, request)
                && o.response.as_ref().is_some_and(|x| same(x, response))
                && o.provenance != Provenance::Autonomous
                && o.findings.len() <= 64,
        )?;
        match (&o.issued, &o.result) {
            (Some(i), Some(r)) => {
                i.validate()
                    .map_err(|_| AcceptanceReason::InvalidPilotEvidence)?;
                r.validate_for_operation(i)
                    .map_err(|_| AcceptanceReason::InvalidPilotEvidence)?;
                need(
                    i.operation.kind == scope.kind
                        && i.operation.recipe_sha256 == *scope.recipe
                        && i.operation.model == scope.model
                        && scope
                            .inputs
                            .iter()
                            .all(|f| i.operation.inputs.iter().any(|a| a.file == **f))
                        && scope
                            .artifacts
                            .iter()
                            .all(|expected| i.operation.inputs.iter().any(|a| same(a, *expected)))
                        && scope.output.is_none_or(|expected| {
                            r.result.outputs.iter().any(|a| same(a, expected))
                        })
                        && same(&i.operation.payload, request)
                        && r.result.completion == ProviderCompletion::Completed
                        && r.epoch_disposition == ProviderEpochDisposition::Current
                        && r.retry.is_none()
                        && r.recovery.is_none()
                        && r.result
                            .raw_response
                            .as_ref()
                            .is_some_and(|a| same(a, response)),
                )?;
                if let Some(e) = scope.reviewer {
                    need(
                        i.identity.run_id == e.reviewer.run_id
                            && i.identity.epoch == e.reviewer.epoch
                            && i.operation.unit_id == e.reviewer.unit_id
                            && i.operation.provider == e.provider
                            && i.operation.model == e.model,
                    )?;
                }
            }
            (None, None) => {}
            _ => return Err(AcceptanceReason::InvalidPilotEvidence),
        }
        Ok(o.findings)
    }
}
pub(crate) fn inspect_pilot(
    reads: &mut Reads,
    root: &Path,
    p: &PilotReviewPolicyV1,
    binding: &AcceptanceBinding,
) -> PilotReviewObservationV1 {
    let mut summary = PilotReviewObservationV1 {
        schema_version: 1,
        policy_sha256: digest(&serde_json::to_vec(p).unwrap()),
        plan_sha256: p.plan.sha256.clone(),
        source_sha256: p.source.sha256.clone(),
        binding_outcome: ReviewOutcome::Missing,
        review_outcome: ReviewOutcome::Missing,
        recording_count: 0,
        review_count: 0,
        production_review_executed: false,
        criteria: vec![],
        reasons: vec![AcceptanceReason::UnverifiedPilotReviewer],
        consumed_sha256s: vec![],
    };
    let mut i = Inspect {
        reads,
        root,
        hashes: BTreeSet::new(),
        values: BTreeMap::new(),
    };
    // Retain adverse claims from independently hash-bound metadata even when a
    // recording/context dependency is absent. Cached results are reused below;
    // no missing read is retried and no second metadata read can change its meaning.
    let mut adverse_files = vec![&p.plan];
    for performer in &p.performers {
        adverse_files.extend(performer.takes.iter().map(|t| &t.observation));
        if let Some(r) = &performer.review {
            adverse_files.extend([&r.observation, &r.response.file]);
        }
    }
    for f in adverse_files {
        if let Ok(v) = i.value(f) {
            for field in ["findings", "criteria"] {
                if let Some(items) = v[field].as_array() {
                    for item in items {
                        if let Ok(outcome) =
                            serde_json::from_value::<ReviewOutcome>(item["outcome"].clone())
                        {
                            if outcome != ReviewOutcome::Pass {
                                summary.review_outcome = worse(summary.review_outcome, outcome);
                                if !summary.reasons.contains(&AcceptanceReason::AdverseEvidence) {
                                    summary.reasons.push(AcceptanceReason::AdverseEvidence);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let result = (|| {
        need(p.source.sha256 == binding.source_sha256)?;
        let source = i.read(&p.source)?;
        let text = std::str::from_utf8(&source)
            .map_err(|_| AcceptanceReason::InvalidPilotEvidence)?
            .chars()
            .collect::<Vec<_>>();
        for f in [
            &p.source_record,
            &p.direction,
            &p.registry,
            &p.catalog,
            &p.recipe,
        ] {
            i.read(f)?;
        }
        let plan = i.value(&p.plan)?;
        need(
            plan["schema_version"] == 1
                && plan["bulk_speech_approved"] == false
                && plan["production_approved"] == false,
        )?;
        let takes = plan["takes"]
            .as_array()
            .ok_or(AcceptanceReason::InvalidPilotEvidence)?;
        let reviews = plan["reviews"]
            .as_array()
            .ok_or(AcceptanceReason::InvalidPilotEvidence)?;
        let context = |v: &Value| {
            v["schema_version"] == 1
                && v["source_sha256"] == p.source.sha256
                && same(&v["source"], &p.source_record)
                && same(&v["direction"], &p.direction)
                && same(&v["registry"], &p.registry)
                && same(&v["catalog"], &p.catalog)
                && same(&v["recipe"], &p.recipe)
                && v["production_approved"] == false
        };
        let mut add_findings = |findings: Vec<Finding>| {
            for f in findings {
                summary.review_outcome = worse(
                    summary.review_outcome,
                    if f.outcome == ReviewOutcome::Pass {
                        ReviewOutcome::Uncertain
                    } else {
                        f.outcome
                    },
                );
            }
        };
        if let Some(findings) = plan.get("findings") {
            let findings: Vec<Finding> = serde_json::from_value(findings.clone())
                .map_err(|_| AcceptanceReason::InvalidPilotEvidence)?;
            need(findings.len() <= 128)?;
            add_findings(findings);
        }
        if takes.len() != p.performers.iter().map(|s| s.takes.len()).sum::<usize>()
            || reviews.len() != p.performers.len()
        {
            return Err(AcceptanceReason::IncompletePilotEvidence);
        }
        for performer in &p.performers {
            for t in &performer.takes {
                need(t.source_span[1] <= text.len() as u64)?;
                let req = i.value(&t.request.file)?;
                need(
                    context(&req)
                        && req["task"] == "voice-pilot-take-v1"
                        && req["speaker"] == performer.speaker
                        && req["canonical_id"] == performer.canonical_id
                        && req["model"] == performer.model
                        && req["voice_id"] == performer.voice_id
                        && req["passage"]["span"] == json!(t.source_span)
                        && req["passage"]["text"]
                            == text[t.source_span[0] as usize..t.source_span[1] as usize]
                                .iter()
                                .collect::<String>(),
                )?;
                let matching = takes
                    .iter()
                    .filter(|v| {
                        same(&v["request"], &t.request)
                            && same(&v["response"], &t.response)
                            && same(&v["observation"], &t.observation)
                            && same(&v["audio"], &t.audio)
                            && v["production_approved"] == false
                    })
                    .count();
                need(matching == 1)?;
                add_findings(i.observation(
                    &t.observation,
                    &t.request,
                    &t.response,
                    ObservationScope {
                        kind: OperationKind::Speech,
                        recipe: &p.recipe.sha256,
                        model: &performer.model,
                        inputs: vec![
                            &p.source_record,
                            &p.direction,
                            &p.registry,
                            &p.catalog,
                            &p.recipe,
                        ],
                        artifacts: vec![],
                        output: Some(&t.audio),
                        reviewer: None,
                    },
                )?);
                let raw = i.value(&t.response.file)?;
                need(
                    raw["schema_version"] == 1
                        && raw["provenance"] == json!(t.response.provenance)
                        && t.response.provenance != Provenance::Autonomous
                        && raw["request_sha256"] == t.request.file.sha256
                        && raw["recipe_sha256"] == p.recipe.sha256
                        && raw["source_sha256"] == p.source.sha256
                        && raw["direction_sha256"] == p.direction.sha256
                        && raw["catalog_sha256"] == p.catalog.sha256
                        && raw["speaker"] == performer.speaker
                        && raw["model"] == performer.model
                        && raw["voice_id"] == performer.voice_id
                        && raw["passage"] == req["passage"]
                        && same(&raw["audio"], &t.audio),
                )?;
                let audio = i.audio(&t.audio)?;
                need(
                    raw["format"]
                        == json!({"sample_rate":audio.sample_rate,"channels":audio.channels,"bits_per_sample":audio.bits_per_sample,"frames":audio.frame_count}),
                )?;
                summary.recording_count += 1;
            }
            let r = performer
                .review
                .as_ref()
                .ok_or(AcceptanceReason::IncompletePilotEvidence)?;
            let request = i.value(&r.request.file)?;
            need(
                context(&request)
                    && request["task"] == "independent-voice-pilot-review-v1"
                    && request["speaker"] == performer.speaker,
            )?;
            let targets = performer
                .takes
                .iter()
                .map(|t| json!(t.audio))
                .collect::<Vec<_>>();
            let requested = request["takes"]
                .as_array()
                .ok_or(AcceptanceReason::InvalidPilotEvidence)?;
            need(requested.len() == performer.takes.len())?;
            for (v, t) in requested.iter().zip(&performer.takes) {
                need(
                    same(&v["request"], &t.request)
                        && same(&v["response"], &t.response)
                        && same(&v["observation"], &t.observation)
                        && same(&v["audio"], &t.audio),
                )?;
            }
            for (role, a) in [
                ("reference_audio", r.reference_audio.as_ref()),
                ("assembled_audio", r.assembled_audio.as_ref()),
            ] {
                let a = a.ok_or(AcceptanceReason::IncompletePilotEvidence)?;
                need(same(&request[role], a))?;
                i.audio(a)?;
            }
            for (role, f) in [
                ("join_recipe", r.join_recipe.as_ref()),
                ("music_context", r.music_context.as_ref()),
            ] {
                let f = f.ok_or(AcceptanceReason::IncompletePilotEvidence)?;
                need(same(&request[role], f))?;
                i.read(f)?;
            }
            let expected = PILOT_REVIEW_CRITERIA
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let criteria = request["criteria"]
                .as_array()
                .ok_or(AcceptanceReason::InvalidPilotEvidence)?;
            need(
                criteria.len() == 8
                    && criteria
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<BTreeSet<_>>()
                        == expected,
            )?;
            need(
                reviews
                    .iter()
                    .filter(|v| {
                        same(&v["request"], &r.request)
                            && same(&v["response"], &r.response)
                            && v["targets"] == json!(targets)
                            && v["production_approved"] == false
                    })
                    .count()
                    == 1,
            )?;
            let mut expected_files = vec![
                &p.source_record,
                &p.direction,
                &p.registry,
                &p.catalog,
                &p.recipe,
            ];
            expected_files.extend(performer.takes.iter().map(|t| &t.observation));
            expected_files.extend(r.join_recipe.iter());
            expected_files.extend(r.music_context.iter());
            let mut expected_artifacts = performer
                .takes
                .iter()
                .flat_map(|t| [&t.request, &t.response, &t.audio])
                .collect::<Vec<_>>();
            expected_artifacts.extend(r.reference_audio.iter());
            expected_artifacts.extend(r.assembled_audio.iter());
            add_findings(i.observation(
                &r.observation,
                &r.request,
                &r.response,
                ObservationScope {
                    kind: OperationKind::Review,
                    recipe: &p.recipe.sha256,
                    model: &r.model,
                    inputs: expected_files,
                    artifacts: expected_artifacts,
                    output: None,
                    reviewer: Some(r),
                },
            )?);
            let raw = i.value(&r.response.file)?;
            need(
                raw["schema_version"] == 1
                    && r.response.provenance != Provenance::Autonomous
                    && raw["provenance"] == json!(r.response.provenance)
                    && raw["request_sha256"] == r.request.file.sha256
                    && raw["recipe_sha256"] == p.recipe.sha256
                    && raw["speaker"] == performer.speaker
                    && raw["targets"] == json!(targets)
                    && same(&raw["reference_audio"], &r.reference_audio),
            )?;
            let criteria = raw["criteria"]
                .as_array()
                .ok_or(AcceptanceReason::InvalidPilotEvidence)?;
            need(
                criteria.len() == 8
                    && criteria
                        .iter()
                        .filter_map(|c| c["id"].as_str())
                        .collect::<BTreeSet<_>>()
                        == expected,
            )?;
            for c in criteria {
                let claimed: ReviewOutcome = serde_json::from_value(c["outcome"].clone())
                    .map_err(|_| AcceptanceReason::InvalidPilotEvidence)?;
                let effective = if claimed == ReviewOutcome::Pass {
                    ReviewOutcome::Uncertain
                } else {
                    claimed
                };
                summary.criteria.push(PilotCriterionObservationV1 {
                    speaker: performer.speaker.clone(),
                    id: c["id"].as_str().unwrap().into(),
                    claimed_outcome: claimed,
                    effective_outcome: effective,
                });
            }
            summary.review_count += 1;
        }
        Ok(())
    })();
    summary.consumed_sha256s = i.hashes.into_iter().collect();
    for c in &summary.criteria {
        summary.review_outcome = worse(summary.review_outcome, c.effective_outcome);
    }
    match result {
        Ok(()) => summary.binding_outcome = ReviewOutcome::Pass,
        Err(r) => {
            summary.binding_outcome = if matches!(
                r,
                AcceptanceReason::MissingBytes | AcceptanceReason::IncompletePilotEvidence
            ) {
                ReviewOutcome::Missing
            } else {
                ReviewOutcome::Fail
            };
            summary.reasons.push(r);
        }
    }
    summary
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotPlanStructureV1 {
    pub schema_version: u32,
    pub plan_sha256: Digest,
    pub take_count: u32,
    pub review_count: u32,
    pub production_approved: bool,
    pub bulk_speech_approved: bool,
}
impl PilotPlanStructureV1 {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 1
                && valid_digest(&self.plan_sha256)
                && (1..=64).contains(&self.take_count)
                && self.review_count <= 16
                && !self.production_approved
                && !self.bulk_speech_approved,
            "pilot_review",
            "structure",
            "invalid structural plan result",
        )
    }
}
/// Only plan shape and non-approval boundaries. Empty reviews are a legitimate unfinished plan.
pub fn validate_pilot_plan_structure(plan: &Value) -> ContractResult<PilotPlanStructureV1> {
    let check = (|| {
        need(
            plan["schema_version"] == 1
                && plan["bulk_speech_approved"] == false
                && plan["production_approved"] == false,
        )?;
        let takes = plan["takes"]
            .as_array()
            .ok_or(AcceptanceReason::InvalidPilotEvidence)?;
        let reviews = plan["reviews"]
            .as_array()
            .ok_or(AcceptanceReason::InvalidPilotEvidence)?;
        need((1..=64).contains(&takes.len()) && reviews.len() <= 16)?;
        for t in takes {
            need(t["production_approved"] == false)?;
            for r in ["request", "audio"] {
                need(t[r]["file"]["sha256"].as_str().is_some_and(valid_digest))?;
            }
        }
        for r in reviews {
            need(
                r["production_approved"] == false
                    && r["targets"]
                        .as_array()
                        .is_some_and(|a| !a.is_empty() && a.len() <= 4),
            )?;
        }
        Ok(PilotPlanStructureV1 {
            schema_version: 1,
            plan_sha256: digest(&serde_json::to_vec(plan).unwrap()),
            take_count: takes.len() as u32,
            review_count: reviews.len() as u32,
            production_approved: false,
            bulk_speech_approved: false,
        })
    })();
    check.map_err(|_: AcceptanceReason| ContractError {
        code: "pilot_review".into(),
        field: "structure".into(),
        message: "invalid plan shape; structural validity is not a recording/listening review"
            .into(),
    })
}
