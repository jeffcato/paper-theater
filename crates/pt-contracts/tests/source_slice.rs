#[path = "support/source_slice_fixture.rs"]
mod fixture;
use fixture::*;
use pt_contracts::*;
#[test]
fn independently_pinned_zero_and_nonzero_unicode_slices_pass_without_retiming() {
    for (prefix, origin) in [
        ("", 0),
        ("🧭é\u{301} “前”\n", 0),
        ("🧭é\u{301} “前”\n", 600),
    ] {
        let x = Fixture::new(prefix, origin);
        let before = serde_json::to_value(&x.f.doc).unwrap();
        let v = x.verify();
        let r = v.report();
        assert_eq!(source(r).outcome, ReviewOutcome::Pass, "{:?}", source(r));
        assert!(source(r).executed);
        let b = r
            .source_slice
            .as_ref()
            .expect("actual full/slice binding missing");
        assert_eq!(b.full_source_span, x.policy.full_source_span);
        assert_eq!(b.global_origin, x.relationship.global_origin);
        assert_eq!(b.local_span, x.policy.base.source_span);
        for h in [
            &x.policy.full_source.file.sha256,
            &x.policy.relationship.as_ref().unwrap().sha256,
            &x.relationship.slice.sha256,
        ] {
            assert!(source(r).consumed_sha256s.contains(h));
        }
        assert_eq!(before, serde_json::to_value(&x.f.doc).unwrap());
        assert_eq!(r.binding.source_sha256, x.relationship.slice.sha256);
        assert!(!r.production_approved);
        assert!(
            r.blockers
                .iter()
                .any(|s| s == "production-verifier-pending")
        );
        r.validate(&acceptance_binding_checker()).unwrap();
        v.recheck_inputs().unwrap();
        let raw = serde_json::to_value(r).unwrap();
        fn private(v: &serde_json::Value, p: &str) {
            match v {
                serde_json::Value::String(s) => assert!(!s.contains(p)),
                serde_json::Value::Array(a) => a.iter().for_each(|v| private(v, p)),
                serde_json::Value::Object(o) => o.values().for_each(|v| private(v, p)),
                _ => {}
            }
        }
        private(&raw, x.f.private.to_str().unwrap());
        assert!(
            !raw["source_slice"]
                .as_object()
                .unwrap()
                .contains_key("file")
        );
    }
}
#[test]
fn absent_or_unsupported_mappings_never_fall_back_to_file_local_success() {
    let mut x = Fixture::new("🧭prefix", 12);
    x.policy.relationship = None;
    assert_eq!(source(x.verify().report()).outcome, ReviewOutcome::Missing);
    let mut raw = serde_json::to_value(&x.relationship).unwrap();
    raw["schema_version"] = serde_json::json!(2);
    x.policy.relationship = Some(acceptance_fixture::put(
        &x.f.private,
        "unsupported.json",
        &serde_json::to_vec(&raw).unwrap(),
        "application/json",
    ));
    assert_eq!(source(x.verify().report()).outcome, ReviewOutcome::Missing);
    let mut omitted = Fixture::new("prefix", 12);
    omitted
        .f
        .doc
        .acceptance
        .required_check_ids
        .retain(|id| id != "complete-source-coverage");
    omitted.f.save();
    let v = omitted.verify();
    assert_eq!(source(v.report()).outcome, ReviewOutcome::Missing);
    assert!(
        source(v.report())
            .reasons
            .contains(&AcceptanceReason::MissingDeclaration)
    );
    let mut p = x.f.policy.clone();
    p.source_span[0] = 1;
    let legacy = verify_acceptance(&x.f.doc, &x.f.root, &p, &[], &x.f.private).unwrap();
    assert_eq!(source(legacy.report()).outcome, ReviewOutcome::Missing);
}
#[test]
fn origins_hashes_counts_partial_overlap_and_scope_conflicts_fail() {
    for case in 0..15 {
        let mut x = Fixture::new("🧭é\u{301} 前\n", 100);
        match case {
            0 => x.relationship.global_origin += 1,
            1 => x.relationship.local_origin = 1,
            2 => x.relationship.full_source.origin += 1,
            3 => x.relationship.full_source.file.sha256 = "0".repeat(64),
            4 => x.relationship.slice.sha256 = "0".repeat(64),
            5 => x.relationship.full_source.scalar_count += 1,
            6 => x.relationship.slice_scalar_count -= 1,
            7 => x.relationship.local_span[1] -= 1,
            8 => x.relationship.local_span[0] = 1,
            9 => x.relationship.full_source_span[0] -= 1,
            10 => x.relationship.full_source_span[1] += 1,
            11 => x.relationship.slice_scope_id = "other-slice".into(),
            12 => x.relationship.full_source.scope_id = "other-full-source".into(),
            13 => x.relationship.local_span = [0, 0],
            _ => x.relationship.full_source_span = [0, 0],
        }
        x.save_relationship();
        assert_ne!(
            source(x.verify().report()).outcome,
            ReviewOutcome::Pass,
            "case {case}"
        );
    }
}
#[test]
fn byte_or_utf16_offsets_and_false_full_counts_cannot_replace_scalar_equality() {
    for offset in [
        "🧭é\u{301}".len() as u64,
        "🧭é\u{301}".encode_utf16().count() as u64,
    ] {
        let mut x = Fixture::new("🧭é\u{301}", 10);
        x.policy.full_source_span = [10 + offset, 10 + offset + x.relationship.slice_scalar_count];
        x.relationship.full_source_span = x.policy.full_source_span;
        x.relationship.global_origin = 10 + offset;
        x.save_relationship();
        assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
    }
    let mut x = Fixture::new("prefix", 0);
    x.policy.full_source.scalar_count += 1;
    x.relationship.full_source = x.policy.full_source.clone();
    x.save_relationship();
    assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
}
#[test]
fn changed_full_slice_mapping_bytes_and_post_receipt_mutation_are_rejected() {
    for case in 0..3 {
        let x = Fixture::new("prefix", 42);
        let v = x.verify();
        assert_eq!(source(v.report()).outcome, ReviewOutcome::Pass);
        let path = match case {
            0 => &x.policy.full_source.file.path,
            1 => &x.relationship.slice.path,
            _ => &x.policy.relationship.as_ref().unwrap().path,
        };
        std::fs::write(x.f.private.join(path), b"changed bytes").unwrap();
        assert!(v.recheck_inputs().is_err());
        assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
    }
}
#[test]
fn duplicate_relationships_global_facts_and_repeated_text_do_not_create_authority() {
    let mut x = Fixture::new("prefix", 100);
    let mut raw = serde_json::to_value(&x.relationship).unwrap();
    raw["relationships"] = serde_json::json!([x.relationship, x.relationship]);
    x.policy.relationship = Some(acceptance_fixture::put(
        &x.f.private,
        "ambiguous.json",
        &serde_json::to_vec(&raw).unwrap(),
        "application/json",
    ));
    assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
    let mut x = Fixture::new("prefix", 100);
    let input = x
        .policy
        .base
        .inputs
        .iter_mut()
        .find(|i| i.role == AcceptanceInputRole::FactInventory)
        .unwrap();
    let mut facts: AcceptanceFactInventory =
        serde_json::from_slice(&std::fs::read(x.f.private.join(&input.file.path)).unwrap())
            .unwrap();
    facts.source_span = x.policy.full_source_span;
    input.file = acceptance_fixture::put(
        &x.f.private,
        "facts.json",
        &serde_json::to_vec(&facts).unwrap(),
        "application/json",
    );
    assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
    let text = Fixture::new("", 0).f.doc.source_text.clone();
    let mut x = Fixture::new(&text, 0);
    x.relationship.full_source_span = [0, x.relationship.slice_scalar_count];
    x.relationship.global_origin = 0;
    x.save_relationship();
    assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
}

#[test]
fn bounded_reads_and_private_relationship_export_remain_fail_closed() {
    let mut x = Fixture::new("prefix", 42);
    x.policy.full_source.file.bytes = 32 * 1024 * 1024 + 1;
    assert!(x.policy.validate().is_err());
    let mut x = Fixture::new("prefix", 42);
    x.policy.relationship.as_mut().unwrap().bytes = 1024 * 1024 + 1;
    assert!(x.policy.validate().is_err());
    #[cfg(unix)]
    for mapping in [false, true] {
        let x = Fixture::new("prefix", 42);
        let file = if mapping {
            x.policy.relationship.as_ref().unwrap()
        } else {
            &x.policy.full_source.file
        };
        let p = x.f.private.join(&file.path);
        let outside = x.f.root.join("outside-private-input");
        std::fs::copy(&p, &outside).unwrap();
        std::fs::remove_file(&p).unwrap();
        std::os::unix::fs::symlink(&outside, &p).unwrap();
        assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
    }
    for mapping in [false, true] {
        let mut x = Fixture::new("prefix", 42);
        x.policy.base.scope = AcceptanceScope::AutonomousExcerpt;
        let hashes = x
            .policy
            .base
            .inputs
            .iter()
            .filter(|i| {
                matches!(
                    i.role,
                    AcceptanceInputRole::Source
                        | AcceptanceInputRole::Narration
                        | AcceptanceInputRole::Alignment
                        | AcceptanceInputRole::Recipe
                        | AcceptanceInputRole::FactInventory
                        | AcceptanceInputRole::EvidenceInventory
                )
            })
            .map(|i| i.file.sha256.clone())
            .collect::<Vec<_>>();
        x.f.doc
            .manifest
            .assets
            .retain(|a| !hashes.contains(&a.sha256));
        let freeze = |x: &mut Fixture| {
            x.policy.base.asset_sha256s =
                x.f.doc
                    .manifest
                    .assets
                    .iter()
                    .map(|f| f.sha256.clone())
                    .collect();
            x.policy.base.asset_sha256s.sort();
            x.policy.base.asset_sha256s.dedup();
            let mut inv =
                x.f.doc
                    .manifest
                    .entries
                    .files()
                    .into_iter()
                    .chain(x.f.doc.manifest.assets.iter())
                    .cloned()
                    .collect::<Vec<_>>();
            inv.sort_by(|a, b| a.path.cmp(&b.path));
            let file = acceptance_fixture::put(
                &x.f.root,
                "inventory.json",
                &serde_json::to_vec(&inv).unwrap(),
                "application/json",
            );
            x.policy.base.payload_inventory_sha256 = file.sha256.clone();
            x.f.doc.acceptance.inventory_sha256 = file.sha256;
            x.f.save();
        };
        freeze(&mut x);
        assert!(
            !x.verify()
                .report()
                .checks
                .iter()
                .find(|c| c.check_id == "evidence-binding-integrity")
                .unwrap()
                .reasons
                .contains(&AcceptanceReason::PrivateEvidenceExported)
        );
        let f = if mapping {
            x.policy.relationship.as_ref().unwrap()
        } else {
            &x.policy.full_source.file
        };
        let bytes = std::fs::read(x.f.private.join(&f.path)).unwrap();
        let public =
            acceptance_fixture::put(&x.f.root, "unsafe-private-export", &bytes, &f.media_type);
        x.f.doc.manifest.assets.push(public);
        freeze(&mut x);
        assert!(
            x.verify()
                .report()
                .checks
                .iter()
                .find(|c| c.check_id == "evidence-binding-integrity")
                .unwrap()
                .reasons
                .contains(&AcceptanceReason::PrivateEvidenceExported)
        );
    }
}

#[test]
fn exact_endpoints_and_punctuation_are_not_normalized() {
    for prefix in ["", "🧭é\u{301}"] {
        let mut x = Fixture::new(prefix, 700);
        let text = format!("{prefix}{}", x.f.doc.source_text);
        x.policy.full_source.file = acceptance_fixture::put(
            &x.f.private,
            "full-source.txt",
            text.as_bytes(),
            "text/plain",
        );
        x.policy.full_source.scalar_count = text.chars().count() as u64;
        if prefix.is_empty() {
            x.policy.full_source.scope_id = x.policy.base.source_scope_id.clone();
        }
        x.relationship.full_source = x.policy.full_source.clone();
        x.save_relationship();
        assert_eq!(
            x.policy.full_source_span[1],
            700 + x.policy.full_source.scalar_count
        );
        assert_eq!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
        let changed = text.replacen('“', "\"", 1);
        assert_ne!(changed, text);
        x.policy.full_source.file = acceptance_fixture::put(
            &x.f.private,
            "full-source.txt",
            changed.as_bytes(),
            "text/plain",
        );
        x.policy.full_source.scope_id = "trusted-full-source".into();
        x.relationship.full_source = x.policy.full_source.clone();
        x.save_relationship();
        assert_ne!(source(x.verify().report()).outcome, ReviewOutcome::Pass);
    }
}
