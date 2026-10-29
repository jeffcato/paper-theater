//! Exact native exchange identities. This record grants no dispatch/retry authority.
use crate::*;
use serde::{Deserialize, Serialize};

pub const MAX_NATIVE_PROVIDER_RECORD_BYTES: usize = 64 * 1024;
pub const MAX_NATIVE_PROVIDER_WIRE_BYTES: u64 = 1024 * 1024;
pub const MAX_NATIVE_PROVIDER_RAW_BYTES: u64 = 12 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderNativeExchangePolicyV1 {
    pub schema_version: u32,
    pub issued_operation: FileRef,
    pub codec_sha256: Digest,
    pub origin_id: Id,
    pub path: String,
    pub wire_request: FileRef,
    pub max_raw_response_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderNativeExchangeV1 {
    pub schema_version: u32,
    pub issued_operation: FileRef,
    pub codec_sha256: Digest,
    pub origin_id: Id,
    pub path: String,
    pub wire_request: FileRef,
    pub raw_response: Option<ArtifactRef>,
}

/// Actual bytes supplied by runtime's verified private reads, never JSON flags.
pub struct ProviderNativeExchangeBytesV1<'a> {
    pub issued_operation: &'a [u8],
    pub logical_request: &'a [u8],
    pub wire_request: &'a [u8],
    pub raw_response: Option<&'a [u8]>,
}

impl ProviderNativeExchangeV1 {
    /// Runtime selects policy/issued/result independently from durable records.
    /// Byte validation is not proof of provider execution, listening or acceptance.
    pub fn validate_bytes(
        &self,
        policy: &ProviderNativeExchangePolicyV1,
        issued: &ProviderOperationV2,
        result: &ProviderResultV2,
        bytes: ProviderNativeExchangeBytesV1<'_>,
    ) -> ContractResult<()> {
        issued.validate()?;
        result.validate_for_operation(issued)?;
        require(
            issued.recovery.is_none(),
            "native_exchange",
            "issued_operation.recovery",
            "existing-job queries use the separate recovery boundary; never a native submission",
        )?;
        require(
            self.schema_version == 1 && policy.schema_version == 1,
            "native_exchange",
            "schema_version",
            "expected 1",
        )?;
        require(
            serde_json::to_vec(self).is_ok_and(|b| b.len() <= MAX_NATIVE_PROVIDER_RECORD_BYTES)
                && serde_json::to_vec(policy)
                    .is_ok_and(|b| b.len() <= MAX_NATIVE_PROVIDER_RECORD_BYTES),
            "native_exchange",
            "bounds",
            "native metadata remains bounded to 64 KiB",
        )?;
        require(
            valid_digest(&self.codec_sha256),
            "native_exchange",
            "codec_sha256",
            "expected pinned codec digest",
        )?;
        crate::provider_metadata::id(&self.origin_id, "origin_id")?;
        require(
            self.path.starts_with('/')
                && !self.path.starts_with("//")
                && self.path.len() <= 2048
                && self
                    .path
                    .bytes()
                    .all(|b| (33..=126).contains(&b) && !b"?#%\\".contains(&b))
                && self.path.split('/').all(|s| s != "." && s != ".."),
            "native_exchange",
            "path",
            "fixed relative provider path required; credentials and redirects are not metadata",
        )?;
        require(
            self.issued_operation.bytes <= 64 * 1024
                && self.wire_request.bytes <= 1024 * 1024
                && self.issued_operation.media_type == "application/json"
                && self.wire_request.media_type == "application/json"
                && policy.max_raw_response_bytes > 0
                && policy.max_raw_response_bytes <= 12 * 1024 * 1024
                && (policy.max_raw_response_bytes <= 1024 * 1024
                    || issued.operation.kind == OperationKind::Speech
                        && issued.operation.provider == "gemini"),
            "native_exchange",
            "bounds",
            "bounded metadata/request and selected native-response role required",
        )?;
        require(
            self.issued_operation == policy.issued_operation
                && self.codec_sha256 == policy.codec_sha256
                && self.origin_id == policy.origin_id
                && self.path == policy.path
                && self.wire_request == policy.wire_request,
            "native_exchange",
            "policy",
            "native exchange differs from independently selected pins",
        )?;
        self.issued_operation.verify(bytes.issued_operation)?;
        let decoded: ProviderOperationV2 =
            serde_json::from_slice(bytes.issued_operation).map_err(|e| ContractError {
                code: "native_exchange".into(),
                field: "issued_operation".into(),
                message: e.to_string(),
            })?;
        require(
            serde_json::to_value(decoded).unwrap() == serde_json::to_value(issued).unwrap(),
            "native_exchange",
            "issued_operation",
            "original issued operation differs",
        )?;
        issued
            .operation
            .payload
            .file
            .verify(bytes.logical_request)?;
        self.wire_request.verify(bytes.wire_request)?;
        require(
            serde_json::to_value(&self.raw_response).unwrap()
                == serde_json::to_value(&result.result.raw_response).unwrap(),
            "native_exchange",
            "raw_response",
            "original raw response identity differs",
        )?;
        match (&self.raw_response, bytes.raw_response) {
            (Some(raw), Some(actual)) => {
                require(
                    raw.file.bytes <= policy.max_raw_response_bytes,
                    "native_exchange",
                    "raw_response",
                    "native response role exceeds selected cap",
                )?;
                raw.file.verify(actual)?;
            }
            (None, None) => {}
            _ => {
                return invalid(
                    "native_exchange",
                    "raw_response",
                    "raw response bytes missing or undeclared",
                );
            }
        }
        Ok(())
    }
}
