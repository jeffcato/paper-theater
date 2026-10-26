use monotonic_align::{align, extract_evidence, extract_paragraphs, Transcript};

#[test]
fn test_real_chapter_alignment_parity() {
    let chapter_text = include_str!("fixtures/chapter.txt");
    let transcript: Transcript =
        serde_json::from_str(include_str!("fixtures/transcript.json")).unwrap();

    let audio_hash = "3202d74deec26c853c9b6c611247768a1e01550a3f199429d3372623e39ef1aa";
    let result = align(chapter_text, &transcript, audio_hash).unwrap();

    // 1. Version matches protocol
    assert_eq!(result.version, "monotonic-exact-v1");
    assert_eq!(result.audio_hash, audio_hash);

    // 2. High coverage: >98% of book tokens matched
    assert!(
        result.coverage > 0.98,
        "Expected coverage > 98%, got {:.2}%",
        result.coverage * 100.0
    );

    // 3. LibriVox introduction is detected as the first mismatch
    assert!(!result.mismatches.is_empty());
    assert!(result.mismatches[0].heard_text.contains("librivox"));
    assert_eq!(result.mismatches[0].start, Some(0.0));

    // 4. First spoken chapter token is "Dorothy" starting around 23s (after LibriVox intro)
    assert_eq!(result.tokens[0].text, "Dorothy");
    let first_start = result.tokens[0].start.expect("First token should be timed");
    assert!(
        (first_start - 23.0).abs() < 1.0,
        "Expected Dorothy to start near 23s, got {}",
        first_start
    );

    // 5. Strict monotonic progression across all evidenced tokens
    let times: Vec<f64> = result.tokens.iter().filter_map(|t| t.end).collect();
    let mut sorted_times = times.clone();
    sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        times, sorted_times,
        "Token end times must be strictly monotonic"
    );

    // 6. Test paragraph extraction and evidence reveal calculation
    let paras = extract_paragraphs(chapter_text);
    assert!(!paras.is_empty());

    let p0 = &paras[0];
    let ev = extract_evidence(chapter_text, &result, p0.start_char, p0.end_char).unwrap();
    assert_eq!(ev.quote, p0.text);
    assert!(ev.coverage > 0.95);
    assert!(ev.reveal_at.is_some());
    let reveal = ev.reveal_at.unwrap();
    assert!(reveal > first_start);
}
