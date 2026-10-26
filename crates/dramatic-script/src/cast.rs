use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceType {
    Prebuilt,
    VoiceDesign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub voice_type: VoiceType,
    pub voice_id: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastManifest {
    pub book_id: String,
    pub narrator_voice: VoiceProfile,
    #[serde(default)]
    pub characters: HashMap<String, VoiceProfile>,
}

impl CastManifest {
    pub fn new(book_id: impl Into<String>, narrator_voice: VoiceProfile) -> Self {
        Self {
            book_id: book_id.into(),
            narrator_voice,
            characters: HashMap::new(),
        }
    }

    pub fn add_character(&mut self, name: impl Into<String>, profile: VoiceProfile) {
        self.characters.insert(name.into(), profile);
    }

    pub fn voice_for(&self, speaker: &str) -> &VoiceProfile {
        if speaker.eq_ignore_ascii_case("Narrator") {
            return &self.narrator_voice;
        }
        self.characters.get(speaker).unwrap_or(&self.narrator_voice)
    }

    pub fn is_cast(&self, speaker: &str) -> bool {
        speaker.eq_ignore_ascii_case("Narrator") || self.characters.contains_key(speaker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_narrator() -> VoiceProfile {
        VoiceProfile {
            voice_type: VoiceType::Prebuilt,
            voice_id: "Aoede".to_string(),
            description: "Warm, atmospheric Victorian storyteller".to_string(),
            actor_notes: None,
        }
    }

    fn sample_heroine() -> VoiceProfile {
        VoiceProfile {
            voice_type: VoiceType::VoiceDesign,
            voice_id: "voice_heroine_q78a".to_string(),
            description: "Young girl, earnest, sweet, brave cadence".to_string(),
            actor_notes: Some("Neutral accent".to_string()),
        }
    }

    #[test]
    fn test_cast_manifest_creation_and_lookup() {
        let mut cast = CastManifest::new("sample-book-01", sample_narrator());
        cast.add_character("Heroine", sample_heroine());

        assert_eq!(cast.book_id, "sample-book-01");
        assert!(cast.is_cast("Heroine"));
        assert!(cast.is_cast("Narrator"));
        assert!(!cast.is_cast("The Stranger"));

        let heroine_voice = cast.voice_for("Heroine");
        assert_eq!(heroine_voice.voice_id, "voice_heroine_q78a");
        assert_eq!(heroine_voice.voice_type, VoiceType::VoiceDesign);

        let narrator_voice = cast.voice_for("Narrator");
        assert_eq!(narrator_voice.voice_id, "Aoede");
        assert_eq!(narrator_voice.voice_type, VoiceType::Prebuilt);

        // Fallback for uncast speaker returns narrator
        let uncast_voice = cast.voice_for("The Stranger");
        assert_eq!(uncast_voice.voice_id, "Aoede");
    }

    #[test]
    fn test_cast_manifest_json_roundtrip() {
        let mut cast = CastManifest::new("sherlock-01", sample_narrator());
        cast.add_character("Heroine", sample_heroine());

        let json = serde_json::to_string_pretty(&cast).unwrap();
        assert!(json.contains("voice_design"));
        assert!(json.contains("prebuilt"));

        let deserialized: CastManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(cast, deserialized);
    }
}
