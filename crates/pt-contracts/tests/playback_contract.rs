use pt_contracts::*;
#[test]
fn dark_hold_is_exactly_96000_samples() {
    let mut t = SceneTransition {
        from_scene_id: "a".into(),
        to_scene_id: "b".into(),
        fade_out_start: 384000,
        swap_sample: 432000,
        fade_in_start: 528000,
        fade_in_end: 576000,
    };
    validate_transition(&t, 864000).unwrap();
    t.fade_in_start -= 1;
    let error = validate_transition(&t, 864000).unwrap_err();
    assert_eq!(error.code, "dark_hold");
    assert!(error.field.contains("fade_in_start"));
}
#[test]
fn imported_fixture_is_structurally_valid_but_not_production_accepted() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/shared/two-scene-v1");
    let d: PlaybackDocument =
        serde_json::from_slice(&std::fs::read(root.join("fixture.json")).unwrap()).unwrap();
    validate_playback(&d).unwrap();
    verify_package_files(&d, &root).unwrap();
    assert_eq!(d.read_along.words.len(), 176);
    assert_eq!(d.read_along.groups.len(), 170);
    assert!(!production_blockers(&d).is_empty());
}
fn fixture() -> PlaybackDocument {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/shared/two-scene-v1/fixture.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn readalong_preserves_scalar_spans_points_and_complete_word_coverage() {
    let good = fixture();
    let mut bad = good.clone();
    bad.read_along.words[0].source_span[1] += 1;
    assert_eq!(validate_playback(&bad).unwrap_err().code, "source_span");
    let mut bad = good.clone();
    bad.read_along.words[0].end_sample = MAX_SAFE_INTEGER + 1;
    assert_eq!(validate_playback(&bad).unwrap_err().code, "interval");
    let mut bad = good.clone();
    bad.read_along.groups.pop();
    assert_eq!(validate_playback(&bad).unwrap_err().code, "word_coverage");
    let mut bad = good.clone();
    let point = bad
        .read_along
        .words
        .iter_mut()
        .find(|w| w.start_sample == w.end_sample)
        .unwrap();
    point.start_sample -= 1;
    point.end_sample -= 1;
    assert_eq!(validate_playback(&bad).unwrap_err().code, "point_policy");
}
#[test]
fn reveal_evidence_is_hash_bound_unique_and_conservative() {
    let good = fixture();
    let mut bad = good.clone();
    bad.acceptance.attestations.remove(0);
    assert_eq!(
        validate_playback(&bad).unwrap_err().code,
        "attestation_missing"
    );
    let mut bad = good.clone();
    bad.acceptance.attestations[1].delivery_audio_sha256 = "0".repeat(64);
    assert_eq!(
        validate_playback(&bad).unwrap_err().code,
        "attestation_hash"
    );
    let mut bad = good.clone();
    let terminal = bad.acceptance.attestations[1].terminal_word_ids[0].clone();
    bad.acceptance.attestations[1]
        .terminal_word_ids
        .push(terminal);
    assert_eq!(
        validate_playback(&bad).unwrap_err().code,
        "terminal_ambiguous"
    );
    let mut bad = good.clone();
    bad.manifest.reveals[0].not_before_sample = Some(0);
    assert_eq!(validate_playback(&bad).unwrap_err().code, "reveal_early");
    let mut unknown = good.clone();
    unknown.manifest.reveals[0].not_before_sample = None;
    unknown.acceptance.attestations.remove(0);
    validate_playback(&unknown).unwrap();
    assert!(!reveal_eligible(&unknown.manifest.reveals[0], u64::MAX));
}
#[test]
fn registration_and_authored_cues_are_validated_before_playback() {
    let good = fixture();
    let mut bad = good.clone();
    bad.actors[0].pivot_top_left[0] = -0.1;
    assert_eq!(validate_playback(&bad).unwrap_err().code, "registration");
    let mut bad = good.clone();
    bad.timeline.scenes[0].entry_snapshot.instances[0].mirrored = true;
    assert_eq!(
        validate_playback(&bad).unwrap_err().code,
        "mirror_permission"
    );
    let mut bad = good.clone();
    bad.timeline.cues[0].end_sample = good.manifest.sample_count + 1;
    assert_eq!(validate_playback(&bad).unwrap_err().code, "interval");
    let mut wire = serde_json::to_value(good).unwrap();
    wire["timeline"]["cues"][0]["action"] = "hop".into();
    assert!(serde_json::from_value::<PlaybackDocument>(wire).is_err());
}
