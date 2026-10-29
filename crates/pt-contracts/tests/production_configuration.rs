use pt_contracts::*;
use serde_json::json;

#[path = "support/production_configuration_cases.rs"]
mod cases;
use cases::configuration;

#[test]
fn shared_configuration_cases_match_the_public_validator() {
    for row in cases::records() {
        let valid = serde_json::from_value::<ProductionConfigurationV1>(row["value"].clone())
            .ok()
            .is_some_and(|v| v.validate().is_ok());
        assert_eq!(valid, row["valid"].as_bool().unwrap(), "{}", row["id"]);
    }
    for row in cases::proposal_records() {
        let config: ProductionConfigurationV1 =
            serde_json::from_value(row["configuration"].clone()).unwrap();
        let input: StageInput = serde_json::from_value(row["input"].clone()).unwrap();
        let proposal: ProviderProposalV2 = serde_json::from_value(row["proposal"].clone()).unwrap();
        assert_eq!(
            config
                .validate_proposal(row["scope_id"].as_str().unwrap(), &input, &proposal)
                .is_ok(),
            row["valid"].as_bool().unwrap(),
            "{}",
            row["id"]
        );
    }
}

#[test]
fn administrative_configuration_retains_exact_request_and_accounting_identities() {
    let value = configuration();
    let config: ProductionConfigurationV1 = serde_json::from_value(value.clone()).unwrap();
    config.validate().unwrap();
    assert_eq!(serde_json::to_value(config).unwrap(), value);
}

#[test]
fn configuration_rejects_missing_history_and_widened_operation_scope() {
    let mut no_history = configuration();
    no_history["accounting_original_sha256s"] = json!([]);
    assert!(
        serde_json::from_value::<ProductionConfigurationV1>(no_history)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut widened = configuration();
    widened["operations"][0]["limits"]["requests"] = json!(3);
    assert!(
        serde_json::from_value::<ProductionConfigurationV1>(widened)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut relabeled = configuration();
    relabeled["operations"][0]["stage"] = json!("speech");
    assert!(
        serde_json::from_value::<ProductionConfigurationV1>(relabeled)
            .unwrap()
            .validate()
            .is_err()
    );
}

#[test]
fn configured_operation_matches_exact_stage_request_and_limits_without_issuing() {
    let config: ProductionConfigurationV1 = serde_json::from_value(configuration()).unwrap();
    let op = &config.operations[0];
    let mut input = StageInput {
        schema_version: 1,
        run_id: "run".into(),
        stage: Stage::Pilot,
        unit_id: op.unit_id.clone(),
        epoch: 1,
        revision: 0,
        inputs: vec![],
        pins: config.pins.clone(),
        mode: ExecutionMode::Live,
        limits: op.limits.clone(),
        library_revision: 0,
    };
    let payload = ArtifactRef {
        id: "committed-request".into(),
        provenance: Provenance::Diagnostic,
        file: FileRef {
            path: "private/request.json".into(),
            sha256: op.request_sha256.clone(),
            bytes: 13,
            media_type: "application/json".into(),
            dimensions: None,
        },
    };
    let mut proposal = ProviderProposalV2 {
        schema_version: 2,
        kind: op.kind,
        provider: op.context.provider.clone(),
        model: op.model.clone(),
        recipe_sha256: op.recipe_sha256.clone(),
        payload,
        inputs: vec![],
        estimate: Usage {
            requests: 1,
            input_tokens: 30,
            output_tokens: 20,
            money_microusd: 10,
        },
        timeout_ms: 1000,
    };
    config.validate_proposal(&op.id, &input, &proposal).unwrap();
    proposal.payload.file.sha256 = digest(b"substituted-request");
    assert!(config.validate_proposal(&op.id, &input, &proposal).is_err());
    proposal.payload.file.sha256 = op.request_sha256.clone();
    input.stage = Stage::Speech;
    assert!(config.validate_proposal(&op.id, &input, &proposal).is_err());
    input.stage = Stage::Pilot;
    proposal.estimate.input_tokens = 101;
    assert!(config.validate_proposal(&op.id, &input, &proposal).is_err());
}
