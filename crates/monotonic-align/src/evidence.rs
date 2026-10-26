use crate::error::AlignError;
use crate::models::{AlignmentResult, Evidence};
use sha2::{Digest, Sha256};

pub fn extract_evidence(
    text: &str,
    alignment: &AlignmentResult,
    start_char: usize,
    end_char: usize,
) -> Result<Evidence, AlignError> {
    if start_char > end_char || end_char > text.chars().count() {
        return Err(AlignError::InvalidSpanBounds {
            start_char,
            end_char,
        });
    }

    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let actual_hash = format!("{:x}", hasher.finalize());
    if alignment.source_hash != actual_hash {
        return Err(AlignError::SourceMismatch {
            expected: alignment.source_hash.clone(),
            actual: actual_hash,
        });
    }

    // Validate imported alignment structure against canonical tokenization of text
    let canonical = crate::normalize::tokenize_source(text);
    if alignment.tokens.len() != canonical.len() {
        return Err(AlignError::TokenStructureMismatch {
            reason: format!(
                "alignment contains {} tokens, canonical text requires {}",
                alignment.tokens.len(),
                canonical.len()
            ),
        });
    }

    let mut last_start: Option<f64> = None;
    let mut last_end: Option<f64> = None;
    for (a, c) in alignment.tokens.iter().zip(&canonical) {
        if a.text != c.text || a.start_char != c.start_char || a.end_char != c.end_char {
            return Err(AlignError::TokenStructureMismatch {
                reason: format!(
                    "token '{}' ({}-{}) does not match canonical '{}' ({}-{})",
                    a.text, a.start_char, a.end_char, c.text, c.start_char, c.end_char
                ),
            });
        }
        match (a.start, a.end) {
            (Some(s), Some(e)) => {
                if !s.is_finite() || !e.is_finite() || s < 0.0 || e < s {
                    return Err(AlignError::InvalidTokenTimestamp {
                        token: a.text.clone(),
                        start: a.start,
                        end: a.end,
                    });
                }
                if let Some(prev_e) = last_end {
                    if e < prev_e {
                        return Err(AlignError::InvalidTokenTimestamp {
                            token: a.text.clone(),
                            start: a.start,
                            end: a.end,
                        });
                    }
                }
                if let Some(prev_s) = last_start {
                    if s < prev_s {
                        return Err(AlignError::NonMonotonicTranscript {
                            word: a.text.clone(),
                            current: s,
                            previous: prev_s,
                        });
                    }
                }
                last_start = Some(s);
                last_end = Some(e);
            }
            (Some(s), None) => {
                return Err(AlignError::InvalidTokenTimestamp {
                    token: a.text.clone(),
                    start: Some(s),
                    end: None,
                });
            }
            (None, Some(e)) => {
                return Err(AlignError::InvalidTokenTimestamp {
                    token: a.text.clone(),
                    start: None,
                    end: Some(e),
                });
            }
            (None, None) => {}
        }
    }

    // Reject spans that cut through tokens
    for token in &alignment.tokens {
        if token.start_char < start_char && start_char < token.end_char {
            return Err(AlignError::TokenBoundaryViolation {
                offset: start_char,
                token: token.text.clone(),
            });
        }
        if token.start_char < end_char && end_char < token.end_char {
            return Err(AlignError::TokenBoundaryViolation {
                offset: end_char,
                token: token.text.clone(),
            });
        }
    }

    let tokens: Vec<_> = alignment
        .tokens
        .iter()
        .filter(|t| t.start_char >= start_char && t.end_char <= end_char)
        .collect();

    let matched_count = tokens.iter().filter(|t| t.end.is_some()).count();
    let coverage = if tokens.is_empty() {
        0.0
    } else {
        matched_count as f64 / tokens.len() as f64
    };

    let unmatched: Vec<String> = tokens
        .iter()
        .filter(|t| t.end.is_none())
        .map(|t| t.text.clone())
        .collect();

    let reveal_at = tokens.last().and_then(|t| t.end);

    let quote: String = text
        .chars()
        .skip(start_char)
        .take(end_char.saturating_sub(start_char))
        .collect();

    Ok(Evidence {
        quote,
        start_char,
        end_char,
        reveal_at,
        coverage,
        unmatched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::align;
    use crate::models::{Transcript, TranscriptSegment, TranscriptWord};

    fn simple_transcript(words: &[&str]) -> Transcript {
        let mut t_words = Vec::new();
        for (i, word) in words.iter().enumerate() {
            t_words.push(TranscriptWord {
                word: word.to_string(),
                start: i as f64,
                end: i as f64 + 0.5,
            });
        }
        Transcript {
            text: Some(words.join(" ")),
            segments: vec![TranscriptSegment {
                id: Some(0),
                start: 0.0,
                end: words.len() as f64,
                text: Some(words.join(" ")),
                words: t_words,
            }],
            audio_hash: Some("mock-audio".to_string()),
        }
    }

    #[test]
    fn test_evidence_computes_terminal_reveal_time() {
        let text = "The traveller walked across the open landscape.";
        let transcript = simple_transcript(&[
            "The",
            "traveller",
            "walked",
            "across",
            "the",
            "open",
            "landscape",
        ]);
        let alignment = align(text, &transcript, "audio").unwrap();

        let ev = extract_evidence(text, &alignment, 0, text.chars().count()).unwrap();

        assert_eq!(ev.quote, text);
        assert_eq!(ev.start_char, 0);
        assert_eq!(ev.end_char, text.chars().count());
        assert_eq!(ev.reveal_at, Some(6.5));
        assert_eq!(ev.coverage, 1.0);
        assert!(ev.unmatched.is_empty());
    }

    #[test]
    fn test_unmatched_final_word_yields_none_reveal_time() {
        let text = "The girl vanished";
        // Audio only spoke "The girl"
        let transcript = simple_transcript(&["The", "girl"]);
        let alignment = align(text, &transcript, "audio").unwrap();

        let ev = extract_evidence(text, &alignment, 0, text.chars().count()).unwrap();

        // Essential anti-spoiler invariant: final word wasn't spoken, so reveal_at MUST be None!
        assert_eq!(ev.reveal_at, None);
        assert_eq!(ev.unmatched, vec!["vanished"]);
        assert!(ev.coverage < 1.0);
    }

    #[test]
    fn test_evidence_slices_intermediate_span() {
        let text = "First sentence here. Second sentence starts now. Third sentence ends.";
        let transcript = simple_transcript(&[
            "First", "sentence", "here", "Second", "sentence", "starts", "now", "Third",
            "sentence", "ends",
        ]);
        let alignment = align(text, &transcript, "audio").unwrap();

        // Extract "Second sentence starts now."
        let start = text.find("Second").unwrap();
        let end = start + "Second sentence starts now.".len();
        let ev = extract_evidence(text, &alignment, start, end).unwrap();

        assert_eq!(ev.quote, "Second sentence starts now.");
        assert_eq!(ev.reveal_at, Some(6.5));
        assert_eq!(ev.coverage, 1.0);
    }
}
