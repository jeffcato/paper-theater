//! Bounded, private voice-pilot proposals. No voice or bulk-speech acceptance authority.
use super::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
const META: u64 = 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    book_id: Id,
    chapter_id: Id,
    source: FileRef,
    direction: FileRef,
    direction_input: FileRef,
    registry: FileRef,
    catalog: FileRef,
    selection: FileRef,
    recipe: FileRef,
    first_audition_model: Id,
    speech_provider: Id,
    review_provider: Id,
    review_model: Id,
    estimate: Usage,
    timeout_ms: u64,
    observations: Vec<FileRef>,
    reviews: Vec<FileRef>,
    #[serde(default)]
    request_payloads: Vec<FileRef>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    provenance: Provenance,
    provider: Id,
    models: Vec<Model>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    id: Id,
    voices: Vec<Voice>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Voice {
    id: Id,
    language: String,
    accent: String,
    reference: Option<ArtifactRef>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    schema_version: u32,
    provenance: Provenance,
    source_sha256: Digest,
    direction_sha256: Digest,
    catalog_sha256: Digest,
    registry_sha256: Digest,
    performers: Vec<Performer>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Performer {
    speaker: Id,
    model: Id,
    voice_id: Id,
    language: String,
    accent: String,
    baseline: String,
    passages: Vec<Passage>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Passage {
    turn_id: Id,
    span: SourceSpan,
    text: String,
    acting: String,
    acting_support: Option<Citation>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Citation {
    span: SourceSpan,
    quote: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Established {
    schema_version: u32,
    book_id: Id,
    canonical_id: Id,
    provider: Id,
    model: Id,
    voice_id: Id,
    catalog: ArtifactRef,
    language: String,
    accent: String,
    baseline: String,
    reference: ArtifactRef,
    recordings: Vec<FileRef>,
}
fn need(ok: bool, message: &str) -> Result<(), StageError> {
    require(ok, "pilot", "evidence", message)?;
    Ok(())
}
fn bytes<'a>(files: &'a Files, f: &FileRef) -> Result<&'a [u8], StageError> {
    let (actual, b) = files
        .get(&f.path)
        .ok_or_else(|| StageError::Evidence(format!("missing pilot input: {}", f.path)))?;
    need(actual == f, "pilot FileRef changed")?;
    Ok(b)
}
fn artifact(input: &StageInput, f: &FileRef) -> Result<ArtifactRef, StageError> {
    input
        .inputs
        .iter()
        .find(|a| a.file == *f)
        .cloned()
        .ok_or_else(|| StageError::Evidence("pilot committed ArtifactRef required".into()))
}
fn slice(text: &[char], [a, b]: SourceSpan) -> Option<String> {
    (a < b && b <= text.len() as u64).then(|| text[a as usize..b as usize].iter().collect())
}
fn sentence_boundary(text: &[char], offset: u64) -> bool {
    let i = offset as usize;
    if i == 0 || i == text.len() {
        return true;
    }
    let before = text[..i].iter().collect::<String>();
    let end = before.trim_end().trim_end_matches(['”', '"', '\'', '’']);
    end.ends_with(['.', '!', '?']) && (text[i].is_whitespace() || text[i - 1].is_whitespace())
}
fn finding(findings: &mut Vec<Finding>, id: &str, message: &str) {
    findings.push(missing(id, message));
}
pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
) -> Result<StageOutput, StageError> {
    need(
        input.inputs.len() <= 64
            && input.inputs.iter().all(|a| {
                a.file.bytes
                    <= if ["raw-source.txt", "clean-source.txt"].contains(&a.file.path.as_str()) {
                        16 * META
                    } else if a.file.media_type == "audio/wav" && a.file.path.ends_with(".wav") {
                        8 * META
                    } else {
                        META
                    }
            })
            && input.inputs.iter().map(|a| a.file.bytes).sum::<u64>() <= 64 * META,
        "pilot input envelope limit",
    )?;
    let mut files = inputs::load(input, services).await?.files;
    let Some((_, ctx_bytes)) = files.get("private/pilot-context.json") else {
        return Ok(StageOutput {
            outcome: StageOutcome::NeedsRepair,
            artifacts: vec![],
            decisions: vec![],
            findings: vec![missing(
                "pilot-context-missing",
                "Runtime-pinned pilot context is required.",
            )],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        });
    };
    let ctx: Context = parse(ctx_bytes)?;
    need(
        ctx.schema_version == 1
            && ctx.recipe.sha256 == input.pins.recipe_sha256
            && [
                &ctx.book_id,
                &ctx.chapter_id,
                &ctx.first_audition_model,
                &ctx.speech_provider,
                &ctx.review_provider,
                &ctx.review_model,
            ]
            .iter()
            .all(|s| valid_id(s)),
        "pilot context identity",
    )?;
    bytes(&files, &ctx.recipe)?;
    need(
        ctx.observations.len() + ctx.reviews.len() <= 32 && ctx.request_payloads.len() <= 32,
        "pilot observation/request inventory limit",
    )?;
    let mut registered = BTreeSet::new();
    for f in &ctx.request_payloads {
        need(
            registered.insert(f.path.clone()),
            "duplicate pilot request payload",
        )?;
        bytes(&files, f)?;
    }
    let direction = pilot_direction::verify(
        input,
        services,
        &files,
        &ctx.direction_input,
        &ctx.direction,
    )
    .await?;
    let direction_context: Value = parse(
        &files
            .get("private/direction-context.json")
            .ok_or_else(|| StageError::Evidence("missing Direction context".into()))?
            .1,
    )?;
    need(
        direction["book_id"] == ctx.book_id
            && direction["chapter_id"] == ctx.chapter_id
            && direction_context["source"] == json!(ctx.source)
            && direction_context["registry"] == json!(ctx.registry),
        "pilot source/book/chapter/registry differs from verified Direction",
    )?;
    bytes(&files, &ctx.source)?;
    bytes(&files, &ctx.registry)?;
    let source = direction["source_text"]
        .as_str()
        .ok_or_else(|| StageError::Evidence("missing Direction prose".into()))?;
    let chars = source.chars().collect::<Vec<_>>();
    let catalog: Catalog = parse(bytes(&files, &ctx.catalog)?)?;
    let mut model_ids = BTreeSet::new();
    for model in &catalog.models {
        need(
            valid_id(&model.id) && model_ids.insert(model.id.clone()) && model.voices.len() <= 128,
            "ambiguous/oversized catalog models",
        )?;
        let mut voice_ids = BTreeSet::new();
        for voice in &model.voices {
            need(
                valid_id(&voice.id)
                    && voice_ids.insert(voice.id.clone())
                    && !voice.language.trim().is_empty()
                    && voice.language.len() <= 80
                    && !voice.accent.trim().is_empty()
                    && voice.accent.len() <= 160,
                "ambiguous/invalid catalog voices",
            )?;
        }
    }
    let selection: Selection = parse(bytes(&files, &ctx.selection)?)?;
    need(
        catalog.schema_version == 1
            && catalog.provider == ctx.speech_provider
            && catalog.models.len() <= 32
            && selection.schema_version == 1
            && selection.source_sha256 == direction["source_sha256"]
            && selection.direction_sha256 == ctx.direction.sha256
            && selection.catalog_sha256 == ctx.catalog.sha256
            && selection.registry_sha256 == ctx.registry.sha256
            && selection.performers.len() <= 32,
        "pilot catalog/selection bindings",
    )?;
    let mut findings: Vec<Finding> = serde_json::from_value(direction["findings"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    let mut outputs: Vec<(String, Value)> = vec![];
    let mut jobs = vec![];
    let mut performers = vec![];
    let mut seen = BTreeSet::new();
    let mut references = BTreeMap::<Id, Option<ArtifactRef>>::new();
    let mut inherited_recordings = vec![];
    let base = [
        &ctx.source,
        &ctx.direction,
        &ctx.direction_input,
        &ctx.registry,
        &ctx.catalog,
        &ctx.selection,
        &ctx.recipe,
    ]
    .into_iter()
    .map(|f| artifact(input, f))
    .collect::<Result<Vec<_>, _>>()?;
    let mut selected = selection.performers.clone();
    selected.sort_by(|a, b| a.speaker.cmp(&b.speaker));
    for (pi, p) in selected.iter().enumerate() {
        need(
            valid_id(&p.speaker)
                && seen.insert(p.speaker.clone())
                && p.passages.len() <= 4
                && p.baseline.len() <= 240,
            "pilot performer bounds/identity",
        )?;
        let speaker_ok = direction["speakers"]
            .as_array()
            .unwrap()
            .contains(&json!(p.speaker));
        if !speaker_ok {
            finding(
                &mut findings,
                &format!("pilot-unsupported-speaker:{}", p.speaker),
                "Mentioned/visible/unsupported characters receive no speech requests.",
            );
            continue;
        }
        let cast = direction["cast"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["proposal"]["id"] == p.speaker)
            .unwrap();
        let retained = cast["retained_registry_identity"]["voice"].clone();
        let established = if !retained.is_null() {
            let a: ArtifactRef = serde_json::from_value(retained.clone())
                .map_err(|e| StageError::Evidence(e.to_string()))?;
            let Some((f, b)) = files.get(&a.file.path) else {
                finding(
                    &mut findings,
                    &format!("pilot-established-identity:{}", p.speaker),
                    "Established voice metadata is missing; preserve binding without regeneration.",
                );
                performers.push(json!({"speaker":p.speaker,"retained_binding":{"artifact":a},"decision":"reuse-unresolved","approved":false}));
                continue;
            };
            need(
                f == &a.file && json!(artifact(input, f)?) == json!(a),
                "established voice ArtifactRef changed",
            )?;
            let binding: Established = parse(b)?;
            let identity = binding.schema_version == 1
                && binding.book_id == ctx.book_id
                && binding.canonical_id == cast["proposal"]["canonical_id"]
                && binding.provider == ctx.speech_provider
                && binding.catalog.file == ctx.catalog
                && json!(artifact(input, &ctx.catalog)?) == json!(binding.catalog)
                && binding.recordings.len() <= 16;
            let compatible = identity
                && binding.model == p.model
                && binding.voice_id == p.voice_id
                && binding.language == p.language
                && binding.accent == p.accent
                && binding.baseline == p.baseline;
            if !compatible {
                finding(
                    &mut findings,
                    &format!("pilot-established-identity:{}", p.speaker),
                    "Preserve established model/voice/catalog/reference/language/baseline exactly; no recast, silent upgrade or regeneration.",
                );
                performers.push(json!({"speaker":p.speaker,"model":binding.model,"voice_id":binding.voice_id,"language":binding.language,"accent":binding.accent,"baseline":binding.baseline,"retained_binding":{"artifact":a,"record":binding},"decision":"reuse-unresolved","approved":false}));
                continue;
            }
            need(
                binding.reference.file.media_type == "audio/wav"
                    && json!(artifact(input, &binding.reference.file)?) == json!(binding.reference),
                "established reference ArtifactRef changed",
            )?;
            pilot_evidence::format(bytes(&files, &binding.reference.file)?)?;
            inherited_recordings.extend(binding.recordings.clone());
            Some((a, binding))
        } else {
            None
        };
        let voice = catalog
            .models
            .iter()
            .find(|m| m.id == p.model)
            .and_then(|m| {
                m.voices.iter().find(|v| {
                    v.id == p.voice_id && v.language == p.language && v.accent == p.accent
                })
            });
        if voice.is_none() || (established.is_none() && p.model != ctx.first_audition_model) {
            finding(
                &mut findings,
                &format!("pilot-catalog-identity:{}", p.speaker),
                "Unknown/stale catalog model/voice or unconfigured first audition remains unresolved.",
            );
            continue;
        }
        let reference = established
            .as_ref()
            .map(|(_, b)| b.reference.clone())
            .or_else(|| voice.unwrap().reference.clone());
        if let Some(a) = &reference {
            need(
                a.file.media_type == "audio/wav" && json!(artifact(input, &a.file)?) == json!(a),
                "catalog reference ArtifactRef changed",
            )?;
            pilot_evidence::format(bytes(&files, &a.file)?)?;
        }
        references.insert(p.speaker.clone(), reference.clone());
        let mut passages = vec![];
        for (ti, t) in p.passages.iter().enumerate() {
            let turn = direction["turns"].as_array().unwrap().iter().find(|turn| {
                turn["proposal"]["id"] == t.turn_id
                    && turn["proposal"]["speaker"] == p.speaker
                    && turn["speaker_source_supported"] == true
            });
            let span_ok = turn.is_some_and(|turn| {
                let s = &turn["proposal"]["span"];
                s[0].as_u64().unwrap() <= t.span[0] && t.span[1] <= s[1].as_u64().unwrap()
            }) && slice(&chars, t.span).is_some_and(|s| s == t.text);
            if !span_ok {
                finding(
                    &mut findings,
                    &format!("pilot-passage-source:{}:{ti}", p.speaker),
                    "Auditions must retain exact supported source turns and scalar spans.",
                );
                continue;
            }
            let turn = turn.unwrap();
            let boundary_ok = (t.span[0] == turn["proposal"]["span"][0].as_u64().unwrap()
                || sentence_boundary(&chars, t.span[0]))
                && (t.span[1] == turn["proposal"]["span"][1].as_u64().unwrap()
                    || sentence_boundary(&chars, t.span[1]));
            if !boundary_ok {
                finding(
                    &mut findings,
                    &format!("pilot-passage-boundary:{}:{ti}", p.speaker),
                    "Do not cut words/clauses to manufacture an adjacent audition; retain complete source-turn/sentence boundaries.",
                );
                continue;
            }
            if ti > 0 && p.passages[ti - 1].span[1] != t.span[0] {
                finding(
                    &mut findings,
                    &format!("pilot-adjacency:{}", p.speaker),
                    "Adjacent takes require explicit complete neighbor/handoff context.",
                );
            }
            let acting_ok = t.acting.is_empty()
                || t.acting_support.as_ref().is_some_and(|c| {
                    slice(&chars, c.span).is_some_and(|s| s == c.quote)
                        && c.span[0] >= t.span[0]
                        && c.span[1] <= t.span[1]
                });
            if !acting_ok {
                finding(
                    &mut findings,
                    &format!("pilot-acting-support:{}:{ti}", p.speaker),
                    "Expressive variation lacks source support and remains unresolved.",
                );
            }
            let style = if ti == 0 && established.is_none() {
                String::new()
            } else {
                [
                    p.baseline.as_str(),
                    if acting_ok { t.acting.as_str() } else { "" },
                ]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("; ")
            };
            let request = json!({"schema_version":1,"task":"voice-pilot-take-v1","source":ctx.source,"source_sha256":direction["source_sha256"],"direction":ctx.direction,"registry":ctx.registry,"catalog":ctx.catalog,"recipe":ctx.recipe,"speaker":p.speaker,"canonical_id":cast["proposal"]["canonical_id"],"established_binding":established.as_ref().map(|(a,_)|a),"model":p.model,"voice_id":p.voice_id,"language":p.language,"accent":p.accent,"baseline":p.baseline,"passage":t,"text":t.text,"style":style,"production_approved":false});
            let mut dependencies = base.clone();
            if let Some((a, b)) = &established {
                dependencies.push(a.clone());
                dependencies.push(b.reference.clone());
            }
            jobs.push(pilot_provider::Job {
                id: format!("p{pi:03}-t{ti:02}"),
                request,
                kind: OperationKind::Speech,
                provider: ctx.speech_provider.clone(),
                model: p.model.clone(),
                inputs: dependencies,
                allow_dispatch: established.is_none(),
            });
            passages.push(json!(t));
        }
        if passages.len() < 2 {
            finding(
                &mut findings,
                &format!("pilot-adjacent-takes:{}", p.speaker),
                "Two adjacent verbatim takes are required; no extra source performance is invented.",
            );
        }
        performers.push(json!({"speaker":p.speaker,"model":p.model,"voice_id":p.voice_id,"catalog":ctx.catalog,"language":p.language,"accent":p.accent,"baseline":p.baseline,"passages":passages,"reference_audio":reference,"retained_binding":established.as_ref().map(|(a,b)|json!({"artifact":a,"record":b})),"decision":if established.is_some(){"reuse-established"}else{"propose-first-audition"},"approved":false}));
    }
    for speaker in direction["speakers"].as_array().unwrap() {
        if !seen.contains(speaker.as_str().unwrap()) {
            finding(
                &mut findings,
                &format!("pilot-speaker-selection:{}", speaker.as_str().unwrap()),
                "Director-selected performer is missing; no fallback selection.",
            );
        }
    }
    let mut refs = ctx
        .observations
        .iter()
        .chain(&ctx.reviews)
        .chain(&inherited_recordings)
        .cloned()
        .collect::<Vec<_>>();
    refs.sort_by(|a, b| a.path.cmp(&b.path));
    refs.dedup();
    let mut saved = pilot_provider::saved(&files, &refs)?;
    let mut observations = pilot_provider::collect(
        input,
        services,
        &mut files,
        &mut saved,
        &jobs,
        &ctx.recipe,
        &ctx.estimate,
        ctx.timeout_ms,
        &ctx.request_payloads,
        &mut outputs,
        &mut findings,
    )
    .await?;
    let mut takes = vec![];
    for (id, f, obs) in &observations {
        let job = jobs.iter().find(|j| j.id == *id).unwrap();
        if let Some(take) = pilot_evidence::take(input, services, &mut files, job, f, obs).await? {
            takes.push(take);
        }
    }
    let mut reviews = vec![];
    let mut review_jobs = vec![];
    for (pi, p) in selected.iter().enumerate() {
        let selected_takes = takes
            .iter()
            .filter(|t| t["speaker"] == p.speaker)
            .cloned()
            .collect::<Vec<_>>();
        if selected_takes.len() < 2 {
            continue;
        }
        let mut dependencies = base.clone();
        for take in &selected_takes {
            for k in ["request", "response", "audio"] {
                dependencies.push(serde_json::from_value(take[k].clone()).unwrap());
            }
        }
        let start = selected_takes.first().unwrap()["passage"]["span"][0]
            .as_u64()
            .unwrap();
        let end = selected_takes.last().unwrap()["passage"]["span"][1]
            .as_u64()
            .unwrap();
        let reference = references.get(&p.speaker).cloned().flatten();
        if let Some(a) = &reference {
            dependencies.push(a.clone());
        }
        let request = json!({"schema_version":1,"task":"independent-voice-pilot-review-v1","source":ctx.source,"source_sha256":direction["source_sha256"],"direction":ctx.direction,"catalog":ctx.catalog,"registry":ctx.registry,"recipe":ctx.recipe,"speaker":p.speaker,"model":p.model,"voice_id":p.voice_id,"baseline":p.baseline,"takes":selected_takes,"reference_audio":reference,"expected_context":slice(&chars,[start,end]),"expected_turns":direction["turns"],"assembled_audio":null,"join_recipe":null,"music_context":null,"criteria":["listening","identity","continuity","intelligibility","speech-fidelity","room-tone","safe-joins","music-context"],"production_approved":false});
        review_jobs.push(pilot_provider::Job {
            id: format!("p{pi:03}-review"),
            request,
            kind: OperationKind::Review,
            provider: ctx.review_provider.clone(),
            model: ctx.review_model.clone(),
            inputs: dependencies,
            allow_dispatch: ctx.review_model != p.model,
        });
    }
    let review_observations = pilot_provider::collect(
        input,
        services,
        &mut files,
        &mut saved,
        &review_jobs,
        &ctx.recipe,
        &ctx.estimate,
        ctx.timeout_ms,
        &ctx.request_payloads,
        &mut outputs,
        &mut findings,
    )
    .await?;
    for (id, _, obs) in &review_observations {
        if let Some(review) = pilot_evidence::review(
            &files,
            review_jobs.iter().find(|j| j.id == *id).unwrap(),
            obs,
            &mut findings,
        )? {
            reviews.push(review);
        }
    }
    observations.extend(review_observations);
    jobs.extend(review_jobs);
    need(
        ctx.request_payloads.iter().all(|f| {
            jobs.iter()
                .any(|j| f.path == format!("private/pilot/requests/{}.json", j.id))
        }),
        "registered payload has no current pilot job",
    )?;
    need(
        saved.is_empty(),
        "saved observation has no matching current pilot request",
    )?;
    outputs.push(("private/pilot-handoff.json".into(),json!({"schema_version":1,"caller_authority":false,"runtime_committed_payload_required":true,"jobs":jobs.iter().map(|j|json!({"request_path":format!("private/pilot/requests/{}.json",j.id),"kind":j.kind,"provider":j.provider,"model":j.model,"recipe":ctx.recipe,"inputs":j.inputs,"estimate":ctx.estimate,"timeout_ms":ctx.timeout_ms})).collect::<Vec<_>>()})));
    for id in [
        "pilot-listening",
        "pilot-continuity",
        "pilot-joins",
        "pilot-intelligibility",
        "pilot-speech-fidelity",
        "pilot-music-context",
        "voice-pilot",
        "production-verifier-pending",
    ] {
        finding(
            &mut findings,
            id,
            "Independent listening/neighbor/reference/mix evidence and runtime support remain unresolved; synthetic format checks grant no acceptance.",
        );
    }
    let compiler_sha256 = digest(
        &[
            include_bytes!("pilot.rs").as_slice(),
            include_bytes!("pilot_direction.rs").as_slice(),
            include_bytes!("pilot_provider.rs").as_slice(),
            include_bytes!("pilot_evidence.rs").as_slice(),
        ]
        .concat(),
    );
    outputs.push(("private/pilot-plan.json".into(),json!({"schema_version":1,"provenance":"diagnostic","compiler_sha256":compiler_sha256,"source":ctx.source,"direction":ctx.direction,"source_text":source,"catalog":ctx.catalog,"registry":ctx.registry,"recipe":ctx.recipe,"selection":ctx.selection,"request_payloads":ctx.request_payloads,"performers":performers,"takes":takes,"reviews":reviews,"observations":observations,"production_approved":false,"bulk_speech_approved":false,"runtime_policy_authority":false,"reveal_permissions":[],"findings":findings})));
    let key = digest(&json(
        &outputs
            .iter()
            .filter(|(p, _)| !p.starts_with("private/pilot/observations/"))
            .collect::<Vec<_>>(),
    )?);
    let mut artifacts = vec![];
    for (path, v) in outputs {
        let b = json(&v)?;
        need(b.len() as u64 <= META, "pilot candidate limit")?;
        if input.mode == ExecutionMode::Replay
            && let Some((f, _)) = files.get(&path)
        {
            need(
                f.sha256 == digest(&b) && f.bytes == b.len() as u64,
                "stale recorded pilot candidate",
            )?;
        }
        artifacts.push(write(services, input, &path, &v).await?);
    }
    Ok(StageOutput {
        outcome: StageOutcome::NeedsRepair,
        artifacts,
        decisions: vec![],
        findings,
        cache_keys: vec![key],
        library_updates: vec![],
        package_candidate: None,
    })
}
