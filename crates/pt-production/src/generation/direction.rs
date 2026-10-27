//! Private source-backed planning proposals. Runtime owns issuance, commits and acceptance.
use super::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
const META: u64 = 1024 * 1024;
const REQUEST: &str = "private/direction-request.json";
const OBSERVATION: &str = "private/direction-observation.json";
const PLAN: &str = "private/direction-plan.json";
const STATUS: &str = "private/direction-status.json";
const PROPOSAL: &str = "private/direction-provider-proposal.json";
const HANDOFF: &str = "private/direction-request-handoff.json";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    book_id: Id,
    chapter_id: Id,
    source: FileRef,
    registry: FileRef,
    recipe: FileRef,
    provider: Id,
    model: Id,
    estimate: Usage,
    timeout_ms: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: u32,
    book_id: Id,
    revision: u64,
    entities: Vec<Entity>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entity {
    id: Id,
    name: String,
    aliases: Vec<String>,
    sprite: Option<ArtifactRef>,
    voice: Option<ArtifactRef>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    schema_version: u32,
    provenance: Provenance,
    source_sha256: Digest,
    recipe_sha256: Digest,
    provider: Id,
    model: Id,
    request: ArtifactRef,
    response: Option<ArtifactRef>,
    issued: Option<ProviderOperationV2>,
    result: Option<ProviderResultV2>,
    findings: Vec<Finding>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Citation {
    span: SourceSpan,
    quote: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cast {
    id: Id,
    name: String,
    canonical_id: Id,
    kind: String,
    mentions: Vec<Citation>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Turn {
    id: Id,
    span: SourceSpan,
    text: String,
    speaker: Id,
    kind: String,
    attribution: Option<Citation>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Beat {
    id: Id,
    span: SourceSpan,
    text: String,
    depiction: String,
    description: String,
    subjects: Vec<Id>,
    fact_ids: Vec<Id>,
    action_ids: Vec<Id>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Picture {
    id: Id,
    span: SourceSpan,
    beat_ids: Vec<Id>,
    subjects: Vec<Id>,
    description: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scene {
    id: Id,
    span: SourceSpan,
    text: String,
    pictures: Vec<Picture>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fact {
    id: Id,
    entity_ids: Vec<Id>,
    assertion: String,
    support: Option<Citation>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    id: Id,
    actor_id: Id,
    kind: String,
    assertion: String,
    support: Option<Citation>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    id: Id,
    subject_type_id: Id,
    entity_ids: Vec<Id>,
    support: Option<Citation>,
    features: Vec<String>,
    usage: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    schema_version: u32,
    book_id: Id,
    chapter_id: Id,
    source_sha256: Digest,
    request_sha256: Option<Digest>,
    cast: Vec<Cast>,
    turns: Vec<Turn>,
    beats: Vec<Beat>,
    scenes: Vec<Scene>,
    facts: Vec<Fact>,
    actions: Vec<Action>,
    references: Vec<Reference>,
    questions: Vec<String>,
}
fn need(ok: bool, message: &str) -> Result<(), StageError> {
    require(ok, "direction", "evidence", message)?;
    Ok(())
}
fn get<'a>(files: &'a Files, path: &str) -> Result<&'a (FileRef, Vec<u8>), StageError> {
    files
        .get(path)
        .ok_or_else(|| StageError::Evidence(format!("missing direction input: {path}")))
}
fn verified<'a>(files: &'a Files, f: &FileRef) -> Result<&'a [u8], StageError> {
    f.validate()?;
    let actual = get(files, &f.path)?;
    need(actual.0 == *f, "direction reference changed")?;
    Ok(&actual.1)
}
fn artifact(input: &StageInput, f: &FileRef) -> Result<ArtifactRef, StageError> {
    input
        .inputs
        .iter()
        .find(|a| a.file == *f)
        .cloned()
        .ok_or_else(|| StageError::Evidence("committed ArtifactRef required".into()))
}
fn equal<T: Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}
fn finding(id: impl Into<Id>, outcome: ReviewOutcome, message: &str) -> Finding {
    Finding {
        id: id.into(),
        field: "direction".into(),
        outcome,
        message: message.into(),
        evidence: vec![],
    }
}
fn note(findings: &mut Vec<Finding>, id: impl Into<Id>, ok: bool, message: &str) {
    if !ok {
        findings.push(finding(id, ReviewOutcome::Fail, message));
    }
}
fn slice(source: &[char], [a, b]: SourceSpan) -> Option<String> {
    (a < b && b <= source.len() as u64).then(|| source[a as usize..b as usize].iter().collect())
}
fn cited(source: &[char], c: &Citation) -> bool {
    slice(source, c.span).is_some_and(|s| s == c.quote)
}
fn ids<'a>(
    rows: impl Iterator<Item = &'a Id>,
    label: &str,
    findings: &mut Vec<Finding>,
) -> BTreeSet<Id> {
    let mut out = BTreeSet::new();
    for id in rows {
        note(
            findings,
            format!("{label}-identity"),
            valid_id(id) && out.insert(id.clone()),
            "Missing/invalid/duplicate planning identity.",
        );
    }
    out
}
fn partition(
    source: &[char],
    rows: &[(Id, SourceSpan, String)],
    domain: SourceSpan,
    label: &str,
    findings: &mut Vec<Finding>,
) -> Value {
    let mut cursor = domain[0];
    let mut spans = vec![];
    let mut valid = true;
    for (id, [a, b], text) in rows {
        let ok = *a == cursor
            && a < b
            && *b <= domain[1]
            && slice(source, [*a, *b]).is_some_and(|s| s == *text);
        valid &= ok;
        note(
            findings,
            format!("{label}:{id}"),
            ok,
            "Coverage must preserve exact prose in positive contiguous source order; no gap, duplicate or reordering.",
        );
        spans.push([*a, *b]);
        cursor = *b;
    }
    valid &= !rows.is_empty() && cursor == domain[1];
    note(
        findings,
        format!("{label}-coverage"),
        valid,
        "Complete source coverage is unresolved.",
    );
    // Coverage union is diagnostic only; never sort the authored order into a valid plan.
    let mut missing = vec![];
    let mut clipped = spans
        .iter()
        .filter(|[a, b]| a < b && *b <= domain[1] && *a >= domain[0])
        .copied()
        .collect::<Vec<_>>();
    clipped.sort();
    let mut end = domain[0];
    for [a, b] in clipped {
        if a > end {
            missing.push([end, a]);
        }
        end = end.max(b);
    }
    if end < domain[1] {
        missing.push([end, domain[1]]);
    }
    json!({"ordered_verbatim_complete":valid,"proposed_spans":spans,"missing_source_spans":missing})
}
fn quote_ranges(source: &[char]) -> (Vec<SourceSpan>, bool) {
    let mut rows = vec![];
    let mut open = None;
    let mut ok = true;
    for (i, c) in source.iter().enumerate() {
        match c {
            '“' => {
                if open.is_some() {
                    ok = false;
                }
                open = Some((i, '”'));
            }
            '"' => {
                if let Some((a, '"')) = open {
                    rows.push([a as u64, (i + 1) as u64]);
                    open = None;
                } else if open.is_none() {
                    open = Some((i, '"'));
                } else {
                    ok = false;
                }
            }
            '”' => {
                if let Some((a, '”')) = open {
                    rows.push([a as u64, (i + 1) as u64]);
                    open = None;
                } else {
                    ok = false;
                }
            }
            '‘' => ok = false,
            _ => {}
        }
    }
    (rows, ok && open.is_none())
}
fn contains_name(text: &str, name: &str) -> bool {
    let words = |s: &str| {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    let haystack = words(text);
    let needle = words(name);
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}
fn attributed(source: &[char], turn: &Turn, citation: &Citation, name: &str) -> bool {
    if !cited(source, citation)
        || citation.span[0] > turn.span[0]
        || citation.span[1] < turn.span[1]
    {
        return false;
    }
    let words = |s: &str| {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    let name = words(name);
    if name.is_empty() {
        return false;
    }
    let before = source[citation.span[0] as usize..turn.span[0] as usize]
        .iter()
        .collect::<String>();
    let after = source[turn.span[1] as usize..citation.span[1] as usize]
        .iter()
        .collect::<String>();
    let (before, after) = (words(&before), words(&after));
    [
        "said",
        "asked",
        "replied",
        "called",
        "shouted",
        "whispered",
        "answered",
        "cried",
    ]
    .iter()
    .any(|verb| {
        let mut named = name.clone();
        named.push((*verb).into());
        let mut inverted = vec![(*verb).to_owned()];
        inverted.extend(name.clone());
        before.ends_with(&named)
            || before.ends_with(&inverted)
            || after.starts_with(&named)
            || after.starts_with(&inverted)
    })
}
fn compile(
    ctx: &Context,
    registry: &Registry,
    source_text: &str,
    response: &Response,
    findings: &mut Vec<Finding>,
) -> Result<Value, StageError> {
    let source = source_text.chars().collect::<Vec<_>>();
    let n = source.len() as u64;
    need(
        response.schema_version == 1
            && response.book_id == ctx.book_id
            && response.chapter_id == ctx.chapter_id
            && response.source_sha256 == digest(source_text.as_bytes()),
        "response source/book/chapter binding",
    )?;
    need(
        [
            response.cast.len(),
            response.turns.len(),
            response.beats.len(),
            response.scenes.len(),
            response.facts.len(),
            response.actions.len(),
            response.references.len(),
            response.questions.len(),
        ]
        .iter()
        .all(|n| *n <= 1024),
        "direction response inventory bound",
    )?;
    let cast_ids = ids(response.cast.iter().map(|c| &c.id), "cast", findings);
    let fact_ids = ids(response.facts.iter().map(|c| &c.id), "fact", findings);
    let action_ids = ids(response.actions.iter().map(|c| &c.id), "action", findings);
    ids(response.turns.iter().map(|c| &c.id), "turn", findings);
    ids(response.beats.iter().map(|c| &c.id), "beat", findings);
    ids(response.scenes.iter().map(|c| &c.id), "scene", findings);
    ids(
        response.references.iter().map(|c| &c.id),
        "reference",
        findings,
    );
    let mut cast = vec![];
    let mut supported_cast = BTreeSet::new();
    for c in &response.cast {
        let source_ok = c.kind == "narrator" && c.name == "Narrator"
            || matches!(c.kind.as_str(), "character" | "subject" | "location")
                && !c.mentions.is_empty()
                && c.mentions.iter().all(|m| cited(&source, m))
                && c.mentions.iter().any(|m| contains_name(&m.quote, &c.name));
        note(
            findings,
            format!("cast-source:{}", c.id),
            source_ok,
            "A character/subject requires a literal source mention; narration is a separate functional role.",
        );
        let alias = registry.entities.iter().find(|e| {
            e.name.eq_ignore_ascii_case(&c.name)
                || e.aliases.iter().any(|a| a.eq_ignore_ascii_case(&c.name))
        });
        let canonical = registry.entities.iter().find(|e| e.id == c.canonical_id);
        let canonical_ok = alias.is_none_or(|e| e.id == c.canonical_id)
            && canonical.is_none_or(|e| {
                e.name.eq_ignore_ascii_case(&c.name)
                    || e.aliases.iter().any(|a| a.eq_ignore_ascii_case(&c.name))
            });
        note(
            findings,
            format!("canonical-identity:{}", c.id),
            canonical_ok,
            "An existing book identity/alias cannot be replaced by a new canonical identity.",
        );
        if canonical.is_none() {
            findings.push(missing(&format!("canonical-binding:{}",c.id),"New entity identity/bindings are proposed only; no canonical image/voice is assigned."));
        }
        if source_ok && canonical_ok {
            supported_cast.insert(c.id.clone());
        }
        cast.push(json!({"proposal":c,"source_citation_valid":source_ok,"retained_registry_identity":canonical,"binding_acceptance_verified":false}));
    }
    note(
        findings,
        "narrator-role",
        response
            .cast
            .iter()
            .filter(|c| c.kind == "narrator")
            .count()
            == 1,
        "Exactly one separate narrator role is required.",
    );
    let (quoted, grammar) = quote_ranges(&source);
    if !grammar {
        findings.push(finding("dialogue-grammar",ReviewOutcome::Uncertain,"Unsupported/nested/unclosed quotation grammar needs a bounded parser review; source remains verbatim."));
    }
    let turn_coverage = partition(
        &source,
        &response
            .turns
            .iter()
            .map(|t| (t.id.clone(), t.span, t.text.clone()))
            .collect::<Vec<_>>(),
        [0, n],
        "turn",
        findings,
    );
    let mut speakers = BTreeSet::new();
    let mut script = vec![];
    let mut turns = vec![];
    for (i, t) in response.turns.iter().enumerate() {
        let speaker = response.cast.iter().find(|c| c.id == t.speaker);
        let exact = slice(&source, t.span).is_some_and(|s| s == t.text);
        let allowed = if t.kind == "narration" {
            supported_cast.contains(&t.speaker)
                && speaker.is_some_and(|c| c.kind == "narrator" && c.name == "Narrator")
        } else if t.kind == "dialogue" {
            supported_cast.contains(&t.speaker)
                && speaker.is_some_and(|c| c.kind == "character")
                && grammar
                && quoted.contains(&t.span)
                && t.attribution
                    .as_ref()
                    .is_some_and(|a| speaker.is_some_and(|c| attributed(&source, t, a, &c.name)))
        } else {
            false
        };
        note(
            findings,
            format!("speaker-source:{}", t.id),
            exact && allowed,
            "Invented dialogue, unsupported attribution or a merely mentioned speaker cannot be compiled into speech.",
        );
        let mut candidate = None;
        if exact && allowed {
            speakers.insert(t.speaker.clone());
            let s = speaker.unwrap();
            match dramatic_script::ScriptTurn::new(i,&s.name,&t.text,None::<String>,Some((t.span[0] as usize,t.span[1] as usize))){Ok(value)=>{candidate=Some(value.clone());script.push(value);},Err(_)=>findings.push(finding(format!("script-grammar:{}",t.id),ReviewOutcome::Uncertain,"Generic speech grammar rejected the original text; retain prose and require a parser review."))};
        }
        turns.push(json!({"proposal":t,"source_text":slice(&source,t.span),"speaker_source_supported":exact&&allowed,"speech_candidate":candidate,"voice_pilot_approved":false}));
    }
    for q in &quoted {
        if !response
            .turns
            .iter()
            .any(|t| t.kind == "dialogue" && t.span == *q)
        {
            findings.push(finding("unattributed-quoted-prose",ReviewOutcome::Uncertain,"Quoted prose is not automatically a character performance; attribution remains unresolved."));
        }
    }
    let beat_coverage = partition(
        &source,
        &response
            .beats
            .iter()
            .map(|b| (b.id.clone(), b.span, b.text.clone()))
            .collect::<Vec<_>>(),
        [0, n],
        "beat",
        findings,
    );
    let scene_coverage = partition(
        &source,
        &response
            .scenes
            .iter()
            .map(|s| (s.id.clone(), s.span, s.text.clone()))
            .collect::<Vec<_>>(),
        [0, n],
        "scene",
        findings,
    );
    let mut pictured = BTreeSet::new();
    let mut picture_ids = BTreeSet::new();
    for s in &response.scenes {
        let rows = s
            .pictures
            .iter()
            .map(|p| {
                (
                    p.id.clone(),
                    p.span,
                    slice(&source, p.span).unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();
        partition(
            &source,
            &rows,
            s.span,
            &format!("picture:{}", s.id),
            findings,
        );
        for p in &s.pictures {
            note(
                findings,
                format!("picture-identity:{}", p.id),
                valid_id(&p.id)
                    && picture_ids.insert(p.id.clone())
                    && !p.description.trim().is_empty(),
                "Picture identity/depiction is missing or repeated.",
            );
            note(
                findings,
                format!("picture-subjects:{}", p.id),
                !p.subjects.is_empty() && p.subjects.iter().all(|id| cast_ids.contains(id)),
                "Concrete pictured subjects must refer to known source-backed entities.",
            );
            for id in &p.beat_ids {
                let beat = response.beats.iter().find(|b| b.id == *id);
                note(
                    findings,
                    format!("picture-beat:{id}"),
                    beat.is_some_and(|b| {
                        s.span[0] <= b.span[0]
                            && b.span[1] <= s.span[1]
                            && p.span[0] < b.span[1]
                            && b.span[0] < p.span[1]
                    }),
                    "Picture references an absent/unrelated source beat.",
                );
                pictured.insert(id.clone());
            }
        }
    }
    for b in &response.beats {
        note(
            findings,
            format!("beat-membership:{}", b.id),
            b.subjects.iter().all(|i| cast_ids.contains(i))
                && b.fact_ids.iter().all(|i| fact_ids.contains(i))
                && b.action_ids.iter().all(|i| action_ids.contains(i)),
            "Beat references an absent subject/fact/action.",
        );
        note(
            findings,
            format!("beat-depiction:{}", b.id),
            matches!(
                b.depiction.as_str(),
                "picture" | "continuation" | "omission"
            ) && !b.description.trim().is_empty()
                && (pictured.contains(&b.id) || b.depiction == "omission"),
            "A beat needs a pictured/continued depiction or explicit omission reason.",
        );
        if b.depiction == "omission" {
            findings.push(finding(format!("beat-omission:{}",b.id),ReviewOutcome::Uncertain,"An explicit omission remains a source-coverage review question; no accepted coverage is inferred."));
        }
    }
    let mut facts = vec![];
    for f in &response.facts {
        let ok = f
            .support
            .as_ref()
            .is_some_and(|c| cited(&source, c) && c.quote == f.assertion)
            && f.entity_ids.iter().all(|i| cast_ids.contains(i));
        if !ok {
            findings.push(finding(format!("fact-source:{}",f.id),ReviewOutcome::Uncertain,"Unsupported factual claim retained privately; a paraphrase or citation is not semantic entailment."));
        }
        facts.push(json!({"proposal":f,"literal_source_support":ok,"semantic_review":"missing","reveal_permission":false}));
    }
    let supported = [
        "enter", "exit", "move", "facing", "nod", "shrug", "recoil", "bow", "wobble",
    ];
    let mut actions = vec![];
    for a in &response.actions {
        let literal = a
            .support
            .as_ref()
            .is_some_and(|c| cited(&source, c) && c.quote == a.assertion)
            && cast_ids.contains(&a.actor_id);
        let narrated = a
            .support
            .as_ref()
            .is_some_and(|c| !quoted.iter().any(|q| q[0] < c.span[1] && c.span[0] < q[1]));
        note(
            findings,
            format!("action-kind:{}", a.id),
            supported.contains(&a.kind.as_str()),
            "Required action is not supported by the player capabilities.",
        );
        if !literal || !narrated {
            findings.push(finding(format!("action-source:{}",a.id),ReviewOutcome::Uncertain,"Unsupported/quoted/conditional action claim needs independent source review; do not enact dialogue as an event."));
        }
        actions.push(json!({"proposal":a,"literal_narrated_support":literal&&narrated,"semantic_review":"missing","ready_for_execution":false}));
    }
    let mut reference_requirements = vec![];
    for r in &response.references {
        note(
            findings,
            format!("reference-identity:{}", r.id),
            valid_id(&r.subject_type_id) && r.entity_ids.iter().all(|i| cast_ids.contains(i)),
            "Reference subject/type identity is unresolved.",
        );
        let ok = if r.usage == "style_only" {
            r.entity_ids.is_empty() && r.support.is_none()
        } else {
            r.usage == "subject"
                && !r.entity_ids.is_empty()
                && r.support.as_ref().is_some_and(|c| cited(&source, c))
        };
        if !ok {
            findings.push(finding(format!("reference-scope:{}",r.id),ReviewOutcome::Uncertain,"Reference applicability/source scope is unresolved; unrelated references cannot fill a field."));
        }
        reference_requirements.push(json!({"proposal":r,"source_citation_valid":ok,"research_outcome":"missing","canonical_image":null,"approved_for_asset_request":false,"feature_applicability_review":"missing"}));
    }
    Ok(
        json!({"schema_version":1,"provenance":"diagnostic","production_approved":false,"book_id":ctx.book_id,"chapter_id":ctx.chapter_id,"source_sha256":digest(source_text.as_bytes()),"source_text":source_text,"source_span":[0,n],"compiler_sha256":digest(include_bytes!("direction.rs")),"cast":cast,"speakers":speakers,"turns":turns,"script_candidate":script,"coverage":{"turns":turn_coverage,"beats":beat_coverage,"scenes":scene_coverage},"facts":facts,"actions":actions,"reference_requirements":reference_requirements,"raw_direction":response,"reveal_permissions":[],"final_master":null,"runtime_policy_authority":false}),
    )
}
fn request(ctx: &Context, registry: &Registry, source: &Value) -> Value {
    let text = source["text"].as_str().unwrap();
    json!({"schema_version":1,"task":"source-backed-direction-v1","book_id":ctx.book_id,"chapter_id":ctx.chapter_id,"source":{"artifact":ctx.source,"sha256":source["source_sha256"],"span":[0,text.chars().count()],"text":text},"registry":{"file":ctx.registry,"snapshot":registry},"recipe":ctx.recipe,"response_schema":"direction-plan-v1","response_json_schema":parse::<Value>(include_bytes!("../../../../fixtures/generation/direction-v1/response-schema.json")).expect("compiled private direction schema"),"constraints":["verbatim-prose-and-unicode-scalar-spans","mentioned-characters-are-not-speakers","concrete-source-backed-pictures-and-actions","canonical-per-book-identities","initial-reference-research-required","no-final-timing-voices-images-or-acceptance-authority"]})
}
pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
) -> Result<StageOutput, StageError> {
    need(
        input.inputs.len() <= 64
            && input.inputs.iter().all(|a| a.file.bytes <= 16 * META)
            && input.inputs.iter().map(|a| a.file.bytes).sum::<u64>() <= 64 * META,
        "direction input envelope",
    )?;
    for a in &input.inputs {
        if !["raw-source.txt", "clean-source.txt"].contains(&a.file.path.as_str()) {
            need(a.file.bytes <= META, "direction metadata limit")?;
        }
    }
    let mut files = inputs::load(input, services).await?.files;
    let ctx: Context = parse(&get(&files, "private/direction-context.json")?.1)?;
    need(
        ctx.schema_version == 1
            && [&ctx.book_id, &ctx.chapter_id, &ctx.provider, &ctx.model]
                .iter()
                .all(|s| valid_id(s))
            && ctx.recipe.sha256 == input.pins.recipe_sha256,
        "direction book/chapter/recipe context",
    )?;
    verified(&files, &ctx.recipe)?;
    let source_receipt: Value = parse(verified(&files, &ctx.source)?)?;
    need(
        source_receipt["schema_version"] == 1,
        "source-stage receipt schema",
    )?;
    let frozen = source::validate(
        &get(&files, "frozen-source.json")?.1,
        &get(&files, "raw-source.txt")?.1,
        &get(&files, "clean-source.txt")?.1,
    )?;
    need(
        frozen == source_receipt["source"] && frozen["book_id"] == ctx.book_id,
        "source-stage receipt identity",
    )?;
    let source_inputs: Vec<ArtifactRef> = serde_json::from_value(source_receipt["inputs"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    for a in &source_inputs {
        verified(&files, &a.file)?;
        need(
            input.inputs.iter().any(|actual| equal(actual, a)),
            "source-stage original ArtifactRef changed",
        )?;
    }
    need(
        ["raw-source.txt", "clean-source.txt", "frozen-source.json"]
            .iter()
            .all(|p| source_inputs.iter().any(|a| a.file.path == *p)),
        "source-stage inputs incomplete",
    )?;
    let registry: Registry = parse(verified(&files, &ctx.registry)?)?;
    need(
        registry.schema_version == 1
            && registry.book_id == ctx.book_id
            && registry.revision == input.library_revision
            && registry.entities.len() <= 1024,
        "book registry identity/revision",
    )?;
    let (mut registry_ids, mut aliases) = (BTreeSet::new(), BTreeSet::new());
    for e in &registry.entities {
        need(
            valid_id(&e.id) && registry_ids.insert(e.id.clone()),
            "registry entity identity",
        )?;
        let names = e
            .aliases
            .iter()
            .chain(std::iter::once(&e.name))
            .map(|s| s.to_lowercase())
            .collect::<BTreeSet<_>>();
        for name in names {
            need(
                !name.trim().is_empty() && aliases.insert(name),
                "conflicting canonical alias",
            )?;
        }
        for a in [&e.sprite, &e.voice].into_iter().flatten() {
            a.file.validate()?;
        }
    }
    let request_value = request(&ctx, &registry, &frozen);
    let request_bytes = json(&request_value)?;
    need(
        request_bytes.len() as u64 <= META,
        "direction request bound",
    )?;
    let request_file = FileRef {
        path: REQUEST.into(),
        sha256: digest(&request_bytes),
        bytes: request_bytes.len() as u64,
        media_type: "application/json".into(),
        dimensions: None,
    };
    let mut base = source_inputs;
    for f in [&ctx.source, &ctx.registry, &ctx.recipe] {
        let a = artifact(input, f)?;
        if !base.iter().any(|old| equal(old, &a)) {
            base.push(a);
        }
    }
    base.sort_by(|a, b| a.file.path.cmp(&b.file.path));
    let mut outputs = vec![(REQUEST, request_value)];
    let mut findings = vec![];
    let mut proposal = None;
    if let Some((f, _)) = files.get(REQUEST) {
        need(
            *f == request_file,
            "committed director request differs from exact compilation",
        )?;
        let p = ProviderProposalV2 {
            schema_version: 2,
            kind: OperationKind::Direction,
            provider: ctx.provider.clone(),
            model: ctx.model.clone(),
            recipe_sha256: ctx.recipe.sha256.clone(),
            payload: artifact(input, f)?,
            inputs: base.clone(),
            estimate: ctx.estimate.clone(),
            timeout_ms: ctx.timeout_ms,
        };
        p.validate()?;
        outputs.push((PROPOSAL, serde_json::to_value(&p).unwrap()));
        proposal = Some(p);
    } else {
        findings.push(missing("direction-request-commit-pending","Runtime must commit the proposed request before a ProviderProposalV2 can name its genuine ArtifactRef."));
    }
    let observer = ProviderObserverV2 {
        schema_version: 2,
        run_id: input.run_id.clone(),
        epoch: input.epoch,
        unit_id: input.unit_id.clone(),
    };
    let mut observation = files
        .get(OBSERVATION)
        .map(|(_, b)| parse::<Observation>(b))
        .transpose()?;
    let mut fresh = false;
    if observation.is_none()
        && input.mode == ExecutionMode::Live
        && let Some(p) = &proposal
    {
        match services.issue_provider_v2(p.clone()).await{
            Ok(issued)=>{
                issued.validate_for_proposal(p,&observer)?;services.check_cancelled().await?;
                let mut obs=Observation{schema_version:1,provenance:Provenance::Diagnostic,source_sha256:frozen["source_sha256"].as_str().unwrap().into(),recipe_sha256:ctx.recipe.sha256.clone(),provider:ctx.provider.clone(),model:ctx.model.clone(),request:p.payload.clone(),response:None,issued:Some(issued.clone()),result:None,findings:vec![]};
                match services.execute_provider_v2(&issued).await{
                    Ok(result)=>{
                        result.validate_for_operation(&issued)?;
                        if result.result.completion==ProviderCompletion::Completed&&result.epoch_disposition==ProviderEpochDisposition::Current&&result.retry.is_none()&&result.recovery.is_none(){obs.response=result.result.raw_response.clone();}
                        else{obs.findings.push(finding("direction-provider-result-unresolved",ReviewOutcome::Uncertain,"Ambiguous/rejected/obsolete/retry/recovery result is retained privately; no usable direction result is inferred."));}
                        obs.result=Some(result);
                    },
                    Err(ServiceError::Cancelled)=>return Err(ServiceError::Cancelled.into()),
                    Err(_)=>obs.findings.push(missing("direction-provider-v2-unavailable","Issued execution unavailable; retain issuance and resolve the existing attempt without fallback or redispatch.")),
                }
                services.check_cancelled().await?;
                observation=Some(obs);fresh=true;
            },
            Err(ServiceError::Cancelled)=>return Err(ServiceError::Cancelled.into()),
            Err(_)=>findings.push(missing("direction-provider-v2-unavailable","Runtime v2 issuance is unavailable/denied; no legacy/helper fallback or fabricated result.")),
        }
    }
    let provider_metadata_present = observation.as_ref().is_some_and(|o| o.issued.is_some());
    let mut observation_file = files.get(OBSERVATION).map(|(f, _)| f.clone());
    let mut plan = None;
    if let Some(obs) = observation {
        let p = proposal.as_ref().ok_or_else(|| {
            StageError::Evidence("saved direction requires the committed exact request".into())
        })?;
        need(
            obs.schema_version == 1
                && matches!(
                    obs.provenance,
                    Provenance::Synthetic | Provenance::Imported | Provenance::Diagnostic
                )
                && obs.provider == ctx.provider
                && obs.model == ctx.model
                && obs.source_sha256 == frozen["source_sha256"]
                && obs.recipe_sha256 == ctx.recipe.sha256
                && equal(&obs.request, &p.payload),
            "saved source/recipe/model/request binding",
        )?;
        if let Some(issued) = &obs.issued {
            issued.validate_for_proposal(p, &observer)?;
            if let Some(result) = &obs.result {
                result.validate_for_operation(issued)?;
                need(
                    obs.response.as_ref().is_none_or(|a| {
                        result.result.completion == ProviderCompletion::Completed
                            && result.epoch_disposition == ProviderEpochDisposition::Current
                            && result.retry.is_none()
                            && result.recovery.is_none()
                            && result
                                .result
                                .raw_response
                                .as_ref()
                                .is_some_and(|raw| equal(a, raw))
                    }),
                    "unusable or mismatched provider-v2 result",
                )?;
            } else {
                need(
                    obs.response.is_none(),
                    "response without a completed issued result",
                )?;
            }
        } else {
            need(
                obs.result.is_none(),
                "result without original issued operation",
            )?;
        }
        if fresh {
            if let Some(raw) = &obs.response {
                need(raw.file.bytes <= META, "direction raw response bound")?;
                services.check_cancelled().await?;
                let b = services.read_artifact(raw).await?;
                raw.file.verify(&b)?;
                services.check_cancelled().await?;
                files.insert(raw.file.path.clone(), (raw.file.clone(), b));
            }
            let value = serde_json::to_value(&obs).unwrap();
            let b = json(&value)?;
            observation_file = Some(FileRef {
                path: OBSERVATION.into(),
                sha256: digest(&b),
                bytes: b.len() as u64,
                media_type: "application/json".into(),
                dimensions: None,
            });
            outputs.push((OBSERVATION, value));
        }
        findings.extend(obs.findings.clone());
        if let Some(raw) = &obs.response {
            let b = verified(&files, &raw.file)?;
            need(
                fresh || input.inputs.iter().any(|a| equal(a, raw)),
                "raw response must be a committed supplied ArtifactRef",
            )?;
            let response: Response = parse(b)?;
            need(
                response
                    .request_sha256
                    .as_ref()
                    .is_none_or(|h| *h == request_file.sha256),
                "raw response request binding",
            )?;
            let mut compiled = compile(
                &ctx,
                &registry,
                frozen["text"].as_str().unwrap(),
                &response,
                &mut findings,
            )?;
            compiled["request"] = json!(request_file);
            compiled["response"] = json!(raw);
            compiled["observation"] = json!(observation_file);
            plan = Some(compiled);
        } else {
            findings.push(missing(
                "direction-response-missing",
                "Saved attempt has no usable response; preserve observation and do not redispatch.",
            ));
        }
    } else {
        findings.push(missing(
            "direction-response-missing",
            "No saved direction observation is available; fixture/replay never redispatch.",
        ));
    }
    for id in [
        "initial-reference-research",
        "canonical-reference-applicability",
        "voice-pilot",
        "direction-semantic-review",
        "rendered-stage-clarity",
        "production-verifier-pending",
    ] {
        findings.push(missing(id,"Planning is private diagnostic data; required independent research/pilot/semantic/rendered acceptance remains unresolved."));
    }
    if let Some(mut plan) = plan {
        plan["findings"] = json!(findings);
        outputs.push((PLAN, plan));
    }
    outputs.push((HANDOFF,json!({"schema_version":1,"request_file":request_file,"content":{"kind":"direction","provider":ctx.provider,"model":ctx.model,"recipe_sha256":ctx.recipe.sha256,"inputs":base,"estimate":ctx.estimate,"timeout_ms":ctx.timeout_ms},"runtime_committed_payload_required":true,"caller_authority":false})));
    outputs.push((STATUS,json!({"schema_version":1,"provenance":"diagnostic","production_approved":false,"provider_calls":if provider_metadata_present{Value::Null}else{json!(0)},"source":ctx.source,"registry":ctx.registry,"recipe":ctx.recipe,"request":request_file,"observation":observation_file,"findings":findings,"reveal_permissions":[],"runtime_policy_authority":false})));
    for (path, value) in &outputs {
        let b = json(value)?;
        need(b.len() as u64 <= META, "direction candidate bound")?;
        if input.mode == ExecutionMode::Replay
            && let Some((old, _)) = files.get(*path)
        {
            need(
                old.sha256 == digest(&b) && old.bytes == b.len() as u64,
                "stale recorded direction candidate",
            )?;
        }
    }
    let key = digest(&json(
        &outputs
            .iter()
            .filter(|(path, _)| *path != OBSERVATION)
            .collect::<Vec<_>>(),
    )?);
    let mut artifacts = vec![];
    for (path, value) in outputs {
        artifacts.push(write(services, input, path, &value).await?);
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
