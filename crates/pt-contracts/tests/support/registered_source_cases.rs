use super::fixture;
use pt_contracts::*;

/// Source-only diagnostic candidate. Synthetic caption positions exercise exact
/// prose reconstruction; speech, timing, narrative and production proof stay absent.
pub fn registered(
    book: usize,
    parent: bool,
    scope: AcceptanceScope,
    excerpt: Option<SourceSpan>,
) -> (fixture::Fixture, AcceptanceVerificationPolicyV2) {
    let mut f = fixture::Fixture::new();
    let catalog = registered_source_catalog_v1();
    let entry = &catalog.sources[book];
    let prefix = if book == 0 { "pg55" } else { "pg36" };
    let base =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generation/source");
    let frozen: serde_json::Value =
        serde_json::from_slice(&std::fs::read(base.join(format!("{prefix}-frozen.json"))).unwrap())
            .unwrap();
    let chapter = frozen["text"].as_str().unwrap();
    let frame = &entry.frames[if parent { 0 } else { 1 }];
    let full_bytes = if parent {
        std::fs::read(base.join(format!("{prefix}-clean.txt"))).unwrap()
    } else {
        chapter.as_bytes().to_vec()
    };
    let relative = excerpt.unwrap_or([0, entry.canonical_scalar_count]);
    let text: String = chapter
        .chars()
        .skip(relative[0] as usize)
        .take((relative[1] - relative[0]) as usize)
        .collect();
    let count = text.chars().count() as u64;
    let source = fixture::put(
        &f.private,
        "selected-source.txt",
        text.as_bytes(),
        "text/plain",
    );
    let full_file = fixture::put(&f.private, "full-source.txt", &full_bytes, "text/plain");
    f.policy.scope = scope;
    f.policy.source_span = [0, count];
    f.policy.source_scope_id = format!("{}-selected", entry.source_id);
    f.policy
        .inputs
        .iter_mut()
        .find(|i| i.role == AcceptanceInputRole::Source)
        .unwrap()
        .file = source.clone();
    f.doc.source_text = text.clone();
    f.doc.manifest.source_sha256 = source.sha256.clone();
    f.doc.manifest.reveals.clear();
    for (i, s) in f.doc.timeline.scenes.iter_mut().enumerate() {
        s.source_span = if i == 0 {
            [0, count / 2]
        } else {
            [count / 2, count]
        };
    }
    f.doc.read_along.source_sha256 = source.sha256.clone();
    f.doc.read_along.words.clear();
    f.doc.read_along.groups.clear();
    f.doc.read_along.preferred_break_after.clear();
    let chars: Vec<char> = text.chars().collect();
    let mut cursor = 0;
    while cursor < chars.len() && chars[cursor].is_whitespace() {
        cursor += 1;
    }
    f.doc.read_along.leading_text = chars[..cursor].iter().collect();
    while cursor < chars.len() {
        let start = cursor;
        while cursor < chars.len() && !chars[cursor].is_whitespace() {
            cursor += 1;
        }
        let end = cursor;
        while cursor < chars.len() && chars[cursor].is_whitespace() {
            cursor += 1;
        }
        let index = f.doc.read_along.words.len();
        let id = format!("scope-word-{index}");
        let sample = 96000 + index as u64 * 1000;
        f.doc.read_along.words.push(ReadAlongWord {
            id: id.clone(),
            source_span: [start as u64, end as u64],
            text: chars[start..end].iter().collect(),
            following_text: chars[end..cursor].iter().collect(),
            start_sample: sample,
            end_sample: sample + 1,
        });
        f.doc.read_along.groups.push(ReadAlongGroup {
            id: format!("scope-group-{index}"),
            word_ids: vec![id.clone()],
            timing_owner_word_id: id,
            start_sample: sample,
            end_sample: sample + 1,
        });
    }
    f.doc.manifest.entries.timeline = fixture::put(
        &f.root,
        "timeline.json",
        &serde_json::to_vec(&f.doc.timeline).unwrap(),
        "application/json",
    );
    f.doc.manifest.entries.read_along = fixture::put(
        &f.root,
        "read-along.json",
        &serde_json::to_vec(&f.doc.read_along).unwrap(),
        "application/json",
    );
    let facts = AcceptanceFactInventory {
        schema_version: 1,
        source_scope_id: f.policy.source_scope_id.clone(),
        source_sha256: source.sha256.clone(),
        source_span: [0, count],
        facts: vec![],
        required_cue_ids: vec![],
    };
    f.policy
        .inputs
        .iter_mut()
        .find(|i| i.role == AcceptanceInputRole::FactInventory)
        .unwrap()
        .file = fixture::put(
        &f.private,
        "scope-facts.json",
        &serde_json::to_vec(&facts).unwrap(),
        "application/json",
    );
    f.doc
        .acceptance
        .findings
        .retain(|x| !matches!(x.id.as_str(), "complete-chapter" | "complete-excerpt"));
    f.doc
        .acceptance
        .required_check_ids
        .extend(required_acceptance_checks(scope).into_iter().map(|r| r.id));
    f.doc.acceptance.required_check_ids.sort();
    f.doc.acceptance.required_check_ids.dedup();
    f.save();
    let full = FullSourcePinV1 {
        file: full_file,
        scope_id: frame.scope_id.clone(),
        scalar_count: frame.scalar_count,
        origin: frame.origin,
    };
    let selected = [
        frame.chapter_span[0] + relative[0],
        frame.chapter_span[0] + relative[1],
    ];
    let relation = SourceSliceRelationshipV1 {
        schema_version: 1,
        full_source: full.clone(),
        slice: source,
        slice_scope_id: f.policy.source_scope_id.clone(),
        slice_scalar_count: count,
        full_source_span: selected,
        local_span: [0, count],
        local_origin: 0,
        global_origin: selected[0],
    };
    let relationship = fixture::put(
        &f.private,
        "scope-relationship.json",
        &serde_json::to_vec(&relation).unwrap(),
        "application/json",
    );
    let policy = AcceptanceVerificationPolicyV2 {
        schema_version: 2,
        base: f.policy.clone(),
        full_source: full,
        full_source_span: selected,
        relationship: Some(relationship),
    };
    (f, policy)
}

pub fn records() -> Vec<serde_json::Value> {
    use serde_json::json;
    let mut records = Vec::new();
    for (book, parent, scope, excerpt, name) in [
        (
            0,
            true,
            AcceptanceScope::ProductionChapter,
            None,
            "oz-book-coordinates",
        ),
        (
            0,
            false,
            AcceptanceScope::ProductionChapter,
            None,
            "oz-chapter-coordinates",
        ),
        (
            1,
            true,
            AcceptanceScope::ProductionChapter,
            None,
            "wells-book-coordinates",
        ),
        (
            1,
            false,
            AcceptanceScope::ProductionChapter,
            None,
            "wells-chapter-coordinates",
        ),
        (
            0,
            true,
            AcceptanceScope::AutonomousExcerpt,
            Some([0, 200]),
            "oz-selected-excerpt",
        ),
        (
            1,
            true,
            AcceptanceScope::AutonomousExcerpt,
            Some([0, 200]),
            "wells-selected-excerpt",
        ),
    ] {
        let (f, p) = registered(book, parent, scope, excerpt);
        let v = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
        v.report().validate(&acceptance_binding_checker()).unwrap();
        records
            .push(json!({"id":name,"valid":true,"report":v.report(),"checker":v.report().checker}));
    }
    let baseline = records[0].clone();
    for name in [
        "missing-source-mapping",
        "forged-full-scalar-count",
        "shifted-registered-origin",
        "shifted-chapter-extent",
        "missing-catalog-consumption",
        "unregistered-frame-pass",
        "invented-source-execution",
    ] {
        let mut c = baseline.clone();
        c["id"] = json!(name);
        c["valid"] = json!(false);
        match name {
            "missing-source-mapping" => c["report"]["source_slice"] = serde_json::Value::Null,
            "forged-full-scalar-count" => {
                let x = c["report"]["source_slice"]["full_source_scalar_count"]
                    .as_u64()
                    .unwrap();
                c["report"]["source_slice"]["full_source_scalar_count"] = json!(x + 1);
            }
            "shifted-registered-origin" => {
                let s = &mut c["report"]["source_slice"];
                s["full_source_origin"] = json!(1);
                for i in [0, 1] {
                    s["full_source_span"][i] =
                        json!(s["full_source_span"][i].as_u64().unwrap() + 1);
                }
                s["global_origin"] = json!(s["global_origin"].as_u64().unwrap() + 1);
            }
            "shifted-chapter-extent" => {
                let s = &mut c["report"]["source_slice"];
                for i in [0, 1] {
                    s["full_source_span"][i] =
                        json!(s["full_source_span"][i].as_u64().unwrap() + 1);
                }
                s["global_origin"] = json!(s["global_origin"].as_u64().unwrap() + 1);
            }
            "missing-catalog-consumption" => {
                let hash = registered_source_catalog_sha256_v1();
                c["report"]["consumed_hashes"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|h| h["sha256"] != hash);
                for row in c["report"]["checks"].as_array_mut().unwrap() {
                    row["consumed_sha256s"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|h| h != &hash);
                }
            }
            "unregistered-frame-pass" => {
                c["report"]["source_slice"]["full_source_scope_id"] = json!("unregistered-frame")
            }
            _ => {
                let row = c["report"]["checks"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["check_id"] == "complete-chapter")
                    .unwrap();
                row["executed"] = json!(false);
                row["executed_checker"] = serde_json::Value::Null;
            }
        }
        records.push(c);
    }
    let (f, mut p) = registered(0, true, AcceptanceScope::ProductionChapter, None);
    p.relationship = None;
    let v = verify_acceptance_v2(&f.doc, &f.root, &p, &[], &f.private).unwrap();
    v.report().validate(&acceptance_binding_checker()).unwrap();
    records.push(json!({"id":"missing-mapping-remains-blocked","valid":true,"report":v.report(),"checker":v.report().checker}));
    for path in ["acceptance-report-v4.json", "acceptance-report-v5.json"] {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/contracts")
                .join(path),
        )
        .unwrap();
        let report: AcceptanceVerificationReport = serde_json::from_slice(&bytes).unwrap();
        report.validate(&report.checker).unwrap();
        records.push(json!({"id":path,"valid":true,"report":report,"checker":report.checker}));
    }
    records
}
