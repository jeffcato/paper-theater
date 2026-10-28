use super::*;
fn replace(
    input: &mut StageInput,
    services: &mut OfflineServices,
    path: &str,
    bytes: Vec<u8>,
) -> FileRef {
    let file = FileRef {
        path: path.into(),
        sha256: digest(&bytes),
        bytes: bytes.len() as u64,
        media_type: "application/json".into(),
        dimensions: None,
    };
    if let Some(a) = input.inputs.iter_mut().find(|a| a.file.path == path) {
        a.file = file.clone();
    } else {
        input.inputs.push(ArtifactRef {
            id: format!("acceptance:{path}"),
            file: file.clone(),
            provenance: Provenance::Diagnostic,
        });
    }
    services.inputs.insert(path.into(), bytes);
    file
}
fn context(scope: AcceptanceScope) -> serde_json::Value {
    serde_json::json!({"schema_version":1,"scope":scope,"minimum_scope":scope,"purpose":scope,"source_scope_id":"pg-55-chapter-1-diagnostic","source_span":[0,938],"narration_sample_rate":24000,"narration_sample_count":1451280,"player_version":"pending-r186","candidate_checker_version":CONTRACT_REVISION,"device_id":null,"inputs":[],"checker_identity":null,"additional_required_check_ids":[],"inherited_inventory":null,"observations":[],"reports":[]})
}
fn written<T: serde::de::DeserializeOwned>(services: &OfflineServices, path: &str) -> T {
    let writes = services.writes.lock().unwrap();
    serde_json::from_slice(&writes.iter().find(|(c, _)| c.file.path == path).unwrap().1).unwrap()
}
fn export_example(path: &str, bytes: &[u8]) {
    if let Ok(root) = std::env::var("PT_ACCEPTANCE_EXPORT") {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(root);
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}
#[tokio::test]
async fn scoped_behavior_fixture_preserves_inheritance_and_rejects_downgrades() {
    let mut rows = vec![];
    for (name, scope, inherited_chapter, downgrade) in [
        ("diagnostic", AcceptanceScope::Diagnostic, false, false),
        ("excerpt", AcceptanceScope::AutonomousExcerpt, false, false),
        ("chapter", AcceptanceScope::ProductionChapter, false, false),
        (
            "excerpt-inherited-chapter",
            AcceptanceScope::AutonomousExcerpt,
            true,
            false,
        ),
        (
            "rejected-downgrade",
            AcceptanceScope::Diagnostic,
            false,
            true,
        ),
    ] {
        let (mut input, mut services) = fixture();
        let mut doc: PlaybackDocument = parse(&services.inputs["fixture.json"]).unwrap();
        doc.acceptance.required_check_ids = vec!["custom-review".into()];
        if inherited_chapter {
            doc.acceptance
                .required_check_ids
                .push("complete-chapter".into());
        }
        doc.acceptance.findings = vec![missing(
            "retained-failure",
            "original private adverse observation",
        )];
        doc.acceptance.findings[0].outcome = ReviewOutcome::Fail;
        let inherited = doc.acceptance.required_check_ids.clone();
        replace(
            &mut input,
            &mut services,
            "fixture.json",
            json(&doc).unwrap(),
        );
        let mut ctx = context(scope);
        ctx["additional_required_check_ids"] = serde_json::json!(["runtime-custom"]);
        if downgrade {
            ctx["minimum_scope"] = serde_json::json!("production_chapter");
            ctx["purpose"] = serde_json::json!("production_chapter");
        }
        replace(
            &mut input,
            &mut services,
            "acceptance-context.json",
            json(&ctx).unwrap(),
        );
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        let acceptance: Acceptance = written(&services, "acceptance.json");
        let status: serde_json::Value =
            written(&services, "private/acceptance-proposal-status.json");
        let selected: AcceptanceScope = serde_json::from_value(status["scope"].clone()).unwrap();
        let mut shared = required_acceptance_checks(selected)
            .into_iter()
            .map(|c| c.id)
            .collect::<Vec<_>>();
        shared.sort();
        assert!(
            shared
                .iter()
                .chain(inherited.iter())
                .all(|id| acceptance.required_check_ids.contains(id))
        );
        assert!(
            acceptance
                .required_check_ids
                .contains(&"runtime-custom".into())
        );
        assert!(
            out.findings
                .iter()
                .any(|f| f.id == "retained-failure" && f.outcome == ReviewOutcome::Fail)
        );
        if downgrade {
            assert!(status["binding"].is_null());
            assert_eq!(selected, AcceptanceScope::ProductionChapter);
        }
        if scope == AcceptanceScope::AutonomousExcerpt && !inherited_chapter {
            assert!(
                !acceptance
                    .required_check_ids
                    .contains(&"complete-chapter".into())
            );
        }
        rows.push(serde_json::json!({"case":name,"requested_scope":scope,"selected_scope":selected,"shared_required_check_ids":shared,"inherited_required_check_ids":inherited,"custom_required_check_ids":["runtime-custom"],"adverse_findings":[{"id":"retained-failure","outcome":"fail"}],"effective_required_check_ids":acceptance.required_check_ids,"scope_rejected":downgrade,"outcome":out.outcome}));
    }
    let example = serde_json::json!({"schema_version":1,"matrix_version":ACCEPTANCE_MATRIX_VERSION,"scopes":rows});
    export_example("scopes.json", &json(&example).unwrap());
    for version in [1, 2, 3] {
        let archived_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../fixtures/generation/acceptance-proposals-v{version}/scopes.json"
        ));
        let archived: serde_json::Value = parse(&std::fs::read(archived_path).unwrap()).unwrap();
        // Execution metadata advances without dropping inherited IDs or adverse findings.
        assert_eq!(example["scopes"], archived["scopes"]);
        for row in archived["scopes"].as_array().unwrap() {
            let scope: AcceptanceScope =
                serde_json::from_value(row["selected_scope"].clone()).unwrap();
            let mut ids = required_acceptance_checks_for_version(
                scope,
                archived["matrix_version"].as_str().unwrap(),
            )
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect::<Vec<_>>();
            ids.sort();
            assert_eq!(
                serde_json::to_value(ids).unwrap(),
                row["shared_required_check_ids"]
            );
        }
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/generation/acceptance-proposals-v4/scopes.json");
    assert_eq!(
        example,
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(path).unwrap()).unwrap()
    );
}
#[tokio::test]
async fn package_requirements_follow_the_shared_scope_instead_of_the_legacy_literal() {
    for scope in [
        AcceptanceScope::Diagnostic,
        AcceptanceScope::AutonomousExcerpt,
        AcceptanceScope::ProductionChapter,
    ] {
        let (mut input, mut services) = fixture();
        let mut doc: PlaybackDocument = parse(&services.inputs["fixture.json"]).unwrap();
        doc.acceptance.required_check_ids.clear();
        doc.acceptance.findings.clear();
        replace(
            &mut input,
            &mut services,
            "fixture.json",
            json(&doc).unwrap(),
        );
        replace(
            &mut input,
            &mut services,
            "acceptance-context.json",
            json(&context(scope)).unwrap(),
        );
        let output = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(output.outcome, StageOutcome::NeedsRepair);
        let acceptance: Acceptance = written(&services, "acceptance.json");
        let shared = required_acceptance_checks(scope);
        for check in shared {
            assert!(
                acceptance.required_check_ids.contains(&check.id),
                "missing {:?}: {}",
                scope,
                check.id
            );
        }
        if scope == AcceptanceScope::AutonomousExcerpt {
            assert!(
                !acceptance
                    .required_check_ids
                    .contains(&"complete-chapter".into())
            );
        }
    }
}

#[tokio::test]
async fn audience_payload_excludes_raw_source_and_alignment_records() {
    let (input, services) = fixture();
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    let package = output.package_candidate.unwrap();
    assert!(
        !package
            .payload_inventory
            .iter()
            .any(|f| f.path == "source.txt" || f.path == "evidence/alignment.json"),
        "private records entered the audience allowlist"
    );
    assert!(
        package
            .payload_inventory
            .iter()
            .any(|f| f.path == "audio/master.wav")
    );
    assert!(
        package
            .payload_inventory
            .iter()
            .any(|f| f.path == "assets/figure.png")
    );
}

fn put(
    input: &mut StageInput,
    services: &mut OfflineServices,
    path: &str,
    bytes: Vec<u8>,
    media: &str,
) -> FileRef {
    let mut f = replace(input, services, path, bytes);
    f.media_type = media.into();
    for a in input.inputs.iter_mut().filter(|a| a.file.path == path) {
        a.file = f.clone();
    }
    f
}
fn reference(input: &StageInput, path: &str) -> FileRef {
    input
        .inputs
        .iter()
        .find(|a| a.file.path == path)
        .unwrap()
        .file
        .clone()
}
fn bound_fixture() -> (StageInput, OfflineServices) {
    let (mut input, mut services) = fixture();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut ctx = context(AcceptanceScope::Diagnostic);
    let mut roles = vec![];
    for (role, path) in [
        (AcceptanceInputRole::Source, "source.txt"),
        (AcceptanceInputRole::Alignment, "evidence/alignment.json"),
        (
            AcceptanceInputRole::PresentationMap,
            "presentation-map.json",
        ),
        (AcceptanceInputRole::Profile, "profile.json"),
    ] {
        roles.push(AcceptanceInput {
            role,
            file: reference(&input, path),
        });
    }
    for (role, path, bytes, media) in [
        (
            AcceptanceInputRole::Narration,
            "private/narration.wav",
            std::fs::read(root.join("fixtures/generation/acoustic/excerpt.wav")).unwrap(),
            "audio/wav",
        ),
        (
            AcceptanceInputRole::Recipe,
            "private/recipe.txt",
            b"synthetic scoped package binding recipe v1".to_vec(),
            "text/plain",
        ),
        (
            AcceptanceInputRole::Player,
            "private/player.bin",
            b"synthetic player fixture; not an autonomous build".to_vec(),
            "application/octet-stream",
        ),
    ] {
        let file = put(&mut input, &mut services, path, bytes, media);
        if role == AcceptanceInputRole::Recipe {
            input.pins.recipe_sha256 = file.sha256.clone();
        }
        roles.push(AcceptanceInput { role, file });
    }
    ctx["inputs"] = serde_json::to_value(roles).unwrap();
    let checker = put(
        &mut input,
        &mut services,
        "private/checker.json",
        json(&acceptance_binding_checker()).unwrap(),
        "application/json",
    );
    ctx["checker_identity"] = serde_json::to_value(checker).unwrap();
    ctx["player_version"] = serde_json::json!("synthetic-player-v1");
    let mut doc: PlaybackDocument = parse(&services.inputs["fixture.json"]).unwrap();
    doc.acceptance.recipe_sha256 = input.pins.recipe_sha256.clone();
    doc.acceptance.player_version = "synthetic-player-v1".into();
    let raw = put(
        &mut input,
        &mut services,
        "private/review.json",
        br#"{"provenance":"synthetic","observation":"uncertain diagnostic binding case"}"#.to_vec(),
        "application/json",
    );
    let fact = AcceptanceFact {
        id: "reveal-1-fact-0".into(),
        reveal_id: doc.manifest.reveals[0].id.clone(),
        source_span: doc.manifest.reveals[0].facts[0].source_span,
        asset_sha256: doc.actors[0].image.sha256.clone(),
    };
    ctx["observations"] = serde_json::json!([{"id":"synthetic-image-observation","check_id":"image-scope-review","target":AcceptanceTarget::Fact{fact,domain:AcceptanceSampleDomain::Source},"outcome":"uncertain","provenance":"diagnostic","raw_evidence":raw}]);
    replace(
        &mut input,
        &mut services,
        "fixture.json",
        json(&doc).unwrap(),
    );
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    (input, services)
}
#[tokio::test]
async fn typed_fact_and_evidence_candidates_bind_exact_bytes_and_stay_private() {
    let (input, services) = bound_fixture();
    let output = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let facts: AcceptanceFactInventory = written(&services, "private/acceptance-facts.json");
    let proposals: Vec<AcceptanceEvidenceProposal> =
        written(&services, "private/acceptance-proposals.json");
    let inventory: AcceptanceEvidenceInventory =
        written(&services, "private/acceptance-evidence-inventory.json");
    assert_eq!(facts.facts.len(), 2);
    assert_eq!(facts.required_cue_ids, vec!["nod-em"]);
    assert_eq!(proposals.len(), 1);
    assert_eq!(inventory.proposals.len(), 1);
    let p = &proposals[0];
    let package = output.package_candidate.unwrap();
    assert_eq!(p.binding.inventory_sha256, package.inventory_sha256);
    assert_eq!(p.binding.source_sha256, facts.source_sha256);
    assert_eq!(p.binding.recipe_sha256, input.pins.recipe_sha256);
    assert_eq!(
        p.binding.checker_sha256,
        acceptance_binding_checker().sha256
    );
    assert_eq!(p.outcome, ReviewOutcome::Uncertain);
    p.raw_evidence
        .verify(&services.inputs[&p.raw_evidence.path])
        .unwrap();
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    assert!(
        package
            .payload_inventory
            .iter()
            .all(|f| !f.path.starts_with("private/")
                && f.path != "source.txt"
                && f.path != "evidence/alignment.json")
    );
    assert!(
        package
            .findings
            .iter()
            .any(|f| f.outcome != ReviewOutcome::Pass)
    );
    export_example(
        "acceptance-context.json",
        &services.inputs["acceptance-context.json"],
    );
    export_example(
        "public-allowlist.json",
        &json(&package.payload_inventory).unwrap(),
    );
    for (c, b) in services
        .writes
        .lock()
        .unwrap()
        .iter()
        .filter(|(c, _)| c.file.path.starts_with("private/"))
    {
        export_example(&c.file.path, b);
    }
}

#[tokio::test]
async fn explicit_audience_export_verifies_without_private_records_or_pass_authority() {
    let (mut input, mut services) = bound_fixture();
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    ctx["observations"][0]["outcome"] = serde_json::json!("pass");
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    let package = output.package_candidate.unwrap();
    let doc: PlaybackDocument = written(&services, "diagnostic-document.json");
    let public = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let save = |root: &std::path::Path, path: &str, bytes: &[u8]| {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    // Exactly the runtime export allowlist: verified payload and three public metadata files.
    for f in &package.payload_inventory {
        f.verify(&services.inputs[&f.path]).unwrap();
        save(public.path(), &f.path, &services.inputs[&f.path]);
    }
    for (path, bytes) in &services.inputs {
        save(private.path(), path, bytes);
    }
    for (c, bytes) in services.writes.lock().unwrap().iter() {
        c.file.verify(bytes).unwrap();
        save(private.path(), &c.file.path, bytes);
        if ["inventory.json", "acceptance.json", "manifest.json"].contains(&c.file.path.as_str()) {
            save(public.path(), &c.file.path, bytes);
        }
    }
    verify_package_files(&doc, public.path()).unwrap();
    for path in [
        "source.txt",
        "evidence/alignment.json",
        "private",
        "acceptance-context.json",
        "diagnostic-document.json",
    ] {
        assert!(!public.path().join(path).exists(), "exported {path}");
    }
    let inventory_bytes = std::fs::read(public.path().join("inventory.json")).unwrap();
    assert_eq!(digest(&inventory_bytes), package.inventory_sha256);
    let inventory: Vec<FileRef> = parse(&inventory_bytes).unwrap();
    assert_eq!(inventory, package.payload_inventory);
    assert!(inventory.windows(2).all(|w| w[0].path < w[1].path));
    assert!(
        doc.acceptance
            .findings
            .iter()
            .all(|f| f.evidence.is_empty())
    );
    assert!(
        doc.acceptance
            .attestations
            .iter()
            .all(|a| a.raw_evidence.is_none())
    );
    let status: serde_json::Value = written(&services, "private/acceptance-proposal-status.json");
    let binding: AcceptanceBinding = serde_json::from_value(status["binding"].clone()).unwrap();
    let mut inputs: Vec<AcceptanceInput> = serde_json::from_value(ctx["inputs"].clone()).unwrap();
    for (role, field) in [
        (AcceptanceInputRole::FactInventory, "fact_inventory"),
        (AcceptanceInputRole::EvidenceInventory, "evidence_inventory"),
    ] {
        inputs.push(AcceptanceInput {
            role,
            file: serde_json::from_value(status[field].clone()).unwrap(),
        });
    }
    // Test harness stands in for runtime; generation never constructs this authority.
    let policy = AcceptanceVerificationPolicy {
        schema_version: 1,
        scope: AcceptanceScope::Diagnostic,
        inputs,
        source_span: [0, 938],
        narration_sample_rate: 24000,
        narration_sample_count: 1451280,
        source_scope_id: "pg-55-chapter-1-diagnostic".into(),
        payload_inventory_sha256: package.inventory_sha256,
        asset_sha256s: binding.asset_sha256s,
        device_id: None,
        player_version: "synthetic-player-v1".into(),
        candidate_checker_version: CONTRACT_REVISION.into(),
        checker_sha256: acceptance_binding_checker().sha256,
        additional_required_check_ids: vec![],
    };
    let verification =
        verify_acceptance(&doc, public.path(), &policy, &[], private.path()).unwrap();
    verification.recheck_inputs().unwrap();
    let report = verification.report();
    assert!(!report.production_approved);
    assert!(
        report
            .blockers
            .contains(&"production-verifier-pending".into())
    );
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.check_id == "image-scope-review"
                && !c.executed
                && c.outcome != ReviewOutcome::Pass)
    );
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn original_acoustic_presentation_and_chapter_observations_remain_private_and_unresolved() {
    let (mut input, mut services) = bound_fixture();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut reports = vec![];
    for (i, path) in [
        "fixtures/generation/presentation-evidence-v1/fact-evidence.json",
        "fixtures/generation/presentation-evidence-v1/presentation-fact-evidence.json",
        "fixtures/generation/chapter-acoustic-v1/chapter-acoustic-validation.json",
    ]
    .iter()
    .enumerate()
    {
        reports.push(put(
            &mut input,
            &mut services,
            &format!("private/original-{i}.json"),
            std::fs::read(root.join(path)).unwrap(),
            "application/json",
        ));
    }
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    ctx["reports"] = serde_json::to_value(&reports).unwrap();
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    let status: serde_json::Value = written(&services, "private/acceptance-proposal-status.json");
    assert_eq!(
        status["original_reports"],
        serde_json::to_value(&reports).unwrap()
    );
    for report in &reports {
        report.verify(&services.inputs[&report.path]).unwrap();
        for (id, outcome) in [
            ("cellar-instruction", ReviewOutcome::Pass),
            ("Dorothy-urged", ReviewOutcome::Pass),
            ("screaming", ReviewOutcome::Pass),
            ("urged-cellar", ReviewOutcome::Pass),
            ("strange-event", ReviewOutcome::Uncertain),
            ("balloon-ascent", ReviewOutcome::Uncertain),
            ("unprovided-fact", ReviewOutcome::Missing),
        ] {
            let id = format!("observed:{}:{id}", &report.sha256[..12]);
            assert!(
                out.findings
                    .iter()
                    .any(|f| f.id == id && f.outcome == outcome),
                "lost {id}"
            );
        }
        assert!(
            !out.package_candidate
                .as_ref()
                .unwrap()
                .payload_inventory
                .iter()
                .any(|f| f.sha256 == report.sha256)
        );
    }
    let facts: AcceptanceFactInventory = written(&services, "private/acceptance-facts.json");
    assert_eq!(facts.facts.len(), 2); // Acoustic observations cannot invent image applicability.
    let proposals: Vec<AcceptanceEvidenceProposal> =
        written(&services, "private/acceptance-proposals.json");
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].outcome, ReviewOutcome::Uncertain);
    assert_eq!(status["reveal_permissions"], serde_json::json!([]));
}

#[tokio::test]
async fn replay_does_not_emit_journal_proposals() {
    let (input, services) = bound_fixture();
    GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_mime_or_path_alias_cannot_export_a_known_private_record() {
    let (mut input, mut services) = bound_fixture();
    let secret = br#"{"private_receipt":"private-binding-sentinel"}"#.to_vec();
    let private = put(
        &mut input,
        &mut services,
        "private/receipt.json",
        secret.clone(),
        "application/json",
    );
    let alias = put(
        &mut input,
        &mut services,
        "assets/receipt.svg",
        secret,
        "image/svg+xml",
    );
    let mut doc: PlaybackDocument = parse(&services.inputs["fixture.json"]).unwrap();
    doc.manifest.assets.push(alias);
    replace(
        &mut input,
        &mut services,
        "fixture.json",
        json(&doc).unwrap(),
    );
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    ctx["reports"] = serde_json::json!([private.clone()]);
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    assert!(
        !output
            .package_candidate
            .unwrap()
            .payload_inventory
            .iter()
            .any(|f| f.sha256 == private.sha256)
    );
}

#[tokio::test]
async fn malformed_or_forged_context_cannot_reduce_the_shared_scope() {
    let (mut input, mut services) = bound_fixture();
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    ctx["runtime_authority"] = serde_json::json!(true);
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    let output = GenerationExecutor.execute(input, &services).await.unwrap();
    let a: Acceptance = written(&services, "acceptance.json");
    assert!(
        a.required_check_ids
            .contains(&"complete-source-coverage".into())
    );
    assert_eq!(output.outcome, StageOutcome::NeedsRepair);
    let p: Vec<AcceptanceEvidenceProposal> =
        written(&services, "private/acceptance-proposals.json");
    assert!(p.is_empty());
}

#[tokio::test]
async fn inherited_custom_requirements_and_adverse_findings_survive_scope_selection() {
    let (mut input, mut services) = bound_fixture();
    let mut doc: PlaybackDocument = parse(&services.inputs["fixture.json"]).unwrap();
    doc.acceptance.required_check_ids = vec!["inherited-custom".into(), "complete-chapter".into()];
    doc.acceptance.findings = vec![
        Finding {
            id: "older-failure".into(),
            field: "private.old".into(),
            outcome: ReviewOutcome::Fail,
            message: "private original failure".into(),
            evidence: vec![],
        },
        Finding {
            id: "older-uncertain".into(),
            field: "private.old".into(),
            outcome: ReviewOutcome::Uncertain,
            message: "private original uncertainty".into(),
            evidence: vec![],
        },
    ];
    replace(
        &mut input,
        &mut services,
        "fixture.json",
        json(&doc).unwrap(),
    );
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    for field in ["scope", "minimum_scope", "purpose"] {
        ctx[field] = serde_json::json!("autonomous_excerpt");
    }
    ctx["additional_required_check_ids"] = serde_json::json!(["runtime-custom"]);
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let a: Acceptance = written(&services, "acceptance.json");
    for id in [
        "inherited-custom",
        "runtime-custom",
        "complete-chapter",
        "complete-excerpt",
        "complete-source-coverage",
        "older-failure",
        "older-uncertain",
    ] {
        assert!(a.required_check_ids.contains(&id.into()), "lost {id}");
    }
    assert!(out.findings.iter().any(|f| f.id == "older-failure"
        && f.outcome == ReviewOutcome::Fail
        && f.message == "private original failure"));
    assert!(a.findings.iter().all(|f| f.evidence.is_empty()));
    assert!(a.attestations.iter().all(|a| a.raw_evidence.is_none()));
    let original: Acceptance = written(&services, "private/authored-acceptance.json");
    assert_eq!(original.findings[0].message, "private original failure");
}
#[tokio::test]
async fn missing_image_association_produces_an_unresolved_record_instead_of_dummy_facts() {
    let (mut input, mut services) = bound_fixture();
    let mut doc: PlaybackDocument = parse(&services.inputs["fixture.json"]).unwrap();
    let id = doc.manifest.reveals[0].facts[0]
        .image_attestation_id
        .clone();
    doc.manifest.reveals[0].not_before_sample = None;
    doc.acceptance.attestations.retain(|a| a.id != id);
    replace(
        &mut input,
        &mut services,
        "fixture.json",
        json(&doc).unwrap(),
    );
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let f: AcceptanceFactInventory = written(&services, "private/acceptance-facts.json");
    assert_eq!(f.facts.len(), 1);
    assert_eq!(f.required_cue_ids, vec!["nod-em"]);
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "unbound-authored-fact:reveal-1-fact-0"
                && f.outcome == ReviewOutcome::Missing)
    );
    let p: Vec<AcceptanceEvidenceProposal> =
        written(&services, "private/acceptance-proposals.json");
    assert!(p.is_empty());
}
#[tokio::test]
async fn missing_or_stale_implementation_media_and_domain_bindings_emit_no_complete_proposal() {
    for fault in [
        "missing-player",
        "checker",
        "version",
        "rate",
        "source-span",
        "recipe",
        "domain",
        "asset",
        "reveal",
        "raw-bytes",
        "downgrade",
    ] {
        let (mut input, mut services) = bound_fixture();
        let mut ctx: serde_json::Value =
            parse(&services.inputs["acceptance-context.json"]).unwrap();
        match fault {
            "missing-player" => ctx["inputs"]
                .as_array_mut()
                .unwrap()
                .retain(|i| i["role"] != "player"),
            "checker" => ctx["checker_identity"]["sha256"] = serde_json::json!("0".repeat(64)),
            "version" => ctx["player_version"] = serde_json::json!("unprovided-version"),
            "rate" => ctx["narration_sample_rate"] = serde_json::json!(48000),
            "source-span" => ctx["source_span"] = serde_json::json!([0, 939]),
            "recipe" => {
                ctx["inputs"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|i| i["role"] == "recipe")
                    .unwrap()["file"]["sha256"] = serde_json::json!("0".repeat(64))
            }
            "domain" => ctx["observations"][0]["target"]["domain"] = serde_json::json!("narration"),
            "asset" => {
                ctx["observations"][0]["target"]["fact"]["asset_sha256"] =
                    serde_json::json!("0".repeat(64))
            }
            "reveal" => {
                ctx["observations"][0]["target"]["fact"]["reveal_id"] =
                    serde_json::json!("unprovided-reveal")
            }
            "raw-bytes" => {
                ctx["observations"][0]["raw_evidence"]["sha256"] = serde_json::json!("0".repeat(64))
            }
            "downgrade" => {
                ctx["minimum_scope"] = serde_json::json!("production_chapter");
                ctx["purpose"] = serde_json::json!("production_chapter");
            }
            _ => unreachable!(),
        }
        replace(
            &mut input,
            &mut services,
            "acceptance-context.json",
            json(&ctx).unwrap(),
        );
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair, "{fault}");
        let p: Vec<AcceptanceEvidenceProposal> =
            written(&services, "private/acceptance-proposals.json");
        assert!(p.is_empty(), "accepted {fault}");
    }
    let (input, mut services) = bound_fixture();
    services.inputs.get_mut("private/review.json").unwrap()[0] ^= 1;
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
}
#[tokio::test]
async fn duplicate_passing_claim_cannot_erase_an_inherited_adverse_proposal() {
    let (mut input, mut services) = bound_fixture();
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let mut previous: Vec<AcceptanceEvidenceProposal> =
        written(&services, "private/acceptance-proposals.json");
    previous[0].outcome = ReviewOutcome::Fail;
    let inherited = put(
        &mut input,
        &mut services,
        "private/inherited-inventory.json",
        json(&AcceptanceEvidenceInventory {
            schema_version: 1,
            proposals: previous,
        })
        .unwrap(),
        "application/json",
    );
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    ctx["inherited_inventory"] = serde_json::to_value(inherited).unwrap();
    ctx["observations"][0]["outcome"] = serde_json::json!("pass");
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let inventory: AcceptanceEvidenceInventory =
        written(&services, "private/acceptance-evidence-inventory.json");
    assert_eq!(inventory.proposals.len(), 2);
    assert_eq!(inventory.proposals[0].outcome, ReviewOutcome::Fail);
    assert_eq!(inventory.proposals[1].outcome, ReviewOutcome::Pass);
    assert!(
        out.findings
            .iter()
            .any(|f| f.id.starts_with("proposal-conflict:") && f.outcome == ReviewOutcome::Fail)
    );
    let a: Acceptance = written(&services, "acceptance.json");
    assert!(
        a.findings
            .iter()
            .any(|f| f.id == "image-scope-review" && f.outcome == ReviewOutcome::Fail)
    );
}
#[tokio::test]
async fn invalid_scope_retains_inherited_requirements_and_adverse_records() {
    let (mut input, mut services) = bound_fixture();
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let mut ledger: AcceptanceEvidenceInventory =
        written(&services, "private/acceptance-evidence-inventory.json");
    ledger.proposals[0].outcome = ReviewOutcome::Fail;
    let inherited = put(
        &mut input,
        &mut services,
        "private/old.json",
        json(&ledger).unwrap(),
        "application/json",
    );
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    ctx["minimum_scope"] = serde_json::json!("production_chapter");
    ctx["additional_required_check_ids"] = serde_json::json!(["retain-custom"]);
    ctx["inherited_inventory"] = serde_json::to_value(inherited).unwrap();
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let a: Acceptance = written(&services, "acceptance.json");
    assert!(a.required_check_ids.contains(&"retain-custom".into()));
    let retained: AcceptanceEvidenceInventory =
        written(&services, "private/acceptance-evidence-inventory.json");
    assert_eq!(json(&retained).unwrap(), json(&ledger).unwrap());
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "image-scope-review" && f.outcome == ReviewOutcome::Fail)
    );
    let status: serde_json::Value = written(&services, "private/acceptance-proposal-status.json");
    assert_eq!(status["context"]["path"], "acceptance-context.json");
    assert!(status["binding"].is_null());
}
#[tokio::test]
async fn unbound_adverse_observations_keep_their_original_outcome() {
    for outcome in [
        ReviewOutcome::Fail,
        ReviewOutcome::Uncertain,
        ReviewOutcome::Missing,
    ] {
        let (mut input, mut services) = bound_fixture();
        let mut ctx: serde_json::Value =
            parse(&services.inputs["acceptance-context.json"]).unwrap();
        ctx["checker_identity"] = serde_json::Value::Null;
        ctx["observations"][0]["outcome"] = serde_json::to_value(outcome).unwrap();
        replace(
            &mut input,
            &mut services,
            "acceptance-context.json",
            json(&ctx).unwrap(),
        );
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert!(
            out.findings
                .iter()
                .any(|f| f.id == "image-scope-review" && f.outcome == outcome)
        );
        let proposals: Vec<AcceptanceEvidenceProposal> =
            written(&services, "private/acceptance-proposals.json");
        assert!(proposals.is_empty());
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    }
}
#[tokio::test]
async fn private_proposal_bytes_replay_identically_without_redispatch() {
    let (mut input, mut services) = bound_fixture();
    input.mode = ExecutionMode::Fixture;
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let originals = services
        .writes
        .lock()
        .unwrap()
        .iter()
        .filter(|(f, _)| f.file.path.starts_with("private/"))
        .cloned()
        .collect::<Vec<_>>();
    for (c, b) in &originals {
        services.inputs.insert(c.file.path.clone(), b.clone());
        input.inputs.push(ArtifactRef {
            id: format!("recorded:{}", c.file.path),
            file: c.file.clone(),
            provenance: Provenance::Diagnostic,
        });
    }
    input.mode = ExecutionMode::Replay;
    services.writes.lock().unwrap().clear();
    services.emits.store(0, Ordering::SeqCst);
    let second = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    for (c, b) in &originals {
        let now = services.writes.lock().unwrap();
        assert_eq!(
            &now.iter()
                .find(|(f, _)| f.file.path == c.file.path)
                .unwrap()
                .1,
            b
        );
    }
    assert_eq!(services.emits.load(Ordering::SeqCst), 0);
    assert_eq!(first.outcome, second.outcome);
    let mut ctx: serde_json::Value = parse(&services.inputs["acceptance-context.json"]).unwrap();
    ctx["player_version"] = serde_json::json!("stale-replay-version");
    replace(
        &mut input,
        &mut services,
        "acceptance-context.json",
        json(&ctx).unwrap(),
    );
    assert!(GenerationExecutor.execute(input, &services).await.is_err());
}
