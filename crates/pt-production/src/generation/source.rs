use super::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct FrozenSource {
    schema_version: u32,
    book_id: String,
    selector: Selector,
    raw_sha256: String,
    clean_sha256: String,
    source_sha256: String,
    clean_span: [usize; 2],
    raw_span: [usize; 2],
    text: String,
    scalar_to_raw: Vec<[usize; 2]>,
}
#[derive(Deserialize)]
struct Selector {
    book_id: String,
    opening: String,
    ending: String,
}
/// Validate the frozen body against the exact raw/clean artifacts without opening files.
pub(super) fn validate(
    frozen: &[u8],
    raw: &[u8],
    clean: &[u8],
) -> Result<serde_json::Value, StageError> {
    let f: FrozenSource = parse(frozen)?;
    require(
        f.schema_version == 1 && f.book_id == f.selector.book_id,
        "generation_source",
        "source",
        "source selection differs",
    )?;
    require(
        digest(raw) == f.raw_sha256
            && digest(clean) == f.clean_sha256
            && digest(f.text.as_bytes()) == f.source_sha256,
        "generation_source_hash",
        "source",
        "raw, clean or body bytes changed",
    )?;
    let decoded =
        std::str::from_utf8(raw).map_err(|_| StageError::Evidence("source is not UTF-8".into()))?;
    let raw_chars: Vec<char> = decoded.chars().collect();
    let clean_text = std::str::from_utf8(clean)
        .map_err(|_| StageError::Evidence("clean source is not UTF-8".into()))?;
    let clean_chars: Vec<char> = clean_text.chars().collect();
    let [a, b] = f.clean_span;
    require(
        a < b && b <= clean_chars.len() && clean_chars[a..b].iter().collect::<String>() == f.text,
        "generation_source_span",
        "source",
        "body differs from clean Unicode scalar span",
    )?;
    require(
        !f.selector.opening.is_empty()
            && !f.selector.ending.is_empty()
            && f.text.starts_with(&f.selector.opening)
            && f.text.ends_with(&f.selector.ending),
        "generation_source_boundary",
        "source",
        "actual opening/ending differs",
    )?;
    require(
        f.scalar_to_raw.len() == f.text.chars().count(),
        "generation_source_mapping",
        "source",
        "incomplete scalar map",
    )?;
    let mut previous = [0, 0];
    for (ch, [a, b]) in f.text.chars().zip(f.scalar_to_raw.iter().copied()) {
        require(
            a <= b && b <= raw_chars.len() && a >= previous[0] && b >= previous[1],
            "generation_source_mapping",
            "source",
            "unordered or invalid raw scalar span",
        )?;
        if !ch.is_whitespace() {
            require(
                b == a + 1 && raw_chars[a] == ch,
                "generation_source_mapping",
                "source",
                "retained scalar differs from raw source",
            )?;
        }
        previous = [a, b];
    }
    require(
        f.scalar_to_raw
            .first()
            .is_some_and(|p| p[0] == f.raw_span[0])
            && f.scalar_to_raw
                .last()
                .is_some_and(|p| p[1] == f.raw_span[1]),
        "generation_source_mapping",
        "source",
        "raw body boundary differs",
    )?;
    parse(frozen)
}
