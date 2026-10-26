use crate::cast::CastManifest;
use crate::error::ScriptError;
use crate::turn::ScriptTurn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterScript {
    pub book_id: String,
    pub chapter_number: usize,
    pub title: String,
    pub turns: Vec<ScriptTurn>,
}

impl ChapterScript {
    /// Validates the chapter script: verifies sequential turn ordering, non-empty speech,
    /// cast manifest membership (if cast provided), and strict spoken text syntax.
    /// Returns an immutable `ValidatedScript` wrapper.
    pub fn validate(&self, cast: Option<&CastManifest>) -> Result<ValidatedScript, ScriptError> {
        if let Some(c) = cast {
            self.validate_cast(c)?;
        }

        for (i, turn) in self.turns.iter().enumerate() {
            if turn.turn_index != i {
                return Err(ScriptError::NonSequentialTurnOrder {
                    expected: i,
                    actual: turn.turn_index,
                });
            }
            if turn.clean_spoken_text.trim().is_empty() {
                return Err(ScriptError::EmptySpokenText { turn_index: i });
            }
            crate::validator::validate_spoken_text(&turn.speaker, &turn.clean_spoken_text)?;
        }

        Ok(ValidatedScript {
            script: self.clone(),
        })
    }

    pub fn full_spoken_text(&self) -> Result<String, ScriptError> {
        let validated = self.validate(None)?;
        Ok(validated.full_spoken_text())
    }

    pub fn speakers(&self) -> Vec<String> {
        let mut result = Vec::new();
        for turn in &self.turns {
            if !result.contains(&turn.speaker) {
                result.push(turn.speaker.clone());
            }
        }
        result
    }

    pub fn word_count(&self) -> usize {
        self.turns
            .iter()
            .map(|t| t.clean_spoken_text.split_whitespace().count())
            .sum()
    }

    pub fn validate_cast(&self, cast: &CastManifest) -> Result<(), ScriptError> {
        for turn in &self.turns {
            if !cast.is_cast(&turn.speaker) {
                return Err(ScriptError::MissingCharacterCast {
                    character: turn.speaker.clone(),
                });
            }
        }
        Ok(())
    }
}

pub struct ScriptBuilder {
    book_id: String,
    chapter_number: usize,
    title: String,
    turns: Vec<ScriptTurn>,
}

impl ScriptBuilder {
    pub fn new(
        book_id: impl Into<String>,
        chapter_number: usize,
        title: impl Into<String>,
    ) -> Self {
        Self {
            book_id: book_id.into(),
            chapter_number,
            title: title.into(),
            turns: Vec::new(),
        }
    }

    pub fn add_narrator(
        &mut self,
        text: impl Into<String>,
        span: Option<(usize, usize)>,
    ) -> Result<&mut Self, ScriptError> {
        let turn = ScriptTurn::narrator(self.turns.len(), text, span)?;
        self.turns.push(turn);
        Ok(self)
    }

    pub fn add_dialogue(
        &mut self,
        speaker: impl Into<String>,
        text: impl Into<String>,
        style: impl Into<String>,
        span: Option<(usize, usize)>,
    ) -> Result<&mut Self, ScriptError> {
        let turn = ScriptTurn::dialogue(self.turns.len(), speaker, text, style, span)?;
        self.turns.push(turn);
        Ok(self)
    }

    pub fn build(self, cast: Option<&CastManifest>) -> Result<ChapterScript, ScriptError> {
        let script = ChapterScript {
            book_id: self.book_id,
            chapter_number: self.chapter_number,
            title: self.title,
            turns: self.turns,
        };

        if let Some(cast_manifest) = cast {
            script.validate_cast(cast_manifest)?;
        }

        Ok(script)
    }
}

/// An immutable validated chapter script boundary wrapper.
/// Guaranteed to satisfy all spoken-text grammar, turn ordering, and cast requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedScript {
    script: ChapterScript,
}

impl ValidatedScript {
    pub fn validate(
        script: ChapterScript,
        cast: Option<&CastManifest>,
    ) -> Result<Self, ScriptError> {
        script.validate(cast)
    }

    pub fn full_spoken_text(&self) -> String {
        self.script
            .turns
            .iter()
            .map(|t| t.clean_spoken_text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn turns(&self) -> &[ScriptTurn] {
        &self.script.turns
    }

    pub fn book_id(&self) -> &str {
        &self.script.book_id
    }

    pub fn chapter_number(&self) -> usize {
        self.script.chapter_number
    }

    pub fn title(&self) -> &str {
        &self.script.title
    }

    pub fn word_count(&self) -> usize {
        self.script.word_count()
    }

    pub fn speakers(&self) -> Vec<String> {
        self.script.speakers()
    }

    pub fn into_inner(self) -> ChapterScript {
        self.script
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cast::{VoiceProfile, VoiceType};

    fn sample_cast() -> CastManifest {
        let mut cast = CastManifest::new(
            "book-01",
            VoiceProfile {
                voice_type: VoiceType::Prebuilt,
                voice_id: "Aoede".into(),
                description: "Narrator".into(),
                actor_notes: None,
            },
        );
        cast.add_character(
            "Mary",
            VoiceProfile {
                voice_type: VoiceType::VoiceDesign,
                voice_id: "voice_em_01".into(),
                description: "Friend".into(),
                actor_notes: None,
            },
        );
        cast
    }

    #[test]
    fn test_script_builder_assembles_chapter_script() {
        let cast = sample_cast();
        let mut builder = ScriptBuilder::new("book-01", 1, "The Storm");

        builder
            .add_narrator("Mary dropped her work and came to the door.", Some((0, 43)))
            .unwrap();

        builder
            .add_dialogue(
                "Mary",
                "Quick, Alice! <screams> Run for the cellar!",
                "screaming, terrified",
                Some((44, 88)),
            )
            .unwrap();

        let script = builder.build(Some(&cast)).unwrap();

        assert_eq!(script.book_id, "book-01");
        assert_eq!(script.chapter_number, 1);
        assert_eq!(script.title, "The Storm");
        assert_eq!(script.turns.len(), 2);
        assert_eq!(script.speakers(), vec!["Narrator", "Mary"]);

        let full_text = script.full_spoken_text().unwrap();
        assert!(full_text.contains("Mary dropped her work"));
        assert!(full_text.contains("Quick, Alice!"));

        assert!(script.word_count() > 10);
    }

    #[test]
    fn test_script_validation_fails_for_unregistered_character() {
        let cast = sample_cast(); // Only has Narrator and Mary
        let mut builder = ScriptBuilder::new("book-01", 1, "The Storm");

        builder
            .add_dialogue("The Villain", "I'll get you, my pretty!", "cackling", None)
            .unwrap();

        let err = builder.build(Some(&cast)).unwrap_err();
        match err {
            ScriptError::MissingCharacterCast { character } => {
                assert_eq!(character, "The Villain");
            }
            other => panic!("Expected MissingCharacterCast, got {:?}", other),
        }
    }
}
