use monotonic_align::{align_chapter, Transcript};
use timed_captions::{
    cue_at, export_json, export_srt, export_webvtt, segment_tokens, CaptionConfig, CaptionCue,
};

#[test]
fn test_timed_captions_end_to_end_chapter_parity() {
    let chapter_text = include_str!("../../monotonic-align/tests/fixtures/chapter.txt");
    let transcript: Transcript = serde_json::from_str(include_str!(
        "../../monotonic-align/tests/fixtures/transcript.json"
    ))
    .unwrap();

    let book_title = "The Wonderful Wizard of Oz";
    let chapter_title = "Chapter I. The Cyclone";
    let audio_hash = "3202d74deec26c853c9b6c611247768a1e01550a3f199429d3372623e39ef1aa";

    // 1. Run Monotonic Alignment
    let result = align_chapter(
        Some(book_title),
        chapter_title,
        chapter_text,
        &transcript,
        audio_hash,
    )
    .unwrap();

    assert!(result.header_timing.is_some());
    let header_timing = result.header_timing.as_ref().unwrap();
    assert!(header_timing.title_range.is_some());
    assert!(header_timing.chapter_range.is_some());

    // 2. Segment Cues with default config (max_chars = 72, soft_chars = 36)
    let config = CaptionConfig::default();
    let cues = segment_tokens(
        &result.tokens,
        &config,
        result.header_timing.as_ref(),
        Some(book_title),
        Some(chapter_title),
    )
    .unwrap();

    assert!(!cues.is_empty(), "Cues should not be empty");

    // 3. Verify Header Cues
    assert!(
        cues[0].is_header,
        "First cue should be book title announcement"
    );
    assert_eq!(cues[0].text, book_title);
    assert!(
        cues[1].is_header,
        "Second cue should be chapter title announcement"
    );
    assert_eq!(cues[1].text, chapter_title);

    // 4. Bug Verification: Zero cues exceed 72 characters
    for (i, cue) in cues.iter().enumerate() {
        assert!(
            cue.text.chars().count() <= config.max_chars,
            "Cue {} exceeds max_chars {}: '{}' (chars {})",
            i,
            config.max_chars,
            cue.text,
            cue.text.chars().count()
        );
        assert!(
            cue.start <= cue.end,
            "Cue {} time range inverted: start {} > end {}",
            i,
            cue.start,
            cue.end
        );
    }

    // 5. Test Binary Search Playback Scrubber (cue_at)
    let max_end = cues.last().unwrap().end;
    let step = max_end / 100.0;
    for i in 0..100 {
        let t = i as f64 * step;
        if let Some(cue) = cue_at(&cues, t) {
            assert!(
                cue.start <= t && t <= cue.end,
                "cue_at returned cue [{}, {}] for timestamp {}",
                cue.start,
                cue.end,
                t
            );
        }
    }

    // 6. Test WebVTT Export
    let vtt = export_webvtt(&cues).unwrap();
    assert!(vtt.starts_with("WEBVTT\n\n"));
    assert!(vtt.contains(book_title));
    assert!(vtt.contains(chapter_title));
    assert!(vtt.contains("-->"));

    // 7. Test SRT Export
    let srt = export_srt(&cues).unwrap();
    assert!(srt.starts_with("1\n"));
    assert!(srt.contains(","));
    assert!(srt.contains(book_title));

    // 8. Test JSON Roundtrip Export
    let json_str = export_json(&cues).unwrap();
    let roundtripped: Vec<CaptionCue> = serde_json::from_str(&json_str).unwrap();
    assert_eq!(roundtripped.len(), cues.len());
    assert_eq!(roundtripped[0], cues[0]);
    assert_eq!(roundtripped[1], cues[1]);
}
