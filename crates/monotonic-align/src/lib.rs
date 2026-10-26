pub mod error;
pub mod evidence;
pub mod matcher;
pub mod models;
pub mod normalize;
pub mod paragraphs;

pub use error::AlignError;
pub use evidence::extract_evidence;
pub use matcher::{align, align_chapter};
pub use models::*;
pub use normalize::{normalize_token, tokenize_source};
pub use paragraphs::extract_paragraphs;

pub const VERSION: &str = "monotonic-exact-v1";
