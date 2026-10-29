use pt_contracts::*;
#[path = "support/registered_source_cases.rs"]
mod cases;
#[path = "support/acceptance_fixture.rs"]
mod fixture;

#[test]
fn an_excerpt_cannot_be_registered_as_the_complete_selected_chapter() {
    let mut f = fixture::Fixture::new();
    f.policy.scope = AcceptanceScope::ProductionChapter;
    f.doc
        .acceptance
        .findings
        .retain(|x| x.id != "complete-chapter");
    f.doc.acceptance.required_check_ids.extend(
        required_acceptance_checks(AcceptanceScope::ProductionChapter)
            .into_iter()
            .map(|r| r.id),
    );
    f.doc.acceptance.required_check_ids.sort();
    f.doc.acceptance.required_check_ids.dedup();
    f.save();
    let source = f
        .policy
        .inputs
        .iter()
        .find(|i| i.role == AcceptanceInputRole::Source)
        .unwrap()
        .file
        .clone();
    let count = f.doc.source_text.chars().count() as u64;
    let full = FullSourcePinV1 {
        file: source.clone(),
        scope_id: "pg-55-chapter-1".into(),
        scalar_count: count,
        origin: 0,
    };
    let relation = SourceSliceRelationshipV1 {
        schema_version: 1,
        full_source: full.clone(),
        slice: source,
        slice_scope_id: f.policy.source_scope_id.clone(),
        slice_scalar_count: count,
        full_source_span: [0, count],
        local_span: [0, count],
        local_origin: 0,
        global_origin: 0,
    };
    let relationship = fixture::put(
        &f.private,
        "source-relationship.json",
        &serde_json::to_vec(&relation).unwrap(),
        "application/json",
    );
    let policy = AcceptanceVerificationPolicyV2 {
        schema_version: 2,
        base: f.policy.clone(),
        full_source: full,
        full_source_span: [0, count],
        relationship: Some(relationship),
    };
    let execution = verify_acceptance_v2(&f.doc, &f.root, &policy, &[], &f.private).unwrap();
    let report = execution.report();
    assert_eq!(
        report
            .checks
            .iter()
            .find(|c| c.check_id == "complete-source-coverage")
            .unwrap()
            .outcome,
        ReviewOutcome::Pass
    );
    let chapter = report
        .checks
        .iter()
        .find(|c| c.check_id == "complete-chapter")
        .unwrap();
    assert_eq!(chapter.outcome, ReviewOutcome::Fail);
    assert!(chapter.executed);
    assert!(!report.production_approved);
    assert!(
        report
            .blockers
            .iter()
            .any(|b| b == "production-verifier-pending")
    );
}

#[test]
fn complete_registered_oz_and_wells_sources_verify_in_local_and_book_coordinates() {
    for book in [0, 1] {
        for parent in [false, true] {
            let (f, p) = cases::registered(book, parent, AcceptanceScope::ProductionChapter, None);
            let execution = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
            let report = execution.report();
            let row = report
                .checks
                .iter()
                .find(|r| r.check_id == "complete-chapter")
                .unwrap();
            assert_eq!(
                row.outcome,
                ReviewOutcome::Pass,
                "book {book}, parent {parent}: {:?}",
                row.reasons
            );
            assert!(row.executed);
            if parent {
                assert!(report.source_slice.as_ref().unwrap().full_source_span[0] > 0);
            }
            assert!(!report.production_approved);
            assert!(
                report
                    .blockers
                    .iter()
                    .any(|b| b == "production-verifier-pending")
            );
            execution.recheck_inputs().unwrap();
            report.validate(&acceptance_binding_checker()).unwrap();
        }
    }
}

#[test]
fn serialized_scope_pass_requires_registered_mapping_and_consumed_catalog() {
    let (f, p) = cases::registered(0, true, AcceptanceScope::ProductionChapter, None);
    let execution = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
    let original = execution.report();
    let checker = acceptance_binding_checker();
    original.validate(&checker).unwrap();
    let mut missing = original.clone();
    missing.source_slice = None;
    assert!(
        missing.validate(&checker).is_err(),
        "scope Pass accepted without full-source mapping"
    );
    let mut forged = original.clone();
    forged
        .source_slice
        .as_mut()
        .unwrap()
        .full_source_scalar_count += 1;
    assert!(
        forged.validate(&checker).is_err(),
        "registered source scalar count was forged"
    );
    let mut stale = original.clone();
    let s = stale.source_slice.as_mut().unwrap();
    s.full_source_origin += 1;
    s.full_source_span[0] += 1;
    s.full_source_span[1] += 1;
    s.global_origin += 1;
    assert!(
        stale.validate(&checker).is_err(),
        "registered origin was shifted"
    );
    let mut unconsumed = original.clone();
    let catalog = registered_source_catalog_sha256_v1();
    unconsumed.consumed_hashes.retain(|c| c.sha256 != catalog);
    for c in &mut unconsumed.checks {
        c.consumed_sha256s.retain(|h| h != &catalog);
    }
    assert!(
        unconsumed.validate(&checker).is_err(),
        "catalog byte identity was omitted"
    );
}

#[test]
fn selected_excerpts_verify_but_cannot_satisfy_a_whole_chapter_requirement() {
    for book in [0, 1] {
        let (mut f, p) = cases::registered(
            book,
            true,
            AcceptanceScope::AutonomousExcerpt,
            Some([0, 200]),
        );
        let execution = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
        assert_eq!(
            execution
                .report()
                .checks
                .iter()
                .find(|r| r.check_id == "complete-excerpt")
                .unwrap()
                .outcome,
            ReviewOutcome::Pass
        );
        execution
            .report()
            .validate(&acceptance_binding_checker())
            .unwrap();
        f.doc
            .acceptance
            .required_check_ids
            .push("complete-chapter".into());
        f.save();
        let execution = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
        assert_eq!(
            execution
                .report()
                .checks
                .iter()
                .find(|r| r.check_id == "complete-chapter")
                .unwrap()
                .outcome,
            ReviewOutcome::Fail
        );
        assert!(!execution.report().production_approved);
    }
}

#[test]
fn missing_full_source_or_mapping_stays_missing_and_post_verification_mutation_fails() {
    let (f, mut p) = cases::registered(0, true, AcceptanceScope::ProductionChapter, None);
    let original = p.clone();
    p.relationship = None;
    let missing = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
    assert_eq!(
        missing
            .report()
            .checks
            .iter()
            .find(|r| r.check_id == "complete-chapter")
            .unwrap()
            .outcome,
        ReviewOutcome::Missing
    );
    let verified = verify_acceptance_v2(&f.doc, &f.root, &original, &[], &f.private).unwrap();
    verified.recheck_inputs().unwrap();
    std::fs::remove_file(f.private.join(&original.full_source.file.path)).unwrap();
    assert!(verified.recheck_inputs().is_err());
    let missing = verify_acceptance_v2(&f.doc, &f.root, &original, &[], &f.private).unwrap();
    assert_eq!(
        missing
            .report()
            .checks
            .iter()
            .find(|r| r.check_id == "complete-chapter")
            .unwrap()
            .outcome,
        ReviewOutcome::Missing
    );
}

#[test]
fn source_verification_preserves_original_fail_uncertain_and_missing_observations() {
    let (mut f, p) = cases::registered(0, false, AcceptanceScope::ProductionChapter, None);
    for outcome in [
        ReviewOutcome::Fail,
        ReviewOutcome::Uncertain,
        ReviewOutcome::Missing,
    ] {
        f.doc.acceptance.findings.push(Finding {
            id: "complete-chapter".into(),
            field: "source".into(),
            outcome,
            message: "synthetic adverse source observation".into(),
            evidence: vec![],
        });
        f.save();
        let execution = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
        assert_ne!(
            execution
                .report()
                .checks
                .iter()
                .find(|r| r.check_id == "complete-chapter")
                .unwrap()
                .outcome,
            ReviewOutcome::Pass
        );
        assert!(
            execution
                .report()
                .observations
                .iter()
                .any(|o| o.check_id == "complete-chapter" && o.claimed_outcome == outcome)
        );
        execution
            .report()
            .validate(&acceptance_binding_checker())
            .unwrap();
    }
}
