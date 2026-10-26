use serde::{Deserialize, Deserializer, Serialize};

use crate::error::CaptionError;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptionCue {
    pub index: usize,
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub is_header: bool,
}

impl<'de> Deserialize<'de> for CaptionCue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawCue {
            index: usize,
            start: f64,
            end: f64,
            text: String,
            #[serde(default)]
            is_header: bool,
        }
        let raw = RawCue::deserialize(deserializer)?;
        CaptionCue::new(raw.index, raw.start, raw.end, raw.text, raw.is_header)
            .map_err(serde::de::Error::custom)
    }
}

impl CaptionCue {
    pub fn new(
        index: usize,
        start: f64,
        end: f64,
        text: impl Into<String>,
        is_header: bool,
    ) -> Result<Self, CaptionError> {
        let text_str = text.into();
        let char_count = text_str.chars().count();
        if char_count > 72 {
            return Err(CaptionError::CueLengthExceeded {
                chars: char_count,
                max: 72,
            });
        }
        if text_str.trim().is_empty() {
            return Err(CaptionError::InvalidConfig {
                reason: format!("cue {} text is empty", index),
            });
        }
        if !start.is_finite() || start < 0.0 {
            return Err(CaptionError::InvalidTimestamp {
                value: start,
                reason: "cue start must be finite and non-negative".into(),
            });
        }
        if !end.is_finite() || end < 0.0 {
            return Err(CaptionError::InvalidTimestamp {
                value: end,
                reason: "cue end must be finite and non-negative".into(),
            });
        }
        if start > end {
            return Err(CaptionError::InvalidTimeRange { start, end });
        }

        Ok(Self {
            index,
            start,
            end,
            text: text_str,
            is_header,
        })
    }

    pub fn duration(&self) -> f64 {
        self.end - self.start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptionConfig {
    pub max_chars: usize,
    pub soft_chars: usize,
    pub max_pause_seconds: f64,
    pub max_duration_seconds: f64,
}

impl Default for CaptionConfig {
    fn default() -> Self {
        Self {
            max_chars: 72,
            soft_chars: 36,
            max_pause_seconds: 0.45,
            max_duration_seconds: 6.0,
        }
    }
}

impl CaptionConfig {
    pub fn validate(&self) -> Result<(), CaptionError> {
        if !self.max_pause_seconds.is_finite() || self.max_pause_seconds <= 0.0 {
            return Err(CaptionError::InvalidConfig {
                reason: "max_pause_seconds must be finite and positive".into(),
            });
        }
        if !self.max_duration_seconds.is_finite() || self.max_duration_seconds <= 0.0 {
            return Err(CaptionError::InvalidConfig {
                reason: "max_duration_seconds must be finite and positive".into(),
            });
        }
        if self.soft_chars < 1 || self.soft_chars > self.max_chars || self.max_chars > 72 {
            return Err(CaptionError::InvalidConfig {
                reason: format!(
                    "must satisfy 1 <= soft_chars ({}) <= max_chars ({}) <= 72",
                    self.soft_chars, self.max_chars
                ),
            });
        }
        Ok(())
    }
}

/// An immutable validated caption track wrapper.
/// Enforces character limits (<= max_chars <= 72), ordered non-overlapping cues, and positive durations.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidatedCaptionTrack {
    cues: Vec<CaptionCue>,
    config: CaptionConfig,
}

impl<'de> Deserialize<'de> for ValidatedCaptionTrack {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawTrack {
            cues: Vec<CaptionCue>,
            config: CaptionConfig,
        }
        let raw = RawTrack::deserialize(deserializer)?;
        ValidatedCaptionTrack::new(raw.cues, raw.config).map_err(serde::de::Error::custom)
    }
}

impl ValidatedCaptionTrack {
    pub fn new(cues: Vec<CaptionCue>, config: CaptionConfig) -> Result<Self, CaptionError> {
        config.validate()?;
        let mut prev_end = 0.0;
        for (i, cue) in cues.iter().enumerate() {
            let char_count = cue.text.chars().count();
            if char_count > config.max_chars {
                return Err(CaptionError::CueLengthExceeded {
                    chars: char_count,
                    max: config.max_chars,
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
                    reason: "nonfinite or negative cue timestamp".into(),
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

        Ok(Self { cues, config })
    }

    pub fn cues(&self) -> &[CaptionCue] {
        &self.cues
    }

    pub fn config(&self) -> &CaptionConfig {
        &self.config
    }

    pub fn into_cues(self) -> Vec<CaptionCue> {
        self.cues
    }
}

impl std::ops::Deref for ValidatedCaptionTrack {
    type Target = [CaptionCue];

    fn deref(&self) -> &Self::Target {
        &self.cues
    }
}

impl<'a> IntoIterator for &'a ValidatedCaptionTrack {
    type Item = &'a CaptionCue;
    type IntoIter = std::slice::Iter<'a, CaptionCue>;

    fn into_iter(self) -> Self::IntoIter {
        self.cues.iter()
    }
}

impl IntoIterator for ValidatedCaptionTrack {
    type Item = CaptionCue;
    type IntoIter = std::vec::IntoIter<CaptionCue>;

    fn into_iter(self) -> Self::IntoIter {
        self.cues.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caption_cue_creation_and_duration() {
        let cue = CaptionCue::new(1, 2.5, 5.0, "Hello world", false).unwrap();
        assert_eq!(cue.index, 1);
        assert_eq!(cue.start, 2.5);
        assert_eq!(cue.end, 5.0);
        assert_eq!(cue.text, "Hello world");
        assert_eq!(cue.duration(), 2.5);
        assert!(!cue.is_header);

        // Invalid time range
        let err = CaptionCue::new(2, 6.0, 5.0, "Time travel", false).unwrap_err();
        assert!(matches!(err, CaptionError::InvalidTimeRange { .. }));
    }

    #[test]
    fn test_caption_config_defaults_and_validation() {
        let default_config = CaptionConfig::default();
        assert_eq!(default_config.max_chars, 72);
        assert_eq!(default_config.soft_chars, 36);
        assert!(default_config.validate().is_ok());

        let invalid = CaptionConfig {
            max_chars: 40,
            soft_chars: 50,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_caption_cue_serde_roundtrip() {
        let cue = CaptionCue::new(0, 0.0, 3.2, "Chapter 1: The Gathering", true).unwrap();
        let json = serde_json::to_string(&cue).unwrap();
        let deserialized: CaptionCue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, cue);
    }
}
