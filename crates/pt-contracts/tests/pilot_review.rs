#[path = "support/acceptance_fixture.rs"]
mod fixture;
use pt_contracts::*;
use serde_json::{Value, json};
fn artifact(file: FileRef) -> ArtifactRef {
    ArtifactRef {
        id: format!("recorded:{}", file.sha256),
        file,
        provenance: Provenance::Synthetic,
    }
}
fn write(f: &fixture::Fixture, path: &str, v: &Value) -> FileRef {
    fixture::put(
        &f.private,
        path,
        &serde_json::to_vec(v).unwrap(),
        "application/json",
    )
}
fn wav(seed: u8) -> Vec<u8> {
    let mut b = b"RIFF".to_vec();
    b.extend(44u32.to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(24000u32.to_le_bytes());
    b.extend(48000u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(8u32.to_le_bytes());
    b.extend([seed; 8]);
    b
}
fn evidence() -> (fixture::Fixture, PilotReviewPolicyV1) {
    evidence_audio(|_, _| {})
}
fn evidence_audio(
    mut edit: impl FnMut(u64, &mut Vec<u8>),
) -> (fixture::Fixture, PilotReviewPolicyV1) {
    let mut f = fixture::Fixture::new();
    f.doc
        .acceptance
        .required_check_ids
        .push("pilot-recording-review".into());
    f.save();
    let source = f
        .policy
        .inputs
        .iter()
        .find(|i| i.role == AcceptanceInputRole::Source)
        .unwrap()
        .file
        .clone();
    let recipe = f
        .policy
        .inputs
        .iter()
        .find(|i| i.role == AcceptanceInputRole::Recipe)
        .unwrap()
        .file
        .clone();
    let direction = write(
        &f,
        "pilot/direction.json",
        &json!({"diagnostic":"direction"}),
    );
    let registry = write(&f, "pilot/registry.json", &json!({"diagnostic":"registry"}));
    let catalog = write(&f, "pilot/catalog.json", &json!({"diagnostic":"catalog"}));
    let mut takes = vec![];
    let mut plan_takes = vec![];
    let chars = f.doc.source_text.chars().collect::<Vec<_>>();
    for i in 0..2u64 {
        let span = [i * 10, (i + 1) * 10];
        let passage = json!({"turn_id":"narrator","span":span,"text":chars[span[0] as usize..span[1] as usize].iter().collect::<String>(),"acting":"","acting_support":null});
        let req = json!({"schema_version":1,"task":"voice-pilot-take-v1","source":source,"source_sha256":source.sha256,"direction":direction,"registry":registry,"catalog":catalog,"recipe":recipe,"speaker":"narrator","canonical_id":"narrator","model":"diagnostic-tts","voice_id":"n","passage":passage,"production_approved":false});
        let request = artifact(write(&f, &format!("pilot/request-{i}.json"), &req));
        let mut pcm = wav(i as u8);
        edit(i, &mut pcm);
        let audio = artifact(fixture::put(
            &f.private,
            &format!("pilot/take-{i}.wav"),
            &pcm,
            "audio/wav",
        ));
        let response = artifact(write(
            &f,
            &format!("pilot/raw-{i}.json"),
            &json!({"schema_version":1,"provenance":"synthetic","request_sha256":request.file.sha256,"recipe_sha256":recipe.sha256,"source_sha256":source.sha256,"direction_sha256":direction.sha256,"catalog_sha256":catalog.sha256,"speaker":"narrator","model":"diagnostic-tts","voice_id":"n","passage":passage,"audio":audio,"format":{"sample_rate":24000,"channels":1,"bits_per_sample":16,"frames":4}}),
        ));
        let observation = write(
            &f,
            &format!("pilot/observation-{i}.json"),
            &json!({"schema_version":1,"provenance":"synthetic","request":request,"response":response,"issued":null,"result":null,"findings":[]}),
        );
        plan_takes.push(json!({"request":request,"response":response,"observation":observation,"audio":audio,"production_approved":false}));
        takes.push(json!({"request":request,"response":response,"observation":observation,"audio":audio,"source_span":span}));
    }
    let reference_audio = artifact(fixture::put(
        &f.private,
        "pilot/reference.wav",
        &wav(3),
        "audio/wav",
    ));
    let assembled_audio = artifact(fixture::put(
        &f.private,
        "pilot/assembled.wav",
        &wav(4),
        "audio/wav",
    ));
    let join_recipe = write(&f, "pilot/join.json", &json!({"diagnostic":"join recipe"}));
    let music_context = write(
        &f,
        "pilot/music.json",
        &json!({"diagnostic":"music context"}),
    );
    let criteria = [
        "listening",
        "identity",
        "continuity",
        "intelligibility",
        "speech-fidelity",
        "room-tone",
        "safe-joins",
        "music-context",
    ];
    let request = artifact(write(
        &f,
        "pilot/review-request.json",
        &json!({"schema_version":1,"task":"independent-voice-pilot-review-v1","source":source,"source_sha256":source.sha256,"direction":direction,"registry":registry,"catalog":catalog,"recipe":recipe,"speaker":"narrator","takes":plan_takes,"reference_audio":reference_audio,"assembled_audio":assembled_audio,"join_recipe":join_recipe,"music_context":music_context,"criteria":criteria,"production_approved":false}),
    ));
    let targets = takes.iter().map(|t| t["audio"].clone()).collect::<Vec<_>>();
    let response = artifact(write(
        &f,
        "pilot/review-raw.json",
        &json!({"schema_version":1,"provenance":"synthetic","request_sha256":request.file.sha256,"recipe_sha256":recipe.sha256,"speaker":"narrator","targets":targets,"reference_audio":reference_audio,"criteria":criteria.map(|id|json!({"id":id,"outcome":"pass","message":"Synthetic mechanics only; no listening approval"}))}),
    ));
    let observation = write(
        &f,
        "pilot/review-observation.json",
        &json!({"schema_version":1,"provenance":"synthetic","request":request,"response":response,"issued":null,"result":null,"findings":[]}),
    );
    let plan = write(
        &f,
        "pilot/plan.json",
        &json!({"schema_version":1,"takes":plan_takes,"reviews":[{"request":request,"response":response,"targets":targets,"production_approved":false}],"bulk_speech_approved":false,"production_approved":false}),
    );
    let p: PilotReviewPolicyV1=serde_json::from_value(json!({"schema_version":1,"plan":plan,"source":source,"source_record":source,"direction":direction,"registry":registry,"catalog":catalog,"recipe":recipe,"performers":[{"speaker":"narrator","canonical_id":"narrator","model":"diagnostic-tts","voice_id":"n","takes":takes,"review":{"request":request,"response":response,"observation":observation,"reviewer":{"schema_version":2,"run_id":"diagnostic-review","epoch":1,"unit_id":"review"},"provider":"diagnostic","model":"diagnostic-reviewer","reference_audio":reference_audio,"assembled_audio":assembled_audio,"join_recipe":join_recipe,"music_context":music_context}}]})).unwrap();
    (f, p)
}
#[test]
fn verified_recorded_mechanics_do_not_authenticate_listening_or_grant_a_pilot_pass() {
    let (f, p) = evidence();
    let v = verify_acceptance_with_pilot(&f.doc, &f.root, &f.policy, &p, &[], &f.private).unwrap();
    let r = v.report();
    let p = r.pilot_review.as_ref().unwrap();
    assert_eq!(p.binding_outcome, ReviewOutcome::Pass);
    assert_eq!(p.review_outcome, ReviewOutcome::Uncertain);
    assert_eq!(p.recording_count, 2);
    assert_eq!(p.review_count, 1);
    assert!(!p.production_review_executed);
    assert!(r.blockers.contains(&"pilot-recording-review".into()));
    assert!(r.blockers.contains(&"production-verifier-pending".into()));
    assert!(!r.production_approved);
    v.recheck_inputs().unwrap();
    r.validate(&acceptance_binding_checker()).unwrap();
}
fn report(f: &fixture::Fixture, p: &PilotReviewPolicyV1) -> AcceptanceVerificationReport {
    verify_acceptance_with_pilot(&f.doc, &f.root, &f.policy, p, &[], &f.private)
        .unwrap()
        .report()
        .clone()
}
fn change(f: &fixture::Fixture, file: &mut FileRef, edit: impl FnOnce(&mut Value)) {
    let mut v: Value =
        serde_json::from_slice(&std::fs::read(f.private.join(&file.path)).unwrap()).unwrap();
    edit(&mut v);
    *file = write(f, &file.path, &v);
}
#[test]
fn original_plan_adverse_findings_cannot_be_erased_by_happy_recorded_criteria() {
    let (f, mut p) = evidence();
    change(
        &f,
        &mut p.plan,
        |v| v["findings"] = json!([{"id":"original-pilot-failure","field":"pilot","outcome":"fail","message":"retained original","evidence":[]} ]),
    );
    let r = report(&f, &p);
    assert_eq!(r.pilot_review.unwrap().review_outcome, ReviewOutcome::Fail);
}
#[test]
fn private_pilot_evidence_cannot_alias_a_public_package_entry() {
    let (mut f, p) = evidence();
    f.doc.manifest.assets.push(
        p.performers[0]
            .review
            .as_ref()
            .unwrap()
            .response
            .file
            .clone(),
    );
    f.save();
    let r = report(&f, &p);
    assert!(
        r.pilot_review
            .unwrap()
            .reasons
            .contains(&AcceptanceReason::PrivateEvidenceExported)
    );
}
#[test]
fn structure_is_distinct_from_a_completed_recorded_review() {
    let (f, p) = evidence();
    let mut plan: Value =
        serde_json::from_slice(&std::fs::read(f.private.join(&p.plan.path)).unwrap()).unwrap();
    plan["reviews"] = json!([]);
    let structure = validate_pilot_plan_structure(&plan).unwrap();
    assert_eq!(structure.take_count, 2);
    assert_eq!(structure.review_count, 0);
}
#[test]
fn missing_targets_do_not_erase_a_bound_original_plan_failure() {
    let (f, mut p) = evidence();
    change(&f, &mut p.plan, |v| {
        v["reviews"] = json!([]);
        v["findings"] = json!([{"id":"retained","field":"pilot","outcome":"fail","message":"original adverse evidence","evidence":[]}]);
    });
    let r = report(&f, &p);
    assert_eq!(
        r.pilot_review.as_ref().unwrap().review_outcome,
        ReviewOutcome::Fail
    );
    assert_eq!(
        r.checks
            .iter()
            .find(|c| c.check_id == "pilot-recording-review")
            .unwrap()
            .outcome,
        ReviewOutcome::Fail
    );
}
#[test]
fn serialized_report_cannot_downgrade_an_independently_consumed_pilot_failure() {
    let (f, mut p) = evidence();
    change(
        &f,
        &mut p.plan,
        |v| v["findings"] = json!([{"id":"original","field":"pilot","outcome":"fail","message":"retained","evidence":[]} ]),
    );
    let mut r = report(&f, &p);
    r.checks
        .iter_mut()
        .find(|c| c.check_id == "pilot-recording-review")
        .unwrap()
        .outcome = ReviewOutcome::Missing;
    assert!(r.validate(&acceptance_binding_checker()).is_err());
}

fn alter_review(f: &fixture::Fixture, p: &mut PilotReviewPolicyV1, edit: impl FnOnce(&mut Value)) {
    let r = p.performers[0].review.as_mut().unwrap();
    change(f, &mut r.response.file, edit);
    change(f, &mut r.observation, |v| v["response"] = json!(r.response));
    let response = r.response.clone();
    change(f, &mut p.plan, |v| {
        v["reviews"][0]["response"] = json!(response)
    });
}
#[test]
fn missing_swapped_malformed_and_wrong_frame_recordings_stay_blocked() {
    let (f, p) = evidence();
    std::fs::remove_file(f.private.join(&p.performers[0].takes[0].audio.file.path)).unwrap();
    let r = report(&f, &p);
    assert_eq!(
        r.pilot_review.unwrap().binding_outcome,
        ReviewOutcome::Missing
    );
    let (f, p) = evidence();
    std::fs::write(
        f.private.join(&p.performers[0].takes[0].audio.file.path),
        wav(99),
    )
    .unwrap();
    let r = report(&f, &p);
    assert!(
        r.pilot_review
            .unwrap()
            .reasons
            .contains(&AcceptanceReason::ChangedBytes)
    );
    for malformed in [true, false] {
        let (f, p) = evidence_audio(|i, b| {
            if i == 0 {
                if malformed {
                    b[20] = 3;
                } else {
                    b[4..8].copy_from_slice(&46u32.to_le_bytes());
                    b[40..44].copy_from_slice(&10u32.to_le_bytes());
                    b.extend([0, 0]);
                }
            }
        });
        let r = report(&f, &p);
        assert_eq!(
            r.pilot_review.as_ref().unwrap().binding_outcome,
            ReviewOutcome::Fail
        );
        assert!(r.pilot_review.unwrap().reasons.contains(&if malformed {
            AcceptanceReason::InvalidAudioFormat
        } else {
            AcceptanceReason::InvalidPilotEvidence
        }));
    }
}
#[test]
fn omitted_adjacent_take_and_missing_reference_join_or_music_remain_unresolved() {
    let (f, mut p) = evidence();
    p.performers[0].takes.pop();
    assert!(p.validate().is_err());
    let (f, mut p) = evidence();
    change(&f, &mut p.plan, |v| {
        v["takes"].as_array_mut().unwrap().pop();
    });
    assert_eq!(
        report(&f, &p).pilot_review.unwrap().binding_outcome,
        ReviewOutcome::Missing
    );
    for missing in 0..4 {
        let (f, mut p) = evidence();
        let r = p.performers[0].review.as_mut().unwrap();
        match missing {
            0 => r.reference_audio = None,
            1 => r.assembled_audio = None,
            2 => r.join_recipe = None,
            _ => r.music_context = None,
        };
        assert_eq!(
            report(&f, &p).pilot_review.unwrap().binding_outcome,
            ReviewOutcome::Missing
        );
    }
}
#[test]
fn independent_expected_source_voice_model_and_recipe_bindings_cannot_drift() {
    for field in 0..5 {
        let (f, mut p) = evidence();
        match field {
            0 => p.performers[0].voice_id = "other-voice".into(),
            1 => p.performers[0].model = "other-model".into(),
            2 => p.performers[0].canonical_id = "other-person".into(),
            3 => p.recipe = write(&f, "pilot/changed-recipe.json", &json!({"changed":true})),
            _ => {
                p.source = fixture::put(
                    &f.private,
                    "pilot/other-source.txt",
                    b"other source",
                    "text/plain",
                )
            }
        };
        let r = report(&f, &p);
        assert_eq!(r.pilot_review.unwrap().binding_outcome, ReviewOutcome::Fail);
    }
}
#[test]
fn partial_duplicate_and_stale_review_targets_or_criteria_fail_closed() {
    for field in 0..5 {
        let (f, mut p) = evidence();
        alter_review(&f, &mut p, |v| match field {
            0 => {
                v["targets"].as_array_mut().unwrap().pop();
            }
            1 => v["targets"][1] = v["targets"][0].clone(),
            2 => {
                v["criteria"].as_array_mut().unwrap().pop();
            }
            3 => v["criteria"][1] = v["criteria"][0].clone(),
            _ => v["request_sha256"] = json!("0".repeat(64)),
        });
        let r = report(&f, &p);
        assert_eq!(r.pilot_review.unwrap().binding_outcome, ReviewOutcome::Fail);
    }
}
#[test]
fn recorded_missing_uncertain_and_fail_criteria_retain_their_original_outcomes() {
    for outcome in [
        ReviewOutcome::Missing,
        ReviewOutcome::Uncertain,
        ReviewOutcome::Fail,
    ] {
        let (f, mut p) = evidence();
        alter_review(&f, &mut p, |v| v["criteria"][0]["outcome"] = json!(outcome));
        let r = report(&f, &p);
        let c = &r.pilot_review.as_ref().unwrap().criteria[0];
        assert_eq!(c.claimed_outcome, outcome);
        assert_eq!(c.effective_outcome, outcome);
        assert_ne!(
            r.checks
                .iter()
                .find(|c| c.check_id == "pilot-recording-review")
                .unwrap()
                .outcome,
            ReviewOutcome::Pass
        );
    }
}
#[test]
fn changed_recordings_after_inspection_fail_immutable_recheck() {
    let (f, p) = evidence();
    let v = verify_acceptance_with_pilot(&f.doc, &f.root, &f.policy, &p, &[], &f.private).unwrap();
    std::fs::write(
        f.private.join(&p.performers[0].takes[0].audio.file.path),
        wav(99),
    )
    .unwrap();
    assert!(v.recheck_inputs().is_err());
}
#[test]
fn policy_bounds_unsafe_paths_and_provenance_flags_never_supply_approval() {
    let (f, mut p) = evidence();
    p.performers[0].takes[0].audio.file.path = "../outside.wav".into();
    assert!(p.validate().is_err());
    let (f, mut p) = evidence();
    p.performers[0].takes[0].audio.file.bytes = 8 * 1024 * 1024 + 1;
    assert!(p.validate().is_err());
    let (f, mut p) = evidence();
    let r = p.performers[0].review.as_mut().unwrap();
    change(&f, &mut r.observation, |v| {
        v["provenance"] = json!("autonomous")
    });
    assert_eq!(
        report(&f, &p).pilot_review.unwrap().binding_outcome,
        ReviewOutcome::Fail
    );
}

pub fn cross_language_cases() -> Value {
    let (f, p) = evidence();
    let r = report(&f, &p);
    let plan: Value =
        serde_json::from_slice(&std::fs::read(f.private.join(&p.plan.path)).unwrap()).unwrap();
    let structure = validate_pilot_plan_structure(&plan).unwrap();
    let mut rows = vec![
        json!({"id":"bound-private-policy","kind":"policy","valid":true,"value":p}),
        json!({"id":"recorded-mechanics-only","kind":"observation","valid":true,"value":r.pilot_review}),
        json!({"id":"plan-structure-only","kind":"structure","valid":true,"value":structure}),
        json!({"id":"private-report-review-pending","kind":"report","valid":true,"value":r}),
    ];
    for (id, kind, path, value) in [
        ("unsupported-policy", "policy", "/schema_version", json!(2)),
        (
            "single-take-policy",
            "policy",
            "/performers/0/takes",
            json!([p.performers[0].takes[0]]),
        ),
        (
            "unsafe-recording",
            "policy",
            "/performers/0/takes/0/audio/file/path",
            json!("../escape.wav"),
        ),
        (
            "oversize-recording",
            "policy",
            "/performers/0/takes/0/audio/file/bytes",
            json!(8 * 1024 * 1024 + 1),
        ),
        (
            "promoted-listening",
            "observation",
            "/review_outcome",
            json!("pass"),
        ),
        (
            "claimed-review-execution",
            "observation",
            "/production_review_executed",
            json!(true),
        ),
        ("dropped-criteria", "observation", "/criteria", json!([])),
        (
            "promoted-criterion",
            "observation",
            "/criteria/0/effective_outcome",
            json!("pass"),
        ),
        (
            "unknown-reason",
            "observation",
            "/reasons",
            json!(["unknown"]),
        ),
        (
            "structural-grant",
            "structure",
            "/bulk_speech_approved",
            json!(true),
        ),
        (
            "forged-report-review",
            "report",
            "/pilot_review/review_outcome",
            json!("pass"),
        ),
        (
            "forged-policy-consumption",
            "report",
            "/pilot_review/policy_sha256",
            json!("0".repeat(64)),
        ),
        (
            "stale-source",
            "report",
            "/pilot_review/source_sha256",
            json!("0".repeat(64)),
        ),
        (
            "fake-executed-row",
            "report",
            "/checks/0/executed",
            json!(!r.checks[0].executed),
        ),
    ] {
        let mut v = rows.iter().find(|v| v["kind"] == kind).unwrap()["value"].clone();
        *v.pointer_mut(path).unwrap() = value;
        rows.push(json!({"id":id,"kind":kind,"valid":false,"value":v}));
    }
    let mut v = rows[0]["value"].clone();
    v["unexpected"] = json!(true);
    rows.push(json!({"id":"unknown-policy-field","kind":"policy","valid":false,"value":v}));
    let mut v = rows[0]["value"].clone();
    v["source"]["unexpected"] = json!(true);
    rows.push(json!({"id":"unknown-file-field","kind":"policy","valid":false,"value":v}));
    let mut v = rows[1]["value"].clone();
    v["criteria"][1] = v["criteria"][0].clone();
    rows.push(json!({"id":"duplicate-criteria","kind":"observation","valid":false,"value":v}));
    let mut v = rows[3]["value"].clone();
    v["pilot_review"]["binding_outcome"] = json!("fail");
    rows.push(json!({"id":"downgraded-report-failure","kind":"report","valid":false,"value":v}));
    for outcome in [
        ReviewOutcome::Missing,
        ReviewOutcome::Uncertain,
        ReviewOutcome::Fail,
    ] {
        let mut v = rows[1]["value"].clone();
        v["criteria"][0]["claimed_outcome"] = json!(outcome);
        v["criteria"][0]["effective_outcome"] = json!(outcome);
        if outcome == ReviewOutcome::Fail {
            v["review_outcome"] = json!("fail");
        }
        rows.push(json!({"id":format!("retained-{:?}",outcome),"kind":"observation","valid":true,"value":v}));
    }
    for c in &mut rows {
        if c["kind"] == "report" {
            c["expected_checker"] = json!(r.checker);
        }
    }
    let archive: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/contracts/acceptance-report-v4.json"
    ))
    .unwrap();
    rows.push(json!({"id":"genuine-baseline-report-v4","kind":"report","valid":true,"value":archive,"expected_checker":archive["checker"]}));
    let mut forged = archive.clone();
    forged["pilot_review"] = rows[1]["value"].clone();
    rows.push(json!({"id":"old-report-cannot-acquire-pilot-evidence","kind":"report","valid":false,"value":forged,"expected_checker":archive["checker"]}));
    let (f, mut p) = evidence();
    alter_review(&f, &mut p, |v| v["criteria"][0]["outcome"] = json!("fail"));
    std::fs::remove_file(f.private.join(&p.performers[0].takes[0].audio.file.path)).unwrap();
    let adverse = report(&f, &p);
    rows.push(json!({"id":"retained-review-fail-with-missing-recording","kind":"observation","valid":true,"value":adverse.pilot_review}));
    rows.push(json!({"id":"retained-review-fail-report","kind":"report","valid":true,"value":adverse,"expected_checker":adverse.checker}));
    json!(rows)
}
#[test]
fn cross_language_records_match_native_validation() {
    let cases = cross_language_cases();
    for c in cases.as_array().unwrap() {
        let v = c["value"].clone();
        let valid = match c["kind"].as_str().unwrap() {
            "policy" => {
                serde_json::from_value::<PilotReviewPolicyV1>(v).is_ok_and(|p| p.validate().is_ok())
            }
            "observation" => serde_json::from_value::<PilotReviewObservationV1>(v)
                .is_ok_and(|p| p.validate().is_ok()),
            "structure" => serde_json::from_value::<PilotPlanStructureV1>(v)
                .is_ok_and(|p| p.validate().is_ok()),
            "report" => serde_json::from_value::<AcceptanceVerificationReport>(v).is_ok_and(|p| {
                p.validate(
                    &c.get("expected_checker")
                        .map(|v| serde_json::from_value(v.clone()).unwrap())
                        .unwrap_or_else(acceptance_binding_checker),
                )
                .is_ok()
            }),
            _ => false,
        };
        assert_eq!(valid, c["valid"].as_bool().unwrap(), "{}", c["id"]);
    }
}
fn attach_review_issuance(
    f: &fixture::Fixture,
    p: &mut PilotReviewPolicyV1,
    recipe: Digest,
    omit_inputs: bool,
) {
    let template: Vec<Value> = serde_json::from_slice(include_bytes!(
        "../../../fixtures/contracts/provider-metadata-v2.json"
    ))
    .unwrap();
    let r = p.performers[0].review.as_ref().unwrap();
    let mut issued = template
        .iter()
        .find(|v| v["id"] == "free-money-request")
        .unwrap()["wire"]
        .clone();
    issued["operation"]["kind"] = json!("review");
    issued["operation"]["provider"] = json!(r.provider);
    issued["operation"]["model"] = json!(r.model);
    issued["operation"]["recipe_sha256"] = json!(recipe);
    issued["operation"]["payload"] = json!(r.request);
    issued["operation"]["request_sha256"] = json!(r.request.file.sha256);
    issued["operation"]["unit_id"] = json!(r.reviewer.unit_id);
    issued["identity"]["run_id"] = json!(r.reviewer.run_id);
    issued["identity"]["epoch"] = json!(r.reviewer.epoch);
    issued["context"]["provider"] = json!(r.provider);
    issued["quota_windows"][0]["scope"]["provider"] = json!(r.provider);
    let mut inputs = vec![];
    for file in [
        &p.source_record,
        &p.direction,
        &p.registry,
        &p.catalog,
        &p.recipe,
    ] {
        inputs.push(artifact(file.clone()));
    }
    for t in &p.performers[0].takes {
        inputs.extend([
            t.request.clone(),
            t.response.clone(),
            t.audio.clone(),
            artifact(t.observation.clone()),
        ]);
    }
    inputs.extend([
        r.reference_audio.clone().unwrap(),
        r.assembled_audio.clone().unwrap(),
        artifact(r.join_recipe.clone().unwrap()),
        artifact(r.music_context.clone().unwrap()),
    ]);
    if omit_inputs {
        inputs.retain(|a| a.file != p.performers[0].takes[1].audio.file);
    }
    issued["operation"]["inputs"] = json!(inputs);
    let mut result = template
        .iter()
        .find(|v| v["id"] == "partial-observed-usage")
        .unwrap()["wire"]
        .clone();
    result["identity"] = issued["identity"].clone();
    result["context"] = issued["context"].clone();
    result["quota_windows"][0]["scope"] = issued["quota_windows"][0]["scope"].clone();
    result["result"]["completion"] = json!("completed");
    result["result"]["request_sha256"] = issued["operation"]["request_sha256"].clone();
    result["result"]["raw_response"] = json!(r.response);
    result["result"]["response_sha256"] = json!(r.response.file.sha256);
    let i: ProviderOperationV2 = serde_json::from_value(issued.clone()).unwrap();
    i.validate().unwrap();
    let reply: ProviderResultV2 = serde_json::from_value(result.clone()).unwrap();
    reply.validate_for_operation(&i).unwrap();
    change(
        f,
        &mut p.performers[0].review.as_mut().unwrap().observation,
        |v| {
            v["issued"] = issued;
            v["result"] = result;
        },
    );
}
#[test]
fn paired_provider_envelopes_must_match_the_independently_expected_review_recipe_and_inputs() {
    for bad_recipe in [true, false] {
        let (f, mut p) = evidence();
        let recipe = if bad_recipe {
            "a".repeat(64)
        } else {
            p.recipe.sha256.clone()
        };
        attach_review_issuance(&f, &mut p, recipe, !bad_recipe);
        assert_eq!(
            report(&f, &p).pilot_review.unwrap().binding_outcome,
            ReviewOutcome::Fail
        );
    }
}

#[test]
fn internally_consistent_provider_metadata_still_does_not_authenticate_listening() {
    let (f, mut p) = evidence();
    let recipe = p.recipe.sha256.clone();
    attach_review_issuance(&f, &mut p, recipe, false);
    let r = report(&f, &p);
    assert_eq!(
        r.pilot_review.as_ref().unwrap().binding_outcome,
        ReviewOutcome::Pass
    );
    assert_eq!(
        r.pilot_review.unwrap().review_outcome,
        ReviewOutcome::Uncertain
    );
}
#[test]
fn missing_recordings_or_context_cannot_hide_a_bound_recorded_review_failure() {
    for missing in 0..3 {
        let (f, mut p) = evidence();
        alter_review(&f, &mut p, |v| v["criteria"][0]["outcome"] = json!("fail"));
        match missing {
            0 => {
                std::fs::remove_file(f.private.join(&p.performers[0].takes[0].audio.file.path))
                    .unwrap();
            }
            1 => p.performers[0].review.as_mut().unwrap().reference_audio = None,
            _ => {
                std::fs::remove_file(f.private.join(&p.source.path)).unwrap();
            }
        }
        let r = report(&f, &p);
        assert_eq!(
            r.pilot_review.as_ref().unwrap().review_outcome,
            ReviewOutcome::Fail
        );
        assert_eq!(
            r.checks
                .iter()
                .find(|c| c.check_id == "pilot-recording-review")
                .unwrap()
                .outcome,
            ReviewOutcome::Fail
        );
    }
}
#[test]
fn pilot_policy_requires_an_explicit_report_row_even_if_the_candidate_omits_the_check() {
    let (mut f, p) = evidence();
    f.doc
        .acceptance
        .required_check_ids
        .retain(|id| id != "pilot-recording-review");
    f.save();
    let r = report(&f, &p);
    assert!(r.validate(&acceptance_binding_checker()).is_ok());
    let row = r
        .checks
        .iter()
        .find(|c| c.check_id == "pilot-recording-review")
        .unwrap();
    assert_ne!(row.outcome, ReviewOutcome::Pass);
    assert!(row.reasons.contains(&AcceptanceReason::MissingDeclaration));
}
