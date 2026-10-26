use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ScriptError {
    #[error("Verbatim boundary violation: text contains speaker tag prefix '{found_prefix}' for speaker '{speaker}'")]
    VerbatimSpeakerTagLeak {
        speaker: String,
        found_prefix: String,
    },

    #[error("Verbatim boundary violation: stage direction or markup '{matched}' leaked into spoken text '{text}'")]
    VerbatimStageDirectionLeak { text: String, matched: String },

    #[error("Unsanctioned vocalization tag '<{tag}>' found in spoken text. Only approved natural vocalizations are allowed.")]
    UnsanctionedVocalization { tag: String },

    #[error("Script turn {turn_index} has empty spoken text")]
    EmptySpokenText { turn_index: usize },

    #[error("Character '{character}' is not registered in the cast manifest")]
    MissingCharacterCast { character: String },

    #[error("Unmatched delimiter '{delimiter}' in spoken text: {text}")]
    UnmatchedDelimiter { delimiter: char, text: String },

    #[error("Non-sequential turn order: expected turn index {expected}, found {actual}")]
    NonSequentialTurnOrder { expected: usize, actual: usize },

    #[error("JSON serialization error: {0}")]
    Json(String),
}

impl From<serde_json::Error> for ScriptError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}
