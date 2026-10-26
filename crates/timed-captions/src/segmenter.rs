use monotonic_align::{HeaderTiming, SourceToken, TranscriptWord};

use crate::cue::{CaptionConfig, CaptionCue, ValidatedCaptionTrack};
use crate::error::CaptionError;

#[derive(Debug, Clone)]
struct TimedWord<'a> {
    text: &'a str,
    start: Option<f64>,
    end: Option<f64>,
}

fn is_sentence_end(word: &str) -> bool {
    let trimmed = word.trim_matches(|c: char| {
        c == '"'
            || c == '\''
            || c == '”'
            || c == '“'
            || c == '’'
            || c == '‘'
            || c == ')'
            || c == ']'
            || c == '}'
    });
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "mr."
            | "mrs."
            | "ms."
            | "dr."
            | "prof."
            | "st."
            | "sr."
            | "jr."
            | "etc."
            | "vs."
            | "co."
            | "inc."
    ) {
        return false;
    }
    trimmed.ends_with('.')
        || trimmed.ends_with('!')
        || trimmed.ends_with('?')
        || trimmed.ends_with('…')
        || trimmed.ends_with("...")
}

fn is_clause_boundary(word: &str) -> bool {
    let trimmed = word.trim_matches(|c: char| {
        c == '"'
            || c == '\''
            || c == '”'
            || c == '“'
            || c == '’'
            || c == '‘'
            || c == ')'
            || c == ']'
            || c == '}'
    });
    if trimmed.is_empty() {
        return false;
    }
    trimmed.ends_with(',')
        || trimmed.ends_with(';')
        || trimmed.ends_with(':')
        || trimmed.ends_with('—')
        || trimmed.ends_with('–')
        || trimmed.ends_with("--")
}

fn chunk_text_proportionally(
    text: &str,
    start: f64,
    end: f64,
    max_chars: usize,
    is_header: bool,
    start_index: usize,
) -> Result<Vec<CaptionCue>, CaptionError> {
    if !start.is_finite() || start < 0.0 || !end.is_finite() || end < 0.0 {
        return Err(CaptionError::InvalidTimestamp {
            value: start,
            reason: "timestamps must be finite and non-negative".into(),
        });
    }
    if end <= start {
        return Err(CaptionError::ZeroDurationCue { start, end });
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut chunks = Vec::new();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let mut current_chunk = String::new();

    for word in words {
        let word_char_count = word.chars().count();
        if word_char_count > max_chars {
            if !current_chunk.is_empty() {
                chunks.push(std::mem::take(&mut current_chunk));
            }
            let mut word_chars = word.chars().peekable();
            while word_chars.peek().is_some() {
                let piece: String = word_chars.by_ref().take(max_chars).collect();
                chunks.push(piece);
            }
        } else {
            let candidate_len = if current_chunk.is_empty() {
                word_char_count
            } else {
                current_chunk.chars().count() + 1 + word_char_count
            };
            if candidate_len > max_chars {
                chunks.push(std::mem::take(&mut current_chunk));
                current_chunk.push_str(word);
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
                current_chunk.push_str(word);
            }
        }
    }
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    let total_chars: usize = chunks.iter().map(|c| c.chars().count()).sum();
    let total_duration = end - start;

    let mut cues = Vec::new();
    let mut accumulated_chars = 0;

    for (i, chunk) in chunks.into_iter().enumerate() {
        let chunk_chars = chunk.chars().count();
        let t_start = start + (accumulated_chars as f64 / total_chars as f64) * total_duration;
        accumulated_chars += chunk_chars;
        let t_end = start + (accumulated_chars as f64 / total_chars as f64) * total_duration;

        cues.push(CaptionCue::new(
            start_index + i,
            t_start,
            t_end,
            chunk,
            is_header,
        )?);
    }

    Ok(cues)
}

pub fn segment_tokens(
    tokens: &[SourceToken],
    config: &CaptionConfig,
    header: Option<&HeaderTiming>,
    book_title: Option<&str>,
    chapter_title: Option<&str>,
) -> Result<ValidatedCaptionTrack, CaptionError> {
    config.validate()?;

    // Validate header timestamps if provided
    if let Some(h) = header {
        if !h.prose_start.is_finite() || h.prose_start < 0.0 {
            return Err(CaptionError::InvalidTimestamp {
                value: h.prose_start,
                reason: "header prose_start must be finite and non-negative".into(),
            });
        }
        if let Some((start, end)) = h.title_range {
            if !start.is_finite() || start < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: start,
                    reason: "header title start must be finite and non-negative".into(),
                });
            }
            if !end.is_finite() || end < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: end,
                    reason: "header title end must be finite and non-negative".into(),
                });
            }
            if start > end {
                return Err(CaptionError::InvalidTimeRange { start, end });
            }
        }
        if let Some((start, end)) = h.chapter_range {
            if !start.is_finite() || start < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: start,
                    reason: "header chapter start must be finite and non-negative".into(),
                });
            }
            if !end.is_finite() || end < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: end,
                    reason: "header chapter end must be finite and non-negative".into(),
                });
            }
            if start > end {
                return Err(CaptionError::InvalidTimeRange { start, end });
            }
        }
    }

    // Validate token timestamps
    let mut prev_token_start: Option<f64> = None;
    let mut prev_token_end: Option<f64> = None;
    for tok in tokens {
        if tok.start.is_some() != tok.end.is_some() {
            return Err(CaptionError::PartialTokenTimestamp {
                start: tok.start,
                end: tok.end,
            });
        }
        if let Some(start) = tok.start {
            if !start.is_finite() || start < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: start,
                    reason: "token start must be finite and non-negative".into(),
                });
            }
        }
        if let Some(end) = tok.end {
            if !end.is_finite() || end < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: end,
                    reason: "token end must be finite and non-negative".into(),
                });
            }
        }
        if let (Some(start), Some(end)) = (tok.start, tok.end) {
            if end <= start {
                return Err(CaptionError::ZeroDurationCue { start, end });
            }
            if let Some(ps) = prev_token_start {
                if start < ps {
                    return Err(CaptionError::InvalidTimestamp {
                        value: start,
                        reason: "token start times must be non-decreasing".into(),
                    });
                }
            }
            if let Some(pe) = prev_token_end {
                if end < pe {
                    return Err(CaptionError::InvalidTimestamp {
                        value: end,
                        reason: "token end times must be non-decreasing".into(),
                    });
                }
            }
            prev_token_start = Some(start);
            prev_token_end = Some(end);
        }
    }

    let mut cues = Vec::new();
    let mut cue_idx = 0;

    // 1. Emit header cues if provided, chunked to max_chars with proportional timing
    if let Some(h) = header {
        if let (Some((start, end)), Some(title)) = (h.title_range, book_title) {
            let t = title.trim();
            if !t.is_empty() {
                let h_cues =
                    chunk_text_proportionally(t, start, end, config.max_chars, true, cue_idx)?;
                cue_idx += h_cues.len();
                cues.extend(h_cues);
            }
        }
        if let (Some((start, end)), Some(chap)) = (h.chapter_range, chapter_title) {
            let c = chap.trim();
            if !c.is_empty() {
                let h_cues =
                    chunk_text_proportionally(c, start, end, config.max_chars, true, cue_idx)?;
                cue_idx += h_cues.len();
                cues.extend(h_cues);
            }
        }
    }

    let prose_start = header.map(|h| h.prose_start).unwrap_or(0.0);

    // Filter tokens: if header timing was given, omit tokens that ended before or at header chapter_range end
    let timed_words: Vec<TimedWord> = tokens
        .iter()
        .filter(|tok| {
            if tok.text.trim().is_empty() {
                return false;
            }
            if let Some(h) = header {
                if let Some((_, ch_end)) = h.chapter_range {
                    if let Some(end) = tok.end {
                        if end <= ch_end {
                            return false;
                        }
                    }
                }
            }
            true
        })
        .map(|tok| TimedWord {
            text: tok.text.as_str(),
            start: tok.start,
            end: tok.end,
        })
        .collect();

    let prose_cues = segment_timed_words(&timed_words, config, cue_idx, prose_start)?;
    cues.extend(prose_cues);
    ValidatedCaptionTrack::new(cues, *config)
}

pub fn segment_words(
    words: &[TranscriptWord],
    config: &CaptionConfig,
) -> Result<ValidatedCaptionTrack, CaptionError> {
    config.validate()?;

    let mut prev_w_start: Option<f64> = None;
    let mut prev_w_end: Option<f64> = None;
    for w in words {
        if !w.start.is_finite() || w.start < 0.0 {
            return Err(CaptionError::InvalidTimestamp {
                value: w.start,
                reason: "word start must be finite and non-negative".into(),
            });
        }
        if !w.end.is_finite() || w.end < 0.0 {
            return Err(CaptionError::InvalidTimestamp {
                value: w.end,
                reason: "word end must be finite and non-negative".into(),
            });
        }
        if w.end <= w.start {
            return Err(CaptionError::ZeroDurationCue {
                start: w.start,
                end: w.end,
            });
        }
        if let Some(ps) = prev_w_start {
            if w.start < ps {
                return Err(CaptionError::InvalidTimestamp {
                    value: w.start,
                    reason: "word start times must be non-decreasing".into(),
                });
            }
        }
        if let Some(pe) = prev_w_end {
            if w.end < pe {
                return Err(CaptionError::InvalidTimestamp {
                    value: w.end,
                    reason: "word end times must be non-decreasing".into(),
                });
            }
        }
        prev_w_start = Some(w.start);
        prev_w_end = Some(w.end);
    }

    let timed_words: Vec<TimedWord> = words
        .iter()
        .filter(|w| !w.word.trim().is_empty())
        .map(|w| TimedWord {
            text: w.word.as_str(),
            start: Some(w.start),
            end: Some(w.end),
        })
        .collect();

    let cues = segment_timed_words(&timed_words, config, 0, 0.0)?;
    ValidatedCaptionTrack::new(cues, *config)
}

struct FlatWord {
    text: String,
    start: Option<f64>,
    end: Option<f64>,
}

fn segment_timed_words(
    words: &[TimedWord],
    config: &CaptionConfig,
    start_index: usize,
    default_start_time: f64,
) -> Result<Vec<CaptionCue>, CaptionError> {
    for w in words {
        if let Some(start) = w.start {
            if !start.is_finite() || start < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: start,
                    reason: "word start must be finite and non-negative".into(),
                });
            }
        }
        if let Some(end) = w.end {
            if !end.is_finite() || end < 0.0 {
                return Err(CaptionError::InvalidTimestamp {
                    value: end,
                    reason: "word end must be finite and non-negative".into(),
                });
            }
        }
        if let (Some(start), Some(end)) = (w.start, w.end) {
            if start > end {
                return Err(CaptionError::InvalidTimeRange { start, end });
            }
        }
    }

    let mut flat_words: Vec<FlatWord> = Vec::new();

    // Preprocess words: split any single token exceeding max_chars at character boundaries
    for w in words {
        let text = w.text.trim();
        if text.is_empty() {
            continue;
        }
        let char_count = text.chars().count();
        if char_count > config.max_chars {
            let chars: Vec<char> = text.chars().collect();
            let chunks: Vec<String> = chars
                .chunks(config.max_chars)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect();
            let w_start = w.start.unwrap_or(default_start_time);
            let w_end = w.end.unwrap_or(w_start + 0.5);
            let dur = w_end - w_start;
            if dur <= 0.0 || !dur.is_finite() {
                return Err(CaptionError::ZeroDurationCue {
                    start: w_start,
                    end: w_end,
                });
            }
            let total_chars: usize = chunks.iter().map(|c| c.chars().count()).sum();
            let mut acc = 0;
            let chunks_count = chunks.len();
            for (ci, chunk) in chunks.into_iter().enumerate() {
                let len = chunk.chars().count();
                let s = w_start + (acc as f64 / total_chars as f64) * dur;
                acc += len;
                let e = if ci + 1 == chunks_count {
                    w_end
                } else {
                    w_start + (acc as f64 / total_chars as f64) * dur
                };
                flat_words.push(FlatWord {
                    text: chunk,
                    start: Some(s),
                    end: Some(e),
                });
            }
        } else {
            flat_words.push(FlatWord {
                text: text.to_string(),
                start: w.start,
                end: w.end,
            });
        }
    }

    let mut cues = Vec::new();
    let mut cue_idx = start_index;

    let mut current_words: Vec<&FlatWord> = Vec::new();
    let mut current_char_len = 0;
    let mut cue_start: Option<f64> = None;
    let mut last_word_end: Option<f64> = None;
    let mut last_cue_end: f64 = default_start_time;

    let mut flush = |words_buf: &mut Vec<&FlatWord>,
                     char_len: &mut usize,
                     c_start: &mut Option<f64>,
                     last_w_end: &mut Option<f64>,
                     last_c_end: &mut f64,
                     c_idx: &mut usize|
     -> Result<(), CaptionError> {
        if words_buf.is_empty() {
            return Ok(());
        }

        let text = words_buf
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let mut start = (*c_start)
            .or_else(|| words_buf.iter().find_map(|w| w.start))
            .unwrap_or(*last_c_end);

        let end = (*last_w_end)
            .or_else(|| words_buf.iter().rev().find_map(|w| w.end))
            .unwrap_or(start + (text.chars().count() as f64 * 0.05).max(0.5));

        if start < *last_c_end {
            start = *last_c_end;
        }
        if end <= start {
            return Err(CaptionError::ZeroDurationCue { start, end });
        }

        cues.push(CaptionCue::new(*c_idx, start, end, text, false)?);
        *c_idx += 1;
        *last_c_end = end;

        words_buf.clear();
        *char_len = 0;
        *c_start = None;
        *last_w_end = None;
        Ok(())
    };

    for word in &flat_words {
        let text = word.text.as_str();
        let text_chars = text.chars().count();

        // Check if we need to flush BEFORE adding this word:
        // 1. Acoustic pause detected
        let mut pause_flush = false;
        if !current_words.is_empty() {
            if let (Some(prev_end), Some(curr_start)) = (last_word_end, word.start) {
                if curr_start > prev_end && (curr_start - prev_end) > config.max_pause_seconds {
                    pause_flush = true;
                }
            }
        }

        // 2. Hard character limit exceeded
        let candidate_len = if current_words.is_empty() {
            text_chars
        } else {
            current_char_len + 1 + text_chars
        };
        let length_flush = !current_words.is_empty() && candidate_len > config.max_chars;

        // 3. Max duration exceeded
        let duration_flush = if !current_words.is_empty() {
            if let (Some(c_s), Some(w_e)) = (cue_start, word.end) {
                (w_e - c_s) > config.max_duration_seconds
            } else {
                false
            }
        } else {
            false
        };

        if pause_flush || length_flush || duration_flush {
            flush(
                &mut current_words,
                &mut current_char_len,
                &mut cue_start,
                &mut last_word_end,
                &mut last_cue_end,
                &mut cue_idx,
            )?;
        }

        // Add word to current buffer
        if current_words.is_empty() {
            current_char_len = text_chars;
            cue_start = word.start;
        } else {
            current_char_len += 1 + text_chars;
            if cue_start.is_none() && word.start.is_some() {
                cue_start = word.start;
            }
        }
        if word.end.is_some() {
            last_word_end = word.end;
        }
        current_words.push(word);

        // Check if we need to flush AFTER adding this word:
        // Sentence terminator or (clause boundary exceeding soft_chars)
        if is_sentence_end(text)
            || (is_clause_boundary(text) && current_char_len >= config.soft_chars)
        {
            flush(
                &mut current_words,
                &mut current_char_len,
                &mut cue_start,
                &mut last_word_end,
                &mut last_cue_end,
                &mut cue_idx,
            )?;
        }
    }

    // Flush any remaining words
    flush(
        &mut current_words,
        &mut current_char_len,
        &mut cue_start,
        &mut last_word_end,
        &mut last_cue_end,
        &mut cue_idx,
    )?;

    Ok(cues)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_token(text: &str, start: f64, end: f64) -> SourceToken {
        SourceToken {
            text: text.to_string(),
            normalized: text.to_lowercase(),
            start_char: 0,
            end_char: text.len(),
            start: Some(start),
            end: Some(end),
        }
    }

    #[test]
    fn test_segmenter_respects_hard_and_soft_character_limits() {
        let config = CaptionConfig {
            max_chars: 40,
            soft_chars: 20,
            max_pause_seconds: 1.0,
            max_duration_seconds: 10.0,
        };

        let words = vec![
            "The", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog", "running",
            "across", "the", "green", "field",
        ];
        let mut tokens = Vec::new();
        let mut t = 0.0;
        for w in words {
            tokens.push(make_test_token(w, t, t + 0.3));
            t += 0.35; // 0.05 pause (no speech pause trigger)
        }

        let cues = segment_tokens(&tokens, &config, None, None, None).unwrap();
        assert!(cues.len() > 1);
        for cue in &cues {
            assert!(
                cue.text.len() <= config.max_chars,
                "Cue text exceeded max_chars: {} (len {})",
                cue.text,
                cue.text.len()
            );
        }
    }

    #[test]
    fn test_segmenter_flushes_on_speech_pause() {
        let config = CaptionConfig {
            max_chars: 72,
            soft_chars: 36,
            max_pause_seconds: 0.45,
            max_duration_seconds: 6.0,
        };

        // Two short phrases separated by 0.8s silence (> 0.45s)
        let tokens = vec![
            make_test_token("First", 0.0, 0.4),
            make_test_token("phrase", 0.45, 0.9),
            // Silence from 0.9 to 1.7 (0.8s gap)
            make_test_token("Second", 1.7, 2.1),
            make_test_token("phrase.", 2.15, 2.6),
        ];

        let cues = segment_tokens(&tokens, &config, None, None, None).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "First phrase");
        assert_eq!(cues[1].text, "Second phrase.");
        assert_eq!(cues[0].start, 0.0);
        assert_eq!(cues[0].end, 0.9);
        assert_eq!(cues[1].start, 1.7);
        assert_eq!(cues[1].end, 2.6);
    }

    #[test]
    fn test_segmenter_flushes_on_sentence_and_clause_punctuation() {
        let config = CaptionConfig::default(); // max_chars 72, soft_chars 36

        // "Mr. Smith arrived," (clause, but < 36 chars, so doesn't flush at comma)
        // "wearing a dark coat." (sentence ends with '.')
        let tokens = vec![
            make_test_token("Mr.", 0.0, 0.3),
            make_test_token("Smith", 0.35, 0.7),
            make_test_token("arrived,", 0.75, 1.2),
            make_test_token("wearing", 1.25, 1.6),
            make_test_token("a", 1.65, 1.7),
            make_test_token("dark", 1.75, 2.0),
            make_test_token("coat.", 2.05, 2.4),
        ];

        let cues = segment_tokens(&tokens, &config, None, None, None).unwrap();
        // Total length is 41 chars, ends with '.'. Does not prematurely split at "Mr.".
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Mr. Smith arrived, wearing a dark coat.");
    }

    #[test]
    fn test_segmenter_emits_header_timing_cues_first() {
        let config = CaptionConfig::default();
        let header = HeaderTiming {
            title_range: Some((0.5, 3.2)),
            chapter_range: Some((3.5, 5.8)),
            prose_start: 6.2,
        };

        let tokens = vec![
            make_test_token("The", 6.2, 6.4),
            make_test_token("story", 6.45, 6.8),
            make_test_token("begins.", 6.85, 7.3),
        ];

        let cues = segment_tokens(
            &tokens,
            &config,
            Some(&header),
            Some("A Grand Tale"),
            Some("Chapter 1: The Beginning"),
        )
        .unwrap();

        assert_eq!(cues.len(), 3);
        assert_eq!(cues[0].index, 0);
        assert_eq!(cues[0].text, "A Grand Tale");
        assert!(cues[0].is_header);
        assert_eq!(cues[0].start, 0.5);
        assert_eq!(cues[0].end, 3.2);

        assert_eq!(cues[1].index, 1);
        assert_eq!(cues[1].text, "Chapter 1: The Beginning");
        assert!(cues[1].is_header);
        assert_eq!(cues[1].start, 3.5);
        assert_eq!(cues[1].end, 5.8);

        assert_eq!(cues[2].index, 2);
        assert_eq!(cues[2].text, "The story begins.");
        assert!(!cues[2].is_header);
        assert_eq!(cues[2].start, 6.2);
    }

    #[test]
    fn test_segment_words_with_raw_asr() {
        let config = CaptionConfig::default();
        let words = vec![
            TranscriptWord {
                word: "Once".to_string(),
                start: 1.0,
                end: 1.3,
            },
            TranscriptWord {
                word: "upon".to_string(),
                start: 1.35,
                end: 1.6,
            },
            TranscriptWord {
                word: "a".to_string(),
                start: 1.62,
                end: 1.7,
            },
            TranscriptWord {
                word: "time.".to_string(),
                start: 1.75,
                end: 2.1,
            },
        ];

        let cues = segment_words(&words, &config).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Once upon a time.");
        assert_eq!(cues[0].start, 1.0);
        assert_eq!(cues[0].end, 2.1);
    }
}
