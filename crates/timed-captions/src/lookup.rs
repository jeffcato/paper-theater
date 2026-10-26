use crate::cue::{CaptionCue, ValidatedCaptionTrack};

/// Finds the active caption cue at playback timestamp `time` using binary search.
///
/// Returns `Some(&CaptionCue)` if `cue.start <= time && time <= cue.end`, or `None`
/// if `time` falls before, between, or after cues.
pub fn cue_at(track: &ValidatedCaptionTrack, time: f64) -> Option<&CaptionCue> {
    if track.is_empty() || time < 0.0 {
        return None;
    }

    let idx = track.cues().partition_point(|cue| cue.start <= time);
    if idx == 0 {
        return None;
    }

    let candidate = &track.cues()[idx - 1];
    if time <= candidate.end {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cue::CaptionConfig;

    #[test]
    fn test_binary_search_cue_lookup() {
        let cues = vec![
            CaptionCue::new(0, 1.0, 3.0, "First cue", false).unwrap(),
            CaptionCue::new(1, 4.0, 6.5, "Second cue", false).unwrap(),
            CaptionCue::new(2, 7.0, 10.0, "Third cue", false).unwrap(),
        ];
        let track = ValidatedCaptionTrack::new(cues, CaptionConfig::default()).unwrap();

        // Before any cue
        assert_eq!(cue_at(&track, -1.0), None);
        assert_eq!(cue_at(&track, 0.5), None);

        // First cue boundary and interior
        assert_eq!(cue_at(&track, 1.0).map(|c| c.index), Some(0));
        assert_eq!(cue_at(&track, 2.0).map(|c| c.index), Some(0));
        assert_eq!(cue_at(&track, 3.0).map(|c| c.index), Some(0));

        // Gap between cue 0 and cue 1 (3.0 < 3.5 < 4.0)
        assert_eq!(cue_at(&track, 3.5), None);

        // Second cue
        assert_eq!(cue_at(&track, 4.0).map(|c| c.index), Some(1));
        assert_eq!(cue_at(&track, 5.0).map(|c| c.index), Some(1));
        assert_eq!(cue_at(&track, 6.5).map(|c| c.index), Some(1));

        // Gap between cue 1 and cue 2 (6.5 < 6.8 < 7.0)
        assert_eq!(cue_at(&track, 6.8), None);

        // Third cue
        assert_eq!(cue_at(&track, 7.0).map(|c| c.index), Some(2));
        assert_eq!(cue_at(&track, 8.5).map(|c| c.index), Some(2));
        assert_eq!(cue_at(&track, 10.0).map(|c| c.index), Some(2));

        // After all cues
        assert_eq!(cue_at(&track, 10.5), None);
        assert_eq!(cue_at(&track, 100.0), None);

        // Empty cues
        let empty_track = ValidatedCaptionTrack::new(vec![], CaptionConfig::default()).unwrap();
        assert_eq!(cue_at(&empty_track, 1.0), None);
    }
}
