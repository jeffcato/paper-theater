//! Verify one recorded bounded chapter unit; runtime registration/fanout is deliberately separate.
use super::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
const FILE: u64 = 16 * 1024 * 1024;
const META: u64 = 1024 * 1024;
const TOTAL: u64 = 64 * 1024 * 1024;
const OUTPUT: &str = "chapter-acoustic-validation.json";
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
fn recipe() -> Digest {
    digest(
        &[
            include_bytes!("../../../../tools/generation/chapter_helper.py").as_slice(),
            include_bytes!("../../../../tools/generation/chapter_acoustic.py").as_slice(),
        ]
        .concat(),
    )
}
fn need(ok: bool, message: &str) -> Result<(), StageError> {
    require(ok, "chapter_bundle", "evidence", message)?;
    Ok(())
}
fn number(v: &Value) -> Result<u64, StageError> {
    let n = v
        .as_u64()
        .ok_or_else(|| StageError::Evidence("integer chapter coordinate required".into()))?;
    need(n <= MAX_SAFE_INTEGER, "unsafe chapter integer")?;
    Ok(n)
}
fn interval(v: &Value) -> Result<[u64; 2], StageError> {
    let a = v
        .as_array()
        .ok_or_else(|| StageError::Evidence("chapter interval required".into()))?;
    need(a.len() == 2, "chapter interval length")?;
    Ok([number(&a[0])?, number(&a[1])?])
}
fn file<'a>(files: &'a Files, path: &str) -> Result<&'a (FileRef, Vec<u8>), StageError> {
    files
        .get(path)
        .ok_or_else(|| StageError::Evidence(format!("missing chapter input: {path}")))
}
fn array(v: &Value) -> Result<&Vec<Value>, StageError> {
    v.as_array()
        .ok_or_else(|| StageError::Evidence("chapter list required".into()))
}
fn header(rate: u64, frames: u64) -> Result<Vec<u8>, StageError> {
    need(
        (8000..=192000).contains(&rate) && frames > 0 && frames <= (256 * 1024 * 1024 - 44) / 2,
        "chapter PCM domain",
    )?;
    let mut b = vec![];
    b.extend(b"RIFF");
    b.extend((36 + frames as u32 * 2).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend((rate as u32).to_le_bytes());
    b.extend((rate as u32 * 2).to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend((frames as u32 * 2).to_le_bytes());
    Ok(b)
}
fn observations(items: &[Value], [a, b]: [u64; 2], offset: u64) -> Result<Vec<Value>, StageError> {
    let mut rows = vec![];
    for item in items {
        let x = number(&item["start_sample"])? + offset;
        let y = number(&item["end_sample"])? + offset;
        if (x == y && a <= x && x < b) || (x < y && x < b && a < y) {
            rows.push(json!({"original":item,"global_samples":[x,y],"local_samples":[a.max(x)-a,b.min(y)-a],"complete":a<=x && y<=b}));
        }
    }
    Ok(rows)
}
fn facts(
    index: &Value,
    original: &Value,
    present: &BTreeSet<String>,
    complete: bool,
) -> Result<Vec<Value>, StageError> {
    let mut rows = vec![];
    let offset = number(&index["evidence_scope"][0])?;
    for fact in array(&original["facts"])? {
        let mut required = BTreeSet::new();
        let mut bound = Value::Null;
        let mut outcome = fact["outcome"].clone();
        let mut reason = "original_fact_unresolved";
        if outcome == "pass" {
            for key in array(&fact["interval_ids"])? {
                let i = array(&original["intervals"])?
                    .iter()
                    .find(|i| i["id"] == *key)
                    .ok_or_else(|| {
                        StageError::Evidence("global supporting interval missing".into())
                    })?;
                need(
                    i["outcome"] == "pass",
                    "global supporting interval unresolved",
                )?;
                let a = number(&i["primary_start_sample"])?
                    .min(number(&i["independent_start_sample"])?)
                    + offset;
                let b = number(&i["primary_end_sample"])?
                    .max(number(&i["independent_end_sample"])?)
                    + offset;
                let mut cursor = a;
                for c in array(&index["chunks"])? {
                    let [x, y] = interval(&c["frames"])?;
                    if x < b && a < y {
                        need(x <= cursor, "supporting chunk gap")?;
                        required.insert(c["id"].as_str().unwrap().to_owned());
                        cursor = b.min(y);
                    }
                }
                need(cursor == b, "global fact outside chapter")?;
            }
            if !complete {
                outcome = json!("uncertain");
                reason = "chapter_global_evidence_incomplete";
            } else if !required.is_subset(present) {
                outcome = json!("missing");
                reason = "cross_cut_evidence_missing";
            } else {
                bound = json!(number(&fact["not_before_sample"])? + offset);
                reason = "all_global_supporting_chunks_verified";
            }
        }
        rows.push(json!({"original":fact,"required_chunks":required,"present_chunks":required.intersection(present).cloned().collect::<Vec<_>>(),"outcome":outcome,"chapter_not_before_sample":bound,"reason":reason}));
    }
    Ok(rows)
}
fn originals(input: &StageInput, files: &Files) -> Result<(Value, Vec<Finding>), StageError> {
    acoustic::verify_recorded(files)?;
    let original = acoustic_facts::evaluate(
        input,
        files,
        &file(files, "acoustic-replay.json")?.1,
        &file(files, "acoustic-replay.json")?.0,
    )?
    .ok_or_else(|| StageError::Evidence("global fact inventory missing".into()))?;
    need(
        json(&original.report)? == file(files, "fact-evidence.json")?.1,
        "stale global fact evidence",
    )?;
    let summary: Value = parse(&file(files, "acoustic-validation.json")?.1)?;
    need(
        summary["schema_version"] == 1
            && summary["stage"] == "acoustic"
            && summary["provenance"] == "diagnostic"
            && summary["production_approved"] == false
            && summary["provider_calls"] == 0
            && summary["source_sha256"] == original.report["source_sha256"]
            && summary["narration_sha256"] == original.report["narration_sha256"]
            && summary["fact_matching"]
                == serde_json::to_value(&file(files, "fact-evidence.json")?.0).unwrap()
            && summary["normalized_report_sha256"] == file(files, "acoustic-replay.json")?.0.sha256
            && summary["source_facts_sha256"] == original.report["source_facts_sha256"]
            && summary["fact_matching_recipe_sha256"] == original.report["matching_recipe_sha256"],
        "stale global findings receipt",
    )?;
    let findings: Vec<Finding> = serde_json::from_value(summary["findings"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    Ok((original.report, findings))
}
fn verify(input: &StageInput, files: &Files) -> Result<(Value, Vec<Finding>), StageError> {
    let (original, mut findings) = originals(input, files)?;
    let normalized: Value = parse(&file(files, "acoustic-replay.json")?.1)?;
    let unit: Value = parse(&file(files, "chapter-unit.json")?.1)?;
    let index: Value = parse(&file(files, "chapter-index.json")?.1)?;
    let recorded: Value = parse(&file(files, "chapter-helper-record.json")?.1)?;
    need(
        index["schema_version"] == 1
            && index["provenance"] == "diagnostic"
            && index["production_approved"] == false
            && index["recipe_sha256"] == recipe()
            && unit["schema_version"] == 1
            && unit["recipe_sha256"] == recipe()
            && unit["index"]
                == serde_json::to_value(&file(files, "chapter-index.json")?.0).unwrap(),
        "chapter recipe/index binding",
    )?;
    let rate = number(&index["sample_rate"])?;
    let frames = number(&index["frame_count"])?;
    let chapter: FileRef = serde_json::from_value(index["narration"].clone())
        .map_err(|e| StageError::Evidence(e.to_string()))?;
    chapter.validate()?;
    need(
        chapter.media_type == "audio/wav" && chapter.dimensions.is_none(),
        "chapter narration descriptor",
    )?;
    let head = header(rate, frames)?;
    let hex = head.iter().map(|b| format!("{b:02x}")).collect::<String>();
    need(
        index["header_hex"] == hex
            && number(&index["narration"]["bytes"])? == 44 + frames * 2
            && index["channels"] == 1
            && index["bits_per_sample"] == 16,
        "chapter native format",
    )?;
    let scope = interval(&index["evidence_scope"])?;
    let source = file(files, "excerpt.wav")?;
    need(
        scope[0] < scope[1]
            && scope[1] <= frames
            && source.1.len() as u64 == 44 + 2 * (scope[1] - scope[0])
            && source.1[..44] == header(rate, scope[1] - scope[0])?,
        "original narration scope",
    )?;
    need(
        file(files, "chapter-source.txt")?.1 == file(files, "source.txt")?.1
            && index["source"]
                == serde_json::to_value(&file(files, "chapter-source.txt")?.0).unwrap(),
        "chapter source binding",
    )?;
    let complete = scope == [0, frames] && index["narration"]["sha256"] == source.0.sha256;
    need(
        index["complete_original_scope"] == complete,
        "unproved chapter-wide evidence scope",
    )?;
    let manifest: Value = parse(&file(files, "manifest.json")?.1)?;
    need(
        index["source_origin"] == manifest["source_span_characters"][0],
        "source origin mismatch",
    )?;
    let expected_units=array(&manifest["units"])?.iter().map(|u|Ok(json!({"id":format!("turn-{:03}",number(&u["turn"])?),"source_span":u["source_span_characters"],"frames":[scope[0]+number(&u["start_sample"])?,scope[0]+number(&u["end_sample"])?]}))).collect::<Result<Vec<Value>,StageError>>()?;
    need(
        index["source_units"] == json!(expected_units),
        "source unit/global offset mismatch",
    )?;
    let receipt_paths = files
        .keys()
        .filter(|p| {
            (p.starts_with("transcribe/") || p.starts_with("meta/")) && p.ends_with(".receipt.json")
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut listed = BTreeSet::new();
    for r in array(&index["receipts"])? {
        let p = r["path"]
            .as_str()
            .ok_or_else(|| StageError::Evidence("receipt path missing".into()))?;
        need(listed.insert(p.to_owned()), "duplicate receipt identity")?;
        let actual = file(files, p)?;
        let raw: Value = parse(&actual.1)?;
        need(
            r["sha256"] == actual.0.sha256
                && ["model", "request_sha256", "response_sha256", "audio_sha256"]
                    .iter()
                    .all(|k| r[*k] == raw[*k])
                && r["provider_recipe_sha256"] == raw["harness_sha256"],
            "original receipt identity changed",
        )?;
    }
    need(
        listed == receipt_paths,
        "incomplete original receipt inventory",
    )?;
    let mut chunks = BTreeMap::new();
    let mut cursor = 0;
    let mut paths = BTreeSet::new();
    need(
        !array(&index["chunks"])?.is_empty() && array(&index["chunks"])?.len() <= 1024,
        "chunk count",
    )?;
    for c in array(&index["chunks"])? {
        let [a, b] = interval(&c["frames"])?;
        let id = c["id"]
            .as_str()
            .ok_or_else(|| StageError::Evidence("chunk id missing".into()))?;
        let f: FileRef = serde_json::from_value(c["file"].clone())
            .map_err(|e| StageError::Evidence(e.to_string()))?;
        need(
            valid_id(id)
                && a == cursor
                && a < b
                && b <= frames
                && chunks.insert(id.to_owned(), c).is_none()
                && paths.insert(f.path.clone())
                && f.bytes == 44 + (b - a) * 2
                && f.bytes <= FILE
                && f.media_type == "audio/wav",
            "ordered complete chapter chunks",
        )?;
        f.validate()?;
        let contexts=expected_units.iter().filter_map(|u| {let [x,y]=interval(&u["frames"]).unwrap();(x<b && a<y).then(||json!({"unit_id":u["id"],"source_span":u["source_span"],"global_frames":[x,y],"local_frames":[x.max(a)-a,y.min(b)-a],"complete":a<=x && y<=b}))}).collect::<Vec<_>>();
        need(
            c["source_contexts"] == json!(contexts),
            "chunk source contexts changed",
        )?;
        if complete {
            let data = &source.1[44 + a as usize * 2..44 + b as usize * 2];
            let mut hash = sha2::Sha256::new();
            use sha2::Digest as _;
            hash.update(header(rate, b - a)?);
            hash.update(data);
            need(
                f.sha256 == format!("{:x}", hash.finalize()) && c["pcm_sha256"] == digest(data),
                "global chunk identity changed",
            )?;
        }
        cursor = b;
    }
    need(cursor == frames, "chapter coverage incomplete")?;
    let mut expected_files = BTreeMap::new();
    for v in array(&index["context_files"])? {
        let f: FileRef =
            serde_json::from_value(v.clone()).map_err(|e| StageError::Evidence(e.to_string()))?;
        need(
            file(files, &f.path)?.0 == f && expected_files.insert(f.path.clone(), f).is_none(),
            "context file binding",
        )?;
    }
    expected_files.insert(
        "chapter-index.json".into(),
        file(files, "chapter-index.json")?.0.clone(),
    );
    let mut present = BTreeSet::new();
    for id in array(&unit["included_chunks"])? {
        let id = id
            .as_str()
            .ok_or_else(|| StageError::Evidence("included chunk id missing".into()))?;
        need(present.insert(id.to_owned()), "duplicate included chunk")?;
        let c = chunks
            .get(id)
            .ok_or_else(|| StageError::Evidence("unknown included chunk".into()))?;
        let f: FileRef = serde_json::from_value(c["file"].clone()).unwrap();
        let actual = file(files, &f.path)?;
        let [a, b] = interval(&c["frames"])?;
        need(
            actual.0 == f
                && actual.1[..44] == header(rate, b - a)?
                && digest(&actual.1[44..]) == c["pcm_sha256"],
            "chunk PCM changed",
        )?;
        let lo = a.max(scope[0]);
        let hi = b.min(scope[1]);
        if lo < hi {
            need(
                actual.1[44 + (lo - a) as usize * 2..44 + (hi - a) as usize * 2]
                    == source.1
                        [44 + (lo - scope[0]) as usize * 2..44 + (hi - scope[0]) as usize * 2],
                "original PCM placement changed",
            )?;
        }
        expected_files.insert(f.path.clone(), f);
    }
    let base = unit["base_chunk"]
        .as_str()
        .ok_or_else(|| StageError::Evidence("base chunk missing".into()))?;
    need(present.contains(base), "base chunk not supplied")?;
    let actual_files = array(&unit["files"])?
        .iter()
        .map(|v| {
            serde_json::from_value::<FileRef>(v.clone())
                .map_err(|e| StageError::Evidence(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    need(
        actual_files == expected_files.values().cloned().collect::<Vec<_>>(),
        "consumed file allowlist mismatch",
    )?;
    let chunk = chunks[base];
    let span = interval(&chunk["frames"])?;
    let words = observations(array(&normalized["word_track"])?, span, scope[0])?;
    let groups = observations(array(&normalized["highlight_groups"])?, span, scope[0])?;
    let mut rows = facts(&index, &original, &present, complete)?;
    let checked = actual_files
        .iter()
        .map(|f| (f.path.clone(), f.sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    let expected = json!({"schema_version":1,"provenance":"diagnostic","production_approved":false,"provider_calls":0,"outcome":"needs_repair","recipe_sha256":recipe(),"index_sha256":file(files,"chapter-index.json")?.0.sha256,"unit_sha256":file(files,"chapter-unit.json")?.0.sha256,"chapter_sha256":index["narration"]["sha256"],"source_sha256":index["source"]["sha256"],"base_chunk":base,"global_frames":span,"checked_files":checked,"verified_chunks":array(&unit["included_chunks"])?,"words":words,"groups":groups,"facts":rows,"reveal_permissions":[],"scope":"Derived chunk observations; full global transcripts retained. Original provider receipts are not chunk responses."});
    need(recorded == expected, "recorded helper verification changed")?;
    for row in &mut rows {
        let id = row["original"]["id"].as_str().unwrap();
        if let Some(previous) = findings
            .iter()
            .find(|f| f.id == format!("acoustic-fact:{id}") && f.outcome != ReviewOutcome::Pass)
        {
            row["outcome"] = serde_json::to_value(previous.outcome).unwrap();
            row["chapter_not_before_sample"] = Value::Null;
            row["reason"] = json!("recorded_adverse_fact_finding");
        }
    }
    for row in &rows {
        findings.push(Finding {
            id: format!("chapter-fact:{}", row["original"]["id"].as_str().unwrap()),
            field: "acoustic.chapter".into(),
            outcome: serde_json::from_value(row["outcome"].clone()).unwrap(),
            message: row["reason"].as_str().unwrap().into(),
            evidence: input.inputs.clone(),
        });
    }
    findings.push(missing("chapter-helper-registration-pending","Recorded chapter verification only; runtime must register the new recipe and own bounded fanout."));
    let bindings = files
        .iter()
        .filter(|(p, _)| p.as_str() != OUTPUT)
        .map(|(p, (f, _))| (p.clone(), f.clone()))
        .collect::<BTreeMap<_, _>>();
    Ok((
        json!({"schema_version":1,"provenance":"diagnostic","production_approved":false,"provider_calls":0,"helper_calls":0,"outcome":"needs_repair","recipe_sha256":recipe(),"implementation_sha256":digest(include_bytes!("acoustic_bundle.rs")),"bindings":bindings,"chapter_sha256":index["narration"]["sha256"],"base_chunk":base,"global_frames":span,"word_count":array(&normalized["word_track"])?.len(),"highlight_group_count":array(&normalized["highlight_groups"])?.len(),"point_word_count":array(&normalized["word_track"])?.iter().filter(|w|w["start_sample"]==w["end_sample"]).count(),"words":words,"groups":groups,"facts":rows,"reveal_permissions":[],"registration":"pending"}),
        findings,
    ))
}
pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
) -> Result<StageOutput, StageError> {
    need(
        input.inputs.len() <= 64
            && input.inputs.iter().all(|a| a.file.bytes <= FILE)
            && input.inputs.iter().map(|a| a.file.bytes).sum::<u64>() <= TOTAL,
        "bounded chapter input envelope",
    )?;
    for a in &input.inputs {
        if [
            "chapter-unit.json",
            "chapter-index.json",
            "chapter-helper-record.json",
            OUTPUT,
        ]
        .contains(&a.file.path.as_str())
        {
            need(a.file.bytes <= META, "bounded chapter metadata/output")?;
        }
    }
    let loaded = inputs::load(input, services).await?;
    if !loaded.files.contains_key("chapter-helper-record.json") {
        let (_, mut findings) = originals(input, &loaded.files)?;
        findings.push(missing(
            "chapter-helper-registration-pending",
            "A saved bounded helper record is required; unregistered helper dispatch is disabled.",
        ));
        return Ok(StageOutput {
            outcome: StageOutcome::NeedsRepair,
            artifacts: vec![],
            decisions: vec![],
            findings,
            cache_keys: vec![recipe()],
            library_updates: vec![],
            package_candidate: None,
        });
    }
    let (report, findings) = verify(input, &loaded.files)?;
    let bytes = json(&report)?;
    need(
        bytes.len() as u64 <= META,
        "chapter validation output bound",
    )?;
    if input.mode == ExecutionMode::Replay
        && let Some((previous, _)) = loaded.files.get(OUTPUT)
    {
        need(
            previous.sha256 == digest(&bytes) && previous.bytes == bytes.len() as u64,
            "stale recorded chapter validation",
        )?;
    }
    let candidate = write(services, input, OUTPUT, &report).await?;
    services.check_cancelled().await?;
    Ok(StageOutput {
        outcome: StageOutcome::NeedsRepair,
        artifacts: vec![candidate],
        decisions: vec![],
        findings,
        cache_keys: vec![recipe(), digest(&bytes)],
        library_updates: vec![],
        package_candidate: None,
    })
}
