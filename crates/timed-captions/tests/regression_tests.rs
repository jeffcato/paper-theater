use monotonic_align::{HeaderTiming, TranscriptWord};
use timed_captions::{
    cue_at, export_json, export_srt, export_webvtt, segment_tokens, segment_words, CaptionConfig,
    CaptionCue, CaptionError, ValidatedCaptionTrack,
};

#[test]
fn test_f11_caption_rejects_nonfinite_and_negative_timestamps() {
    assert!(
        CaptionCue::new(0, f64::NAN, 1.0, "text", false).is_err(),
        "NaN start time must be rejected"
    );
    assert!(
        CaptionCue::new(0, 0.0, f64::NAN, "text", false).is_err(),
        "NaN end time must be rejected"
    );
    assert!(
        CaptionCue::new(0, 0.0, f64::INFINITY, "text", false).is_err(),
        "Infinite timestamp must be rejected"
    );
    assert!(
        CaptionCue::new(0, -0.5, 1.0, "text", false).is_err(),
        "Negative start time must be rejected"
    );

    let cfg = CaptionConfig {
        max_pause_seconds: f64::NAN,
        ..Default::default()
    };
    assert!(
        cfg.validate().is_err(),
        "NaN max_pause_seconds must be rejected"
    );

    let cfg2 = CaptionConfig {
        max_duration_seconds: -1.0,
        ..Default::default()
    };
    assert!(
        cfg2.validate().is_err(),
        "Negative max_duration_seconds must be rejected"
    );
}

#[test]
fn test_f12_max_chars_exceeding_72_rejected_in_config() {
    let cfg_100 = CaptionConfig {
        max_chars: 100,
        ..Default::default()
    };
    assert!(
        cfg_100.validate().is_err(),
        "CaptionConfig with max_chars > 72 must fail validation"
    );

    let cfg_inverted = CaptionConfig {
        soft_chars: 50,
        max_chars: 40,
        ..Default::default()
    };
    assert!(
        cfg_inverted.validate().is_err(),
        "soft_chars > max_chars must fail validation"
    );

    let cfg_zero = CaptionConfig {
        soft_chars: 0,
        max_chars: 40,
        ..Default::default()
    };
    assert!(
        cfg_zero.validate().is_err(),
        "soft_chars < 1 must fail validation"
    );
}

#[test]
fn test_f12_single_oversized_token_is_split_within_72_bound() {
    let config = CaptionConfig::default();
    let words = vec![TranscriptWord {
        word: "x".repeat(73),
        start: 0.0,
        end: 1.0,
    }];

    let track = segment_words(&words, &config).unwrap();
    assert!(
        track.len() >= 2,
        "73-char token must be split into multiple cues"
    );
    for cue in track.iter() {
        let count = cue.text.chars().count();
        assert!(
            count <= 72,
            "Every chunk must respect the 72-character bound, got {count}"
        );
        assert!(
            cue.duration() > 0.0,
            "Split cue must have positive duration"
        );
    }

    // Combined text matches original
    let combined: String = track.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(combined, "x".repeat(73));
}

#[test]
fn test_f12_header_cue_chunked_within_72_bound() {
    let config = CaptionConfig::default();
    let header = HeaderTiming {
        title_range: Some((0.0, 5.0)),
        chapter_range: None,
        prose_start: 5.0,
    };
    let title = "x".repeat(100);

    let track = segment_tokens(&[], &config, Some(&header), Some(&title), None).unwrap();
    assert!(
        track.len() >= 2,
        "100-char header must be chunked into multiple cues"
    );
    for cue in track.iter() {
        let count = cue.text.chars().count();
        assert!(
            count <= 72,
            "Header cue chunk must not exceed 72 characters, got {count}"
        );
        assert!(cue.is_header);
        assert!(cue.duration() > 0.0);
    }
}

#[test]
fn test_f12_validated_caption_track_enforces_bounds_and_export() {
    let config = CaptionConfig::default();

    // Cue with 50 chars fails track validation with config.max_chars = 40
    let config_40 = CaptionConfig {
        max_chars: 40,
        soft_chars: 20,
        ..Default::default()
    };
    let bad_cues = vec![CaptionCue::new(0, 0.0, 1.0, "a".repeat(50), false).unwrap()];
    let res = ValidatedCaptionTrack::new(bad_cues, config_40);
    assert!(
        matches!(res, Err(CaptionError::CueLengthExceeded { .. })),
        "Track validation must reject cues exceeding config.max_chars"
    );

    // Valid cues succeed and can be exported
    let good_cues = vec![
        CaptionCue::new(0, 0.0, 2.0, "First cue.", false).unwrap(),
        CaptionCue::new(1, 2.5, 4.0, "Second cue.", false).unwrap(),
    ];
    let track = ValidatedCaptionTrack::new(good_cues, config).unwrap();
    assert_eq!(track.len(), 2);

    let vtt = export_webvtt(&track).unwrap();
    assert!(vtt.contains("First cue."));
    let srt = export_srt(&track).unwrap();
    assert!(srt.contains("Second cue."));
    let json = export_json(&track).unwrap();
    assert!(json.contains("First cue."));

    let looked_up = cue_at(&track, 1.0);
    assert_eq!(looked_up.map(|c| c.text.as_str()), Some("First cue."));
}

#[test]
fn test_r06_json_deserialization_and_cue_constructor_bounds() {
    // CaptionCue::new rejects text > 72 chars
    assert!(
        CaptionCue::new(0, 0.0, 1.0, "x".repeat(73), false).is_err(),
        "CaptionCue::new must reject text exceeding 72 characters"
    );

    // Deserializing CaptionCue with 100 characters must fail
    let json_bad_cue = r#"{"index":0,"start":0.0,"end":1.0,"text":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","is_header":false}"#;
    let cue_res: Result<CaptionCue, _> = serde_json::from_str(json_bad_cue);
    assert!(
        cue_res.is_err(),
        "Direct deserialization of oversized CaptionCue must fail"
    );

    // Deserializing CaptionCue with reversed time range must fail
    let json_rev_cue = r#"{"index":0,"start":2.0,"end":1.0,"text":"hello","is_header":false}"#;
    assert!(
        serde_json::from_str::<CaptionCue>(json_rev_cue).is_err(),
        "Direct deserialization of reversed CaptionCue must fail"
    );

    // Deserializing ValidatedCaptionTrack with invalid config (max_chars = 100) must fail
    let json_bad_cfg_track = r#"{
        "cues": [{"index":0,"start":0.0,"end":1.0,"text":"valid","is_header":false}],
        "config": {"max_chars":100,"soft_chars":36,"max_pause_seconds":0.45,"max_duration_seconds":6.0}
    }"#;
    let track_res: Result<ValidatedCaptionTrack, _> = serde_json::from_str(json_bad_cfg_track);
    assert!(
        track_res.is_err(),
        "Deserializing ValidatedCaptionTrack with max_chars > 72 must fail validation"
    );

    // Deserializing ValidatedCaptionTrack with reversed cue interval must fail
    let json_rev_track = r#"{
        "cues": [{"index":0,"start":2.0,"end":1.0,"text":"hello","is_header":false}],
        "config": {"max_chars":72,"soft_chars":36,"max_pause_seconds":0.45,"max_duration_seconds":6.0}
    }"#;
    let track_res2: Result<ValidatedCaptionTrack, _> = serde_json::from_str(json_rev_track);
    assert!(
        track_res2.is_err(),
        "Deserializing ValidatedCaptionTrack with reversed cue times must fail"
    );
}

#[test]
fn test_r06_raw_exporters_validate_all_inputs() {
    let mut bad_cue = CaptionCue::new(0, 0.0, 1.0, "valid", false).unwrap();
    bad_cue.text = "x".repeat(100);

    let bad_cues = vec![bad_cue.clone()];
    assert!(
        export_webvtt(&bad_cues).is_err(),
        "export_webvtt must reject raw oversized cues"
    );
    assert!(
        export_srt(&bad_cues).is_err(),
        "export_srt must reject raw oversized cues"
    );
    assert!(
        export_json(&bad_cues).is_err(),
        "export_json must reject raw oversized cues"
    );

    let rev_cues = vec![
        CaptionCue::new(0, 0.0, 2.0, "one", false).unwrap(),
        CaptionCue::new(1, 1.0, 3.0, "overlapping", false).unwrap(),
    ];
    assert!(
        export_webvtt(&rev_cues).is_err(),
        "export_webvtt must reject overlapping cues"
    );
    assert!(
        export_srt(&rev_cues).is_err(),
        "export_srt must reject overlapping cues"
    );
    assert!(
        export_json(&rev_cues).is_err(),
        "export_json must reject overlapping cues"
    );
}

#[test]
fn test_r11_segmenter_rejects_interior_nan_and_negative_duration() {
    use monotonic_align::SourceToken;

    let config = CaptionConfig::default();

    // 1. Interior word with start/end NaN in segment_words
    let words_with_nan = vec![
        TranscriptWord {
            word: "one".into(),
            start: 0.0,
            end: 0.3,
        },
        TranscriptWord {
            word: "bad".into(),
            start: f64::NAN,
            end: f64::NAN,
        },
        TranscriptWord {
            word: "three".into(),
            start: 0.6,
            end: 0.9,
        },
    ];
    let res = segment_words(&words_with_nan, &config);
    assert!(
        matches!(res, Err(CaptionError::InvalidTimestamp { .. })),
        "segment_words must reject interior word with NaN timestamps, got {:?}",
        res
    );

    // 2. Interior token with start/end NaN in segment_tokens
    let tokens_with_nan = vec![
        SourceToken {
            text: "one".into(),
            start: Some(0.0),
            end: Some(0.3),
            start_char: 0,
            end_char: 3,
            normalized: "one".into(),
        },
        SourceToken {
            text: "bad".into(),
            start: Some(f64::NAN),
            end: Some(f64::NAN),
            start_char: 4,
            end_char: 7,
            normalized: "bad".into(),
        },
        SourceToken {
            text: "three".into(),
            start: Some(0.6),
            end: Some(0.9),
            start_char: 8,
            end_char: 13,
            normalized: "three".into(),
        },
    ];
    let res_tok = segment_tokens(&tokens_with_nan, &config, None, None, None);
    assert!(
        matches!(res_tok, Err(CaptionError::InvalidTimestamp { .. })),
        "segment_tokens must reject interior token with NaN timestamps, got {:?}",
        res_tok
    );

    // 3. Long word (>72 chars) with negative duration (5.0 -> 1.0) in segment_words
    let long_word_rev = vec![TranscriptWord {
        word: "a".repeat(73),
        start: 5.0,
        end: 1.0,
    }];
    let res_long = segment_words(&long_word_rev, &config);
    assert!(
        res_long.is_err(),
        "segment_words must reject long word with negative duration"
    );

    // 4. Long token (>72 chars) with negative duration (5.0 -> 1.0) in segment_tokens
    let long_tok_rev = vec![SourceToken {
        text: "a".repeat(73),
        start: Some(5.0),
        end: Some(1.0),
        start_char: 0,
        end_char: 73,
        normalized: "a".repeat(73),
    }];
    let res_long_tok = segment_tokens(&long_tok_rev, &config, None, None, None);
    assert!(
        res_long_tok.is_err(),
        "segment_tokens must reject long token with negative duration"
    );

    // 5. Absent timestamps (None) alongside valid timestamps should be accepted
    let tokens_with_none = vec![
        SourceToken {
            text: "hello".into(),
            start: Some(0.0),
            end: Some(0.5),
            start_char: 0,
            end_char: 5,
            normalized: "hello".into(),
        },
        SourceToken {
            text: "world".into(),
            start: None,
            end: None,
            start_char: 6,
            end_char: 11,
            normalized: "world".into(),
        },
    ];
    let res_absent = segment_tokens(&tokens_with_none, &config, None, None, None);
    assert!(
        res_absent.is_ok(),
        "segment_tokens should accept absent (None) timestamps and synthesize them, got {:?}",
        res_absent
    );
}

#[test]
fn test_v07_r06_cue_at_requires_validated_track() {
    let config = CaptionConfig::default();
    let words = vec![
        TranscriptWord {
            word: "First".into(),
            start: 1.0,
            end: 2.0,
        },
        TranscriptWord {
            word: "Second".into(),
            start: 2.5,
            end: 4.0,
        },
    ];
    let track = segment_words(&words, &config).unwrap();

    // cue_at takes &ValidatedCaptionTrack directly
    assert_eq!(cue_at(&track, 1.5).map(|c| c.text.as_str()), Some("First"));
    assert_eq!(cue_at(&track, 2.2), None);
    assert_eq!(cue_at(&track, 3.0).map(|c| c.text.as_str()), Some("Second"));
}

#[test]
fn test_v08_r11_preserve_caption_input_timing() {
    let config = CaptionConfig::default();

    // 1. Zero-duration word in segment_words must return typed error
    let zero_dur_words = vec![TranscriptWord {
        word: "instant".into(),
        start: 1.0,
        end: 1.0,
    }];
    let res_zero = segment_words(&zero_dur_words, &config);
    assert!(
        matches!(res_zero, Err(CaptionError::ZeroDurationCue { .. })),
        "segment_words must reject zero-duration word, got {:?}",
        res_zero
    );

    // 2. Decreasing timestamps in segment_words must return typed error
    let decreasing_words = vec![
        TranscriptWord {
            word: "first".into(),
            start: 2.0,
            end: 3.0,
        },
        TranscriptWord {
            word: "second".into(),
            start: 1.0,
            end: 1.5,
        },
    ];
    let res_dec = segment_words(&decreasing_words, &config);
    assert!(
        res_dec.is_err(),
        "segment_words must reject decreasing timestamps"
    );

    // 3. Long word (>72 chars) splitting proportions exact duration without artificial padding
    let long_word = "a".repeat(80);
    let words = vec![TranscriptWord {
        word: long_word,
        start: 1.0,
        end: 3.0,
    }];
    let track = segment_words(&words, &config).unwrap();
    assert_eq!(track.len(), 2);
    assert_eq!(track[0].start, 1.0);
    assert_eq!(track[1].end, 3.0);
    assert_eq!(track[0].end, track[1].start);
}
