use super::*;
use serde_json::{Value, json};

fn put(
    input: &mut StageInput,
    services: &mut OfflineServices,
    path: &str,
    bytes: Vec<u8>,
) -> ArtifactRef {
    let a = ArtifactRef {
        id: format!("committed:test:{}", path.replace('/', ":")),
        file: FileRef {
            path: path.into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            media_type: if path.ends_with(".txt") {
                "text/plain"
            } else {
                "application/json"
            }
            .into(),
            dimensions: None,
        },
        provenance: Provenance::Synthetic,
    };
    input.inputs.retain(|v| v.file.path != path);
    input.inputs.push(a.clone());
    services.inputs.insert(path.into(), bytes);
    a
}
fn output(services: &OfflineServices, path: &str) -> Value {
    let writes = services.writes.lock().unwrap();
    parse(
        &writes
            .iter()
            .rev()
            .find(|(c, _)| c.file.path == path)
            .expect(path)
            .1,
    )
    .unwrap()
}
fn export_case(label: &str, input: &StageInput, services: &OfflineServices, out: &StageOutput) {
    if let Ok(destination) = std::env::var("PT_DIRECTION_EXPORT") {
        assert!(safe_path(&destination) && destination.starts_with("fixtures/generation/"));
        let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = project.join(&destination).join(label);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("stage-input.json"),
            super::super::json(input).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.join("stage-output.json"),
            super::super::json(out).unwrap(),
        )
        .unwrap();
        let mut index = vec![];
        for a in &input.inputs {
            let borrowed = if label == "oz-source" {
                Some("pg55")
            } else if label == "wells-source" {
                Some("pg36")
            } else {
                None
            };
            let original = borrowed.and_then(|book| match a.file.path.as_str() {
                "raw-source.txt" => Some(format!("fixtures/generation/source/{book}-raw.txt")),
                "clean-source.txt" => Some(format!("fixtures/generation/source/{book}-clean.txt")),
                "frozen-source.json" => {
                    Some(format!("fixtures/generation/source/{book}-frozen.json"))
                }
                _ => None,
            });
            let source = if let Some(path) = original {
                a.file
                    .verify(&std::fs::read(project.join(&path)).unwrap())
                    .unwrap();
                path
            } else {
                let path = root.join("inputs").join(&a.file.path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, &services.inputs[&a.file.path]).unwrap();
                format!("{destination}/{label}/inputs/{}", a.file.path)
            };
            index.push(json!({"artifact":a,"source":source}));
        }
        std::fs::write(
            root.join("input-index.json"),
            super::super::json(&index).unwrap(),
        )
        .unwrap();
        for (c, b) in services.writes.lock().unwrap().iter() {
            let path = root.join("candidates").join(&c.file.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b).unwrap();
        }
    }
}
async fn fixture(dialogue: bool) -> (StageInput, OfflineServices) {
    let (mut input, mut services) = super::fixture();
    input.inputs.clear();
    services.inputs.clear();
    services.writes.lock().unwrap().clear();
    input.stage = Stage::Source;
    input.library_revision = 1;
    let text = if dialogue {
        "Mira said, “Open the gate.” Mira moved to the gate."
    } else {
        "Mira watched the rivière. The water was still."
    };
    let n = text.chars().count();
    let frozen = json!({"schema_version":1,"book_id":"test-book","selector":{"book_id":"test-book","opening":text,"ending":text},"raw_sha256":digest(text.as_bytes()),"clean_sha256":digest(text.as_bytes()),"source_sha256":digest(text.as_bytes()),"clean_span":[0,n],"raw_span":[0,n],"text":text,"scalar_to_raw":(0..n).map(|i|[i,i+1]).collect::<Vec<_>>()});
    for (path, b) in [
        ("raw-source.txt", text.as_bytes().to_vec()),
        ("clean-source.txt", text.as_bytes().to_vec()),
        ("frozen-source.json", super::super::json(&frozen).unwrap()),
    ] {
        put(&mut input, &mut services, path, b);
    }
    let source = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    assert_eq!(source.outcome, StageOutcome::Complete);
    // Explicit runtime double commit; production generation never converts a CandidateFile to ArtifactRef.
    let b = services
        .writes
        .lock()
        .unwrap()
        .iter()
        .find(|(c, _)| c.file.path == "source.json")
        .unwrap()
        .1
        .clone();
    let source = put(&mut input, &mut services, "source.json", b);
    let registry = json!({"schema_version":1,"book_id":"test-book","revision":1,"entities":[{"id":"test-book:narrator","name":"Narrator","aliases":["Narrator"],"sprite":null,"voice":null},{"id":"test-book:mira","name":"Mira","aliases":["Mira"],"sprite":null,"voice":null}]});
    let registry = put(
        &mut input,
        &mut services,
        "private/book-registry.json",
        super::super::json(&registry).unwrap(),
    );
    let recipe = put(
        &mut input,
        &mut services,
        "private/direction-recipe.json",
        br#"{"schema_version":1,"status":"synthetic direction recipe v1"}"#.to_vec(),
    );
    input.pins.recipe_sha256 = recipe.file.sha256.clone();
    let ctx = json!({"schema_version":1,"book_id":"test-book","chapter_id":"chapter-1","source":source.file,"registry":registry.file,"recipe":recipe.file,"provider":"provider-a","model":"model-a","estimate":{"requests":1,"input_tokens":100,"output_tokens":20,"money_microusd":0},"timeout_ms":30000});
    put(
        &mut input,
        &mut services,
        "private/direction-context.json",
        super::super::json(&ctx).unwrap(),
    );
    let request = json!({"schema_version":1,"task":"source-backed-direction-v1","book_id":"test-book","chapter_id":"chapter-1","source":{"artifact":ctx["source"],"sha256":digest(text.as_bytes()),"span":[0,n],"text":text},"registry":{"file":ctx["registry"],"snapshot":json!({"schema_version":1,"book_id":"test-book","revision":1,"entities":[{"id":"test-book:narrator","name":"Narrator","aliases":["Narrator"],"sprite":null,"voice":null},{"id":"test-book:mira","name":"Mira","aliases":["Mira"],"sprite":null,"voice":null}]})},"recipe":ctx["recipe"],"response_schema":"direction-plan-v1","response_json_schema":parse::<Value>(include_bytes!("../../../../../fixtures/generation/direction-v1/response-schema.json")).unwrap(),"constraints":["verbatim-prose-and-unicode-scalar-spans","mentioned-characters-are-not-speakers","concrete-source-backed-pictures-and-actions","canonical-per-book-identities","initial-reference-research-required","no-final-timing-voices-images-or-acceptance-authority"]});
    let request = put(
        &mut input,
        &mut services,
        "private/direction-request.json",
        super::super::json(&request).unwrap(),
    );
    let chars = text.chars().collect::<Vec<_>>();
    let quote =
        |a: usize, b: usize| json!({"span":[a,b],"quote":chars[a..b].iter().collect::<String>()});
    let turns = if dialogue {
        let a = chars.iter().position(|c| *c == '“').unwrap();
        let b = chars.iter().position(|c| *c == '”').unwrap() + 1;
        json!([{"id":"t0","span":[0,a],"text":chars[..a].iter().collect::<String>(),"speaker":"narrator","kind":"narration","attribution":null},{"id":"t1","span":[a,b],"text":chars[a..b].iter().collect::<String>(),"speaker":"mira","kind":"dialogue","attribution":quote(0,b)},{"id":"t2","span":[b,n],"text":chars[b..].iter().collect::<String>(),"speaker":"narrator","kind":"narration","attribution":null}])
    } else {
        json!([{"id":"t0","span":[0,n],"text":text,"speaker":"narrator","kind":"narration","attribution":null}])
    };
    let support = if dialogue {
        let a = chars.iter().rposition(|c| *c == 'M').unwrap();
        quote(a, n)
    } else {
        quote(0, n)
    };
    let plan = json!({"schema_version":1,"book_id":"test-book","chapter_id":"chapter-1","source_sha256":digest(text.as_bytes()),"request_sha256":request.file.sha256,"cast":[{"id":"narrator","name":"Narrator","canonical_id":"test-book:narrator","kind":"narrator","mentions":[]},{"id":"mira","name":"Mira","canonical_id":"test-book:mira","kind":"character","mentions":[quote(0,4)]}],"turns":turns,"beats":[{"id":"b0","span":[0,n],"text":text,"depiction":"picture","description":"Recognizable Mira beside the source-backed setting.","subjects":["mira"],"fact_ids":["f0"],"action_ids":if dialogue{vec!["a0"]}else{vec![]}}],"scenes":[{"id":"s0","span":[0,n],"text":text,"pictures":[{"id":"p0","span":[0,n],"beat_ids":["b0"],"subjects":["mira"],"description":"Mira remains clearly visible in a concrete setting."}]}],"facts":[{"id":"f0","entity_ids":["mira"],"assertion":support["quote"],"support":support}],"actions":if dialogue{json!([{"id":"a0","actor_id":"mira","kind":"move","assertion":support["quote"],"support":support}])}else{json!([])},"references":[{"id":"r0","subject_type_id":"test-book:mira","entity_ids":["mira"],"support":quote(0,4),"features":["recognizable subject"],"usage":"subject"}],"questions":["Initial reference research and voice pilot remain unavailable."]});
    let response = put(
        &mut input,
        &mut services,
        "private/direction-response.json",
        super::super::json(&plan).unwrap(),
    );
    let observation = json!({"schema_version":1,"provenance":"synthetic","source_sha256":digest(text.as_bytes()),"recipe_sha256":recipe.file.sha256,"provider":"provider-a","model":"model-a","request":request,"response":response,"issued":null,"result":null,"findings":[]});
    put(
        &mut input,
        &mut services,
        "private/direction-observation.json",
        super::super::json(&observation).unwrap(),
    );
    input.stage = Stage::Direction;
    services.writes.lock().unwrap().clear();
    (input, services)
}

#[tokio::test]
async fn saved_direction_compiles_verbatim_dialogue_and_narration_without_dispatch() {
    for dialogue in [true, false] {
        let (input, services) = fixture(dialogue).await;
        let out = GenerationExecutor
            .execute(input.clone(), &services)
            .await
            .unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        let plan = output(&services, "private/direction-plan.json");
        assert_eq!(
            plan["source_text"],
            if dialogue {
                "Mira said, “Open the gate.” Mira moved to the gate."
            } else {
                "Mira watched the rivière. The water was still."
            }
        );
        assert_eq!(
            plan["speakers"],
            if dialogue {
                json!(["mira", "narrator"])
            } else {
                json!(["narrator"])
            }
        );
        assert_eq!(plan["production_approved"], false);
        assert!(output(&services,"private/direction-request.json")["response_json_schema"]["properties"]["cast"].is_object());
        assert_eq!(
            plan["reference_requirements"][0]["research_outcome"],
            "missing"
        );
        assert_eq!(
            plan["reference_requirements"][0]["approved_for_asset_request"],
            false
        );
        assert!(out.package_candidate.is_none());
        assert!(out.library_updates.is_empty());
        assert_eq!(services.emits.load(Ordering::SeqCst), 0);
        export_case(
            if dialogue {
                "dialogue"
            } else {
                "narration-only"
            },
            &input,
            &services,
            &out,
        );
    }
}

fn remove(input: &mut StageInput, services: &mut OfflineServices, path: &str) {
    input.inputs.retain(|a| a.file.path != path);
    services.inputs.remove(path);
}
#[tokio::test]
async fn direction_request_requires_runtime_commit_and_default_v2_unavailable_stays_unresolved() {
    let (mut input, mut services) = fixture(true).await;
    remove(
        &mut input,
        &mut services,
        "private/direction-observation.json",
    );
    remove(&mut input, &mut services, "private/direction-request.json");
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    export_case("request-phase", &input, &services, &first);
    assert!(
        first
            .findings
            .iter()
            .any(|f| f.id == "direction-request-commit-pending")
    );
    assert!(
        !first
            .artifacts
            .iter()
            .any(|c| c.file.path == "private/direction-provider-proposal.json")
    );
    let (candidate, bytes) = services
        .writes
        .lock()
        .unwrap()
        .iter()
        .find(|(c, _)| c.file.path == "private/direction-request.json")
        .cloned()
        .unwrap();
    // Runtime double commits the candidate; generation is only handed the resulting ArtifactRef.
    let committed = put(&mut input, &mut services, &candidate.file.path, bytes);
    input.mode = ExecutionMode::Live;
    services.writes.lock().unwrap().clear();
    let next = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(next.outcome, StageOutcome::NeedsRepair);
    assert!(
        next.findings
            .iter()
            .any(|f| f.id == "direction-provider-v2-unavailable")
    );
    let proposal: ProviderProposalV2 = serde_json::from_value(output(
        &services,
        "private/direction-provider-proposal.json",
    ))
    .unwrap();
    proposal.validate().unwrap();
    assert_eq!(proposal.payload.id, committed.id);
    let wire = serde_json::to_value(proposal).unwrap();
    for field in [
        "run_id",
        "epoch",
        "attempt_id",
        "credential_context_id",
        "repair",
    ] {
        assert!(wire.get(field).is_none());
    }
}

struct ProviderDouble {
    base: OfflineServices,
    observer: ProviderObserverV2,
    response: ArtifactRef,
    issue: AtomicUsize,
    execute: AtomicUsize,
    fault: &'static str,
}
#[async_trait]
impl RunServices for ProviderDouble {
    async fn read_artifact(&self, a: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        self.base.read_artifact(a).await
    }
    async fn write_candidate(
        &self,
        r: CandidateWrite,
        b: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        self.base.write_candidate(r, b).await
    }
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        self.base.check_cancelled().await
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        panic!("Direction must not emit journal effects")
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("legacy provider forbidden")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("helper fallback forbidden")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("pinned registry required")
    }
    async fn issue_provider_v2(
        &self,
        p: ProviderProposalV2,
    ) -> Result<ProviderOperationV2, ServiceError> {
        self.issue.fetch_add(1, Ordering::SeqCst);
        p.validate().unwrap();
        let fixture: Value = parse(include_bytes!(
            "../../../../../fixtures/contracts/provider-metadata-v2.json"
        ))
        .unwrap();
        let mut issued: ProviderOperationV2 = serde_json::from_value(
            fixture
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == "free-money-request")
                .unwrap()["wire"]
                .clone(),
        )
        .unwrap();
        issued.operation = ProviderOperation {
            attempt_id: "attempt-1".into(),
            unit_id: self.observer.unit_id.clone(),
            kind: p.kind,
            provider: p.provider,
            model: p.model,
            recipe_sha256: p.recipe_sha256,
            request_sha256: p.payload.file.sha256.clone(),
            payload: p.payload,
            inputs: p.inputs,
            estimate: p.estimate,
            timeout_ms: p.timeout_ms,
        };
        issued.identity.run_id = self.observer.run_id.clone();
        issued.identity.epoch = self.observer.epoch;
        match self.fault {
            "issued-model" => issued.operation.model = "other-model".into(),
            "issued-epoch" => issued.identity.epoch += 1,
            "issued-request" => issued.operation.request_sha256 = "0".repeat(64),
            "issued-payload" => issued.operation.payload.id = "foreign-payload".into(),
            _ => {}
        }
        Ok(issued)
    }
    async fn execute_provider_v2(
        &self,
        issued: &ProviderOperationV2,
    ) -> Result<ProviderResultV2, ServiceError> {
        self.execute.fetch_add(1, Ordering::SeqCst);
        issued.validate().unwrap();
        if self.fault == "unavailable-execution" {
            return Err(ServiceError::ProviderV2Unavailable);
        }
        let mut result = ProviderResultV2 {
            schema_version: 2,
            result: ProviderResult {
                attempt_id: issued.operation.attempt_id.clone(),
                completion: if self.fault == "ambiguous" {
                    ProviderCompletion::Ambiguous
                } else {
                    ProviderCompletion::Completed
                },
                request_sha256: issued.operation.request_sha256.clone(),
                response_sha256: Some(self.response.file.sha256.clone()),
                raw_response: Some(self.response.clone()),
                outputs: vec![],
                usage: Some(issued.operation.estimate.clone()),
                started_at: "synthetic-test-start".into(),
                elapsed_ms: 1,
                diagnostic: "Injected test result; no provider transport.".into(),
            },
            context: issued.context.clone(),
            identity: issued.identity.clone(),
            quota_windows: issued.quota_windows.clone(),
            observed_usage: ObservedProviderUsage {
                requests: Some(1),
                tokens: Some(120),
                input_tokens: Some(100),
                output_tokens: Some(20),
                money_microusd: Some(0),
            },
            accounted_usage: QuotaAmounts {
                money_microusd: 0,
                requests: 1,
                tokens: 120,
                repairs: 0,
            },
            started_at_ms: 1000,
            recorded_at_ms: 1001,
            epoch_disposition: if self.fault == "obsolete" {
                ProviderEpochDisposition::Obsolete
            } else {
                ProviderEpochDisposition::Current
            },
            retry: None,
            recovery: None,
        };
        for window in &mut result.quota_windows {
            window.accounted = result.accounted_usage.clone();
        }
        match self.fault {
            "result-request" => result.result.request_sha256 = "0".repeat(64),
            "result-context" => result.context.project_id = "foreign-project".into(),
            "result-response" => result.result.response_sha256 = Some("0".repeat(64)),
            _ => {}
        }
        Ok(result)
    }
}
async fn provider_fixture(fault: &'static str) -> (StageInput, ProviderDouble) {
    let (mut input, mut base) = fixture(true).await;
    remove(&mut input, &mut base, "private/direction-observation.json");
    input.mode = ExecutionMode::Live;
    let response = input
        .inputs
        .iter()
        .find(|a| a.file.path == "private/direction-response.json")
        .unwrap()
        .clone();
    let observer = ProviderObserverV2 {
        schema_version: 2,
        run_id: input.run_id.clone(),
        epoch: input.epoch,
        unit_id: input.unit_id.clone(),
    };
    (
        input,
        ProviderDouble {
            base,
            observer,
            response,
            issue: AtomicUsize::new(0),
            execute: AtomicUsize::new(0),
            fault,
        },
    )
}

#[tokio::test]
async fn issued_provider_double_validates_identity_and_replays_compiled_bytes_without_redispatch() {
    let (mut input, mut services) = provider_fixture("").await;
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    assert_eq!(first.outcome, StageOutcome::NeedsRepair);
    export_case("provider-double", &input, &services.base, &first);
    let plan = output(&services.base, "private/direction-plan.json");
    let observation = output(&services.base, "private/direction-observation.json");
    let observation_bytes = services
        .base
        .writes
        .lock()
        .unwrap()
        .iter()
        .find(|(c, _)| c.file.path == "private/direction-observation.json")
        .unwrap()
        .1
        .clone();
    assert_eq!(plan["observation"]["sha256"], digest(&observation_bytes));
    assert_eq!(observation["result"]["accounted_usage"]["tokens"], 120);
    let originals = services.base.writes.lock().unwrap().clone();
    for (c, b) in &originals {
        put(&mut input, &mut services.base, &c.file.path, b.clone());
    }
    input.mode = ExecutionMode::Replay;
    services.base.writes.lock().unwrap().clear();
    let again = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(first.cache_keys, again.cache_keys);
    for (c, b) in originals
        .iter()
        .filter(|(c, _)| c.file.path != "private/direction-observation.json")
    {
        let actual = services
            .base
            .writes
            .lock()
            .unwrap()
            .iter()
            .find(|(a, _)| a.file.path == c.file.path)
            .unwrap()
            .1
            .clone();
        assert_eq!(*b, actual, "{}", c.file.path);
    }
    assert_eq!(services.issue.load(Ordering::SeqCst), 1);
    assert_eq!(services.execute.load(Ordering::SeqCst), 1);
}

fn replace_plan(
    input: &mut StageInput,
    services: &mut OfflineServices,
    change: impl FnOnce(&mut Value),
) {
    let mut plan: Value = parse(&services.inputs["private/direction-response.json"]).unwrap();
    change(&mut plan);
    let response = put(
        input,
        services,
        "private/direction-response.json",
        super::super::json(&plan).unwrap(),
    );
    let mut obs: Value = parse(&services.inputs["private/direction-observation.json"]).unwrap();
    obs["response"] = json!(response);
    put(
        input,
        services,
        "private/direction-observation.json",
        super::super::json(&obs).unwrap(),
    );
}
#[tokio::test]
async fn missing_duplicate_reordered_prose_and_invented_dialogue_remain_unresolved() {
    for fault in [
        "missing",
        "duplicate",
        "reorder",
        "invented",
        "unsupported-fact",
        "unsupported-action",
        "quote-action",
        "canonical",
        "reference",
    ] {
        let (mut input, mut services) = fixture(true).await;
        replace_plan(&mut input, &mut services, |p| match fault {
            "missing" => {
                p["turns"].as_array_mut().unwrap().pop();
            }
            "duplicate" => {
                let t = p["turns"][1].clone();
                p["turns"].as_array_mut().unwrap().insert(1, t);
            }
            "reorder" => {
                p["turns"].as_array_mut().unwrap().swap(0, 1);
            }
            "invented" => p["turns"][1]["text"] = json!("“I can fly.”"),
            "unsupported-fact" => {
                p["facts"][0]["assertion"] = json!("Mira flies above the clouds.")
            }
            "unsupported-action" => p["actions"][0]["kind"] = json!("hop"),
            "quote-action" => {
                p["actions"][0]["support"] =
                    json!({"span":p["turns"][1]["span"],"quote":p["turns"][1]["text"]});
                p["actions"][0]["assertion"] = p["turns"][1]["text"].clone();
            }
            "canonical" => p["cast"][1]["canonical_id"] = json!("replacement-mira"),
            "reference" => p["references"][0]["entity_ids"] = json!(["unrelated-subject"]),
            _ => unreachable!(),
        });
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(
            out.findings
                .iter()
                .any(|f| matches!(f.outcome, ReviewOutcome::Fail | ReviewOutcome::Uncertain)),
            "{fault}"
        );
        let plan = output(&services, "private/direction-plan.json");
        assert_eq!(plan["production_approved"], false);
        if fault == "missing" {
            assert!(
                !plan["coverage"]["turns"]["missing_source_spans"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        if fault == "invented" {
            assert!(
                !plan["speakers"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("mira"))
            );
        }
        if fault == "canonical" {
            assert_eq!(plan["cast"][1]["retained_registry_identity"], Value::Null);
        }
    }
}
#[tokio::test]
async fn saved_direction_replay_retains_exact_bytes_findings_and_canonical_bindings() {
    let (mut input, mut services) = fixture(true).await;
    let canonical = put(
        &mut input,
        &mut services,
        "private/canonical-voice.json",
        br#"{"voice_id":"synthetic-canonical-voice","approved":false}"#.to_vec(),
    );
    let mut registry: Value = parse(&services.inputs["private/book-registry.json"]).unwrap();
    registry["entities"][1]["voice"] = json!(canonical);
    let registry = put(
        &mut input,
        &mut services,
        "private/book-registry.json",
        super::super::json(&registry).unwrap(),
    );
    let mut ctx: Value = parse(&services.inputs["private/direction-context.json"]).unwrap();
    ctx["registry"] = json!(registry.file);
    put(
        &mut input,
        &mut services,
        "private/direction-context.json",
        super::super::json(&ctx).unwrap(),
    );
    // A changed pinned registry requires a newly compiled request/response, never silent cached reuse.
    remove(&mut input, &mut services, "private/direction-request.json");
    remove(
        &mut input,
        &mut services,
        "private/direction-observation.json",
    );
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let request_bytes = services
        .writes
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(c, _)| c.file.path == "private/direction-request.json")
        .unwrap()
        .1
        .clone();
    let request = put(
        &mut input,
        &mut services,
        "private/direction-request.json",
        request_bytes,
    );
    let mut raw: Value = parse(&services.inputs["private/direction-response.json"]).unwrap();
    raw["request_sha256"] = json!(request.file.sha256);
    let response = put(
        &mut input,
        &mut services,
        "private/direction-response.json",
        super::super::json(&raw).unwrap(),
    );
    let observation = json!({"schema_version":1,"provenance":"synthetic","source_sha256":raw["source_sha256"],"recipe_sha256":input.pins.recipe_sha256,"provider":"provider-a","model":"model-a","request":request,"response":response,"issued":null,"result":null,"findings":[{"id":"older-direction-failure","field":"private.review","outcome":"fail","message":"original unresolved direction defect","evidence":[]}]});
    let obs_bytes = super::super::json(&observation).unwrap();
    put(
        &mut input,
        &mut services,
        "private/direction-observation.json",
        obs_bytes.clone(),
    );
    services.writes.lock().unwrap().clear();
    let first = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    assert!(
        first
            .findings
            .iter()
            .any(|f| f.id == "older-direction-failure" && f.outcome == ReviewOutcome::Fail)
    );
    export_case("canonical-reuse", &input, &services, &first);
    assert_eq!(
        output(&services, "private/direction-plan.json")["cast"][1]["retained_registry_identity"]["voice"]
            ["file"]["sha256"],
        canonical.file.sha256
    );
    let original = services.writes.lock().unwrap().clone();
    for (c, b) in &original {
        put(&mut input, &mut services, &c.file.path, b.clone());
    }
    input.mode = ExecutionMode::Replay;
    services.writes.lock().unwrap().clear();
    let again = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(first.cache_keys, again.cache_keys);
    assert_eq!(
        services.inputs["private/direction-observation.json"],
        obs_bytes
    );
    for (c, b) in original {
        assert_eq!(
            services
                .writes
                .lock()
                .unwrap()
                .iter()
                .find(|(a, _)| a.file.path == c.file.path)
                .unwrap()
                .1,
            b
        );
    }
}

#[tokio::test]
async fn provider_v2_identity_faults_fail_closed_and_uncertain_results_never_become_plans() {
    for fault in [
        "issued-model",
        "issued-epoch",
        "issued-request",
        "issued-payload",
        "result-request",
        "result-context",
        "result-response",
    ] {
        let (input, services) = provider_fixture(fault).await;
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{fault}"
        );
        assert!(services.base.writes.lock().unwrap().is_empty());
        if fault.starts_with("issued") {
            assert_eq!(services.execute.load(Ordering::SeqCst), 0);
        }
    }
    for fault in ["ambiguous", "obsolete", "unavailable-execution"] {
        let (input, services) = provider_fixture(fault).await;
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(
            !out.artifacts
                .iter()
                .any(|c| c.file.path == "private/direction-plan.json")
        );
        assert!(output(&services.base, "private/direction-observation.json")["issued"].is_object());
    }
    for mode in [ExecutionMode::Fixture, ExecutionMode::Replay] {
        let (mut input, services) = provider_fixture("").await;
        input.mode = mode;
        let out = GenerationExecutor.execute(input, &services).await.unwrap();
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert_eq!(services.issue.load(Ordering::SeqCst), 0);
        assert_eq!(services.execute.load(Ordering::SeqCst), 0);
    }
}

async fn selected_source_fixture(name: &str) -> (StageInput, OfflineServices) {
    let (mut input, mut services) = fixture(false).await;
    input.inputs.clear();
    services.inputs.clear();
    services.writes.lock().unwrap().clear();
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generation/source");
    for (alias, suffix) in [
        ("raw-source.txt", "raw.txt"),
        ("clean-source.txt", "clean.txt"),
        ("frozen-source.json", "frozen.json"),
    ] {
        put(
            &mut input,
            &mut services,
            alias,
            std::fs::read(root.join(format!("{name}-{suffix}"))).unwrap(),
        );
    }
    input.stage = Stage::Source;
    let result = GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    assert_eq!(result.outcome, StageOutcome::Complete);
    let bytes = services
        .writes
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(c, _)| c.file.path == "source.json")
        .unwrap()
        .1
        .clone();
    let source = put(&mut input, &mut services, "source.json", bytes);
    let frozen: Value = parse(&services.inputs["frozen-source.json"]).unwrap();
    let book = frozen["book_id"].as_str().unwrap();
    let chapter = if name == "pg36" {
        "book-one-chapter-1"
    } else {
        "chapter-1"
    };
    let registry=put(&mut input,&mut services,"private/book-registry.json",super::super::json(&json!({"schema_version":1,"book_id":book,"revision":1,"entities":[{"id":format!("{book}:narrator"),"name":"Narrator","aliases":["Narrator"],"sprite":null,"voice":null}]})).unwrap());
    let recipe=put(&mut input,&mut services,"private/direction-recipe.json",br#"{"schema_version":1,"status":"synthetic saved-response direction recipe; not a production model execution"}"#.to_vec());
    input.pins.recipe_sha256 = recipe.file.sha256.clone();
    let ctx = json!({"schema_version":1,"book_id":book,"chapter_id":chapter,"source":source.file,"registry":registry.file,"recipe":recipe.file,"provider":"provider-a","model":"model-a","estimate":{"requests":1,"input_tokens":100,"output_tokens":20,"money_microusd":0},"timeout_ms":30000});
    put(
        &mut input,
        &mut services,
        "private/direction-context.json",
        super::super::json(&ctx).unwrap(),
    );
    input.stage = Stage::Direction;
    services.writes.lock().unwrap().clear();
    GenerationExecutor
        .execute(input.clone(), &services)
        .await
        .unwrap();
    let request_bytes = services
        .writes
        .lock()
        .unwrap()
        .iter()
        .find(|(c, _)| c.file.path == "private/direction-request.json")
        .unwrap()
        .1
        .clone();
    let request = put(
        &mut input,
        &mut services,
        "private/direction-request.json",
        request_bytes,
    );
    let text = frozen["text"].as_str().unwrap();
    let n = text.chars().count();
    let response = json!({"schema_version":1,"book_id":book,"chapter_id":chapter,"source_sha256":frozen["source_sha256"],"request_sha256":request.file.sha256,"cast":[{"id":"narrator","name":"Narrator","canonical_id":format!("{book}:narrator"),"kind":"narrator","mentions":[]}],"turns":[{"id":"t0","span":[0,n],"text":text,"speaker":"narrator","kind":"narration","attribution":null}],"beats":[{"id":"b0","span":[0,n],"text":text,"depiction":"omission","description":"Complete source retained; detailed cast/beat/action planning remains unresolved.","subjects":[],"fact_ids":[],"action_ids":[]}],"scenes":[{"id":"s0","span":[0,n],"text":text,"pictures":[]}],"facts":[],"actions":[],"references":[],"questions":["Synthetic structural response only; real source-backed creative planning and research remain unresolved."]});
    let response = put(
        &mut input,
        &mut services,
        "private/direction-response.json",
        super::super::json(&response).unwrap(),
    );
    let observation = json!({"schema_version":1,"provenance":"synthetic","source_sha256":frozen["source_sha256"],"recipe_sha256":recipe.file.sha256,"provider":"provider-a","model":"model-a","request":request,"response":response,"issued":null,"result":null,"findings":[]});
    put(
        &mut input,
        &mut services,
        "private/direction-observation.json",
        super::super::json(&observation).unwrap(),
    );
    services.writes.lock().unwrap().clear();
    (input, services)
}
#[tokio::test]
async fn selected_oz_and_wells_source_identities_and_all_prose_remain_frozen() {
    for (name, hash, n) in [
        (
            "pg55",
            "1d8c591969dd338fd8f251e3b83d991a76b5a7e5c715b503953c5ce4775ec320",
            5958,
        ),
        (
            "pg36",
            "d1cea729eaca946ead7c174aa09d0bc664570bb264f46846dea046d1d3b24ef2",
            12654,
        ),
    ] {
        let (input, services) = selected_source_fixture(name).await;
        let out = GenerationExecutor
            .execute(input.clone(), &services)
            .await
            .unwrap();
        let plan = output(&services, "private/direction-plan.json");
        assert_eq!(plan["source_sha256"], hash);
        assert_eq!(plan["source_text"].as_str().unwrap().chars().count(), n);
        assert_eq!(plan["coverage"]["turns"]["ordered_verbatim_complete"], true);
        assert_eq!(out.outcome, StageOutcome::NeedsRepair);
        assert!(out.findings.iter().any(|f| f.id == "beat-omission:b0"));
        export_case(
            if name == "pg55" {
                "oz-source"
            } else {
                "wells-source"
            },
            &input,
            &services,
            &out,
        );
    }
}

#[tokio::test]
async fn partial_names_cannot_invent_a_speaker_or_rebind_a_canonical_character() {
    let (mut input, mut services) = fixture(true).await;
    replace_plan(&mut input, &mut services, |p| {
        p["cast"][1]["name"] = json!("Mir")
    });
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    let plan = output(&services, "private/direction-plan.json");
    assert_eq!(plan["cast"][1]["source_citation_valid"], false);
    assert!(
        !plan["speakers"]
            .as_array()
            .unwrap()
            .contains(&json!("mira"))
    );
    assert!(
        out.findings
            .iter()
            .any(|f| f.id == "canonical-identity:mira")
    );
}

#[tokio::test]
async fn unrelated_attribution_citation_cannot_assign_a_quoted_performance() {
    let (mut input, mut services) = fixture(true).await;
    replace_plan(&mut input, &mut services, |p| {
        p["turns"][1]["attribution"] = json!({"span":[0,9],"quote":"Mira said"})
    });
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert!(
        !output(&services, "private/direction-plan.json")["speakers"]
            .as_array()
            .unwrap()
            .contains(&json!("mira"))
    );
    assert!(out.findings.iter().any(|f| f.id == "speaker-source:t1"));
}

#[tokio::test]
async fn mentioned_characters_cannot_become_narrators_to_create_voice_variety() {
    let (mut input, mut services) = fixture(false).await;
    replace_plan(&mut input, &mut services, |p| {
        p["cast"][1]["kind"] = json!("narrator");
        p["turns"][0]["speaker"] = json!("mira");
    });
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert!(
        !output(&services, "private/direction-plan.json")["speakers"]
            .as_array()
            .unwrap()
            .contains(&json!("mira"))
    );
    assert!(out.findings.iter().any(|f| f.id == "narrator-role"));
}

#[tokio::test]
async fn stale_source_recipe_model_request_response_and_malformed_claims_are_rejected() {
    for fault in [
        "source-bytes",
        "recipe-bytes",
        "response-bytes",
        "model",
        "recipe",
        "request-id",
        "response-id",
        "chapter",
        "authority",
        "malformed",
    ] {
        let (mut input, mut services) = fixture(true).await;
        match fault {
            "source-bytes" => services.inputs.get_mut("raw-source.txt").unwrap()[0] ^= 1,
            "recipe-bytes" => {
                services
                    .inputs
                    .get_mut("private/direction-recipe.json")
                    .unwrap()[0] ^= 1
            }
            "response-bytes" => {
                services
                    .inputs
                    .get_mut("private/direction-response.json")
                    .unwrap()[0] ^= 1
            }
            "chapter" => replace_plan(&mut input, &mut services, |p| {
                p["chapter_id"] = json!("unrelated-chapter")
            }),
            "authority" => {
                let mut c: Value =
                    parse(&services.inputs["private/direction-context.json"]).unwrap();
                c["attempt_id"] = json!("caller-attempt");
                put(
                    &mut input,
                    &mut services,
                    "private/direction-context.json",
                    super::super::json(&c).unwrap(),
                );
            }
            "malformed" => {
                let r = put(
                    &mut input,
                    &mut services,
                    "private/direction-response.json",
                    b"{".to_vec(),
                );
                let mut o: Value =
                    parse(&services.inputs["private/direction-observation.json"]).unwrap();
                o["response"] = json!(r);
                put(
                    &mut input,
                    &mut services,
                    "private/direction-observation.json",
                    super::super::json(&o).unwrap(),
                );
            }
            _ => {
                let mut o: Value =
                    parse(&services.inputs["private/direction-observation.json"]).unwrap();
                match fault {
                    "model" => o["model"] = json!("unrelated-model"),
                    "recipe" => o["recipe_sha256"] = json!("0".repeat(64)),
                    "request-id" => o["request"]["id"] = json!("foreign-request"),
                    "response-id" => o["response"]["id"] = json!("foreign-response"),
                    _ => unreachable!(),
                };
                put(
                    &mut input,
                    &mut services,
                    "private/direction-observation.json",
                    super::super::json(&o).unwrap(),
                );
            }
        }
        assert!(
            GenerationExecutor.execute(input, &services).await.is_err(),
            "{fault}"
        );
        assert!(services.writes.lock().unwrap().is_empty());
    }
}
struct NoReads;
#[async_trait]
impl RunServices for NoReads {
    async fn check_cancelled(&self) -> Result<(), ServiceError> {
        Ok(())
    }
    async fn read_artifact(&self, _: &ArtifactRef) -> Result<Vec<u8>, ServiceError> {
        panic!("oversized Direction envelope read payload")
    }
    async fn write_candidate(
        &self,
        _: CandidateWrite,
        _: Vec<u8>,
    ) -> Result<CandidateFile, ServiceError> {
        panic!("write")
    }
    async fn emit(&self, _: JournalProposal) -> Result<(), ServiceError> {
        panic!("emit")
    }
    async fn provider(&self, _: ProviderOperation) -> Result<ProviderResult, ServiceError> {
        panic!("provider")
    }
    async fn helper(&self, _: HelperOperation) -> Result<HelperResult, ServiceError> {
        panic!("helper")
    }
    async fn library(&self, _: LibraryQuery) -> Result<Vec<LibraryBinding>, ServiceError> {
        panic!("library")
    }
}
#[tokio::test]
async fn oversized_direction_responses_and_envelopes_reject_before_service_reads() {
    for fault in ["response", "context-mime", "file", "count"] {
        let (mut input, _services) = fixture(true).await;
        match fault {
            "response" => {
                input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "private/direction-response.json")
                    .unwrap()
                    .file
                    .bytes = 1024 * 1024 + 1
            }
            "context-mime" => {
                let f = &mut input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "private/direction-context.json")
                    .unwrap()
                    .file;
                f.bytes = 1024 * 1024 + 1;
                f.media_type = "text/plain".into();
            }
            "file" => {
                input
                    .inputs
                    .iter_mut()
                    .find(|a| a.file.path == "raw-source.txt")
                    .unwrap()
                    .file
                    .bytes = 16 * 1024 * 1024 + 1
            }
            "count" => {
                let a = input.inputs[0].clone();
                for i in 0..64 {
                    let mut b = a.clone();
                    b.id = format!("excess:{i}");
                    b.file.path = format!("private/excess-{i}.json");
                    input.inputs.push(b);
                }
            }
            _ => unreachable!(),
        }
        assert!(
            GenerationExecutor.execute(input, &NoReads).await.is_err(),
            "{fault}"
        );
    }
}

#[tokio::test]
async fn committed_observation_binds_a_model_response_without_a_self_referential_request_hash() {
    let (mut input, mut services) = fixture(false).await;
    replace_plan(&mut input, &mut services, |p| {
        p.as_object_mut().unwrap().remove("request_sha256");
    });
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert_eq!(out.outcome, StageOutcome::NeedsRepair);
    assert_eq!(
        output(&services, "private/direction-plan.json")["coverage"]["turns"]["ordered_verbatim_complete"],
        true
    );
}

#[tokio::test]
async fn direction_candidates_remain_private_even_when_aliased_as_a_public_image() {
    let (input, services) = fixture(true).await;
    let out = GenerationExecutor.execute(input, &services).await.unwrap();
    assert!(
        out.artifacts
            .iter()
            .all(|c| c.file.path.starts_with("private/"))
    );
    let (mut package, mut target) = super::fixture();
    let mut secret = None;
    for (c, b) in services.writes.lock().unwrap().iter() {
        let committed = put(&mut package, &mut target, &c.file.path, b.clone());
        if c.file.path == "private/direction-plan.json" {
            secret = Some(committed.file);
        }
    }
    let secret = secret.unwrap();
    let b = target.inputs[&secret.path].clone();
    let mut alias = put(&mut package, &mut target, "assets/direction.svg", b);
    alias.file.media_type = "image/svg+xml".into();
    package
        .inputs
        .iter_mut()
        .find(|a| a.file.path == alias.file.path)
        .unwrap()
        .file = alias.file.clone();
    let mut doc: PlaybackDocument = parse(&target.inputs["fixture.json"]).unwrap();
    doc.manifest.assets.push(alias.file);
    put(
        &mut package,
        &mut target,
        "fixture.json",
        super::super::json(&doc).unwrap(),
    );
    let result = GenerationExecutor
        .execute(package, &target)
        .await
        .unwrap()
        .package_candidate
        .unwrap();
    assert!(
        result
            .payload_inventory
            .iter()
            .all(|f| !f.path.starts_with("private/") && f.sha256 != secret.sha256)
    );
}
