use crate::cue::CaptionCue;
use crate::error::CaptionError;

/// Formats a time in seconds to HH:MM:SS.mmm or HH:MM:SS,mmm
pub fn format_timestamp(seconds: f64, separator: char) -> String {
    let total_millis = (seconds.max(0.0) * 1000.0).round() as u64;
    let hours = total_millis / 3_600_000;
    let minutes = (total_millis % 3_600_000) / 60_000;
    let secs = (total_millis % 60_000) / 1000;
    let millis = total_millis % 1000;
    format!(
        "{:02}:{:02}:{:02}{}{:03}",
        hours, minutes, secs, separator, millis
    )
}

pub fn validate_cues_for_export(cues: &[CaptionCue]) -> Result<(), CaptionError> {
    let mut prev_end = 0.0;
    for (i, cue) in cues.iter().enumerate() {
        let char_count = cue.text.chars().count();
        if char_count > 72 {
            return Err(CaptionError::CueLengthExceeded {
                chars: char_count,
                max: 72,
            });
        }
        if cue.text.trim().is_empty() {
            return Err(CaptionError::InvalidConfig {
                reason: format!("cue {} text is empty", cue.index),
            });
        }
        if !cue.start.is_finite() || cue.start < 0.0 || !cue.end.is_finite() || cue.end < 0.0 {
            return Err(CaptionError::InvalidTimestamp {
                value: cue.start,
                reason: "cue timestamp must be finite and non-negative".into(),
            });
        }
        if cue.end <= cue.start {
            return Err(CaptionError::ZeroDurationCue {
                start: cue.start,
                end: cue.end,
            });
        }
        if i > 0 && cue.start < prev_end {
            return Err(CaptionError::OverlappingCues {
                prev_end,
                next_start: cue.start,
            });
        }
        prev_end = cue.end;
    }
    Ok(())
}

/// Exports caption cues in standard WebVTT (`.vtt`) format for HTML5 `<track>`.
pub fn export_webvtt(cues: &[CaptionCue]) -> Result<String, CaptionError> {
    validate_cues_for_export(cues)?;
    let mut out = String::from("WEBVTT\n\n");
    for cue in cues {
        let start_str = format_timestamp(cue.start, '.');
        let end_str = format_timestamp(cue.end, '.');
        out.push_str(&format!("{} --> {}\n{}\n\n", start_str, end_str, cue.text));
    }
    Ok(out)
}

/// Exports caption cues in universal SubRip (`.srt`) format.
pub fn export_srt(cues: &[CaptionCue]) -> Result<String, CaptionError> {
    validate_cues_for_export(cues)?;
    let mut out = String::new();
    for (i, cue) in cues.iter().enumerate() {
        let start_str = format_timestamp(cue.start, ',');
        let end_str = format_timestamp(cue.end, ',');
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            start_str,
            end_str,
            cue.text
        ));
    }
    Ok(out)
}

/// Exports caption cues as formatted JSON for frontend player consumption.
pub fn export_json(cues: &[CaptionCue]) -> Result<String, CaptionError> {
    validate_cues_for_export(cues)?;
    serde_json::to_string_pretty(cues).map_err(|e| CaptionError::Json(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0, '.'), "00:00:00.000");
        assert_eq!(format_timestamp(1.5, '.'), "00:00:01.500");
        assert_eq!(format_timestamp(65.123, '.'), "00:01:05.123");
        assert_eq!(format_timestamp(3661.005, ','), "01:01:01,005");
    }

    #[test]
    fn test_export_webvtt_formatting() {
        let cues = vec![
            CaptionCue::new(0, 0.0, 3.2, "A Grand Tale", true).unwrap(),
            CaptionCue::new(1, 3.5, 6.0, "Chapter 1: The Beginning", true).unwrap(),
        ];

        let vtt = export_webvtt(&cues).unwrap();
        assert!(vtt.starts_with("WEBVTT\n\n"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:03.200\nA Grand Tale\n\n"));
        assert!(vtt.contains("00:00:03.500 --> 00:00:06.000\nChapter 1: The Beginning\n\n"));
    }

    #[test]
    fn test_export_srt_formatting() {
        let cues = vec![
            CaptionCue::new(0, 0.0, 3.2, "A Grand Tale", true).unwrap(),
            CaptionCue::new(1, 3.5, 6.0, "Chapter 1: The Beginning", true).unwrap(),
        ];

        let srt = export_srt(&cues).unwrap();
        assert!(srt.contains("1\n00:00:00,000 --> 00:00:03,200\nA Grand Tale\n\n"));
        assert!(srt.contains("2\n00:00:03,500 --> 00:00:06,000\nChapter 1: The Beginning\n\n"));
    }

    #[test]
    fn test_export_json_formatting() {
        let cues = vec![CaptionCue::new(0, 1.0, 2.0, "Hello", false).unwrap()];
        let json = export_json(&cues).unwrap();
        assert!(json.contains("\"text\": \"Hello\""));
        assert!(json.contains("\"start\": 1.0"));
        assert!(json.contains("\"end\": 2.0"));
    }
}
