use crate::error::ScriptError;
use regex::Regex;
use std::sync::LazyLock;

pub const SANCTIONED_VOCALIZATIONS: &[&str] = &[
    "sighs",
    "groans",
    "gasps",
    "screams",
    "yawns",
    "laughs",
    "chuckles",
    "snickers",
    "coughs",
    "clears throat",
];

pub const SANCTIONED_BACKCHANNELS: &[&str] = &["mhm", "uh-huh", "yeah", "hm"];

static PAREN_OR_BRACKET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\([^)]*\)|\[[^\]]*\]").unwrap());

pub fn validate_spoken_text(speaker: &str, text: &str) -> Result<(), ScriptError> {
    let trimmed = text.trim();

    // 1. Check for leading speaker tag leak (e.g. "Speaker: Run!" or "Actor—Hello")
    if !speaker.trim().is_empty() {
        let speaker_lower: Vec<char> = speaker.chars().flat_map(|c| c.to_lowercase()).collect();
        let mut matched_len = None;
        for (idx, _) in trimmed.char_indices() {
            if idx == 0 {
                continue;
            }
            let prefix = &trimmed[..idx];
            let prefix_lower: Vec<char> = prefix.chars().flat_map(|c| c.to_lowercase()).collect();
            if prefix_lower == speaker_lower {
                matched_len = Some(idx);
                break;
            }
        }
        if matched_len.is_none() {
            let prefix_lower: Vec<char> = trimmed.chars().flat_map(|c| c.to_lowercase()).collect();
            if prefix_lower == speaker_lower {
                matched_len = Some(trimmed.len());
            }
        }

        if let Some(matched) = matched_len {
            let remainder = &trimmed[matched..];
            let remainder_trimmed = remainder.trim_start();
            if let Some(first_char) = remainder_trimmed.chars().next() {
                if first_char == ':' || first_char == '-' || first_char == '–' || first_char == '—'
                {
                    let sep_len = first_char.len_utf8();
                    let prefix_end = trimmed.len() - remainder_trimmed.len() + sep_len;
                    let found_prefix = trimmed[..prefix_end].to_string();
                    return Err(ScriptError::VerbatimSpeakerTagLeak {
                        speaker: speaker.to_string(),
                        found_prefix,
                    });
                }
            }
        }
    }

    // 2. Check for parenthetical or bracketed stage directions (e.g. "(whispering)", "[crying]", "[whispering")
    if let Some(m) = PAREN_OR_BRACKET.find(trimmed) {
        return Err(ScriptError::VerbatimStageDirectionLeak {
            text: text.to_string(),
            matched: m.as_str().to_string(),
        });
    }
    if trimmed.contains('(')
        || trimmed.contains(')')
        || trimmed.contains('[')
        || trimmed.contains(']')
    {
        return Err(ScriptError::VerbatimStageDirectionLeak {
            text: text.to_string(),
            matched: "unmatched or stray parentheses/brackets".to_string(),
        });
    }

    // 3. Check for angle bracket tags: only sanctioned vocalizations are permitted
    let mut in_angle = false;
    let mut tag_buf = String::new();
    for c in trimmed.chars() {
        if c == '<' {
            if in_angle {
                return Err(ScriptError::UnmatchedDelimiter {
                    delimiter: '<',
                    text: text.to_string(),
                });
            }
            in_angle = true;
            tag_buf.clear();
        } else if c == '>' {
            if !in_angle {
                return Err(ScriptError::UnmatchedDelimiter {
                    delimiter: '>',
                    text: text.to_string(),
                });
            }
            in_angle = false;
            if !SANCTIONED_VOCALIZATIONS.contains(&tag_buf.as_str()) {
                return Err(ScriptError::UnsanctionedVocalization {
                    tag: tag_buf.clone(),
                });
            }
        } else if in_angle {
            tag_buf.push(c);
        }
    }
    if in_angle {
        return Err(ScriptError::UnmatchedDelimiter {
            delimiter: '<',
            text: text.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_verbatim_text_passes_validation() {
        assert!(validate_spoken_text("Alice", "Run for the cellar!").is_ok());
        assert!(
            validate_spoken_text("Narrator", "The great green meadow was gray and wide.").is_ok()
        );
    }

    #[test]
    fn test_rejects_speaker_tags_in_spoken_text() {
        let err = validate_spoken_text("Mary", "Mary: Run for the cellar!").unwrap_err();
        match err {
            ScriptError::VerbatimSpeakerTagLeak { speaker, .. } => assert_eq!(speaker, "Mary"),
            other => panic!("Expected VerbatimSpeakerTagLeak, got {:?}", other),
        }

        let err2 = validate_spoken_text("Alice", "Alice - Look out!").unwrap_err();
        assert!(matches!(err2, ScriptError::VerbatimSpeakerTagLeak { .. }));
    }

    #[test]
    fn test_rejects_parenthetical_and_bracketed_stage_directions() {
        let err = validate_spoken_text("Robert", "There is a tempest coming (whispering in fear).")
            .unwrap_err();
        match err {
            ScriptError::VerbatimStageDirectionLeak { matched, .. } => {
                assert!(matched.contains("(whispering in fear)"));
            }
            other => panic!("Expected VerbatimStageDirectionLeak, got {:?}", other),
        }

        let err2 =
            validate_spoken_text("Alice", "Cat [running under the bed] is scared.").unwrap_err();
        assert!(matches!(
            err2,
            ScriptError::VerbatimStageDirectionLeak { .. }
        ));
    }

    #[test]
    fn test_accepts_sanctioned_vocalizations_in_angle_brackets() {
        assert!(
            validate_spoken_text("Mary", "Quick, Alice! <screams> Run for the cellar!").is_ok()
        );
        assert!(validate_spoken_text("Robert", "I'll go look after the stock. <sighs>").is_ok());
    }

    #[test]
    fn test_rejects_unsanctioned_angle_bracket_tags() {
        let err =
            validate_spoken_text("Alice", "He went there <quietly walking away>.").unwrap_err();
        match err {
            ScriptError::UnsanctionedVocalization { tag } => {
                assert_eq!(tag, "quietly walking away");
            }
            other => panic!("Expected UnsanctionedVocalization, got {:?}", other),
        }
    }
}
