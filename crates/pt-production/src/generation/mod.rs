//! Offline generation proposals. All durable effects belong to RunServices.
use async_trait::async_trait;
use pt_contracts::*;
use serde::Serialize;
use std::collections::BTreeMap;

mod source;

pub struct GenerationExecutor;
const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;
const REQUIRED_CHECKS: &[&str] = &[
    "autonomous-provenance",
    "complete-chapter",
    "initial-reference-research",
    "image-scope-review",
    "image-style-review",
    "image-reference-review",
    "image-extraction-review",
    "image-stage-readability",
    "independent-acoustic-evidence",
    "narrator-continuity",
    "speech-joins",
    "music-bible",
    "mix-intelligibility",
    "story-coverage",
    "incoming-picture-readiness",
    "required-action-accounting",
    "rendered-creative-review",
    "opening-action-capture-coverage",
    "full-audio-review-coverage",
    "font-bundle",
    "profile-r186-browser",
    "target-device-budget",
    "publication-recovery",
    "product-spend-authorization",
];
fn missing(id: &str, message: &str) -> Finding {
    Finding {
        id: id.into(),
        field: "acceptance".into(),
        outcome: ReviewOutcome::Missing,
        message: message.into(),
        evidence: vec![],
    }
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, StageError> {
    serde_json::to_vec_pretty(value).map_err(|e| StageError::Evidence(e.to_string()))
}
fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, StageError> {
    serde_json::from_slice(bytes).map_err(|e| StageError::Evidence(e.to_string()))
}
async fn write<T: Serialize>(
    services: &dyn RunServices,
    input: &StageInput,
    path: &str,
    value: &T,
) -> Result<CandidateFile, StageError> {
    services.check_cancelled().await?;
    let bytes = json(value)?;
    let candidate_id = digest(&json(&(
        input.run_id.as_str(),
        input.epoch,
        input.unit_id.as_str(),
        path,
        digest(&bytes),
    ))?);
    let file = services
        .write_candidate(
            CandidateWrite {
                candidate_id: candidate_id.clone(),
                relative_path: path.into(),
                media_type: "application/json".into(),
                max_bytes: MAX_ARTIFACT_BYTES,
                provenance: Provenance::Diagnostic,
            },
            bytes.clone(),
        )
        .await?;
    require(
        file.candidate_id == candidate_id
            && file.file.path == path
            && file.file.media_type == "application/json"
            && file.provenance == Provenance::Diagnostic,
        "generation_candidate",
        path,
        "runtime candidate differs from requested identity",
    )?;
    file.file.verify(&bytes)?;
    services.check_cancelled().await?;
    Ok(file)
}
#[async_trait]
impl StageExecutor for GenerationExecutor {
    async fn execute(
        &self,
        input: StageInput,
        services: &dyn RunServices,
    ) -> Result<StageOutput, StageError> {
        input.validate()?;
        services.check_cancelled().await?;
        let mut files = BTreeMap::new();
        let mut total = 0u64;
        require(
            input.inputs.len() <= 64,
            "generation_inputs",
            "inputs",
            "too many input artifacts",
        )?;
        for artifact in &input.inputs {
            require(
                artifact.file.bytes <= MAX_ARTIFACT_BYTES,
                "generation_size",
                &artifact.id,
                "input exceeds bound",
            )?;
            total = total.saturating_add(artifact.file.bytes);
            require(
                total <= 64 * 1024 * 1024,
                "generation_size",
                "inputs",
                "total input exceeds bound",
            )?;
            services.check_cancelled().await?;
            let bytes = services.read_artifact(artifact).await?;
            artifact.file.verify(&bytes)?;
            if let Some((old, _)) =
                files.insert(artifact.file.path.clone(), (artifact.file.clone(), bytes))
            {
                require(
                    old == artifact.file,
                    "generation_duplicate",
                    &artifact.id,
                    "conflicting artifact path",
                )?;
            }
        }
        let cache_key = digest(&json(&input)?);
        if input.stage == Stage::Source {
            let get = |path: &str| {
                files
                    .get(path)
                    .ok_or_else(|| StageError::Evidence(format!("missing source input: {path}")))
            };
            let frozen = source::validate(
                &get("frozen-source.json")?.1,
                &get("raw-source.txt")?.1,
                &get("clean-source.txt")?.1,
            )?;
            let value = serde_json::json!({"schema_version":1,"provenance":"imported","source":frozen,"pins":input.pins,"inputs":input.inputs});
            let candidate = write(services, &input, "source.json", &value).await?;
            return Ok(StageOutput {
                outcome: StageOutcome::Complete,
                artifacts: vec![candidate],
                decisions: vec![],
                findings: vec![],
                cache_keys: vec![cache_key],
                library_updates: vec![],
                package_candidate: None,
            });
        }
        if input.stage != Stage::Package {
            return Ok(StageOutput {
                outcome: StageOutcome::NeedsRepair,
                artifacts: vec![],
                decisions: vec![],
                findings: vec![missing(
                    "stage-adapter-pending",
                    "This stage requires its bounded production adapter; no accepted evidence is inferred from saved inputs.",
                )],
                cache_keys: vec![cache_key],
                library_updates: vec![],
                package_candidate: None,
            });
        }
        let get = |path: &str| {
            files
                .get(path)
                .ok_or_else(|| StageError::Evidence(format!("missing package input: {path}")))
        };
        let mut doc: PlaybackDocument = parse(&get("fixture.json")?.1)?;
        validate_playback(&doc)?;
        let profile: RenderingProfile = parse(&get("profile.json")?.1)?;
        require(
            get("profile.json")?.0.sha256 == input.pins.profile_sha256
                && profile.id == input.pins.profile_id,
            "generation_profile",
            "pins",
            "profile bytes or revision differ",
        )?;
        require(
            serde_json::to_value(&profile).ok() == serde_json::to_value(&doc.shell.profile).ok(),
            "generation_profile",
            "shell.profile",
            "profile differs from playback document",
        )?;
        let mut inventory: Vec<FileRef> = doc
            .manifest
            .entries
            .files()
            .into_iter()
            .chain(doc.manifest.assets.iter())
            .cloned()
            .collect();
        inventory.sort_by(|a, b| a.path.cmp(&b.path));
        require(
            inventory.windows(2).all(|pair| pair[0].path < pair[1].path),
            "generation_inventory",
            "inventory",
            "duplicate payload paths",
        )?;
        for file in &inventory {
            file.verify(&get(&file.path)?.1)?;
        }
        macro_rules! verify_document {
            ($file:expr,$value:expr,$ty:ty) => {{
                let file = $file;
                let actual: $ty = parse(&get(&file.path)?.1)?;
                require(
                    json(&$value)? == json(&actual)?,
                    "generation_envelope",
                    &file.path,
                    "document differs from immutable payload",
                )?;
            }};
        }
        verify_document!(&doc.manifest.entries.timeline, doc.timeline, Timeline);
        verify_document!(
            &doc.manifest.entries.actors,
            doc.actors,
            Vec<RegisteredCard>
        );
        verify_document!(&doc.manifest.entries.shell, doc.shell, Shell);
        verify_document!(&doc.manifest.entries.read_along, doc.read_along, ReadAlong);
        let inventory_file = write(services, &input, "inventory.json", &inventory).await?;
        let mut findings = doc.acceptance.findings.clone();
        let mut required = doc.acceptance.required_check_ids.clone();
        required.extend(REQUIRED_CHECKS.iter().map(|s| (*s).to_owned()));
        required.sort();
        required.dedup();
        // Retain failed/uncertain evidence; an imported pass cannot certify a new production candidate.
        for id in &required {
            if findings
                .iter()
                .any(|finding| finding.id == *id && finding.outcome != ReviewOutcome::Pass)
            {
                continue;
            }
            findings.retain(|finding| finding.id != *id);
            findings.push(missing(
                id,
                "Required production evidence is unresolved for this diagnostic candidate.",
            ));
        }
        findings.sort_by(|a, b| a.id.cmp(&b.id));
        let acceptance = Acceptance {
            schema_version: 1,
            provenance: Provenance::Diagnostic,
            inventory_sha256: inventory_file.file.sha256.clone(),
            profile_sha256: input.pins.profile_sha256.clone(),
            player_version: doc.acceptance.player_version.clone(),
            checker_version: CONTRACT_REVISION.into(),
            recipe_sha256: input.pins.recipe_sha256.clone(),
            required_check_ids: required,
            findings: findings.clone(),
            attestations: doc.acceptance.attestations.clone(),
        };
        let acceptance_file = write(services, &input, "acceptance.json", &acceptance).await?;
        doc.acceptance = acceptance;
        doc.manifest.provenance = Provenance::Diagnostic;
        doc.manifest.acceptance = acceptance_file.file.clone();
        let manifest_file = write(services, &input, "manifest.json", &doc.manifest).await?;
        validate_playback(&doc)?;
        let document = write(services, &input, "diagnostic-document.json", &doc).await?;
        let package = PackageCandidate {
            candidate_id: manifest_file.candidate_id.clone(),
            provenance: Provenance::Diagnostic,
            payload_inventory: inventory,
            inventory_sha256: inventory_file.file.sha256.clone(),
            acceptance: acceptance_file.clone(),
            manifest: manifest_file.clone(),
            required_artifacts: input.inputs.clone(),
            findings: findings.clone(),
            pins: input.pins.clone(),
        };
        services.check_cancelled().await?;
        services.emit(JournalProposal{kind:EventKind::CheckCompleted,task_id:Some(input.unit_id.clone()),decision_id:None,
            artifact_ids:input.inputs.iter().map(|a|a.id.clone()).collect(),summary:"Diagnostic candidate assembled from verified saved payloads; production evidence remains unresolved.".into()}).await?;
        services.check_cancelled().await?;
        Ok(StageOutput{outcome:StageOutcome::NeedsRepair,artifacts:vec![inventory_file,acceptance_file,manifest_file,document],
            decisions:vec![Decision{id:format!("{}:saved-reuse",input.unit_id),summary:"Reuse verified saved media and measured timing; retain every unresolved production gate.".into(),
                inputs:input.inputs.clone(),selected_ids:vec![],rejected_ids:vec![],supersedes:None}],findings,cache_keys:vec![cache_key],library_updates:vec![],package_candidate:Some(package)})
    }
}
#[cfg(test)]
mod tests;
