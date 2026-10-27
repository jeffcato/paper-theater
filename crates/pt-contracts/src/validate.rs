//! Structural validity never grants production acceptance.
use crate::*;
pub fn interval(value: Interval, max: Sample, field: &str, positive: bool) -> ContractResult<()> {
    require(
        value[0] <= value[1]
            && (!positive || value[0] < value[1])
            && value[1] <= max
            && max <= MAX_SAFE_INTEGER,
        "interval",
        field,
        "invalid integer sample interval",
    )
}
pub fn validate_transition(t: &SceneTransition, max: Sample) -> ContractResult<()> {
    let field = format!("transitions.{}", t.to_scene_id);
    interval([t.fade_out_start, t.fade_in_end], max, &field, true)?;
    require(
        t.swap_sample.checked_sub(t.fade_out_start) == Some(48000),
        "fade_out",
        format!("{field}.swap_sample"),
        "fade out must be 48000 samples",
    )?;
    require(
        t.fade_in_start.checked_sub(t.swap_sample) == Some(96000),
        "dark_hold",
        format!("{field}.fade_in_start"),
        "dark hold must be exactly 96000 samples",
    )?;
    require(
        t.fade_in_end.checked_sub(t.fade_in_start) == Some(48000),
        "fade_in",
        format!("{field}.fade_in_end"),
        "fade in must be 48000 samples",
    )
}

pub fn validate_playback(d: &PlaybackDocument) -> ContractResult<()> {
    let m = &d.manifest;
    let t = &d.timeline;
    require(
        m.schema_version == 1
            && t.schema_version == 1
            && d.read_along.schema_version == 1
            && d.shell.schema_version == 1
            && d.acceptance.schema_version == 1,
        "version",
        "schema_version",
        "expected schema version 1",
    )?;
    require(
        m.capability_version == CAPABILITY && t.capability_version == CAPABILITY,
        "capability",
        "capability_version",
        "unknown capability",
    )?;
    require(
        m.sample_rate == 48000
            && t.sample_rate == 48000
            && m.sample_count == t.sample_count
            && m.sample_count > 0
            && m.sample_count <= MAX_SAFE_INTEGER,
        "samples",
        "sample_count",
        "invalid 48 kHz sample domain",
    )?;
    require(
        digest(d.source_text.as_bytes()) == m.source_sha256,
        "source_hash",
        "source_text",
        "source bytes differ",
    )?;
    for f in m
        .entries
        .files()
        .into_iter()
        .chain(m.assets.iter())
        .chain([&m.acceptance])
    {
        f.validate()?;
    }
    require(
        t.delivery_audio_sha256 == m.entries.audio.sha256
            && d.read_along.delivery_audio_sha256 == m.entries.audio.sha256,
        "audio_hash",
        "delivery_audio_sha256",
        "audio identities differ",
    )?;
    require(
        t.presentation_map_sha256 == d.read_along.presentation_map_sha256
            && d.read_along.source_sha256 == m.source_sha256,
        "identity",
        "read_along",
        "source/map identities differ",
    )?;
    for tr in &t.transitions {
        validate_transition(tr, t.sample_count)?;
    }
    require(
        t.scenes.len() > 0 && t.transitions.len() + 1 == t.scenes.len(),
        "scenes",
        "timeline.scenes",
        "one transition per adjacent scene",
    )?;
    let o = &t.opening;
    let c = &t.closing;
    for (f, i) in [
        ("title_hold", o.title_hold),
        ("title_dismiss", o.title_dismiss),
        ("curtain_open", o.curtain_open),
        ("curtain_close", c.curtain_close),
    ] {
        interval(i, t.sample_count, f, true)?;
    }
    require(
        o.title_hold[0] == 0
            && o.title_hold[1] - o.title_hold[0] >= 72000
            && o.title_dismiss[0] == o.title_hold[1]
            && o.title_dismiss[1] - o.title_dismiss[0] >= 24000,
        "opening",
        "opening.title_hold",
        "title hold/dismiss minimums or adjacency invalid",
    )?;
    require(
        o.curtain_open[0] >= o.title_dismiss[1]
            && o.curtain_open[1] - o.curtain_open[0] == 72000
            && o.first_picture_eligible_sample <= o.curtain_open[0],
        "opening",
        "opening.curtain_open",
        "curtain duration or first picture eligibility invalid",
    )?;
    require(
        c.curtain_close[1] - c.curtain_close[0] == 72000
            && c.curtain_close[0] >= c.last_prose_action_sample
            && c.terminal_sample == t.sample_count
            && c.curtain_close[1] <= t.sample_count
            && d.timeline.terminal_snapshot.curtain_open_ratio == 0.0,
        "closing",
        "closing",
        "terminal curtains must be closed inside master",
    )?;
    for (i, s) in t.scenes.iter().enumerate() {
        interval(s.narration, t.sample_count, &s.id, true)?;
        require(
            s.entry_snapshot.set_id == s.set_id,
            "set",
            &s.id,
            "entry snapshot set differs",
        )?;
        if let Some(tr) = t.transitions.get(i) {
            require(
                tr.from_scene_id == s.id
                    && tr.to_scene_id == t.scenes[i + 1].id
                    && s.narration[1] <= tr.fade_out_start
                    && t.scenes[i + 1].narration[0] >= tr.fade_in_end,
                "transition_scene",
                &s.id,
                "adjacent scenes or narration-free blackout invalid",
            )?;
        }
    }
    validate_read_along(&d.read_along, &d.source_text, t.sample_count)?;
    validate_reveals(d)?;
    validate_geometry_and_cues(d)?;
    Ok(())
}
/// Immutable-byte verification; acceptance inventory excludes itself, manifest and catalog.
pub fn verify_package_files(d: &PlaybackDocument, root: &std::path::Path) -> ContractResult<()> {
    let canonical = root.canonicalize().map_err(|_| ContractError {
        code: "path".into(),
        field: "root".into(),
        message: "fixture root missing".into(),
    })?;
    let read = |path: &str| -> ContractResult<Vec<u8>> {
        require(safe_path(path), "path", path, "unsafe release path")?;
        let p = root.join(path);
        let mut current = root.to_path_buf();
        for part in path.split('/') {
            current.push(part);
            require(!current.is_symlink(), "path", path, "symlink forbidden")?;
        }
        require(
            p.canonicalize().is_ok_and(|x| x.starts_with(&canonical)),
            "path",
            path,
            "path escapes release",
        )?;
        std::fs::read(p).map_err(|_| ContractError {
            code: "file".into(),
            field: path.into(),
            message: "file unavailable".into(),
        })
    };
    for f in d
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(d.manifest.assets.iter())
        .chain([&d.manifest.acceptance])
    {
        f.verify(&read(&f.path)?)?;
    }
    let bytes = read("inventory.json")?;
    require(
        digest(&bytes) == d.acceptance.inventory_sha256,
        "inventory_hash",
        "inventory.json",
        "inventory exact bytes differ",
    )?;
    let inventory: Vec<FileRef> = serde_json::from_slice(&bytes).map_err(|e| ContractError {
        code: "inventory".into(),
        field: "inventory.json".into(),
        message: e.to_string(),
    })?;
    let mut expected: Vec<FileRef> = d
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(d.manifest.assets.iter())
        .cloned()
        .collect();
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    require(
        inventory == expected && inventory.windows(2).all(|w| w[0].path < w[1].path),
        "inventory",
        "inventory.json",
        "expected sorted unique payload inventory only",
    )?;
    macro_rules! bound {
        ($path:expr, $value:expr, $ty:ty) => {{
            let path = $path;
            let disk: $ty = serde_json::from_slice(&read(path)?).map_err(|e| ContractError {
                code: "json".into(),
                field: path.clone(),
                message: e.to_string(),
            })?;
            require(
                serde_json::to_value(disk).unwrap() == serde_json::to_value($value).unwrap(),
                "document_hash",
                path,
                "load envelope differs from bound file",
            )?;
        }};
    }
    bound!(&d.manifest.entries.timeline.path, &d.timeline, Timeline);
    bound!(
        &d.manifest.entries.actors.path,
        &d.actors,
        Vec<RegisteredCard>
    );
    bound!(&d.manifest.entries.shell.path, &d.shell, Shell);
    bound!(
        &d.manifest.entries.read_along.path,
        &d.read_along,
        ReadAlong
    );
    bound!(&d.manifest.acceptance.path, &d.acceptance, Acceptance);
    Ok(())
}
/// Full creative/evidence acceptance requires independent worker verifiers. Until
/// integrated, even fabricated autonomous/pass fields cannot authorize publication.
pub fn production_blockers(d: &PlaybackDocument) -> Vec<String> {
    let mut ids = vec!["production-verifier-pending".into()];
    if !declared_production_coverage_complete(d) { ids.push("required-check-coverage".into()); }
    if d.manifest.provenance != Provenance::Autonomous {
        ids.push("autonomous-provenance".into());
    }
    if d.shell.profile.status != ProfileStatus::Verified {
        ids.push("profile-r186-browser".into());
    }
    ids.extend(
        d.acceptance
            .findings
            .iter()
            .filter(|f| f.outcome != ReviewOutcome::Pass)
            .map(|f| f.id.clone()),
    );
    ids.sort();
    ids.dedup();
    ids
}

pub fn validate_read_along(r: &ReadAlong, source: &str, max: Sample) -> ContractResult<()> {
    use std::collections::{HashMap, HashSet};
    require(
        r.policy == "point-following-same-boundary-v1",
        "read_policy",
        "read_along.policy",
        "unknown grouping policy",
    )?;
    let scalars: Vec<char> = source.chars().collect();
    let mut cursor = r.leading_text.chars().count();
    let mut full = r.leading_text.clone();
    let mut words = HashMap::new();
    require(
        !r.words.is_empty() && !r.leading_text.chars().any(char::is_alphanumeric),
        "word_coverage",
        "read_along.words",
        "missing source words",
    )?;
    for w in &r.words {
        require(
            valid_id(&w.id) && words.insert(w.id.as_str(), w).is_none(),
            "duplicate_id",
            &w.id,
            "duplicate or invalid word ID",
        )?;
        let [a, b] = w.source_span;
        require(
            a == cursor as u64
                && a < b
                && b <= scalars.len() as u64
                && scalars[a as usize..b as usize].iter().collect::<String>() == w.text
                && !w.following_text.chars().any(char::is_alphanumeric),
            "source_span",
            format!("{}.source_span", w.id),
            "expected exact Unicode scalar span and punctuation",
        )?;
        interval([w.start_sample, w.end_sample], max, &w.id, false)?;
        cursor = b as usize + w.following_text.chars().count();
        full.push_str(&w.text);
        full.push_str(&w.following_text);
    }
    require(
        full == source,
        "word_coverage",
        "read_along.words",
        "source text not reconstructed exactly",
    )?;
    let mut order = Vec::new();
    let mut ids = HashSet::new();
    let mut previous_end = 0;
    for g in &r.groups {
        require(
            valid_id(&g.id) && ids.insert(&g.id),
            "duplicate_id",
            &g.id,
            "duplicate or invalid group ID",
        )?;
        interval([g.start_sample, g.end_sample], max, &g.id, true)?;
        require(
            g.start_sample >= previous_end,
            "group_overlap",
            &g.id,
            "groups must be ordered without overlap",
        )?;
        previous_end = g.end_sample;
        let owner = words.get(g.timing_owner_word_id.as_str());
        require(
            owner.is_some() && g.word_ids.last() == Some(&g.timing_owner_word_id),
            "point_policy",
            &g.id,
            "timing owner must be the following measured interval",
        )?;
        let owner = owner.unwrap();
        require(
            owner.start_sample == g.start_sample && owner.end_sample == g.end_sample,
            "point_policy",
            &g.id,
            "group must preserve owner's measured interval",
        )?;
        for id in &g.word_ids {
            let w = words.get(id.as_str());
            require(w.is_some(), "word_coverage", id, "word reference missing")?;
            let w = w.unwrap();
            if id != &g.timing_owner_word_id {
                require(
                    w.start_sample == w.end_sample && w.start_sample == g.start_sample,
                    "point_policy",
                    id,
                    "point may share only following interval at same measured boundary",
                )?;
            }
            order.push(id.as_str());
        }
    }
    require(
        order == r.words.iter().map(|w| w.id.as_str()).collect::<Vec<_>>(),
        "word_coverage",
        "read_along.groups",
        "every source word must appear once in order",
    )?;
    require(
        r.preferred_break_after.iter().all(|id| ids.contains(id)),
        "group_reference",
        "preferred_break_after",
        "unknown group",
    )
}

pub fn reveal_eligible(group: &RevealGroup, sample: Sample) -> bool {
    group.not_before_sample.is_some_and(|n| sample >= n)
}
pub fn validate_reveals(d: &PlaybackDocument) -> ContractResult<()> {
    use std::collections::HashMap;
    let mut evidence = HashMap::new();
    let mut groups = HashMap::new();
    for e in &d.acceptance.attestations {
        require(
            valid_id(&e.id) && evidence.insert(&e.id, e).is_none(),
            "duplicate_id",
            &e.id,
            "duplicate or invalid attestation ID",
        )?;
    }
    for g in &d.manifest.reveals {
        require(
            valid_id(&g.id) && groups.insert(&g.id, g).is_none(),
            "duplicate_id",
            &g.id,
            "duplicate or invalid reveal ID",
        )?;
        require(
            !g.facts.is_empty() && !g.contributor_ids.is_empty(),
            "reveal_facts",
            &g.id,
            "each contributor needs fact evidence",
        )?;
        let Some(bound) = g.not_before_sample else {
            continue;
        };
        require(
            bound <= d.manifest.sample_count,
            "interval",
            &g.id,
            "reveal outside master",
        )?;
        for fact in &g.facts {
            require(
                fact.source_span[0] < fact.source_span[1]
                    && fact.source_span[1] <= d.source_text.chars().count() as u64,
                "source_span",
                &g.id,
                "invalid fact span",
            )?;
            let mut ends = Vec::new();
            for (id, kind) in [
                (&fact.image_attestation_id, EvidenceKind::ImageScope),
                (&fact.primary_anchor_id, EvidenceKind::PrimaryAnchor),
                (&fact.independent_anchor_id, EvidenceKind::IndependentAnchor),
            ] {
                let e = evidence.get(id);
                require(
                    e.is_some(),
                    "attestation_missing",
                    id,
                    "attestation does not resolve",
                )?;
                let e = e.unwrap();
                require(
                    e.kind == kind
                        && e.outcome == ReviewOutcome::Pass
                        && e.source_span == fact.source_span,
                    "attestation_scope",
                    id,
                    "kind, outcome or scope mismatch",
                )?;
                require(
                    e.source_sha256 == d.manifest.source_sha256
                        && e.narration_sha256 == d.read_along.narration_sha256
                        && e.alignment_sha256 == d.read_along.alignment_sha256
                        && e.presentation_map_sha256 == d.timeline.presentation_map_sha256
                        && e.delivery_audio_sha256 == d.manifest.entries.audio.sha256
                        && valid_digest(&e.recipe_sha256),
                    "attestation_hash",
                    id,
                    "stale or invalid evidence identity",
                )?;
                if kind == EvidenceKind::ImageScope {
                    let images: Vec<_> = d
                        .timeline
                        .scenes
                        .iter()
                        .flat_map(|s| &s.entry_snapshot.instances)
                        .filter(|i| g.contributor_ids.contains(&i.id))
                        .filter_map(|i| d.actors.iter().find(|a| a.id == i.asset_id))
                        .collect();
                    require(
                        !images.is_empty()
                            && images.iter().all(|a| a.image.sha256 == e.target_sha256),
                        "attestation_hash",
                        id,
                        "image evidence target differs",
                    )?;
                } else {
                    require(
                        e.target_sha256 == d.manifest.entries.audio.sha256,
                        "attestation_hash",
                        id,
                        "anchor target differs",
                    )?;
                    require(
                        e.terminal_word_ids.len() == 1,
                        "terminal_ambiguous",
                        id,
                        "anchor must identify exactly one terminal word occurrence",
                    )?;
                    let w = d
                        .read_along
                        .words
                        .iter()
                        .find(|w| w.id == e.terminal_word_ids[0]);
                    require(
                        w.is_some_and(|w| w.source_span[1] == fact.source_span[1]),
                        "terminal_ambiguous",
                        id,
                        "terminal occurrence does not end fact span",
                    )?;
                    let end = e.measured_end_sample;
                    require(
                        end.is_some_and(|n| n <= d.manifest.sample_count),
                        "anchor_sample",
                        id,
                        "missing or invalid measured bound",
                    )?;
                    if kind == EvidenceKind::PrimaryAnchor {
                        require(
                            end == Some(w.unwrap().end_sample),
                            "anchor_sample",
                            id,
                            "primary anchor differs from measured word",
                        )?;
                    }
                    ends.push(end.unwrap());
                }
            }
            require(
                ends[0].abs_diff(ends[1]) <= 36000,
                "anchor_disagreement",
                &g.id,
                "independent anchor exceeds 0.75 seconds",
            )?;
            require(
                bound >= *ends.iter().max().unwrap(),
                "reveal_early",
                &g.id,
                "reveal precedes later measured anchor",
            )?;
        }
    }
    for s in &d.timeline.scenes {
        for i in &s.entry_snapshot.instances {
            let g = groups.get(&i.reveal_group_id);
            require(
                g.is_some_and(|g| g.contributor_ids.contains(&i.id)),
                "reveal_reference",
                &i.id,
                "instance is not declared contributor",
            )?;
        }
    }
    Ok(())
}

fn unique<'a>(ids: impl Iterator<Item = &'a str>, field: &str) -> ContractResult<()> {
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        require(
            valid_id(id) && seen.insert(id),
            "duplicate_id",
            field,
            "duplicate or invalid ID",
        )?;
    }
    Ok(())
}
fn finite(v: &[f64]) -> bool {
    v.iter().all(|n| n.is_finite())
}
fn validate_lighting(l: &Lighting, field: &str) -> ContractResult<()> {
    let color = |s: &str| {
        s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|c| c.is_ascii_hexdigit())
    };
    require(
        color(&l.ambient.color) && l.ambient.intensity.is_finite() && l.ambient.intensity >= 0.0,
        "lighting",
        field,
        "invalid ambient target",
    )?;
    unique(l.lights.iter().map(|l| l.id.as_str()), field)?;
    for l in &l.lights {
        require(
            finite(&l.position)
                && finite(&l.target)
                && color(&l.color)
                && l.intensity.is_finite()
                && l.intensity >= 0.0
                && l.shadow.map_size > 0
                && l.shadow.map_size <= 2048
                && finite(&[
                    l.shadow.bias,
                    l.shadow.normal_bias,
                    l.shadow.near,
                    l.shadow.far,
                ])
                && l.shadow.near > 0.0
                && l.shadow.far > l.shadow.near,
            "lighting",
            &l.id,
            "invalid complete light/shadow target",
        )?;
        if let LightSource::Spot {
            angle_radians,
            penumbra,
            distance,
            decay,
        } = l.source
        {
            require(
                finite(&[angle_radians, penumbra, distance, decay])
                    && angle_radians > 0.0
                    && angle_radians <= std::f64::consts::FRAC_PI_2
                    && (0.0..=1.0).contains(&penumbra)
                    && distance >= 0.0
                    && decay >= 0.0,
                "lighting",
                &l.id,
                "invalid complete spot target",
            )?;
        }
    }
    Ok(())
}
pub fn validate_geometry_and_cues(d: &PlaybackDocument) -> ContractResult<()> {
    let t = &d.timeline;
    unique(d.actors.iter().map(|x| x.id.as_str()), "actors")?;
    unique(t.scenes.iter().map(|x| x.id.as_str()), "scenes")?;
    unique(t.cues.iter().map(|x| x.id.as_str()), "cues")?;
    for a in &d.actors {
        a.image.validate()?;
        let [w, h] = a.pixel_size;
        let [sw, sh] = a.stage_size;
        require(
            w > 0
                && h > 0
                && w <= 4096
                && h <= 4096
                && a.image.dimensions == Some(a.pixel_size)
                && finite(&a.stage_size)
                && sw > 0.0
                && sh > 0.0
                && (sw / sh - w as f64 / h as f64).abs() <= 1e-6
                && finite(&a.pivot_top_left)
                && a.pivot_top_left.iter().all(|v| (0.0..=1.0).contains(v))
                && finite(&a.crop_to_source)
                && (a.crop_to_source[0] * a.crop_to_source[3]
                    - a.crop_to_source[1] * a.crop_to_source[2])
                    .abs()
                    > 1e-12
                && a.body_height.is_finite()
                && a.body_height > 0.0
                && a.body_height <= sh,
            "registration",
            &a.id,
            "invalid registered dimensions, aspect, pivot or crop",
        )?;
        require(
            d.shell.supports.iter().any(|s| s.id == a.support_id),
            "support",
            &a.id,
            "unknown support",
        )?;
    }
    for snap in t
        .scenes
        .iter()
        .map(|s| &s.entry_snapshot)
        .chain([&t.terminal_snapshot])
    {
        unique(snap.instances.iter().map(|i| i.id.as_str()), &snap.set_id)?;
        let c = &snap.camera;
        require(
            finite(&c.position)
                && finite(&c.target)
                && finite(&c.up)
                && finite(&[c.fov_y_degrees, c.near, c.far])
                && c.fov_y_degrees > 0.0
                && c.fov_y_degrees < 180.0
                && c.near > 0.0
                && c.far > c.near
                && c.position != c.target
                && c.up.iter().map(|v| v * v).sum::<f64>() > 0.0
                && (0.0..=1.0).contains(&snap.curtain_open_ratio),
            "camera",
            &snap.set_id,
            "invalid snapshot camera/curtain",
        )?;
        validate_lighting(&snap.lighting, &snap.set_id)?;
        for i in &snap.instances {
            let a = d.actors.iter().find(|a| a.id == i.asset_id);
            require(
                a.is_some(),
                "asset_reference",
                &i.id,
                "unknown canonical card",
            )?;
            require(
                !i.mirrored || a.unwrap().allow_mirror,
                "mirror_permission",
                &i.id,
                "card cannot flip pixels",
            )?;
            require(
                finite(&i.position)
                    && finite(&i.rotation_xyz)
                    && i.support_id == a.unwrap().support_id,
                "registration",
                &i.id,
                "invalid instance geometry or support",
            )?;
        }
    }
    for cue in &t.cues {
        interval(
            [cue.start_sample, cue.end_sample],
            t.sample_count,
            &cue.id,
            false,
        )?;
        let scene = t.scenes.iter().find(|s| s.id == cue.scene_id);
        require(
            scene.is_some(),
            "scene_reference",
            &cue.id,
            "unknown cue scene",
        )?;
        let scene = scene.unwrap();
        for id in &cue.reveal_group_ids {
            let g = d.manifest.reveals.iter().find(|g| &g.id == id);
            require(
                g.is_some_and(|g| g.not_before_sample.is_some_and(|n| cue.start_sample >= n)),
                "reveal_early",
                &cue.id,
                "cue lacks eligible reveal",
            )?;
        }
        require(
            cue.start_sample >= scene.narration[0] && cue.end_sample <= scene.narration[1],
            "cue_scene",
            &cue.id,
            "cue outside authored scene slot",
        )?;
        match &cue.action {
            CueAction::Gesture { intensity, .. } => require(
                intensity.is_finite() && (0.0..=1.0).contains(intensity),
                "gesture",
                &cue.id,
                "intensity must be 0..1",
            )?,
            CueAction::Move { target, .. } | CueAction::Enter { target, .. } => {
                require(finite(target), "geometry", &cue.id, "invalid target")?
            }
            CueAction::Camera {
                position, target, ..
            } => require(
                finite(position) && finite(target) && position != target,
                "camera",
                &cue.id,
                "invalid camera target",
            )?,
            CueAction::Lighting { lighting } => validate_lighting(lighting, &cue.id)?,
            CueAction::Face { .. } => {
                let a = scene
                    .entry_snapshot
                    .instances
                    .iter()
                    .find(|i| i.id == cue.target_id)
                    .and_then(|i| d.actors.iter().find(|a| a.id == i.asset_id));
                require(
                    a.is_some_and(|a| a.allow_mirror),
                    "mirror_permission",
                    &cue.id,
                    "face cue requires pixel mirroring permission",
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}
