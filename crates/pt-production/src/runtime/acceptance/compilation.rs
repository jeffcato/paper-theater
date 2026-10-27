use super::*;
const V2: &[u8] =
    include_bytes!("../../../../../fixtures/generation/two-scene-readiness-v2/fixture.json");
const V1: &[u8] = include_bytes!("../../../../../fixtures/shared/two-scene-v1/fixture.json");
fn selected(
    store: &Store,
    run: &str,
) -> Result<(c::PlaybackDocument, BTreeMap<String, c::ArtifactRef>)> {
    let seeds = store
        .seed_inputs_for_stage(run, c::Stage::Package)?
        .into_iter()
        .map(|a| (a.file.path.clone(), a))
        .collect::<BTreeMap<_, _>>();
    let fixture = seeds.get("fixture.json").ok_or_else(|| {
        Error::new(
            "acceptance_registry_missing",
            "trusted package compilation is missing",
        )
    })?;
    let bytes = [V2, V1]
        .into_iter()
        .find(|b| digest(b) == fixture.file.sha256)
        .ok_or_else(|| {
            Error::new(
                "acceptance_registry_missing",
                "package is not a registered frozen diagnostic compilation",
            )
        })?;
    fixture
        .file
        .verify(&store.artifact(&fixture.id)?.0)
        .map_err(|e| Error::new("acceptance_input", e.to_string()))?;
    Ok((serde_json::from_slice(bytes)?, seeds))
}
fn native_domain(bytes: &[u8]) -> Result<(u32, u64)> {
    super::super::media::validate_artifact_bytes(bytes, "audio/wav")?;
    let mut pos = 12;
    let mut rate = 0;
    let mut align = 0;
    let mut frames = 0;
    while pos < bytes.len() {
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        match &bytes[pos..pos + 4] {
            b"fmt " => {
                rate = u32::from_le_bytes(bytes[pos + 12..pos + 16].try_into().unwrap());
                align = u16::from_le_bytes(bytes[pos + 20..pos + 22].try_into().unwrap()) as u64;
            }
            b"data" => {
                if align == 0 {
                    return fail("acceptance_input", "native frame alignment missing");
                }
                frames = size as u64 / align;
            }
            _ => {}
        }
        pos += 8 + size + size % 2;
    }
    Ok((rate, frames))
}
pub(super) fn construct(
    store: &Store,
    run: &str,
    epoch: i64,
    player: c::FileRef,
    player_version: &str,
) -> Result<Compilation> {
    Store::active(&store.db()?, run, epoch)?;
    let snapshot = store.snapshot(run)?;
    let request: c::RunRequest = serde_json::from_value(snapshot["request"].clone())?;
    if request.mode != c::ExecutionMode::Fixture
        || request.pins.recipe_sha256 != digest(b"saved-diagnostic-v1")
        || snapshot["request"]["source_id"] != "pg-55-chapter-1"
        || snapshot["request"]["book_id"] != "pg-55"
    {
        return fail(
            "acceptance_scope",
            "only the registered Oz diagnostic excerpt has a trusted compilation",
        );
    }
    let (mut expected, seeds) = selected(store, run)?;
    if expected.acceptance.player_version != player_version {
        return fail(
            "acceptance_registry_invalid",
            "frozen player version differs from trusted compilation",
        );
    }
    let mut private = Vec::new();
    let mut inputs = Vec::new();
    let mut role = |kind: c::AcceptanceInputRole, source: c::FileRef, path: &str| -> Result<()> {
        open_object(store, &source)?;
        let mut file = source;
        file.path = path.into();
        inputs.push(c::AcceptanceInput {
            role: kind,
            file: file.clone(),
        });
        private.push(file);
        Ok(())
    };
    let seed = |path: &str| -> Result<c::FileRef> {
        seeds.get(path).map(|a| a.file.clone()).ok_or_else(|| {
            Error::new(
                "acceptance_registry_missing",
                format!("trusted role unavailable: {path}"),
            )
        })
    };
    let source = seed("source.txt")?;
    if source.sha256 != expected.manifest.source_sha256 {
        return fail("acceptance_input", "trusted source identity differs");
    }
    role(c::AcceptanceInputRole::Source, source, "inputs/source.txt")?;
    let narration = store
        .seed_inputs_for_stage(run, c::Stage::Presentation)?
        .into_iter()
        .find(|a| {
            a.file.path == "excerpt.wav" && a.file.sha256 == expected.read_along.narration_sha256
        })
        .ok_or_else(|| {
            Error::new(
                "acceptance_registry_missing",
                "original narration role unavailable",
            )
        })?
        .file;
    let (rate, frames) = native_domain(&read_object(
        store,
        &narration,
        super::super::media::MAX_ARTIFACT_BYTES,
    )?)?;
    role(
        c::AcceptanceInputRole::Narration,
        narration,
        "inputs/narration.wav",
    )?;
    for (kind, path, to, hash) in [
        (
            c::AcceptanceInputRole::Alignment,
            "evidence/alignment.json",
            "inputs/alignment.json",
            expected.read_along.alignment_sha256.clone(),
        ),
        (
            c::AcceptanceInputRole::PresentationMap,
            "presentation-map.json",
            "inputs/map.json",
            expected.timeline.presentation_map_sha256.clone(),
        ),
        (
            c::AcceptanceInputRole::Profile,
            "profile.json",
            "inputs/profile.json",
            request.pins.profile_sha256.clone(),
        ),
    ] {
        let f = seed(path)?;
        if f.sha256 != hash {
            return fail("acceptance_input", "trusted role identity mismatch");
        }
        if kind == c::AcceptanceInputRole::Profile {
            let p: c::RenderingProfile = serde_json::from_slice(&read_object(
                store,
                &f,
                super::super::media::MAX_ARTIFACT_BYTES,
            )?)?;
            if serde_json::to_value(p)? != serde_json::to_value(&expected.shell.profile)? {
                return fail(
                    "acceptance_input",
                    "profile differs from frozen compilation",
                );
            }
        }
        role(kind, f, to)?;
    }
    store.put(b"saved-diagnostic-v1")?;
    role(
        c::AcceptanceInputRole::Recipe,
        reference("recipe.txt", b"saved-diagnostic-v1", "text/plain"),
        "inputs/recipe.txt",
    )?;
    role(c::AcceptanceInputRole::Player, player, "inputs/player.bin")?;
    // Runtime's frozen diagnostic export policy; candidate metadata cannot expand it.
    expected.manifest.assets.retain(|f| {
        f.media_type.starts_with("image/")
            || f.media_type.starts_with("font/")
            || f.path == "presentation-map.json"
            || f.path == "profile.json"
    });
    let mut payload = expected
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(&expected.manifest.assets)
        .cloned()
        .collect::<Vec<_>>();
    payload.sort_by(|a, b| a.path.cmp(&b.path));
    let payload_sha = digest(&serde_json::to_vec_pretty(&payload)?);
    let mut assets = expected
        .manifest
        .assets
        .iter()
        .map(|f| f.sha256.clone())
        .collect::<Vec<_>>();
    assets.sort();
    assets.dedup();
    let source_span = [0, expected.source_text.chars().count() as u64];
    let scope_id = "pg-55-chapter-1-runtime-diagnostic-excerpt";
    let mut facts = Vec::new();
    for g in &expected.manifest.reveals {
        let asset = expected
            .timeline
            .scenes
            .iter()
            .flat_map(|s| &s.entry_snapshot.instances)
            .find(|i| g.contributor_ids.contains(&i.id))
            .and_then(|i| expected.actors.iter().find(|a| a.id == i.asset_id))
            .ok_or_else(|| Error::new("acceptance_input", "frozen fact asset is unresolved"))?;
        for (i, f) in g.facts.iter().enumerate() {
            facts.push(c::AcceptanceFact {
                id: format!("{}-fact-{i}", g.id),
                reveal_id: g.id.clone(),
                source_span: f.source_span,
                asset_sha256: asset.image.sha256.clone(),
            });
        }
    }
    let fact_file = metadata(
        store,
        "inputs/facts.json",
        &c::AcceptanceFactInventory {
            schema_version: 1,
            source_scope_id: scope_id.into(),
            source_sha256: expected.manifest.source_sha256.clone(),
            source_span,
            facts,
            required_cue_ids: expected
                .timeline
                .cues
                .iter()
                .map(|c| c.id.clone())
                .collect(),
        },
    )?;
    role(
        c::AcceptanceInputRole::FactInventory,
        fact_file,
        "inputs/facts.json",
    )?;
    drop(role);
    let get = |kind| {
        inputs
            .iter()
            .find(|i| i.role == kind)
            .unwrap()
            .file
            .sha256
            .clone()
    };
    let binding = c::AcceptanceBinding {
        inventory_sha256: payload_sha.clone(),
        source_sha256: get(c::AcceptanceInputRole::Source),
        narration_sha256: get(c::AcceptanceInputRole::Narration),
        alignment_sha256: get(c::AcceptanceInputRole::Alignment),
        presentation_map_sha256: get(c::AcceptanceInputRole::PresentationMap),
        master_sha256: expected.manifest.entries.audio.sha256.clone(),
        recipe_sha256: get(c::AcceptanceInputRole::Recipe),
        profile_sha256: get(c::AcceptanceInputRole::Profile),
        player_sha256: get(c::AcceptanceInputRole::Player),
        checker_sha256: c::acceptance_binding_checker().sha256,
        fact_inventory_sha256: get(c::AcceptanceInputRole::FactInventory),
        asset_sha256s: assets.clone(),
        source_span,
        source_scope_id: scope_id.into(),
        player_version: player_version.into(),
        candidate_checker_version: c::CONTRACT_REVISION.into(),
        device_id: None,
        narration_sample_rate: rate,
        narration_sample_count: frames,
        delivery_sample_rate: expected.manifest.sample_rate,
        delivery_sample_count: expected.manifest.sample_count,
    };
    let observed = observations(store, run)?;
    let observations_sha256 = observation_state(store, run)?;
    let mut proposals = Vec::new();
    let mut additional_ids = Vec::new();
    let mut dependencies = BTreeMap::new();
    for f in observed
        .iter()
        .chain(&expected.acceptance.findings)
        .filter(|f| f.outcome != c::ReviewOutcome::Pass)
    {
        for a in &f.evidence {
            let key = serde_json::to_string(&a.file)?;
            if dependencies.contains_key(&key) {
                continue;
            }
            // Imported findings retain their original IDs. Resolve their exact FileRef
            // against this run's immutable records, never against another run or an ID claim.
            let rows = store
                .db()?
                .prepare("SELECT reference FROM artifacts WHERE run=?1 AND sha256=?2 AND bytes=?3")?
                .query_map(params![run, a.file.sha256, a.file.bytes], |r| {
                    r.get::<_, String>(0)
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            if !rows
                .iter()
                .any(|s| serde_json::from_str::<c::ArtifactRef>(s).is_ok_and(|r| r.file == a.file))
            {
                return fail(
                    "acceptance_input",
                    "finding evidence is not an exact runtime-owned input",
                );
            }
            open_object(store, &a.file)?;
            let mut bound = a.file.clone();
            bound.path = format!("dependencies/{}/{}", bound.sha256, bound.path);
            dependencies.insert(key, bound);
        }
        let raw = metadata(
            store,
            &format!("observations/{}.json", digest(&serde_json::to_vec(f)?)),
            f,
        )?;
        additional_ids.push(f.id.clone());
        proposals.push(c::AcceptanceEvidenceProposal {
            schema_version: 1,
            id: format!("observation-{}", raw.sha256),
            check_id: f.id.clone(),
            target: c::AcceptanceTarget::Candidate,
            binding: binding.clone(),
            outcome: f.outcome,
            provenance: c::Provenance::Diagnostic,
            raw_evidence: raw.clone(),
        });
        private.push(raw);
    }
    private.extend(dependencies.into_values());
    for f in ledger(store, run)? {
        let proposal: c::AcceptanceEvidenceProposal =
            serde_json::from_slice(&read_object(store, &f, 64 * 1024)?)?;
        additional_ids.push(proposal.check_id.clone());
        private.push(f);
        private.push(proposal.raw_evidence.clone());
        proposals.push(proposal);
    }
    proposals.sort_by(|a, b| a.id.cmp(&b.id));
    proposals.dedup_by(|a, b| a.id == b.id);
    additional_ids.sort();
    additional_ids.dedup();
    let evidence = metadata(
        store,
        "inputs/evidence-inventory.json",
        &c::AcceptanceEvidenceInventory {
            schema_version: 1,
            proposals,
        },
    )?;
    inputs.push(c::AcceptanceInput {
        role: c::AcceptanceInputRole::EvidenceInventory,
        file: evidence.clone(),
    });
    private.push(evidence);
    let policy = c::AcceptanceVerificationPolicy {
        schema_version: 1,
        scope: c::AcceptanceScope::AutonomousExcerpt,
        inputs,
        source_span,
        narration_sample_rate: rate,
        narration_sample_count: frames,
        source_scope_id: scope_id.into(),
        payload_inventory_sha256: payload_sha,
        asset_sha256s: assets,
        device_id: None,
        player_version: player_version.into(),
        candidate_checker_version: c::CONTRACT_REVISION.into(),
        checker_sha256: c::acceptance_binding_checker().sha256,
        additional_required_check_ids: additional_ids,
    };
    let output:String=store.db()?.query_row("SELECT output FROM units WHERE run=?1 AND stage='package' AND outcome='needs_repair' ORDER BY rowid DESC LIMIT 1",[run],|r|r.get(0)).optional()?.ok_or_else(||Error::new("acceptance_candidate_missing","frozen diagnostic package is required"))?;
    let output: Vec<c::ArtifactRef> = serde_json::from_str(&output)?;
    let document = output
        .iter()
        .find(|a| a.file.path == "diagnostic-document.json")
        .ok_or_else(|| {
            Error::new(
                "acceptance_candidate_missing",
                "private playback document is missing",
            )
        })?
        .file
        .clone();
    let actual: c::PlaybackDocument = serde_json::from_slice(&read_object(
        store,
        &document,
        super::super::media::MAX_ARTIFACT_BYTES,
    )?)?;
    expected.manifest.acceptance = actual.manifest.acceptance.clone();
    expected.manifest.provenance = c::Provenance::Diagnostic;
    expected.acceptance = actual.acceptance.clone();
    if serde_json::to_value(&expected)? != serde_json::to_value(&actual)? {
        return fail(
            "acceptance_candidate_binding",
            "candidate differs from trusted frozen compilation",
        );
    }
    let mut package = payload;
    package.push(actual.manifest.acceptance.clone());
    for path in ["inventory.json", "manifest.json"] {
        let f = output
            .iter()
            .find(|a| a.file.path == path)
            .ok_or_else(|| Error::new("acceptance_candidate_missing", "package identity missing"))?
            .file
            .clone();
        if path == "inventory.json" && f.sha256 != policy.payload_inventory_sha256 {
            return fail(
                "acceptance_candidate_binding",
                "candidate inventory differs from runtime compilation",
            );
        }
        package.push(f);
    }
    policy
        .validate()
        .map_err(|e| Error::new("acceptance_policy", e.to_string()))?;
    Ok(Compilation {
        schema_version: 1,
        run_id: run.into(),
        request_sha256: digest(&serde_json::to_vec(&snapshot["request"])?),
        policy,
        document,
        package,
        private,
        observations_sha256,
    })
}
