use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum AlignError {
    #[error("Transcript word times must be monotonic: '{word}' start {current} < previous {previous} - 0.1")]
    NonMonotonicTranscript {
        word: String,
        current: f64,
        previous: f64,
    },

    #[error("Transcript word '{word}' has invalid time range: end {end} < start {start}")]
    InvalidTimeRange { word: String, start: f64, end: f64 },

    #[error("Source text contains no valid tokens to align")]
    EmptySourceText,

    #[error("Transcript word '{word}' has invalid nonfinite or negative timestamp: start {start}, end {end}")]
    InvalidTimestamp { word: String, start: f64, end: f64 },

    #[error("Transcript word '{word}' has decreasing end time: {current} < previous {previous}")]
    DecreasingEndTime {
        word: String,
        current: f64,
        previous: f64,
    },

    #[error("Evidence span cuts through token '{token}' at offset {offset}")]
    TokenBoundaryViolation { offset: usize, token: String },

    #[error("Invalid evidence span bounds: {start_char}..{end_char}")]
    InvalidSpanBounds { start_char: usize, end_char: usize },

    #[error("Source text hash mismatch: expected {expected}, actual {actual}")]
    SourceMismatch { expected: String, actual: String },

    #[error("Alignment tokens do not match canonical source tokens: {reason}")]
    TokenStructureMismatch { reason: String },

    #[error("Alignment token '{token}' has invalid timestamp: start {start:?}, end {end:?}")]
    InvalidTokenTimestamp {
        token: String,
        start: Option<f64>,
        end: Option<f64>,
    },

    #[error("JSON serialization error: {0}")]
    Json(String),
}

impl From<serde_json::Error> for AlignError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}
