use monotonic_align::SourceToken;
use timed_captions::{segment_tokens, CaptionConfig};

fn token(text: &str, start: Option<f64>, end: Option<f64>) -> SourceToken {
    SourceToken {
        text: text.into(),
        normalized: text.into(),
        start_char: 0,
        end_char: text.len(),
        start,
        end,
    }
}

#[test]
fn round4_partial_timestamps_rejected_before_caption_fallback() {
    for (start, end) in [
        (Some(1.0), None),
        (None, Some(1.0)),
        (Some(7.0), None),
        (None, Some(7.0)),
    ] {
        let tokens = [token("one", Some(5.0), Some(6.0)), token("two", start, end)];
        assert!(
            segment_tokens(&tokens, &CaptionConfig::default(), None, None, None).is_err(),
            "accepted partial timestamp {start:?}, {end:?}"
        );
    }
    let tokens = [token("one", Some(5.0), Some(6.0)), token("two", None, None)];
    let track = segment_tokens(&tokens, &CaptionConfig::default(), None, None, None).unwrap();
    assert_eq!(track.cues()[0].text, "one two");
    assert_eq!((track.cues()[0].start, track.cues()[0].end), (5.0, 6.0));
}
