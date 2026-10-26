use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceToken {
    pub text: String,
    pub normalized: String,
    pub start_char: usize,
    pub end_char: usize,
    pub start: Option<f64>,
    pub end: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeardToken {
    pub text: String,
    pub normalized: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MismatchKind {
    Equal,
    Delete,
    Insert,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mismatch {
    pub kind: MismatchKind,
    pub source_range: (usize, usize),
    pub transcript_range: (usize, usize),
    pub source_text: String,
    pub heard_text: String,
    pub start: Option<f64>,
    pub end: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptWord {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    #[serde(default)]
    pub id: Option<u64>,
    pub start: f64,
    pub end: f64,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub words: Vec<TranscriptWord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    #[serde(default)]
    pub text: Option<String>,
    pub segments: Vec<TranscriptSegment>,
    #[serde(default)]
    pub audio_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeaderTiming {
    pub title_range: Option<(f64, f64)>,
    pub chapter_range: Option<(f64, f64)>,
    pub prose_start: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentResult {
    pub version: String,
    pub source_hash: String,
    pub audio_hash: String,
    pub tokens: Vec<SourceToken>,
    pub transcript_tokens: Vec<HeardToken>,
    pub mismatches: Vec<Mismatch>,
    pub coverage: f64,
    pub header_timing: Option<HeaderTiming>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub quote: String,
    pub start_char: usize,
    pub end_char: usize,
    pub reveal_at: Option<f64>,
    pub coverage: f64,
    pub unmatched: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParagraphSpan {
    pub index: usize,
    pub text: String,
    pub start_char: usize,
    pub end_char: usize,
}
