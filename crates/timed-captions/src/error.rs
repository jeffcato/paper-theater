use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum CaptionError {
    #[error("Invalid time range: start {start}s > end {end}s")]
    InvalidTimeRange { start: f64, end: f64 },

    #[error("Invalid timestamp {value}: {reason}")]
    InvalidTimestamp { value: f64, reason: String },

    #[error("Token timestamps must both be present or both absent: start={start:?}, end={end:?}")]
    PartialTokenTimestamp {
        start: Option<f64>,
        end: Option<f64>,
    },

    #[error("Cue length {chars} exceeds maximum allowed {max} characters")]
    CueLengthExceeded { chars: usize, max: usize },

    #[error("Overlapping cues: previous cue end {prev_end} > next cue start {next_start}")]
    OverlappingCues { prev_end: f64, next_start: f64 },

    #[error("Zero or negative duration cue: start {start}, end {end}")]
    ZeroDurationCue { start: f64, end: f64 },

    #[error("Invalid caption configuration: {reason}")]
    InvalidConfig { reason: String },

    #[error("Input token stream is empty")]
    EmptyTokens,

    #[error("JSON serialization error: {0}")]
    Json(String),
}
