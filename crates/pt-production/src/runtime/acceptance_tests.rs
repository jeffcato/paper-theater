use super::*;
use pt_contracts as c;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn all_events(store: &Store, run: &str) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    let mut cursor = 0;
    loop {
        let page = store.journal(run, cursor, 500).unwrap();
        if page.is_empty() {
            break;
        }
        cursor = page.last().unwrap()["seq"].as_i64().unwrap();
        events.extend(page);
    }
    events
}
fn request() -> serde_json::Value {
    let profile =
        std::fs::read(root().join("fixtures/generation/two-scene-readiness-v2/profile.json"))
            .unwrap();
    json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":digest(b"saved-diagnostic-v1"),"profile_sha256":digest(&profile),"profile_id":"paper-player-r186-v1"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}})
}
#[test]
fn missing_trusted_registry_cannot_be_replaced_by_a_serialized_policy_or_report() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store
        .create("acceptance-missing-registry", &request())
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let forged = json!({"production_approved":true,"provenance":"autonomous","checker_sha256":c::acceptance_binding_checker().sha256,"inputs":[]});
    store.put(&serde_json::to_vec(&forged).unwrap()).unwrap();
    let before = store.snapshot(&run).unwrap();
    assert_eq!(
        store
            .prepare_private_acceptance(&run, epoch, &[])
            .err()
            .unwrap()
            .code,
        "acceptance_registry_missing"
    );
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert!(store.catalog("pg-55").unwrap().is_none());
}

async fn fixture(key: &str) -> (tempfile::TempDir, Store, String, i64, Runtime) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create(key, &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    store
        .seed_fixture(
            &run,
            epoch,
            &root().join("fixtures/generation/two-scene-readiness-v2"),
            Some(&root().join("fixtures/generation/source")),
        )
        .unwrap();
    store
        .seed_presentation_fixture(&run, epoch, &root())
        .unwrap();
    // Explicitly synthetic frozen test artifact. No UI implementation identity is inferred.
    let player = b"frozen synthetic player fixture; no rendering or production execution claimed";
    let file = c::FileRef {
        path: "player-fixture.txt".into(),
        sha256: digest(player),
        bytes: player.len() as u64,
        media_type: "text/plain".into(),
        dimensions: None,
    };
    std::fs::write(temp.path().join(&file.path), player).unwrap();
    store
        .register_acceptance_player(&run, epoch, temp.path(), &file, "pending-r186")
        .unwrap();
    let rt = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(
            crate::generation::GenerationExecutor,
        ))),
    );
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Presentation)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    assert_eq!(
        rt.execute_stage(&run, epoch, c::Stage::Package)
            .await
            .unwrap_err()
            .code,
        "stage_needs_repair"
    );
    (temp, store, run, epoch, rt)
}
#[tokio::test]
async fn actual_independent_verifier_persists_private_bound_report_and_retains_every_gate() {
    let (_temp, store, run, epoch, _rt) = fixture("private-actual").await;
    let prepared = store.prepare_private_acceptance(&run, epoch, &[]).unwrap();
    let reference = prepared.commit().unwrap();
    let report = store.private_acceptance_report(&run).unwrap().unwrap();
    assert_eq!(report.0.sha256, reference.sha256);
    assert_eq!(report.1.checker, c::acceptance_binding_checker());
    assert!(!report.1.production_approved);
    assert!(
        report
            .1
            .blockers
            .iter()
            .any(|b| b == "production-verifier-pending")
    );
    for id in [
        "package-byte-integrity",
        "evidence-binding-integrity",
        "required-check-coverage",
    ] {
        assert!(
            report
                .1
                .checks
                .iter()
                .any(|c| c.check_id == id && c.executed)
        );
    }
    assert!(report.1.checks.iter().any(|c| {
        !c.executed
            && c.reasons
                .contains(&c::AcceptanceReason::UnimplementedVerifier)
    }));
    assert!(
        report
            .1
            .observations
            .iter()
            .any(|o| o.check_id == "presentation-fact:unprovided-fact"
                && o.claimed_outcome != c::ReviewOutcome::Pass)
    );
    let inventory: Vec<c::FileRef> =
        serde_json::from_slice(&store.package_artifact(&run, "inventory.json").unwrap().0).unwrap();
    assert!(!inventory.iter().any(|f| f.sha256 == reference.sha256));
    assert!(
        store.artifact(&reference.sha256).is_err(),
        "private report must not enter the HTTP artifact allowlist"
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    assert!(
        store.snapshot(&run).unwrap()["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "production-verifier-pending")
    );
}

#[tokio::test]
async fn omitted_additional_adverse_proposals_remain_in_the_runtime_owned_inventory() {
    let (_temp, store, run, epoch, _rt) = fixture("private-adverse-ledger").await;
    store.verify_private_acceptance(&run, epoch, &[]).unwrap();
    let report = store.private_acceptance_report(&run).unwrap().unwrap().1;
    let raw = store
        .api_snapshot(&run)
        .unwrap()
        .stages
        .into_iter()
        .find(|s| s.stage == c::Stage::Presentation)
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.file.path == "presentation-fact-evidence.json")
        .unwrap()
        .file;
    let proposal = c::AcceptanceEvidenceProposal {
        schema_version: 1,
        id: "additional-adverse".into(),
        check_id: "speech-fidelity".into(),
        target: c::AcceptanceTarget::Audio {
            domain: c::AcceptanceSampleDomain::Narration,
            span: [0, report.binding.narration_sample_count],
        },
        binding: report.binding,
        outcome: c::ReviewOutcome::Fail,
        provenance: c::Provenance::Diagnostic,
        raw_evidence: raw,
    };
    store
        .verify_private_acceptance(&run, epoch, &[proposal])
        .unwrap();
    assert!(
        store
            .private_acceptance_report(&run)
            .unwrap()
            .unwrap()
            .1
            .observations
            .iter()
            .any(|o| o.id == "additional-adverse")
    );
    assert!(
        store
            .api_snapshot(&run)
            .unwrap()
            .findings
            .iter()
            .any(|f| f.id == "speech-fidelity" && f.outcome == c::ReviewOutcome::Fail)
    );
    store.verify_private_acceptance(&run, epoch, &[]).unwrap();
    let report = store.private_acceptance_report(&run).unwrap().unwrap().1;
    assert!(
        report
            .observations
            .iter()
            .any(|o| o.id == "additional-adverse" && o.claimed_outcome == c::ReviewOutcome::Fail),
        "caller omission erased adverse evidence"
    );
    assert!(!report.production_approved);
}

#[tokio::test]
async fn changed_snapshot_and_cancellation_after_verification_prevent_report_commit() {
    let (_temp, store, run, epoch, _rt) = fixture("private-cancel-recheck").await;
    let prepared = store.prepare_private_acceptance(&run, epoch, &[]).unwrap();
    std::fs::write(
        prepared.private_root().join("inputs/source.txt"),
        b"changed after actual verification",
    )
    .unwrap();
    let before = store.snapshot(&run).unwrap();
    let events = all_events(&store, &run);
    assert_eq!(prepared.commit().unwrap_err().code, "acceptance_changed");
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert_eq!(all_events(&store, &run), events);
    assert!(store.private_acceptance_report(&run).unwrap().is_none());
    let prepared = store.prepare_private_acceptance(&run, epoch, &[]).unwrap();
    store.cancel(&run).unwrap();
    let before = store.snapshot(&run).unwrap();
    assert_eq!(prepared.commit().unwrap_err().code, "stale_epoch");
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert!(store.private_acceptance_report(&run).unwrap().is_none());
    assert!(store.catalog("pg-55").unwrap().is_none());
}

struct NoDispatch;
#[async_trait::async_trait]
impl c::StageExecutor for NoDispatch {
    async fn execute(
        &self,
        _: c::StageInput,
        _: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        panic!("cached stage redispatched")
    }
}
#[tokio::test]
async fn rollback_is_atomic_and_restart_executes_verification_again_without_stage_redispatch() {
    let (temp, store, run, epoch, rt) = fixture("private-rollback-restart").await;
    let prepared = store.prepare_private_acceptance(&run, epoch, &[]).unwrap();
    store.db().unwrap().execute_batch("CREATE TRIGGER reject_private_report BEFORE INSERT ON private_acceptance_reports BEGIN SELECT RAISE(ABORT,'injected private report rollback'); END;").unwrap();
    let before = store.snapshot(&run).unwrap();
    let events = all_events(&store, &run);
    assert!(prepared.commit().is_err());
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert_eq!(all_events(&store, &run), events);
    assert!(store.private_acceptance_report(&run).unwrap().is_none());
    store
        .db()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_private_report")
        .unwrap();
    let expected = store.verify_private_acceptance(&run, epoch, &[]).unwrap();
    drop(rt);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    store.recover().unwrap();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let rt = Runtime::new(store.clone(), Arc::new(NoDispatch));
    for stage in [c::Stage::Presentation, c::Stage::Package] {
        assert_eq!(
            rt.execute_stage(&run, epoch, stage).await.unwrap_err().code,
            "stage_needs_repair"
        );
    }
    assert_eq!(
        store.verify_private_acceptance(&run, epoch, &[]).unwrap(),
        expected
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row(
                "SELECT executions FROM private_acceptance_reports WHERE run=?1",
                [&run],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
    let events = all_events(&store, &run);
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "private_acceptance_verified")
            .count(),
        2
    );
    assert!(
        !events
            .iter()
            .any(|e| e["type"].as_str().unwrap().starts_with("provider_")
                || e["type"].as_str().unwrap().starts_with("helper_"))
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        store
            .db()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM library", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(store.catalog("pg-55").unwrap().is_none());
    // A persisted genuine report cannot compensate for now-stale registered bytes.
    let player = store
        .db()
        .unwrap()
        .query_row(
            "SELECT reference FROM acceptance_players WHERE run=?1",
            [&run],
            |r| r.get::<_, String>(0),
        )
        .unwrap();
    let player: c::FileRef = serde_json::from_str(&player).unwrap();
    let path = store.object(&player.sha256).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, vec![b'x'; bytes.len()]).unwrap();
    assert_eq!(
        store
            .verify_private_acceptance(&run, epoch, &[])
            .unwrap_err()
            .code,
        "corrupt_artifact"
    );
    assert_eq!(all_events(&store, &run), events);
    std::fs::write(&path, bytes).unwrap();
}

#[tokio::test]
async fn forged_expected_identities_swapped_inputs_stale_bytes_and_report_redemption_fail_closed() {
    let (_temp, store, run, epoch, _rt) = fixture("private-forgery").await;
    store.verify_private_acceptance(&run, epoch, &[]).unwrap();
    let (report_ref, report) = store.private_acceptance_report(&run).unwrap().unwrap();
    let raw = store
        .api_snapshot(&run)
        .unwrap()
        .stages
        .into_iter()
        .find(|s| s.stage == c::Stage::Presentation)
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.file.path == "presentation-fact-evidence.json")
        .unwrap()
        .file;
    let proposal =
        |id: &str, raw: c::FileRef, binding: c::AcceptanceBinding| c::AcceptanceEvidenceProposal {
            schema_version: 1,
            id: id.into(),
            check_id: "speech-fidelity".into(),
            target: c::AcceptanceTarget::Audio {
                domain: c::AcceptanceSampleDomain::Narration,
                span: [0, binding.narration_sample_count],
            },
            binding,
            outcome: c::ReviewOutcome::Pass,
            provenance: c::Provenance::Autonomous,
            raw_evidence: raw,
        };
    let mut forged = Vec::new();
    for field in [
        "source_sha256",
        "presentation_map_sha256",
        "master_sha256",
        "player_sha256",
        "checker_sha256",
    ] {
        let mut binding = serde_json::to_value(&report.binding).unwrap();
        binding[field] = json!("0".repeat(64));
        forged.push(proposal(
            &format!("forged-{field}"),
            raw.clone(),
            serde_json::from_value(binding).unwrap(),
        ));
    }
    store
        .verify_private_acceptance(&run, epoch, &forged)
        .unwrap();
    let checked = store.private_acceptance_report(&run).unwrap().unwrap().1;
    assert_eq!(
        checked.binding, report.binding,
        "candidate metadata replaced runtime expectations"
    );
    for p in forged {
        assert!(checked.observations.iter().any(|o| o.id == p.id
            && o.binding_outcome == c::ReviewOutcome::Fail
            && o.reasons.contains(&c::AcceptanceReason::InvalidBinding)));
    }
    assert!(!checked.production_approved);
    let events = all_events(&store, &run);
    assert_eq!(
        store
            .verify_private_acceptance(
                &run,
                epoch,
                &[proposal(
                    "redeem-report",
                    report_ref,
                    report.binding.clone()
                )]
            )
            .unwrap_err()
            .code,
        "acceptance_report_redemption"
    );
    let mut swapped = raw;
    swapped.path = "inputs/player.bin".into();
    assert_eq!(
        store
            .verify_private_acceptance(
                &run,
                epoch,
                &[proposal("swapped-role", swapped, report.binding.clone())]
            )
            .unwrap_err()
            .code,
        "acceptance_input"
    );
    assert_eq!(all_events(&store, &run), events);
    for hash in [
        &report.binding.source_sha256,
        &report.binding.narration_sha256,
        &report.binding.presentation_map_sha256,
        &report.binding.master_sha256,
        &report.binding.player_sha256,
    ] {
        let path = store.object(hash).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut bad = original.clone();
        let last = bad.len() - 1;
        bad[last] ^= 1;
        std::fs::write(&path, bad).unwrap();
        assert_eq!(
            store
                .prepare_private_acceptance(&run, epoch, &[])
                .err()
                .unwrap()
                .code,
            "corrupt_artifact"
        );
        assert_eq!(all_events(&store, &run), events);
        std::fs::write(&path, original).unwrap();
    }
    let prepared = store.prepare_private_acceptance(&run, epoch, &[]).unwrap();
    let path = store
        .object(&report.binding.presentation_map_sha256)
        .unwrap();
    let original = std::fs::read(&path).unwrap();
    std::fs::write(&path, vec![b'x'; original.len()]).unwrap();
    assert_eq!(prepared.commit().unwrap_err().code, "corrupt_artifact");
    assert_eq!(all_events(&store, &run), events);
    std::fs::write(path, original).unwrap();
    assert!(store.catalog("pg-55").unwrap().is_none());
}

#[tokio::test]
async fn saved_runtime_automatically_verifies_registered_inputs_and_keeps_g18_needs_repair() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("private-pipeline", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let bytes = b"frozen synthetic player fixture; no rendering or production execution claimed";
    let player = c::FileRef {
        path: "player-fixture.txt".into(),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
        media_type: "text/plain".into(),
        dimensions: None,
    };
    std::fs::write(temp.path().join(&player.path), bytes).unwrap();
    let rt = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(
            crate::generation::GenerationExecutor,
        ))),
    )
    .with_saved_inputs(
        root().join("fixtures/generation/two-scene-readiness-v2"),
        root().join("fixtures/generation/source"),
    )
    .with_acoustic_inputs(root().join("fixtures/generation/acoustic"))
    .with_presentation_inputs(root())
    .with_acceptance_player(temp.path().to_path_buf(), player, "pending-r186".into())
    .with_offline_acoustic(root().join("tools/generation"))
    .unwrap();
    assert_eq!(rt.work_once().await.unwrap_err().code, "stage_needs_repair");
    let (reference, report) = store
        .private_acceptance_report(&run)
        .unwrap()
        .expect("runtime skipped actual independent verification");
    assert_eq!(
        report.manifest_sha256.as_deref(),
        Some(
            store
                .package_artifact(&run, "manifest.json")
                .unwrap()
                .2
                .as_str()
        )
    );
    assert_eq!(
        report.acceptance_sha256.as_deref(),
        Some(
            store
                .package_artifact(&run, "acceptance.json")
                .unwrap()
                .2
                .as_str()
        )
    );
    let snapshot = store.api_snapshot(&run).unwrap();
    assert_eq!(snapshot.state, c::RunState::Failed);
    assert!(snapshot.findings.iter().any(
        |f| f.id == "presentation-fact:unprovided-fact" && f.outcome != c::ReviewOutcome::Pass
    ));
    assert!(
        snapshot
            .stages
            .iter()
            .filter(|s| matches!(s.stage, c::Stage::Presentation | c::Stage::Package))
            .all(|s| s.outcome == Some(c::StageOutcome::NeedsRepair))
    );
    assert!(!report.production_approved);
    assert!(store.catalog("pg-55").unwrap().is_none());
    let mut journal = Vec::new();
    let mut cursor = 0;
    loop {
        let page = store.journal(&run, cursor, 500).unwrap();
        if page.is_empty() {
            break;
        }
        cursor = page.last().unwrap()["seq"].as_i64().unwrap();
        journal.extend(page);
    }
    assert_eq!(
        journal
            .iter()
            .filter(|e| e["type"] == "helper_authorized")
            .count(),
        1
    );
    assert!(
        !journal
            .iter()
            .any(|e| e["type"].as_str().unwrap().starts_with("provider_"))
    );
    if let Some(path) = std::env::var_os("PT_PRIVATE_ACCEPTANCE_RECEIPT") {
        let encoded=store.db().unwrap().query_row("SELECT reference FROM acceptance_compilations WHERE run=?1 ORDER BY rowid DESC LIMIT 1",[&run],|r|r.get::<_,String>(0)).unwrap();
        let compilation: c::FileRef = serde_json::from_str(&encoded).unwrap();
        let compiled: serde_json::Value = serde_json::from_slice(
            &std::fs::read(store.object(&compilation.sha256).unwrap()).unwrap(),
        )
        .unwrap();
        let role_map=compiled["policy"]["inputs"].as_array().unwrap().iter().map(|i|json!({"role":i["role"],"sha256":i["file"]["sha256"],"bytes":i["file"]["bytes"]})).collect::<Vec<_>>();
        let receipt = json!({"report_sha256":reference.sha256,"report_bytes":reference.bytes,"compilation_sha256":compilation.sha256,"checker":report.checker,"manifest_sha256":report.manifest_sha256,"acceptance_sha256":report.acceptance_sha256,"role_to_cas":role_map,"consumed_hash_count":report.consumed_hashes.len(),"checks":report.checks.iter().map(|c|json!({"id":c.check_id,"outcome":c.outcome,"executed":c.executed})).collect::<Vec<_>>(),"blockers":report.blockers,"production_approved":report.production_approved,"provider_calls":0,"presentation_helper_calls":0,"synthetic_test_player":true});
        std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}

#[test]
fn player_registry_rejects_forged_bytes_and_freezes_the_verified_registration() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let run = store.create("private-registry", &request()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let bytes = b"synthetic frozen registry test";
    let path = temp.path().join("frozen-player.txt");
    std::fs::write(&path, bytes).unwrap();
    let file = c::FileRef {
        path: "frozen-player.txt".into(),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
        media_type: "text/plain".into(),
        dimensions: None,
    };
    let mut bad = file.clone();
    bad.sha256 = "0".repeat(64);
    let before = store.snapshot(&run).unwrap();
    assert!(
        store
            .register_acceptance_player(&run, epoch, temp.path(), &bad, "pending-r186")
            .is_err()
    );
    assert_eq!(store.snapshot(&run).unwrap(), before);
    store
        .register_acceptance_player(&run, epoch, temp.path(), &file, "pending-r186")
        .unwrap();
    let before = store.snapshot(&run).unwrap();
    assert_eq!(
        store
            .register_acceptance_player(&run, epoch, temp.path(), &file, "another-version")
            .unwrap_err()
            .code,
        "acceptance_registry_frozen"
    );
    assert_eq!(store.snapshot(&run).unwrap(), before);
    std::fs::remove_file(path).unwrap();
    store
        .register_acceptance_player(&run, epoch, temp.path(), &file, "pending-r186")
        .expect("restart should reuse verified durable registry bytes");
    assert_eq!(store.snapshot(&run).unwrap(), before);
}

struct LateReview;
#[async_trait::async_trait]
impl c::StageExecutor for LateReview {
    async fn execute(
        &self,
        _: c::StageInput,
        _: &dyn c::RunServices,
    ) -> std::result::Result<c::StageOutput, c::StageError> {
        Ok(c::StageOutput {
            outcome: c::StageOutcome::Complete,
            artifacts: vec![],
            decisions: vec![],
            findings: vec![c::Finding {
                id: "late-adverse-review".into(),
                field: "review".into(),
                outcome: c::ReviewOutcome::Fail,
                message: "Late adverse observation must survive an earlier verification snapshot."
                    .into(),
                evidence: vec![],
            }],
            cache_keys: vec![],
            library_updates: vec![],
            package_candidate: None,
        })
    }
}
#[tokio::test]
async fn evidence_arriving_after_verification_invalidates_the_handle_and_is_retained_on_retry() {
    let (_temp, store, run, epoch, _rt) = fixture("private-late-evidence").await;
    let prepared = store.prepare_private_acceptance(&run, epoch, &[]).unwrap();
    Runtime::new(store.clone(), Arc::new(LateReview))
        .execute_stage(&run, epoch, c::Stage::Review)
        .await
        .unwrap();
    let before = store.snapshot(&run).unwrap();
    assert_eq!(
        prepared.commit().unwrap_err().code,
        "acceptance_evidence_changed"
    );
    assert_eq!(store.snapshot(&run).unwrap(), before);
    assert!(store.private_acceptance_report(&run).unwrap().is_none());
    store.verify_private_acceptance(&run, epoch, &[]).unwrap();
    let report = store.private_acceptance_report(&run).unwrap().unwrap().1;
    assert!(report.observations.iter().any(
        |o| o.check_id == "late-adverse-review" && o.claimed_outcome == c::ReviewOutcome::Fail
    ));
    assert!(!report.production_approved);
    assert!(store.catalog("pg-55").unwrap().is_none());
}

#[tokio::test]
async fn changed_candidate_or_intended_source_after_verification_cannot_commit_an_old_report() {
    let (_temp, store, run, epoch, _rt) = fixture("private-context-recheck").await;
    let original_manifest = store.diagnostic_ref(&run).unwrap().unwrap().manifest_sha256;
    let original_request = store.snapshot(&run).unwrap()["request"].clone();
    for candidate in [true, false] {
        let prepared = store.prepare_private_acceptance(&run, epoch, &[]).unwrap();
        if candidate {
            store
                .db()
                .unwrap()
                .execute(
                    "UPDATE diagnostic_packages SET manifest_hash=?2 WHERE run=?1",
                    rusqlite::params![run, "0".repeat(64)],
                )
                .unwrap();
        } else {
            let mut changed = original_request.clone();
            changed["source_id"] = json!("another-source");
            store
                .db()
                .unwrap()
                .execute(
                    "UPDATE runs SET request=?2 WHERE id=?1",
                    rusqlite::params![run, serde_json::to_string(&changed).unwrap()],
                )
                .unwrap();
        }
        let before = store.snapshot(&run).unwrap();
        assert_eq!(
            prepared.commit().unwrap_err().code,
            "acceptance_context_changed"
        );
        assert_eq!(store.snapshot(&run).unwrap(), before);
        assert!(store.private_acceptance_report(&run).unwrap().is_none());
        store
            .db()
            .unwrap()
            .execute(
                "UPDATE diagnostic_packages SET manifest_hash=?2 WHERE run=?1",
                rusqlite::params![run, original_manifest],
            )
            .unwrap();
        store
            .db()
            .unwrap()
            .execute(
                "UPDATE runs SET request=?2 WHERE id=?1",
                rusqlite::params![run, serde_json::to_string(&original_request).unwrap()],
            )
            .unwrap();
    }
}
