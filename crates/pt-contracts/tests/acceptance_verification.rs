use pt_contracts::*;
#[path = "support/acceptance_fixture.rs"]
mod acceptance_fixture;

fn fixture() -> PlaybackDocument {
    serde_json::from_slice(include_bytes!(
        "../../../fixtures/shared/two-scene-v1/fixture.json"
    ))
    .unwrap()
}

#[test]
fn independent_checks_execute_but_candidate_passes_do_not_become_verifier_receipts() {
    let f = acceptance_fixture::Fixture::new();
    let verified = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    let report = verified.report();
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.check_id == "package-byte-integrity"
                && c.executed
                && c.outcome == ReviewOutcome::Pass)
    );
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.check_id == "required-check-coverage" && c.outcome == ReviewOutcome::Pass)
    );
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.check_id == "independent-acoustic-evidence"
                && !c.executed
                && c.outcome != ReviewOutcome::Pass)
    );
    assert!(!report.production_approved);
    assert!(
        report
            .blockers
            .iter()
            .any(|b| b == "production-verifier-pending")
    );
    verified.recheck_inputs().unwrap();
    report.validate(&acceptance_binding_checker()).unwrap();
    let again = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    assert_eq!(
        serde_json::to_vec(report).unwrap(),
        serde_json::to_vec(again.report()).unwrap()
    );
}

fn proposal(f: &acceptance_fixture::Fixture) -> AcceptanceEvidenceProposal {
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    let r = v.report();
    let target = r
        .requirements
        .iter()
        .find(|r| r.check_id == "rendered-creative-review")
        .unwrap()
        .target
        .clone();
    let raw = acceptance_fixture::put(
        &f.private,
        "review.json",
        br#"{"outcome":"pass","note":"synthetic proposal, not an executed checker"}"#,
        "application/json",
    );
    AcceptanceEvidenceProposal {
        schema_version: 1,
        id: "proposed-review".into(),
        check_id: "rendered-creative-review".into(),
        target,
        binding: r.binding.clone(),
        outcome: ReviewOutcome::Pass,
        provenance: Provenance::Diagnostic,
        raw_evidence: raw,
    }
}

#[test]
fn passing_receipt_with_exact_bytes_is_still_unimplemented_not_execution() {
    let f = acceptance_fixture::Fixture::new();
    let e = proposal(&f);
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[e], &f.private).unwrap();
    let r = v.report();
    assert!(
        r.observations
            .iter()
            .any(|o| o.origin == "proposal" && o.binding_outcome == ReviewOutcome::Pass)
    );
    assert!(
        r.checks
            .iter()
            .filter(|c| c.check_id == "rendered-creative-review")
            .all(|c| !c.executed
                && c.outcome != ReviewOutcome::Pass
                && c.reasons.contains(&AcceptanceReason::UnimplementedVerifier))
    );
    let mut forged = r.clone();
    let c = forged
        .checks
        .iter_mut()
        .find(|c| c.check_id == "rendered-creative-review")
        .unwrap();
    c.executed = true;
    c.outcome = ReviewOutcome::Pass;
    c.executed_checker = Some(acceptance_binding_checker());
    assert!(forged.validate(&acceptance_binding_checker()).is_err());
    let mut forged = r.clone();
    forged.production_approved = true;
    assert!(forged.validate(&acceptance_binding_checker()).is_err());
    assert!(
        !serde_json::to_string(r)
            .unwrap()
            .contains(&f.doc.source_text)
    );
}

#[test]
fn stale_identities_targets_versions_and_conflicting_receipts_fail_closed() {
    let f = acceptance_fixture::Fixture::new();
    let good = proposal(&f);
    let changes: Vec<(&str, Box<dyn Fn(&mut AcceptanceEvidenceProposal)>)> = vec![
        (
            "source",
            Box::new(|e| e.binding.source_sha256 = "0".repeat(64)),
        ),
        (
            "narration",
            Box::new(|e| e.binding.narration_sha256 = "0".repeat(64)),
        ),
        (
            "master",
            Box::new(|e| e.binding.master_sha256 = "0".repeat(64)),
        ),
        (
            "map",
            Box::new(|e| e.binding.presentation_map_sha256 = "0".repeat(64)),
        ),
        (
            "alignment",
            Box::new(|e| e.binding.alignment_sha256 = "0".repeat(64)),
        ),
        (
            "asset",
            Box::new(|e| e.binding.asset_sha256s[0] = "0".repeat(64)),
        ),
        (
            "recipe",
            Box::new(|e| e.binding.recipe_sha256 = "0".repeat(64)),
        ),
        (
            "profile",
            Box::new(|e| e.binding.profile_sha256 = "0".repeat(64)),
        ),
        (
            "player",
            Box::new(|e| e.binding.player_sha256 = "0".repeat(64)),
        ),
        (
            "checker",
            Box::new(|e| e.binding.checker_sha256 = "0".repeat(64)),
        ),
        (
            "player-version",
            Box::new(|e| e.binding.player_version = "other".into()),
        ),
        (
            "checker-version",
            Box::new(|e| e.binding.candidate_checker_version = "other".into()),
        ),
        (
            "source-scope",
            Box::new(|e| e.binding.source_scope_id = "other-book".into()),
        ),
        (
            "span",
            Box::new(|e| e.target = AcceptanceTarget::Source { span: [1, 2] }),
        ),
        (
            "sample-domain",
            Box::new(|e| {
                e.target = AcceptanceTarget::Audio {
                    domain: AcceptanceSampleDomain::Narration,
                    span: [0, 1451280],
                }
            }),
        ),
    ];
    for (name, change) in changes {
        let mut bad = good.clone();
        change(&mut bad);
        let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[bad], &f.private).unwrap();
        assert!(
            v.report()
                .observations
                .iter()
                .any(|o| o.origin == "proposal" && o.binding_outcome == ReviewOutcome::Fail),
            "{name}"
        );
        assert!(!v.report().production_approved);
    }
    let mut second = good.clone();
    second.id = "conflicting-review".into();
    second.outcome = ReviewOutcome::Fail;
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[good, second], &f.private).unwrap();
    assert_eq!(
        v.report()
            .observations
            .iter()
            .filter(|o| o.origin == "proposal")
            .count(),
        2
    );
    assert!(
        v.report()
            .checks
            .iter()
            .any(|c| c.check_id == "rendered-creative-review" && c.outcome == ReviewOutcome::Fail)
    );
}

#[test]
fn missing_checks_adverse_observations_and_post_receipt_mutation_remain_visible() {
    let mut f = acceptance_fixture::Fixture::new();
    let mut e = proposal(&f);
    e.outcome = ReviewOutcome::Uncertain;
    f.doc
        .acceptance
        .required_check_ids
        .retain(|c| c != "required-check-coverage");
    f.save();
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[e.clone()], &f.private).unwrap();
    assert!(
        v.report()
            .checks
            .iter()
            .any(|c| c.check_id == "required-check-coverage" && c.outcome == ReviewOutcome::Fail)
    );
    assert!(
        v.report()
            .checks
            .iter()
            .any(|c| c.check_id == "rendered-creative-review"
                && c.outcome == ReviewOutcome::Uncertain)
    );
    std::fs::write(
        f.private.join(&e.raw_evidence.path),
        b"changed after verification",
    )
    .unwrap();
    assert!(v.recheck_inputs().is_err());
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[e], &f.private).unwrap();
    assert!(
        v.report()
            .observations
            .iter()
            .any(|o| o.origin == "proposal" && o.reasons.contains(&AcceptanceReason::ChangedBytes))
    );
    std::fs::remove_file(f.root.join(&f.doc.manifest.entries.audio.path)).unwrap();
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    assert!(
        v.report()
            .checks
            .iter()
            .any(|c| c.check_id == "package-byte-integrity" && c.outcome != ReviewOutcome::Pass)
    );
}

#[test]
fn diagnostic_imported_synthetic_and_forged_autonomous_labels_never_approve() {
    let mut f = acceptance_fixture::Fixture::new();
    for provenance in [
        Provenance::Diagnostic,
        Provenance::Imported,
        Provenance::Synthetic,
        Provenance::Autonomous,
    ] {
        f.doc.manifest.provenance = provenance;
        f.doc.acceptance.provenance = provenance;
        for finding in &mut f.doc.acceptance.findings {
            finding.outcome = ReviewOutcome::Pass;
        }
        f.save();
        let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
        assert!(!v.report().production_approved);
        assert!(
            v.report()
                .blockers
                .iter()
                .any(|s| s == "production-verifier-pending")
        );
    }
}

#[test]
fn caller_cannot_omit_a_runtime_pinned_adverse_receipt() {
    let mut f = acceptance_fixture::Fixture::new();
    let mut e = proposal(&f);
    e.outcome = ReviewOutcome::Fail;
    let inventory = AcceptanceEvidenceInventory {
        schema_version: 1,
        proposals: vec![e],
    };
    let file = acceptance_fixture::put(
        &f.private,
        "evidence-inventory.json",
        &serde_json::to_vec(&inventory).unwrap(),
        "application/json",
    );
    f.policy
        .inputs
        .iter_mut()
        .find(|i| i.role == AcceptanceInputRole::EvidenceInventory)
        .unwrap()
        .file = file;
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    assert!(
        v.report()
            .checks
            .iter()
            .any(|c| c.check_id == "rendered-creative-review" && c.outcome == ReviewOutcome::Fail)
    );
    assert!(
        v.report()
            .observations
            .iter()
            .any(|o| o.origin == "ledger_proposal" && o.claimed_outcome == ReviewOutcome::Fail)
    );
}

#[test]
fn duplicate_declarations_and_missing_report_rows_are_rejected() {
    let mut f = acceptance_fixture::Fixture::new();
    f.doc
        .acceptance
        .required_check_ids
        .push("package-byte-integrity".into());
    f.save();
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    assert!(
        v.report()
            .checks
            .iter()
            .any(|c| c.check_id == "required-check-coverage" && c.outcome == ReviewOutcome::Fail)
    );
    let mut r = v.report().clone();
    r.checks.pop();
    assert!(r.validate(&acceptance_binding_checker()).is_err());
}

#[test]
fn the_authoritative_matrix_retains_all_legacy_production_requirements() {
    let ids = required_acceptance_checks(AcceptanceScope::ProductionChapter)
        .into_iter()
        .map(|c| c.id)
        .collect::<std::collections::BTreeSet<_>>();
    let generation = include_str!("../../pt-production/src/generation/mod.rs");
    let begin = generation.find("const REQUIRED_CHECKS:").unwrap();
    let end = generation[begin..]
        .find("]; ")
        .map(|n| n + begin)
        .or_else(|| generation[begin..].find("];\n").map(|n| n + begin))
        .unwrap();
    for line in generation[begin..end]
        .lines()
        .filter_map(|s| s.trim().strip_prefix('"').and_then(|s| s.split('"').next()))
    {
        assert!(
            ids.contains(line),
            "dropped legacy production requirement: {line}"
        );
    }
    let excerpt = required_acceptance_checks(AcceptanceScope::AutonomousExcerpt);
    assert!(excerpt.iter().any(|c| c.id == "complete-excerpt"));
    assert!(!excerpt.iter().any(|c| c.id == "complete-chapter"));
}

#[test]
fn removing_required_checks_is_an_explicit_blocker_even_with_forged_labels() {
    let mut candidate = fixture();
    candidate.manifest.provenance = Provenance::Autonomous;
    candidate.acceptance.provenance = Provenance::Autonomous;
    candidate.shell.profile.status = ProfileStatus::Verified;
    candidate.acceptance.required_check_ids.clear();
    candidate.acceptance.findings.clear();
    let blockers = production_blockers(&candidate);
    assert!(
        blockers.iter().any(|id| id == "required-check-coverage"),
        "removed required-check inventory was not diagnosed: {blockers:?}"
    );
    assert!(
        blockers
            .iter()
            .any(|id| id == "production-verifier-pending")
    );
}
