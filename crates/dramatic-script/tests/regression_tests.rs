use dramatic_script::{validate_spoken_text, ChapterScript, ScriptError, ValidatedScript};

#[test]
fn test_f14_unicode_speaker_separators_do_not_panic() {
    let cases = [
        ("Actor", "Actor—Hello", "Actor—"),
        ("Actor", "Actor–Hello", "Actor–"),
        ("Actor", "Actor: Hello", "Actor:"),
        ("Dorothée", "Dorothée—Bonjour", "Dorothée—"),
    ];

    for (speaker, text, expected_prefix) in cases {
        let res = validate_spoken_text(speaker, text);
        assert!(
            res.is_err(),
            "Expected verbatim speaker tag leak for '{text}'"
        );
        match res.unwrap_err() {
            ScriptError::VerbatimSpeakerTagLeak { found_prefix, .. } => {
                assert_eq!(found_prefix, expected_prefix);
            }
            other => panic!("Unexpected error variant: {:?}", other),
        }
    }
}

#[test]
fn test_f31_unterminated_markup_rejected() {
    let unclosed = [
        "[whispering",
        "(whispering",
        "<whispering",
        "<whispering>", // unsanctioned vocalization
    ];

    for text in unclosed {
        let res = validate_spoken_text("Actor", text);
        assert!(
            res.is_err(),
            "Unclosed markup or unsanctioned vocalization '{text}' must be rejected"
        );
    }
}

#[test]
fn test_f31_validated_script_import_boundary() {
    // Deserializing raw JSON script with stage directions
    let bad_json = r#"{
        "book_id": "book-1",
        "chapter_number": 1,
        "title": "Secret Meeting",
        "turns": [
            {
                "turn_index": 0,
                "speaker": "Narrator",
                "clean_spoken_text": "[whispering] Secret."
            }
        ]
    }"#;

    let script: ChapterScript = serde_json::from_str(bad_json).unwrap();
    let validation_result = script.validate(None);
    assert!(
        validation_result.is_err(),
        "Imported script with leaked stage directions must fail validation"
    );

    // Valid script validates and produces ValidatedScript wrapper
    let good_json = r#"{
        "book_id": "book-1",
        "chapter_number": 1,
        "title": "Clean Story",
        "turns": [
            {
                "turn_index": 0,
                "speaker": "Narrator",
                "clean_spoken_text": "The wind howled across the plain."
            }
        ]
    }"#;

    let script: ChapterScript = serde_json::from_str(good_json).unwrap();
    let validated: ValidatedScript = script.validate(None).unwrap();
    assert_eq!(
        validated.full_spoken_text(),
        "The wind howled across the plain."
    );
}

#[test]
fn test_r16_unicode_speaker_prefix_and_whitespace_separators() {
    // 1. Unicode case expansion: speaker 'İ' with 'İ—Secret'
    let res = validate_spoken_text("İ", "İ—Secret");
    assert!(
        res.is_err(),
        "Speaker 'İ' with 'İ—Secret' must be rejected as speaker tag leak"
    );
    match res.unwrap_err() {
        ScriptError::VerbatimSpeakerTagLeak { found_prefix, .. } => {
            assert_eq!(found_prefix, "İ—");
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }

    // 2. Newline before separator: 'Actor\n: Secret'
    let res_nl = validate_spoken_text("Actor", "Actor\n: Secret");
    assert!(
        res_nl.is_err(),
        "Speaker 'Actor' with 'Actor\\n: Secret' must be rejected"
    );
    match res_nl.unwrap_err() {
        ScriptError::VerbatimSpeakerTagLeak { found_prefix, .. } => {
            assert_eq!(found_prefix, "Actor\n:");
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }

    // 3. Non-breaking space before separator: 'Actor\u{00A0}: Secret'
    let res_nbsp = validate_spoken_text("Actor", "Actor\u{00A0}: Secret");
    assert!(
        res_nbsp.is_err(),
        "Speaker 'Actor' with non-breaking space separator must be rejected"
    );
    match res_nbsp.unwrap_err() {
        ScriptError::VerbatimSpeakerTagLeak { found_prefix, .. } => {
            assert_eq!(found_prefix, "Actor\u{00A0}:");
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }
}

#[test]
fn test_r16_raw_script_full_spoken_text_requires_validation() {
    let bad_json = r#"{
        "book_id": "book-1",
        "chapter_number": 1,
        "title": "Secret Meeting",
        "turns": [
            {
                "turn_index": 0,
                "speaker": "Narrator",
                "clean_spoken_text": "[whispering] Secret."
            }
        ]
    }"#;

    let script: ChapterScript = serde_json::from_str(bad_json).unwrap();
    // Raw ChapterScript::full_spoken_text must be fallible and reject unvalidated leaked markup
    assert!(
        script.full_spoken_text().is_err(),
        "Calling full_spoken_text on unvalidated script with leaked markup must fail"
    );

    let unicode_leak_json = r#"{
        "book_id": "book-1",
        "chapter_number": 1,
        "title": "Secret Meeting",
        "turns": [
            {
                "turn_index": 0,
                "speaker": "İ",
                "clean_spoken_text": "İ—Secret."
            }
        ]
    }"#;
    let unicode_script: ChapterScript = serde_json::from_str(unicode_leak_json).unwrap();
    assert!(
        unicode_script.full_spoken_text().is_err(),
        "Calling full_spoken_text on unvalidated script with Unicode speaker leak must fail"
    );
}
