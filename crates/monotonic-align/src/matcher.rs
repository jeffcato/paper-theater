use crate::error::AlignError;
use crate::models::{
    AlignmentResult, HeaderTiming, HeardToken, Mismatch, MismatchKind, Transcript,
};
use crate::normalize::{normalize_token, tokenize_source};
use regex::Regex;
use sha2::{Digest, Sha256};
use similar::{capture_diff_slices, Algorithm, DiffOp};
use std::sync::LazyLock;

static TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\w]+(?:['’][\w]+)*").unwrap());

pub fn parse_heard_tokens(transcript: &Transcript) -> Result<Vec<HeardToken>, AlignError> {
    let mut heard = Vec::new();
    let mut previous_start = 0.0;
    let mut previous_end = 0.0;

    for segment in &transcript.segments {
        for word in &segment.words {
            if !word.start.is_finite()
                || word.start < 0.0
                || !word.end.is_finite()
                || word.end < 0.0
            {
                return Err(AlignError::InvalidTimestamp {
                    word: word.word.clone(),
                    start: word.start,
                    end: word.end,
                });
            }
            if word.start < previous_start - 0.1 {
                return Err(AlignError::NonMonotonicTranscript {
                    word: word.word.clone(),
                    current: word.start,
                    previous: previous_start,
                });
            }
            if word.end < word.start {
                return Err(AlignError::InvalidTimeRange {
                    word: word.word.clone(),
                    start: word.start,
                    end: word.end,
                });
            }
            if word.end < previous_end {
                return Err(AlignError::DecreasingEndTime {
                    word: word.word.clone(),
                    current: word.end,
                    previous: previous_end,
                });
            }
            previous_start = word.start;
            previous_end = word.end;

            for m in TOKEN_REGEX.find_iter(&word.word) {
                let text = m.as_str().to_string();
                let normalized = normalize_token(&text);
                heard.push(HeardToken {
                    text,
                    normalized,
                    start: word.start,
                    end: word.end,
                });
            }
        }
    }

    Ok(heard)
}

pub fn align(
    text: &str,
    transcript: &Transcript,
    audio_hash: &str,
) -> Result<AlignmentResult, AlignError> {
    let mut source = tokenize_source(text);
    let heard = parse_heard_tokens(transcript)?;

    let source_norm: Vec<&str> = source.iter().map(|t| t.normalized.as_str()).collect();
    let heard_norm: Vec<&str> = heard.iter().map(|t| t.normalized.as_str()).collect();

    let diff_ops = capture_diff_slices(Algorithm::Myers, &source_norm, &heard_norm);
    let mut mismatches = Vec::new();

    for op in diff_ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                let a = old_index;
                let b = old_index + len;
                let c = new_index;
                let d = new_index + len;

                for (left, right) in source[a..b].iter_mut().zip(&heard[c..d]) {
                    left.start = Some(right.start);
                    left.end = Some(right.end);
                }
            }
            DiffOp::Delete {
                old_index,
                old_len,
                new_index,
            } => {
                let a = old_index;
                let b = old_index + old_len;
                let c = new_index;
                let d = new_index;

                mismatches.push(Mismatch {
                    kind: MismatchKind::Delete,
                    source_range: (a, b),
                    transcript_range: (c, d),
                    source_text: source[a..b]
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    heard_text: String::new(),
                    start: None,
                    end: None,
                });
            }
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => {
                let a = old_index;
                let b = old_index;
                let c = new_index;
                let d = new_index + new_len;

                mismatches.push(Mismatch {
                    kind: MismatchKind::Insert,
                    source_range: (a, b),
                    transcript_range: (c, d),
                    source_text: String::new(),
                    heard_text: heard[c..d]
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    start: Some(heard[c].start),
                    end: Some(heard[d - 1].end),
                });
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let a = old_index;
                let b = old_index + old_len;
                let c = new_index;
                let d = new_index + new_len;

                mismatches.push(Mismatch {
                    kind: MismatchKind::Replace,
                    source_range: (a, b),
                    transcript_range: (c, d),
                    source_text: source[a..b]
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    heard_text: heard[c..d]
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    start: if c < d { Some(heard[c].start) } else { None },
                    end: if c < d { Some(heard[d - 1].end) } else { None },
                });
            }
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let source_hash = format!("{:x}", hasher.finalize());

    let matched_count = source.iter().filter(|t| t.end.is_some()).count();
    let coverage = matched_count as f64 / source.len().max(1) as f64;

    Ok(AlignmentResult {
        version: crate::VERSION.to_string(),
        source_hash,
        audio_hash: audio_hash.to_string(),
        tokens: source,
        transcript_tokens: heard,
        mismatches,
        coverage,
        header_timing: None,
    })
}

pub fn align_chapter(
    book_title: Option<&str>,
    chapter_title: &str,
    chapter_text: &str,
    transcript: &Transcript,
    audio_hash: &str,
) -> Result<AlignmentResult, AlignError> {
    let mut full_text = String::new();
    let title_token_count = if let Some(title) = book_title {
        full_text.push_str(title);
        full_text.push('\n');
        tokenize_source(title).len()
    } else {
        0
    };

    let chapter_token_count = {
        full_text.push_str(chapter_title);
        full_text.push('\n');
        tokenize_source(chapter_title).len()
    };

    let header_token_count = title_token_count + chapter_token_count;
    full_text.push_str(chapter_text);

    let full_result = align(&full_text, transcript, audio_hash)?;

    let header_tokens = &full_result.tokens[..header_token_count];
    let header_char_count = full_text.chars().count() - chapter_text.chars().count();

    let body_tokens: Vec<_> = full_result.tokens[header_token_count..]
        .iter()
        .cloned()
        .map(|mut t| {
            t.start_char = t.start_char.saturating_sub(header_char_count);
            t.end_char = t.end_char.saturating_sub(header_char_count);
            t
        })
        .collect();

    let mut body_mismatches = Vec::new();
    for m in full_result.mismatches {
        if m.kind == MismatchKind::Insert {
            if m.source_range.0 < header_token_count {
                // Insertion in header
                continue;
            } else {
                // Insertion at or within body (including chapter boundary)
                let mut reb = m;
                reb.source_range.0 = reb.source_range.0.saturating_sub(header_token_count);
                reb.source_range.1 = reb.source_range.1.saturating_sub(header_token_count);
                body_mismatches.push(reb);
                continue;
            }
        }

        if m.source_range.1 <= header_token_count {
            // Entirely in header
            continue;
        }
        if m.source_range.0 >= header_token_count {
            // Entirely in body
            let mut reb = m;
            reb.source_range.0 = reb.source_range.0.saturating_sub(header_token_count);
            reb.source_range.1 = reb.source_range.1.saturating_sub(header_token_count);
            body_mismatches.push(reb);
        } else {
            // Crosses the header/body boundary: split and retain the body portion
            let body_start = header_token_count;
            let body_end = m.source_range.1;
            let body_source_text = full_result.tokens[body_start..body_end]
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            body_mismatches.push(Mismatch {
                kind: m.kind,
                source_range: (0, body_end.saturating_sub(header_token_count)),
                transcript_range: m.transcript_range,
                source_text: body_source_text,
                heard_text: m.heard_text,
                start: m.start,
                end: m.end,
            });
        }
    }

    let title_range = if title_token_count > 0 {
        let slice = &header_tokens[..title_token_count];
        let start = slice.iter().find_map(|t| t.start);
        let end = slice.iter().rev().find_map(|t| t.end);
        match (start, end) {
            (Some(s), Some(e)) => Some((s, e)),
            _ => None,
        }
    } else {
        None
    };

    let chapter_range = {
        let slice = &header_tokens[title_token_count..header_token_count];
        let start = slice.iter().find_map(|t| t.start);
        let end = slice.iter().rev().find_map(|t| t.end);
        match (start, end) {
            (Some(s), Some(e)) => Some((s, e)),
            _ => None,
        }
    };

    let prose_start = body_tokens
        .iter()
        .find_map(|t| t.start)
        .unwrap_or_else(|| chapter_range.map(|r| r.1).unwrap_or(0.0));

    let mut hasher = Sha256::new();
    hasher.update(chapter_text.as_bytes());
    let body_source_hash = format!("{:x}", hasher.finalize());

    let body_matched = body_tokens.iter().filter(|t| t.end.is_some()).count();
    let body_coverage = body_matched as f64 / body_tokens.len().max(1) as f64;

    Ok(AlignmentResult {
        version: crate::VERSION.to_string(),
        source_hash: body_source_hash,
        audio_hash: audio_hash.to_string(),
        tokens: body_tokens,
        transcript_tokens: full_result.transcript_tokens,
        mismatches: body_mismatches,
        coverage: body_coverage,
        header_timing: Some(HeaderTiming {
            title_range,
            chapter_range,
            prose_start,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{TranscriptSegment, TranscriptWord};

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
            audio_hash: Some("mock-audio-hash".to_string()),
        }
    }

    #[test]
    fn test_align_monotonic_exact_tokens() {
        let text = "The girl saw the dog. The girl slept.";
        let transcript = simple_transcript(&[
            "LibriVox", "The", "girl", "saw", "dog", "The", "girl", "slept",
        ]);

        let result = align(text, &transcript, "test-audio").unwrap();

        // 1. First mismatch should be the LibriVox introduction
        assert_eq!(result.mismatches.len(), 2);
        assert_eq!(result.mismatches[0].heard_text, "LibriVox");

        // 2. The second "the" was omitted in audio ("saw dog")
        assert_eq!(result.tokens[3].text, "the");
        assert_eq!(result.tokens[3].start, None);
        assert_eq!(result.tokens[3].end, None);

        // 3. Final token "slept" matches
        let last_token = result.tokens.last().unwrap();
        assert_eq!(last_token.text, "slept");
        assert_eq!(last_token.end, Some(7.5));

        // 4. All evidenced tokens have monotonic ascending timestamps
        let times: Vec<f64> = result.tokens.iter().filter_map(|t| t.end).collect();
        let mut sorted_times = times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(times, sorted_times);
    }

    #[test]
    fn test_rejects_non_monotonic_word_times() {
        let transcript = Transcript {
            text: Some("a b".to_string()),
            segments: vec![TranscriptSegment {
                id: Some(0),
                start: 0.0,
                end: 3.0,
                text: Some("a b".to_string()),
                words: vec![
                    TranscriptWord {
                        word: "a".to_string(),
                        start: 2.0,
                        end: 3.0,
                    },
                    TranscriptWord {
                        word: "b".to_string(),
                        start: 1.0, // Non-monotonic backwards jump!
                        end: 2.0,
                    },
                ],
            }],
            audio_hash: None,
        };

        let err = align("a b", &transcript, "audio").unwrap_err();
        match err {
            AlignError::NonMonotonicTranscript { word, .. } => assert_eq!(word, "b"),
            other => panic!("Expected NonMonotonicTranscript, got {:?}", other),
        }
    }

    #[test]
    fn test_curly_apostrophes_normalize_but_spelling_mismatches_do_not() {
        let text = "Girl’s GRAY coat";
        let transcript = simple_transcript(&["girl's", "grey", "coat"]);

        let result = align(text, &transcript, "audio").unwrap();

        // "Girl’s" matches "girl's"
        assert_eq!(result.tokens[0].end, Some(0.5));

        // "GRAY" does NOT match "grey"
        assert_eq!(result.tokens[1].end, None);

        // "coat" matches "coat"
        assert_eq!(result.tokens[2].end, Some(2.5));
    }

    #[test]
    fn test_align_chapter_extracts_header_timing_and_prose_start() {
        let book_title = "The Great Adventure Story Book";
        let chapter_title = "Chapter I. The Opening";
        let chapter_text = "Alice lived in the midst of London.";

        let words = vec![
            TranscriptWord {
                word: "The".into(),
                start: 0.0,
                end: 0.3,
            },
            TranscriptWord {
                word: "Great".into(),
                start: 0.3,
                end: 0.8,
            },
            TranscriptWord {
                word: "Adventure".into(),
                start: 0.8,
                end: 1.2,
            },
            TranscriptWord {
                word: "Story".into(),
                start: 1.2,
                end: 1.4,
            },
            TranscriptWord {
                word: "Book".into(),
                start: 1.4,
                end: 2.0,
            },
            TranscriptWord {
                word: "Chapter".into(),
                start: 2.0,
                end: 2.5,
            },
            TranscriptWord {
                word: "I".into(),
                start: 2.5,
                end: 2.8,
            },
            TranscriptWord {
                word: "The".into(),
                start: 2.8,
                end: 3.1,
            },
            TranscriptWord {
                word: "Opening".into(),
                start: 3.1,
                end: 4.0,
            },
            TranscriptWord {
                word: "Alice".into(),
                start: 4.0,
                end: 4.6,
            },
            TranscriptWord {
                word: "lived".into(),
                start: 4.6,
                end: 5.2,
            },
            TranscriptWord {
                word: "in".into(),
                start: 5.2,
                end: 5.5,
            },
            TranscriptWord {
                word: "the".into(),
                start: 5.5,
                end: 5.8,
            },
            TranscriptWord {
                word: "midst".into(),
                start: 5.8,
                end: 6.4,
            },
            TranscriptWord {
                word: "of".into(),
                start: 6.4,
                end: 6.7,
            },
            TranscriptWord {
                word: "London".into(),
                start: 6.7,
                end: 7.5,
            },
        ];

        let transcript = Transcript {
            text: None,
            segments: vec![TranscriptSegment {
                id: Some(0),
                start: 0.0,
                end: 7.5,
                text: None,
                words,
            }],
            audio_hash: Some("mock-audio".into()),
        };

        let result = align_chapter(
            Some(book_title),
            chapter_title,
            chapter_text,
            &transcript,
            "mock-audio",
        )
        .unwrap();

        // 1. Header timing should capture title declaration and chapter declaration
        assert!(result.header_timing.is_some());
        let timing = result.header_timing.unwrap();

        assert_eq!(timing.title_range, Some((0.0, 2.0)));
        assert_eq!(timing.chapter_range, Some((2.0, 4.0)));
        assert_eq!(timing.prose_start, 4.0);

        // 2. Prose tokens match accurately starting at 4.0s
        assert_eq!(result.tokens[0].text, "Alice");
        assert_eq!(result.tokens[0].start, Some(4.0));
        assert_eq!(result.tokens.last().unwrap().end, Some(7.5));
    }
}
