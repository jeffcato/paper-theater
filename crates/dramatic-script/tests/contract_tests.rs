use dramatic_script::{validate_spoken_text, CastManifest, ChapterScript, ScriptError, VoiceType};

#[test]
fn test_oz_chapter1_script_full_contract_validation() {
    // 1. Load cast manifest
    let cast_raw = include_str!("fixtures/cast.json");
    let cast: CastManifest = serde_json::from_str(cast_raw).unwrap();

    assert_eq!(cast.book_id, "pg-55-wizard-of-oz");
    assert_eq!(cast.narrator_voice.voice_id, "Aoede");
    assert_eq!(cast.narrator_voice.voice_type, VoiceType::Prebuilt);

    let uncle_henry = cast.voice_for("Uncle Henry");
    assert_eq!(uncle_henry.voice_id, "Fenrir");
    assert_eq!(uncle_henry.voice_type, VoiceType::Prebuilt);

    let aunt_em = cast.voice_for("Aunt Em");
    assert_eq!(aunt_em.voice_id, "voice_aunt_em_k44b");
    assert_eq!(aunt_em.voice_type, VoiceType::VoiceDesign);

    // 2. Load chapter script
    let script_raw = include_str!("fixtures/oz_chapter1_script.json");
    let script: ChapterScript = serde_json::from_str(script_raw).unwrap();

    assert_eq!(script.book_id, "pg-55-wizard-of-oz");
    assert_eq!(script.chapter_number, 1);
    assert_eq!(script.title, "The Cyclone");
    assert_eq!(script.turns.len(), 5);

    // 3. Verify cast validation succeeds
    script.validate_cast(&cast).unwrap();

    // 4. Verify unique speakers
    assert_eq!(
        script.speakers(),
        vec!["Narrator", "Uncle Henry", "Aunt Em"]
    );

    // 5. Verify verbatim boundaries across all turns
    for turn in &script.turns {
        validate_spoken_text(&turn.speaker, &turn.clean_spoken_text).unwrap();
    }

    // Turn 3 has directorial style and inline vocalization
    let aunt_em_turn = &script.turns[3];
    assert_eq!(aunt_em_turn.speaker, "Aunt Em");
    assert_eq!(
        aunt_em_turn
            .speech_metadata
            .as_ref()
            .map(|m| m.style.as_str()),
        Some("screaming in terror, breathless, panicked")
    );
    assert!(aunt_em_turn.clean_spoken_text.contains("<screams>"));

    // 6. Verify continuous narrative text
    let full_text = script.full_spoken_text().unwrap();
    assert!(full_text.starts_with("Suddenly, Uncle Henry stood up."));
    assert!(full_text.contains("There's a cyclone coming, Em."));
    assert!(full_text.contains("Quick, Dorothy! <screams> Run for the cellar!"));
    assert!(full_text.ends_with("girl started to get him."));

    assert!(script.word_count() > 40);
}

#[test]
fn test_verbatim_leak_rejection() {
    // Attempting to speak speaker tag
    let err = validate_spoken_text("Aunt Em", "Aunt Em: Run!").unwrap_err();
    assert!(matches!(err, ScriptError::VerbatimSpeakerTagLeak { .. }));

    // Attempting to speak stage direction in parentheses
    let err2 = validate_spoken_text("Uncle Henry", "Run (in a panicked voice)!").unwrap_err();
    assert!(matches!(
        err2,
        ScriptError::VerbatimStageDirectionLeak { .. }
    ));

    // Attempting to speak stage direction in brackets
    let err3 = validate_spoken_text("Dorothy", "I'm looking [crying] for Toto!").unwrap_err();
    assert!(matches!(
        err3,
        ScriptError::VerbatimStageDirectionLeak { .. }
    ));

    // Attempting unsanctioned angle bracket direction
    let err4 = validate_spoken_text("Dorothy", "Come here <looking down at the dog>!").unwrap_err();
    assert!(matches!(err4, ScriptError::UnsanctionedVocalization { .. }));
}
