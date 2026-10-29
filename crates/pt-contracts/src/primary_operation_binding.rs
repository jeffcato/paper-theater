//! Primary-transcription provenance binding. Runtime persistence grants authority.
//!
//! Binds a primary audio-only transcription record to its committed R19
//! operation and recorded result without issuing new work. Forged, stale or
//! missing bindings are rejected; production approval stays pending.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimaryProvenanceV1 {
    pub schema_version: u32,
    pub operation_attempt_id: Id,
    pub operation_request_sha256: Digest,
    pub primary_request: ArtifactRef,
    pub result_response_sha256: Option<Digest>,
}

impl PrimaryProvenanceV1 {
    /// Independent shared-verifier entry point: the named operation attempt
    /// and request must be the committed ones, the recorded result must
    /// itself validate for that operation, the bound response must be the
    /// recorded one, and the supplied primary bytes must match the bound
    /// request artifact exactly. Verifies, never issues or dispatches.
    pub fn validate_for_operation(
        &self,
        op: &ProviderOperationV2,
        result: &ProviderResultV2,
        primary_bytes: &[u8],
    ) -> ContractResult<()> {
        require(
            self.schema_version == 1,
            "version",
            "schema_version",
            "expected primary provenance version 1",
        )?;
        id(
            &self.operation_attempt_id,
            "provenance.operation_attempt_id",
        )?;
        hash(
            &self.operation_request_sha256,
            "provenance.operation_request_sha256",
        )?;
        artifact(&self.primary_request)?;
        self.primary_request
            .file
            .verify(primary_bytes)
            .map_err(|_| ContractError {
                code: "primary_binding".into(),
                field: "primary_request".into(),
                message: "primary request bytes differ from the bound artifact".into(),
            })?;
        require(
            self.operation_attempt_id == op.operation.attempt_id,
            "primary_binding",
            "operation_attempt_id",
            "primary record names an uncommitted operation attempt",
        )?;
        require(
            self.operation_request_sha256 == op.operation.request_sha256,
            "primary_binding",
            "operation_request_sha256",
            "primary record binds a stale operation request",
        )?;
        result.validate_for_operation(op)?;
        require(
            result.result.response_sha256 == self.result_response_sha256,
            "primary_binding",
            "result_response_sha256",
            "primary record binds a stale or forged recorded response",
        )?;
        Ok(())
    }
}
