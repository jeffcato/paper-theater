//! Approved source snapshot extents. Actual source/relationship/coverage reads
//! execute in acceptance.rs before this catalog comparison can pass.
use crate::*;
use serde::{Deserialize, Serialize};

pub(crate) const CATALOG_BYTES: &[u8] =
    include_bytes!("../../../packages/contracts/src/registered-source-scopes-v1.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredSourceFrameV1 {
    pub scope_id: Id,
    pub sha256: Digest,
    pub scalar_count: u64,
    pub origin: u64,
    pub chapter_span: SourceSpan,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredChapterSourceV1 {
    pub book_id: Id,
    pub source_id: Id,
    pub provenance: Provenance,
    pub source_receipt_sha256: Digest,
    pub raw_sha256: Digest,
    pub canonical_source_sha256: Digest,
    pub canonical_scalar_count: u64,
    pub frames: Vec<RegisteredSourceFrameV1>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredSourceCatalogV1 {
    pub schema_version: u32,
    pub version: String,
    pub sources: Vec<RegisteredChapterSourceV1>,
}

pub fn registered_source_catalog_v1() -> RegisteredSourceCatalogV1 {
    serde_json::from_slice(CATALOG_BYTES).expect("compiled registered source catalog")
}
pub fn registered_source_catalog_sha256_v1() -> Digest {
    digest(CATALOG_BYTES)
}

pub(crate) fn verify_v1(
    mapping: &SourceSliceBindingV1,
    check_id: &str,
) -> Result<(), AcceptanceReason> {
    let catalog = registered_source_catalog_v1();
    let (source, frame) = catalog
        .sources
        .iter()
        .find_map(|s| {
            s.frames
                .iter()
                .find(|f| f.scope_id == mapping.full_source_scope_id)
                .map(|f| (s, f))
        })
        .ok_or(AcceptanceReason::UnregisteredSourceScope)?;
    if mapping.full_source_sha256 != frame.sha256
        || mapping.full_source_scalar_count != frame.scalar_count
        || mapping.full_source_origin != frame.origin
        || mapping.relationship_sha256.is_none()
    {
        return Err(AcceptanceReason::InvalidBinding);
    }
    let selected = mapping.full_source_span;
    if selected[0] < frame.chapter_span[0]
        || selected[1] > frame.chapter_span[1]
        || selected[0] >= selected[1]
    {
        return Err(AcceptanceReason::SourceCoverageOutsideScope);
    }
    if check_id == "complete-chapter"
        && (selected != frame.chapter_span
            || mapping.slice_sha256 != source.canonical_source_sha256
            || mapping.slice_scalar_count != source.canonical_scalar_count)
    {
        return Err(AcceptanceReason::SourceCoverageGap);
    }
    if !matches!(check_id, "complete-chapter" | "complete-excerpt") {
        return Err(AcceptanceReason::UnsupportedRequirement);
    }
    Ok(())
}
