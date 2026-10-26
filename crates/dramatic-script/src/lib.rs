pub mod cast;
pub mod error;
pub mod script;
pub mod turn;
pub mod validator;

pub use cast::{CastManifest, VoiceProfile, VoiceType};
pub use error::ScriptError;
pub use script::{ChapterScript, ScriptBuilder, ValidatedScript};
pub use turn::{ScriptTurn, SpeechMetadata};
pub use validator::{validate_spoken_text, SANCTIONED_BACKCHANNELS, SANCTIONED_VOCALIZATIONS};
