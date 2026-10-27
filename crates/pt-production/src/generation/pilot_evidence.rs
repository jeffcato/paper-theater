//! Validate exact saved take/review identities. PCM checks cannot prove listening criteria.
use super::*;
use serde::Deserialize;
use serde_json::{Value, json};
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Format {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    frames: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Take {
    schema_version: u32,
    provenance: Provenance,
    request_sha256: Digest,
    recipe_sha256: Digest,
    source_sha256: Digest,
    direction_sha256: Digest,
    catalog_sha256: Digest,
    speaker: Id,
    model: Id,
    voice_id: Id,
    passage: Value,
    audio: ArtifactRef,
    format: Format,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    schema_version: u32,
    provenance: Provenance,
    request_sha256: Digest,
    recipe_sha256: Digest,
    speaker: Id,
    targets: Vec<ArtifactRef>,
    reference_audio: Option<ArtifactRef>,
    criteria: Vec<Criterion>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Criterion {
    id: Id,
    outcome: ReviewOutcome,
    message: String,
}
fn need(ok: bool, msg: &str) -> Result<(), StageError> {
    require(ok, "pilot-evidence", "identity", msg)?;
    Ok(())
}
pub(super) fn format(b: &[u8]) -> Result<Value, StageError> {
    Ok(json!(pcm(b)?))
}
fn pcm(b: &[u8]) -> Result<Format, StageError> {
    crate::runtime::validate_artifact_bytes(b, "audio/wav")
        .map_err(|_| StageError::Evidence("invalid pilot PCM16 WAV".into()))?;
    let mut pos = 12;
    let (mut rate, mut channels, mut frames) = (0, 0, 0);
    while pos < b.len() {
        let n = u32::from_le_bytes(b[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let p = &b[pos + 8..pos + 8 + n];
        match &b[pos..pos + 4] {
            b"fmt " => {
                channels = u16::from_le_bytes(p[2..4].try_into().unwrap());
                rate = u32::from_le_bytes(p[4..8].try_into().unwrap());
            }
            b"data" => frames = n as u64 / (channels as u64 * 2),
            _ => {}
        }
        pos += 8 + n + n % 2;
    }
    Ok(Format {
        sample_rate: rate,
        channels,
        bits_per_sample: 16,
        frames,
    })
}
fn raw<'a>(files: &'a Files, a: &ArtifactRef) -> Result<&'a [u8], StageError> {
    let (f, b) = files
        .get(&a.file.path)
        .ok_or_else(|| StageError::Evidence("missing pilot evidence bytes".into()))?;
    need(f == &a.file, "stale pilot evidence FileRef")?;
    Ok(b)
}
pub(super) async fn take(
    input: &StageInput,
    services: &dyn RunServices,
    files: &mut Files,
    job: &pilot_provider::Job,
    observation: &FileRef,
    obs: &pilot_provider::Observation,
) -> Result<Option<Value>, StageError> {
    let Some(response) = &obs.response else {
        return Ok(None);
    };
    let take: Take = parse(raw(files, response)?)?;
    let req = &job.request;
    need(
        take.schema_version == 1
            && matches!(
                take.provenance,
                Provenance::Synthetic | Provenance::Imported | Provenance::Diagnostic
            )
            && take.request_sha256 == obs.request.file.sha256
            && take.recipe_sha256 == req["recipe"]["sha256"]
            && take.source_sha256 == req["source_sha256"]
            && take.direction_sha256 == req["direction"]["sha256"]
            && take.catalog_sha256 == req["catalog"]["sha256"]
            && take.speaker == req["speaker"]
            && take.model == req["model"]
            && take.voice_id == req["voice_id"]
            && take.passage == req["passage"],
        "pilot raw take/request/source/voice/passage binding",
    )?;
    need(
        take.audio.file.media_type == "audio/wav" && take.audio.file.bytes <= 8 * 1024 * 1024,
        "pilot take WAV envelope",
    )?;
    if let Some(result) = &obs.result {
        need(
            result
                .result
                .outputs
                .iter()
                .any(|a| json!(a) == json!(take.audio)),
            "issued pilot result lacks exact audio output",
        )?;
    } else {
        need(
            input.inputs.iter().any(|a| json!(a) == json!(take.audio)),
            "saved pilot audio ArtifactRef missing or changed",
        )?;
    }
    pilot_provider::load(services, files, &take.audio, 8 * 1024 * 1024).await?;
    need(
        pcm(raw(files, &take.audio)?)? == take.format,
        "pilot WAV actual format/frame identity differs",
    )?;
    Ok(Some(
        json!({"id":job.id,"speaker":take.speaker,"model":take.model,"voice_id":take.voice_id,"passage":take.passage,"request":obs.request,"response":response,"observation":observation,"audio":take.audio,"format":take.format,"format_verified":true,"declared_passage_coverage_verified":true,"speech_fidelity":"missing","listening":"missing","production_approved":false}),
    ))
}
pub(super) fn review(
    files: &Files,
    job: &pilot_provider::Job,
    obs: &pilot_provider::Observation,
    findings: &mut Vec<Finding>,
) -> Result<Option<Value>, StageError> {
    let Some(response) = &obs.response else {
        return Ok(None);
    };
    let review: Review = parse(raw(files, response)?)?;
    let req = &job.request;
    let targets = req["takes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["audio"].clone())
        .collect::<Vec<_>>();
    need(
        review.schema_version == 1
            && matches!(
                review.provenance,
                Provenance::Synthetic | Provenance::Imported | Provenance::Diagnostic
            )
            && review.request_sha256 == obs.request.file.sha256
            && review.recipe_sha256 == req["recipe"]["sha256"]
            && review.speaker == req["speaker"]
            && json!(review.targets) == json!(targets)
            && json!(review.reference_audio) == req["reference_audio"]
            && review.criteria.len() <= 32,
        "pilot review request/neighbor/reference binding",
    )?;
    let mut ids = std::collections::BTreeSet::new();
    need(
        review
            .criteria
            .iter()
            .all(|c| valid_id(&c.id) && ids.insert(c.id.clone())),
        "duplicate pilot review criterion",
    )?;
    let failed = review
        .criteria
        .iter()
        .any(|c| c.outcome == ReviewOutcome::Fail);
    for c in &review.criteria {
        findings.push(Finding {
            id: format!("pilot-review:{}:{}", review.speaker, c.id),
            field: "pilot".into(),
            outcome: if c.outcome == ReviewOutcome::Pass {
                ReviewOutcome::Uncertain
            } else {
                c.outcome
            },
            message: c.message.clone(),
            evidence: vec![response.clone()],
        });
    }
    Ok(Some(
        json!({"request":obs.request,"response":response,"targets":review.targets,"claimed_criteria":review.criteria,"effective_outcome":if failed{"fail"}else{"uncertain"},"production_approved":false,"reason":"Saved/imported/synthetic review is an observation, not independent runtime listening authority; reference, joins and actual music context remain required."}),
    ))
}
