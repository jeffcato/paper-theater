//! Execution-bound v2 methods. Successful construction is available only to offline tests.
use super::{
    provider_v2::{Broker, OfflineTransport},
    store::*,
};
use pt_contracts as c;
use std::sync::Arc;
#[derive(Clone)]
pub(super) struct Adapter {
    broker: Broker,
    transport: Arc<dyn OfflineTransport>,
}
pub(super) fn error(e: Error) -> c::ServiceError {
    match e.code {
        "stale_epoch" => c::ServiceError::Cancelled,
        "live_disabled" | "recovery_unavailable" => c::ServiceError::ProviderV2Unavailable,
        "budget_exhausted" | "request_quota" | "token_quota" | "repair_exhausted"
        | "limits_unknown" | "limits_changed" | "limits_expired" | "provider_wait" => {
            c::ServiceError::Limit(e.code.into())
        }
        "provider_binding"
        | "registry_conflict"
        | "unit_conflict"
        | "operation_conflict"
        | "issued_descriptor_mismatch"
        | "recovery_conflict"
        | "recovery_unknown"
        | "retry_lineage"
        | "repair_lineage"
        | "repair_conflict"
        | "attempt_unresolved"
        | "accounting_pending"
        | "accounting_unbound" => c::ServiceError::Unauthorized(e.code.into()),
        _ => c::ServiceError::Internal(e.code.into()),
    }
}
fn denied() -> c::ServiceError {
    c::ServiceError::Unauthorized("artifact is outside this execution's immutable inputs".into())
}
fn projection(unit: &str, p: &c::ProviderProposalV2) -> c::ProviderOperation {
    c::ProviderOperation {
        attempt_id: "runtime-proposal".into(),
        unit_id: unit.into(),
        kind: p.kind,
        provider: p.provider.clone(),
        model: p.model.clone(),
        recipe_sha256: p.recipe_sha256.clone(),
        request_sha256: p.payload.file.sha256.clone(),
        payload: p.payload.clone(),
        inputs: p.inputs.clone(),
        estimate: p.estimate.clone(),
        timeout_ms: p.timeout_ms,
    }
}
fn proposed(op: &c::ProviderOperationV2) -> c::ProviderProposalV2 {
    let p = &op.operation;
    c::ProviderProposalV2 {
        schema_version: 2,
        kind: p.kind,
        provider: p.provider.clone(),
        model: p.model.clone(),
        recipe_sha256: p.recipe_sha256.clone(),
        payload: p.payload.clone(),
        inputs: p.inputs.clone(),
        estimate: p.estimate.clone(),
        timeout_ms: p.timeout_ms,
    }
}
fn allowed(
    inputs: &[c::ArtifactRef],
    p: &c::ProviderProposalV2,
) -> std::result::Result<(), c::ServiceError> {
    p.validate()?;
    for a in std::iter::once(&p.payload).chain(&p.inputs) {
        if !inputs
            .iter()
            .any(|old| serde_json::to_value(old).unwrap() == serde_json::to_value(a).unwrap())
        {
            return Err(denied());
        }
    }
    Ok(())
}
impl Adapter {
    #[cfg(test)]
    pub(super) fn injected(broker: Broker, transport: Arc<dyn OfflineTransport>) -> Self {
        Self { broker, transport }
    }
    pub(super) fn issue(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        inputs: &[c::ArtifactRef],
        p: &c::ProviderProposalV2,
    ) -> std::result::Result<c::ProviderOperationV2, c::ServiceError> {
        allowed(inputs, p)?;
        let projected = projection(unit, p);
        self.broker
            .verify_service_content(run, epoch, unit, &projected)
            .map_err(error)?;
        let issued = self
            .broker
            .bind_service_proposal(run, epoch, unit, &projected)
            .map_err(error)?;
        issued.validate_for_proposal(
            p,
            &c::ProviderObserverV2 {
                schema_version: 2,
                run_id: run.into(),
                epoch: epoch as u64,
                unit_id: unit.into(),
            },
        )?;
        Ok(issued)
    }
    pub(super) async fn execute(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        inputs: &[c::ArtifactRef],
        issued: &c::ProviderOperationV2,
    ) -> std::result::Result<c::ProviderResultV2, c::ServiceError> {
        issued.validate()?;
        if issued.identity.run_id != run
            || issued.identity.epoch != epoch as u64
            || issued.operation.unit_id != unit
            || issued.recovery.is_some()
        {
            return Err(c::ServiceError::Unauthorized(
                "issued operation differs from this execution or selects recovery".into(),
            ));
        }
        allowed(inputs, &proposed(issued))?;
        self.broker
            .verify_service_content(run, epoch, unit, &issued.operation)
            .map_err(error)?;
        let broker = self.broker.clone();
        let transport = self.transport.clone();
        let operation = issued.clone();
        // Keep bounded late receipt/accounting work alive if the stage future is dropped.
        let result = tokio::spawn(async move {
            broker
                .execute_issued_provider(&operation, transport.as_ref())
                .await
        })
        .await
        .map_err(|_| c::ServiceError::Internal("provider worker failed".into()))?
        .map_err(error)?;
        result.validate_for_operation(issued)?;
        self.broker.verify_result_bytes(&result).map_err(error)?;
        Ok(result)
    }
    pub(super) async fn recover(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        request: c::ProviderRecoveryRequestV2,
    ) -> std::result::Result<c::ProviderRecoveryResultV2, c::ServiceError> {
        request.validate()?;
        self.broker
            .verify_observer(run, epoch, unit, &request.original)
            .map_err(error)?;
        let observer = c::ProviderObserverV2 {
            schema_version: 2,
            run_id: run.into(),
            epoch: epoch as u64,
            unit_id: unit.into(),
        };
        let broker = self.broker.clone();
        let transport = self.transport.clone();
        let scope = observer.clone();
        let operation = request.original.clone();
        let receipt = tokio::spawn(async move {
            broker
                .recover_existing_receipt(
                    &scope.run_id,
                    scope.epoch as i64,
                    &scope.unit_id,
                    &operation,
                    transport.as_ref(),
                )
                .await
        })
        .await
        .map_err(|_| c::ServiceError::Internal("query worker failed".into()))?
        .map_err(error)?;
        receipt.validate_for_request(&request, &observer)?;
        self.broker
            .verify_result_bytes(&receipt.original)
            .map_err(error)?;
        Ok(receipt)
    }
}
