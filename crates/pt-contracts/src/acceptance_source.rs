//! Independent ordered source representation, not speech/semantic/chapter certification.
use crate::*;
pub(crate) fn verify(
    doc: &PlaybackDocument,
    bytes: &[u8],
    scope: SourceSpan,
    expected_hash: &str,
) -> Result<(), AcceptanceReason> {
    let source = std::str::from_utf8(bytes).map_err(|_| AcceptanceReason::SourceContentMismatch)?;
    if digest(bytes) != expected_hash
        || doc.manifest.source_sha256 != expected_hash
        || doc.read_along.source_sha256 != expected_hash
        || doc.source_text.as_bytes() != bytes
    {
        return Err(AcceptanceReason::InvalidBinding);
    }
    let count = source.chars().count() as u64;
    if scope[0] >= scope[1] || scope[1] > count {
        return Err(AcceptanceReason::SourceCoverageOutsideScope);
    }
    // Playback-v1 has no global-to-local source offset mapping. Never guess one.
    if scope != [0, count] {
        return Err(AcceptanceReason::MissingSourceMapping);
    }
    validate_read_along(&doc.read_along, source, doc.manifest.sample_count)
        .map_err(|_| AcceptanceReason::SourceContentMismatch)?;
    let scenes = &doc.timeline.scenes;
    if scenes.is_empty() {
        return Err(AcceptanceReason::SourceCoverageGap);
    }
    if scenes
        .windows(2)
        .any(|s| s[0].source_span[0] > s[1].source_span[0])
    {
        return Err(AcceptanceReason::SourceCoverageOrder);
    }
    let mut cursor = scope[0];
    let mut ids = std::collections::BTreeSet::new();
    for scene in scenes {
        let [a, b] = scene.source_span;
        if !valid_id(&scene.id) || !ids.insert(&scene.id) {
            return Err(AcceptanceReason::DuplicateCoverage);
        }
        if a >= b || a < scope[0] || b > scope[1] {
            return Err(AcceptanceReason::SourceCoverageOutsideScope);
        }
        if a < cursor {
            return Err(AcceptanceReason::SourceCoverageOverlap);
        }
        if a > cursor {
            return Err(AcceptanceReason::SourceCoverageGap);
        }
        cursor = b;
    }
    if cursor != scope[1] {
        return Err(AcceptanceReason::SourceCoverageGap);
    }
    Ok(())
}
