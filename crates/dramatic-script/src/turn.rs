use crate::error::ScriptError;
use crate::validator::validate_spoken_text;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechMetadata {
    pub style: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptTurn {
    pub turn_index: usize,
    pub speaker: String,
    pub clean_spoken_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech_metadata: Option<SpeechMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_span: Option<(usize, usize)>,
}

impl ScriptTurn {
    pub fn new(
        turn_index: usize,
        speaker: impl Into<String>,
        clean_spoken_text: impl Into<String>,
        style: Option<impl Into<String>>,
        source_span: Option<(usize, usize)>,
    ) -> Result<Self, ScriptError> {
        let speaker = speaker.into();
        let clean_spoken_text = clean_spoken_text.into();

        if clean_spoken_text.trim().is_empty() {
            return Err(ScriptError::EmptySpokenText { turn_index });
        }

        validate_spoken_text(&speaker, &clean_spoken_text)?;

        let speech_metadata = style.map(|s| SpeechMetadata { style: s.into() });

        Ok(Self {
            turn_index,
            speaker,
            clean_spoken_text,
            speech_metadata,
            source_span,
        })
    }

    pub fn narrator(
        turn_index: usize,
        clean_spoken_text: impl Into<String>,
        source_span: Option<(usize, usize)>,
    ) -> Result<Self, ScriptError> {
        Self::new(
            turn_index,
            "Narrator",
            clean_spoken_text,
            None::<String>,
            source_span,
        )
    }

    pub fn dialogue(
        turn_index: usize,
        speaker: impl Into<String>,
        clean_spoken_text: impl Into<String>,
        style: impl Into<String>,
        source_span: Option<(usize, usize)>,
    ) -> Result<Self, ScriptError> {
        Self::new(
            turn_index,
            speaker,
            clean_spoken_text,
            Some(style),
            source_span,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_turn_creation_and_metadata() {
        let turn = ScriptTurn::dialogue(
            1,
            "Speaker",
            "Quick, friend! Run for the cellar!",
            "screaming, terrified, breathless",
            Some((120, 160)),
        )
        .unwrap();

        assert_eq!(turn.turn_index, 1);
        assert_eq!(turn.speaker, "Speaker");
        assert_eq!(turn.clean_spoken_text, "Quick, friend! Run for the cellar!");
        assert_eq!(
            turn.speech_metadata,
            Some(SpeechMetadata {
                style: "screaming, terrified, breathless".to_string()
            })
        );
        assert_eq!(turn.source_span, Some((120, 160)));
    }

    #[test]
    fn test_script_turn_rejects_empty_text() {
        let err = ScriptTurn::narrator(0, "   ", None).unwrap_err();
        assert_eq!(err, ScriptError::EmptySpokenText { turn_index: 0 });
    }
}
