use monotonic_align::{
    align, align_chapter, extract_evidence, Transcript, TranscriptSegment, TranscriptWord,
};

fn make_transcript(words: &[(&str, f64, f64)]) -> Transcript {
    Transcript {
        text: None,
        audio_hash: None,
        segments: vec![TranscriptSegment {
            id: None,
            start: words.first().map(|w| w.1).unwrap_or(0.0),
            end: words.last().map(|w| w.2).unwrap_or(0.0),
            text: None,
            words: words
                .iter()
                .map(|&(word, start, end)| TranscriptWord {
                    word: word.into(),
                    start,
                    end,
                })
                .collect(),
        }],
    }
}

#[test]
fn test_f10_chapter_alignment_rebases_body_coordinates() {
    let body = "one two";
    let transcript = make_transcript(&[
        ("Book", 0.0, 0.5),
        ("Chapter", 1.0, 1.5),
        ("one", 2.0, 2.5),
        ("two", 3.0, 3.5),
    ]);

    let result = align_chapter(Some("Book"), "Chapter", body, &transcript, "audio-123").unwrap();

    assert_eq!(result.tokens.len(), 2);
    // Offsets must be relative to chapter body!
    assert_eq!(result.tokens[0].start_char, 0);
    assert_eq!(result.tokens[0].end_char, 3);
    assert_eq!(result.tokens[1].start_char, 4);
    assert_eq!(result.tokens[1].end_char, 7);

    // Extracting evidence from body coordinates 0..7 must succeed with accurate reveal time
    let ev = extract_evidence(body, &result, 0, 7).unwrap();
    assert_eq!(ev.quote, "one two");
    assert_eq!(ev.reveal_at, Some(3.5));
    assert_eq!(ev.coverage, 1.0);
}

#[test]
fn test_f11_transcript_rejects_nonfinite_and_negative_timestamps() {
    for (name, start, end) in [
        ("NaN start", f64::NAN, 1.0),
        ("NaN end", 0.0, f64::NAN),
        ("infinity", 0.0, f64::INFINITY),
        ("negative", -0.05, 0.5),
        ("negative end", 0.0, -0.01),
    ] {
        let transcript = make_transcript(&[("word", start, end)]);
        let res = align("word", &transcript, "audio");
        assert!(
            res.is_err(),
            "Nonfinite or negative timestamp ({name}) must be rejected"
        );
    }
}

#[test]
fn test_f33_transcript_rejects_decreasing_end_times() {
    let transcript = make_transcript(&[
        ("first", 0.0, 1.0),
        ("second", 0.95, 0.96), // end time decreases from 1.0 to 0.96!
    ]);
    let res = align("first second", &transcript, "audio");
    assert!(
        res.is_err(),
        "Decreasing word end times must be rejected by monotonic alignment"
    );
}

#[test]
fn test_f32_partial_terminal_word_evidence_rejected() {
    let text = "The girl vanished";
    let transcript = make_transcript(&[("The", 0.0, 0.5), ("girl", 1.0, 1.5)]);
    let alignment = align(text, &transcript, "audio").unwrap();

    // Span 0..12 cuts through "vanished" (which occupies 9..17)
    let cut_res = extract_evidence(text, &alignment, 0, 12);
    assert!(
        cut_res.is_err(),
        "Span cutting through a token boundary must be rejected"
    );

    // Aligned to token boundary 0..8 ("The girl") succeeds
    let valid_ev = extract_evidence(text, &alignment, 0, 8).unwrap();
    assert_eq!(valid_ev.quote, "The girl");
    assert_eq!(valid_ev.reveal_at, Some(1.5));

    // Full span 0..17 ending in omitted terminal word "vanished" succeeds with None reveal
    let omitted_ev = extract_evidence(text, &alignment, 0, 17).unwrap();
    assert_eq!(omitted_ev.quote, "The girl vanished");
    assert_eq!(omitted_ev.reveal_at, None);
    assert_eq!(omitted_ev.unmatched, vec!["vanished"]);
}

#[test]
fn test_r03_imported_alignment_missing_terminal_token_rejected() {
    let text = "one two";
    let transcript = make_transcript(&[("one", 1.0, 1.5), ("two", 2.0, 2.5)]);
    let mut alignment = align(text, &transcript, "audio").unwrap();

    // Malicious or corrupted imported alignment missing the terminal token
    alignment.tokens.pop();
    let res = extract_evidence(text, &alignment, 0, 7);
    assert!(
        res.is_err(),
        "Imported alignment missing terminal token must be rejected against canonical text"
    );
}

#[test]
fn test_r03_imported_alignment_nan_timestamp_rejected() {
    let text = "one two";
    let transcript = make_transcript(&[("one", 1.0, 1.5), ("two", 2.0, 2.5)]);
    let mut alignment = align(text, &transcript, "audio").unwrap();

    // Corrupted terminal timestamp set to NaN
    alignment.tokens[1].end = Some(f64::NAN);
    let res = extract_evidence(text, &alignment, 0, 7);
    assert!(
        res.is_err(),
        "Imported alignment containing NaN timestamp must be rejected"
    );
}

#[test]
fn test_r10_crossing_header_body_mismatch_preserved() {
    let transcript = make_transcript(&[("tail", 2.0, 2.5)]);
    let result = align_chapter(None, "Chapter", "missing tail", &transcript, "audio").unwrap();

    // Body token "missing" has no timing
    assert_eq!(result.tokens[0].text, "missing");
    assert_eq!(result.tokens[0].end, None);

    // Mismatches must contain the deletion crossing the header/body boundary!
    assert!(
        !result.mismatches.is_empty(),
        "Crossing header/body deletion must not be discarded from mismatches"
    );
    let m = &result.mismatches[0];
    assert_eq!(m.source_text, "missing");
    assert_eq!(m.source_range, (0, 1));
}

#[test]
fn test_v03_r03_imported_evidence_timing_validation() {
    let text = "one two";
    let transcript = make_transcript(&[("one", 0.0, 1.0), ("two", 1.0, 2.0)]);
    let base_alignment = align(text, &transcript, "audio").unwrap();

    // 1. Reject decreasing end timestamps [0, 1.0] followed by [0.95, 0.96]
    let mut decreasing = base_alignment.clone();
    decreasing.tokens[1].start = Some(0.95);
    decreasing.tokens[1].end = Some(0.96);
    assert!(
        extract_evidence(text, &decreasing, 0, 7).is_err(),
        "extract_evidence must reject imported alignment with decreasing end timestamps"
    );

    // 2. Reject partial timestamps: (None, Some(1.5)) and (Some(1.0), None)
    let mut partial1 = base_alignment.clone();
    partial1.tokens[1].start = None;
    partial1.tokens[1].end = Some(1.5);
    assert!(
        extract_evidence(text, &partial1, 0, 7).is_err(),
        "extract_evidence must reject partial timestamp (None, Some)"
    );

    let mut partial2 = base_alignment.clone();
    partial2.tokens[1].start = Some(1.0);
    partial2.tokens[1].end = None;
    assert!(
        extract_evidence(text, &partial2, 0, 7).is_err(),
        "extract_evidence must reject partial timestamp (Some, None)"
    );

    // 3. Accept multi-token shared words like "one two" both timed at [0.0, 1.0]
    let mut shared = base_alignment.clone();
    shared.tokens[0].start = Some(0.0);
    shared.tokens[0].end = Some(1.0);
    shared.tokens[1].start = Some(0.0);
    shared.tokens[1].end = Some(1.0);
    let shared_ev = extract_evidence(text, &shared, 0, 7).unwrap();
    assert_eq!(shared_ev.reveal_at, Some(1.0));

    // 4. Terminal token with (None, None) yields reveal_at: None
    let mut untimed_terminal = base_alignment.clone();
    untimed_terminal.tokens[1].start = None;
    untimed_terminal.tokens[1].end = None;
    let untimed_ev = extract_evidence(text, &untimed_terminal, 0, 7).unwrap();
    assert_eq!(untimed_ev.reveal_at, None);
}

#[test]
fn test_v09_r10_chapter_boundary_insertion_preserved() {
    let transcript = make_transcript(&[
        ("Chapter", 0.0, 0.5),
        ("extra", 0.6, 0.9),
        ("body", 1.0, 1.5),
    ]);

    let result = align_chapter(None, "Chapter", "body", &transcript, "audio").unwrap();

    // Body tokens should contain "body"
    assert_eq!(result.tokens.len(), 1);
    assert_eq!(result.tokens[0].text, "body");

    // The boundary insertion "extra" (after Chapter, before body) must NOT be dropped!
    assert!(
        !result.mismatches.is_empty(),
        "Boundary insertion between header and body must be retained in body mismatches"
    );
    let m = &result.mismatches[0];
    assert_eq!(m.kind, monotonic_align::MismatchKind::Insert);
    assert_eq!(m.heard_text, "extra");
    assert_eq!(
        m.source_range,
        (0, 0),
        "Boundary insertion must be rebased to body index (0, 0)"
    );
}
