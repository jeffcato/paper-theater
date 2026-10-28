#[path = "support/acceptance_fixture.rs"]
mod fixture;
use pt_contracts::*;
fn setup(scope: AcceptanceScope) -> fixture::Fixture {
    let mut f = fixture::Fixture::new();
    f.policy.scope = scope;
    f.doc
        .acceptance
        .required_check_ids
        .push("complete-source-coverage".into());
    f.doc.acceptance.required_check_ids.sort();
    f.doc.acceptance.required_check_ids.dedup();
    // Preserve canonical diagnostic bytes; this temporary derivative excludes the
    // historical raw-source asset from its explicit public payload allowlist.
    f.doc
        .manifest
        .assets
        .retain(|a| a.sha256 != f.doc.manifest.source_sha256);
    f.policy
        .asset_sha256s
        .retain(|h| h != &f.doc.manifest.source_sha256);
    freeze(&mut f);
    f
}
fn freeze(f: &mut fixture::Fixture) {
    let p = f.doc.manifest.entries.timeline.path.clone();
    f.doc.manifest.entries.timeline = fixture::put(
        &f.root,
        &p,
        &serde_json::to_vec(&f.doc.timeline).unwrap(),
        "application/json",
    );
    let p = f.doc.manifest.entries.read_along.path.clone();
    f.doc.manifest.entries.read_along = fixture::put(
        &f.root,
        &p,
        &serde_json::to_vec(&f.doc.read_along).unwrap(),
        "application/json",
    );
    let mut inventory = f
        .doc
        .manifest
        .entries
        .files()
        .into_iter()
        .chain(f.doc.manifest.assets.iter())
        .cloned()
        .collect::<Vec<_>>();
    inventory.sort_by(|a, b| a.path.cmp(&b.path));
    let file = fixture::put(
        &f.root,
        "inventory.json",
        &serde_json::to_vec(&inventory).unwrap(),
        "application/json",
    );
    f.doc.acceptance.inventory_sha256 = file.sha256.clone();
    f.policy.payload_inventory_sha256 = file.sha256;
    f.save();
}
fn report(f: &fixture::Fixture) -> AcceptanceVerificationReport {
    verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private)
        .unwrap()
        .report()
        .clone()
}
fn row(r: &AcceptanceVerificationReport) -> &AcceptanceCheckResult {
    r.checks
        .iter()
        .find(|c| c.check_id == "complete-source-coverage")
        .unwrap()
}
#[test]
fn a_scene_gap_is_independently_diagnosed() {
    let mut f = setup(AcceptanceScope::Diagnostic);
    f.doc.timeline.scenes[0].source_span[1] -= 1;
    freeze(&mut f);
    let r = report(&f);
    let c = row(&r);
    assert!(c.executed, "source coverage did not run: {c:?}");
    assert_eq!(c.outcome, ReviewOutcome::Fail);
    assert!(!c.reasons.contains(&AcceptanceReason::UnimplementedVerifier));
}
#[test]
fn complete_scoped_source_passes_but_other_verifiers_and_production_stay_blocked() {
    for scope in [
        AcceptanceScope::Diagnostic,
        AcceptanceScope::AutonomousExcerpt,
        AcceptanceScope::ProductionChapter,
    ] {
        let f = setup(scope);
        let r = report(&f);
        let c = row(&r);
        assert!(c.executed);
        assert_eq!(c.outcome, ReviewOutcome::Pass, "{c:?}");
        assert_eq!(
            c.target,
            AcceptanceTarget::Source {
                span: f.policy.source_span
            }
        );
        assert_eq!(c.executed_checker, Some(acceptance_binding_checker()));
        for h in [
            &f.doc.manifest.source_sha256,
            &f.doc.manifest.entries.timeline.sha256,
            &f.doc.manifest.entries.read_along.sha256,
        ] {
            assert!(c.consumed_sha256s.contains(h));
        }
        assert!(
            !r.production_approved
                && r.blockers
                    .iter()
                    .any(|s| s == "production-verifier-pending")
        );
        assert!(
            r.checks
                .iter()
                .filter(
                    |c| ["complete-chapter", "speech-fidelity", "story-coverage"]
                        .contains(&c.check_id.as_str())
                )
                .all(|c| !c.executed && c.outcome != ReviewOutcome::Pass)
        );
        r.validate(&acceptance_binding_checker()).unwrap();
        fn check_private(value: &serde_json::Value, source: &str, path: &str) {
            match value {
                serde_json::Value::String(s) => {
                    assert!(!s.contains(source));
                    assert!(!s.contains(path));
                }
                serde_json::Value::Array(a) => {
                    for v in a {
                        check_private(v, source, path)
                    }
                }
                serde_json::Value::Object(o) => {
                    for v in o.values() {
                        check_private(v, source, path)
                    }
                }
                _ => {}
            }
        }
        check_private(
            &serde_json::to_value(&r).unwrap(),
            &f.doc.source_text,
            f.private.to_str().unwrap(),
        );
        assert!(
            !f.doc
                .manifest
                .entries
                .files()
                .into_iter()
                .chain(f.doc.manifest.assets.iter())
                .any(|file| file.sha256 == f.doc.manifest.source_sha256)
        );
    }
}
#[test]
fn omitted_duplicate_overlapping_reordered_and_outside_coverage_is_rejected() {
    for case in 0..10 {
        let mut f = setup(AcceptanceScope::Diagnostic);
        match case {
            0 => {
                f.doc.timeline.scenes[0].source_span[1] -= 1;
            }
            1 => {
                f.doc.timeline.scenes[1].source_span[0] -= 1;
            }
            2 => {
                f.doc.timeline.scenes.swap(0, 1);
            }
            3 => {
                f.doc.timeline.scenes.push(f.doc.timeline.scenes[0].clone());
            }
            4 => {
                f.doc.timeline.scenes[1].source_span[1] += 1;
            }
            5 => {
                f.doc.timeline.scenes.clear();
            }
            6 => {
                f.doc.read_along.words.remove(1);
            }
            7 => {
                f.doc.read_along.words.swap(0, 1);
            }
            8 => {
                f.doc
                    .read_along
                    .words
                    .insert(1, f.doc.read_along.words[0].clone());
            }
            _ => {
                f.doc.read_along.words[0].text.push('!');
            }
        }
        freeze(&mut f);
        let r = report(&f);
        let c = row(&r);
        assert!(c.executed);
        assert_eq!(c.outcome, ReviewOutcome::Fail, "case {case}: {c:?}");
        assert!(r.blockers.iter().any(|s| s == "complete-source-coverage"));
    }
}
#[test]
fn source_bytes_hashes_spans_and_actual_files_are_bound() {
    for case in 0..8 {
        let mut f = setup(AcceptanceScope::Diagnostic);
        match case {
            0 => {
                let i = f
                    .policy
                    .inputs
                    .iter()
                    .find(|i| i.role == AcceptanceInputRole::Source)
                    .unwrap();
                std::fs::write(f.private.join(&i.file.path), b"changed source").unwrap();
            }
            1 => {
                f.doc.manifest.source_sha256 = "0".repeat(64);
                f.save();
            }
            2 => {
                f.policy.source_span[1] += 1;
            }
            3 => {
                f.doc.read_along.source_sha256 = "0".repeat(64);
                freeze(&mut f);
            }
            4 => {
                f.doc.timeline.scenes[0].source_span[1] -= 1;
                f.save();
            } // envelope differs from actual bytes
            5 => {
                let input = f
                    .policy
                    .inputs
                    .iter()
                    .find(|i| i.role == AcceptanceInputRole::Source)
                    .unwrap();
                std::fs::remove_file(f.private.join(&input.file.path)).unwrap();
            }
            6 => {
                std::fs::write(
                    f.root.join(&f.doc.manifest.entries.read_along.path),
                    b"changed read-along bytes",
                )
                .unwrap();
            }
            _ => {
                f.doc.read_along.words[0].source_span[1] += 1;
                freeze(&mut f);
            }
        }
        let r = report(&f);
        assert_ne!(row(&r).outcome, ReviewOutcome::Pass, "case {case}");
        assert!(row(&r).executed);
    }
    let mut f = setup(AcceptanceScope::AutonomousExcerpt);
    f.policy.source_span[0] = 1;
    let r = report(&f);
    assert_eq!(row(&r).outcome, ReviewOutcome::Missing);
    assert!(
        serde_json::to_string(&row(&r).reasons)
            .unwrap()
            .contains("missing_source_mapping")
    );
}
#[test]
fn unicode_scalar_content_and_punctuation_are_exact() {
    let mut f = setup(AcceptanceScope::Diagnostic);
    let text = "Súdde🐈ly";
    assert_eq!(text.chars().count(), 8);
    f.doc.source_text = f.doc.source_text.replacen("Suddenly", text, 1);
    f.doc.read_along.words[0].text = text.into();
    let hash = digest(f.doc.source_text.as_bytes());
    f.doc.manifest.source_sha256 = hash.clone();
    f.doc.read_along.source_sha256 = hash;
    let file = fixture::put(
        &f.private,
        "source.txt",
        f.doc.source_text.as_bytes(),
        "text/plain",
    );
    f.policy
        .inputs
        .iter_mut()
        .find(|i| i.role == AcceptanceInputRole::Source)
        .unwrap()
        .file = file;
    let i = f
        .policy
        .inputs
        .iter_mut()
        .find(|i| i.role == AcceptanceInputRole::FactInventory)
        .unwrap();
    let mut inventory: AcceptanceFactInventory =
        serde_json::from_slice(&std::fs::read(f.private.join(&i.file.path)).unwrap()).unwrap();
    inventory.source_sha256 = f.doc.manifest.source_sha256.clone();
    i.file = fixture::put(
        &f.private,
        "facts.json",
        &serde_json::to_vec(&inventory).unwrap(),
        "application/json",
    );
    freeze(&mut f);
    assert_eq!(row(&report(&f)).outcome, ReviewOutcome::Pass);
    f.doc.read_along.words[0].following_text = "  ".into();
    freeze(&mut f);
    assert_eq!(row(&report(&f)).outcome, ReviewOutcome::Fail);
}
#[test]
fn excerpt_cannot_substitute_for_trusted_chapter_source_or_complete_chapter_review() {
    let mut f = setup(AcceptanceScope::AutonomousExcerpt);
    assert_eq!(row(&report(&f)).outcome, ReviewOutcome::Pass);
    // A complete file can establish scoped source coverage even when its untrusted title says chapter.
    f.policy.scope = AcceptanceScope::ProductionChapter;
    let r = report(&f);
    assert_eq!(row(&r).outcome, ReviewOutcome::Pass);
    assert!(r.checks.iter().any(|c| c.check_id == "complete-chapter"
        && !c.executed
        && c.outcome != ReviewOutcome::Pass));
    // Runtime pins the larger authoritative source; candidate's excerpt is then incomplete.
    let longer = format!("{} More original chapter prose.", f.doc.source_text);
    let file = fixture::put(&f.private, "source.txt", longer.as_bytes(), "text/plain");
    f.policy
        .inputs
        .iter_mut()
        .find(|i| i.role == AcceptanceInputRole::Source)
        .unwrap()
        .file = file;
    f.policy.source_span[1] = longer.chars().count() as u64;
    assert_ne!(row(&report(&f)).outcome, ReviewOutcome::Pass);
}

#[test]
fn legacy_matrix_and_receipt_identities_are_preserved() {
    let legacy: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/acceptance-report-v1.json"
    ))
    .unwrap();
    let r: AcceptanceVerificationReport = serde_json::from_value(legacy["report"].clone()).unwrap();
    let checker: AcceptanceCheckerIdentity =
        serde_json::from_value(legacy["checker"].clone()).unwrap();
    r.validate(&checker).unwrap();
    assert!(r.validate(&acceptance_binding_checker()).is_err());
    let old = acceptance_check_matrix_for_version("pt-acceptance-required-v1").unwrap();
    let current = acceptance_check_matrix();
    assert_eq!(
        acceptance_check_matrix_for_version("pt-acceptance-required-v2")
            .unwrap()
            .checks
            .iter()
            .filter(|c| c.implemented)
            .count(),
        4
    );
    assert_eq!(old.checks.iter().filter(|c| c.implemented).count(), 3);
    assert_eq!(current.checks.iter().filter(|c| c.implemented).count(), 6);
    for scope in [
        AcceptanceScope::Diagnostic,
        AcceptanceScope::AutonomousExcerpt,
        AcceptanceScope::ProductionChapter,
    ] {
        assert_eq!(
            required_acceptance_checks_for_version(scope, &old.version)
                .unwrap()
                .iter()
                .map(|c| &c.id)
                .collect::<Vec<_>>(),
            required_acceptance_checks(scope)
                .iter()
                .map(|c| &c.id)
                .collect::<Vec<_>>()
        );
    }
    assert!(acceptance_check_matrix_for_version("unsupported").is_err());
}

#[test]
fn runtime_source_scope_identity_must_match_the_pinned_inventory() {
    let mut f = setup(AcceptanceScope::Diagnostic);
    f.policy.source_scope_id = "unrelated-source-scope".into();
    assert_ne!(row(&report(&f)).outcome, ReviewOutcome::Pass);
}
#[test]
fn successful_source_proof_does_not_erase_adverse_observations() {
    let mut f = setup(AcceptanceScope::Diagnostic);
    f.doc.acceptance.findings.push(Finding {
        id: "complete-source-coverage".into(),
        field: "source".into(),
        outcome: ReviewOutcome::Fail,
        message: "retained adverse source observation".into(),
        evidence: vec![],
    });
    f.save();
    let r = report(&f);
    assert!(row(&r).executed);
    assert_eq!(row(&r).outcome, ReviewOutcome::Fail);
    assert!(row(&r).reasons.contains(&AcceptanceReason::AdverseEvidence));
    assert!(
        r.observations
            .iter()
            .any(|o| o.check_id == "complete-source-coverage"
                && o.claimed_outcome == ReviewOutcome::Fail)
    );
    r.validate(&acceptance_binding_checker()).unwrap();
}
