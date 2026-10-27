//! Private source-coordinate relationships. Descriptors never authenticate execution.
use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FullSourcePinV1 {
    pub file: FileRef,
    pub scope_id: Id,
    pub scalar_count: u64,
    pub origin: u64,
}
impl FullSourcePinV1 {
    pub fn validate(&self) -> ContractResult<()> {
        self.file.validate()?;
        require(
            valid_id(&self.scope_id)
                && self.scalar_count > 0
                && self.file.bytes <= 32 * 1024 * 1024
                && self
                    .origin
                    .checked_add(self.scalar_count)
                    .is_some_and(|n| n <= MAX_SAFE_INTEGER),
            "source_slice",
            "full_source",
            "invalid full-source identity/count/origin/bounds",
        )
    }
}
/// Runtime-owned expected pins. The base Source role and all playback/fact spans remain local.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceVerificationPolicyV2 {
    pub schema_version: u32,
    pub base: AcceptanceVerificationPolicy,
    pub full_source: FullSourcePinV1,
    pub full_source_span: SourceSpan,
    pub relationship: Option<FileRef>,
}
impl AcceptanceVerificationPolicyV2 {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 2,
            "version",
            "policy.schema_version",
            "expected 2",
        )?;
        self.base.validate()?;
        self.full_source.validate()?;
        if let Some(f) = &self.relationship {
            f.validate()?;
            require(
                f.bytes <= 1024 * 1024,
                "source_slice",
                "relationship",
                "relationship exceeds 1 MiB",
            )?;
        }
        self.summary().validate_source_binding(
            &self.slice().sha256,
            &self.base.source_scope_id,
            self.base.source_span,
        )
    }
    pub(crate) fn slice(&self) -> &FileRef {
        &self
            .base
            .inputs
            .iter()
            .find(|i| i.role == AcceptanceInputRole::Source)
            .expect("validated Source role")
            .file
    }
    pub(crate) fn summary(&self) -> SourceSliceBindingV1 {
        SourceSliceBindingV1 {
            schema_version: 1,
            full_source_sha256: self.full_source.file.sha256.clone(),
            full_source_scope_id: self.full_source.scope_id.clone(),
            full_source_scalar_count: self.full_source.scalar_count,
            full_source_origin: self.full_source.origin,
            slice_sha256: self.slice().sha256.clone(),
            slice_scope_id: self.base.source_scope_id.clone(),
            slice_scalar_count: self.base.source_span[1] - self.base.source_span[0],
            full_source_span: self.full_source_span,
            local_span: self.base.source_span,
            local_origin: 0,
            global_origin: self.full_source_span[0],
            relationship_sha256: self.relationship.as_ref().map(|f| f.sha256.clone()),
        }
    }
}
/// Untrusted private proposal; runtime separately pins its exact file plus every expected identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSliceRelationshipV1 {
    pub schema_version: u32,
    pub full_source: FullSourcePinV1,
    pub slice: FileRef,
    pub slice_scope_id: Id,
    pub slice_scalar_count: u64,
    pub full_source_span: SourceSpan,
    pub local_span: SourceSpan,
    pub local_origin: u64,
    pub global_origin: u64,
}
impl SourceSliceRelationshipV1 {
    pub fn validate_for_policy(&self, p: &AcceptanceVerificationPolicyV2) -> ContractResult<()> {
        p.validate()?;
        require(
            self.schema_version == 1,
            "version",
            "relationship.schema_version",
            "expected 1",
        )?;
        require(
            self.full_source == p.full_source
                && self.slice == *p.slice()
                && self.slice_scope_id == p.base.source_scope_id
                && self.slice_scalar_count == p.base.source_span[1]
                && self.full_source_span == p.full_source_span
                && self.local_span == p.base.source_span
                && self.local_origin == 0
                && self.global_origin == p.full_source_span[0],
            "source_slice",
            "relationship",
            "relationship differs from independently pinned full/slice identity, scope, span or origin",
        )
    }
}
/// Path-free private audit summary. No serialized summary is a verifier capability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSliceBindingV1 {
    pub schema_version: u32,
    pub full_source_sha256: Digest,
    pub full_source_scope_id: Id,
    pub full_source_scalar_count: u64,
    pub full_source_origin: u64,
    pub slice_sha256: Digest,
    pub slice_scope_id: Id,
    pub slice_scalar_count: u64,
    pub full_source_span: SourceSpan,
    pub local_span: SourceSpan,
    pub local_origin: u64,
    pub global_origin: u64,
    pub relationship_sha256: Option<Digest>,
}
impl SourceSliceBindingV1 {
    pub fn validate_for_binding(&self, b: &AcceptanceBinding) -> ContractResult<()> {
        self.validate_source_binding(&b.source_sha256, &b.source_scope_id, b.source_span)
    }
    fn validate_source_binding(
        &self,
        source_sha256: &str,
        source_scope_id: &str,
        source_span: SourceSpan,
    ) -> ContractResult<()> {
        require(
            self.schema_version == 1
                && valid_digest(&self.full_source_sha256)
                && valid_digest(&self.slice_sha256)
                && self
                    .relationship_sha256
                    .as_ref()
                    .is_none_or(|s| valid_digest(s))
                && valid_id(&self.full_source_scope_id)
                && valid_id(&self.slice_scope_id),
            "source_slice",
            "binding",
            "invalid source relationship identity",
        )?;
        let end = self
            .full_source_origin
            .checked_add(self.full_source_scalar_count);
        require(
            self.full_source_scalar_count > 0
                && end.is_some_and(|n| n <= MAX_SAFE_INTEGER)
                && self.slice_scalar_count > 0
                && self.slice_scalar_count <= MAX_SAFE_INTEGER
                && self.local_origin == 0
                && self.local_span == [0, self.slice_scalar_count]
                && self.global_origin == self.full_source_span[0]
                && self.full_source_span[0] >= self.full_source_origin
                && self.full_source_span[0] < self.full_source_span[1]
                && end.is_some_and(|n| self.full_source_span[1] <= n)
                && self.full_source_span[1] - self.full_source_span[0] == self.slice_scalar_count,
            "source_slice",
            "coordinates",
            "expected one complete contiguous scalar slice with explicit local/global origins",
        )?;
        require(
            self.slice_sha256 == source_sha256
                && self.slice_scope_id == source_scope_id
                && self.local_span == source_span,
            "source_slice",
            "binding",
            "slice metadata differs from local candidate/source/fact domain",
        )?;
        require(
            self.full_source_scope_id != self.slice_scope_id
                || (self.full_source_sha256 == self.slice_sha256
                    && self.full_source_span == [self.full_source_origin, end.unwrap()]
                    && self.slice_scalar_count == self.full_source_scalar_count),
            "source_slice",
            "scope_id",
            "one scope ID cannot alias different full and sliced source identities",
        )
    }
}
pub(crate) fn verify(
    p: &AcceptanceVerificationPolicyV2,
    root: &std::path::Path,
    slice: &[u8],
    reads: &mut crate::acceptance::Reads,
) -> Result<(), AcceptanceReason> {
    let mapping = p
        .relationship
        .as_ref()
        .ok_or(AcceptanceReason::MissingSourceMapping)?;
    let bytes = reads.read(root, mapping, "source_relationship", 1024 * 1024)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| AcceptanceReason::InvalidBinding)?;
    if value.get("schema_version").and_then(|v| v.as_u64()) != Some(1) {
        return Err(AcceptanceReason::MissingSourceMapping);
    }
    let r: SourceSliceRelationshipV1 =
        serde_json::from_value(value).map_err(|_| AcceptanceReason::InvalidBinding)?;
    r.validate_for_policy(p)
        .map_err(|_| AcceptanceReason::InvalidBinding)?;
    let full = reads.read(root, &p.full_source.file, "full_source", 32 * 1024 * 1024)?;
    let full = std::str::from_utf8(&full).map_err(|_| AcceptanceReason::SourceContentMismatch)?;
    let slice = std::str::from_utf8(slice).map_err(|_| AcceptanceReason::SourceContentMismatch)?;
    let offset = (p.full_source_span[0] - p.full_source.origin) as usize;
    if full.chars().count() as u64 != p.full_source.scalar_count
        || slice.chars().count() as u64 != r.slice_scalar_count
        || !full
            .chars()
            .skip(offset)
            .take(r.slice_scalar_count as usize)
            .eq(slice.chars())
    {
        return Err(AcceptanceReason::SourceContentMismatch);
    }
    Ok(())
}
