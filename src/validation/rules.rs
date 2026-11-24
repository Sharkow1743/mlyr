use std::collections::HashMap;
use std::time::Duration;

use crate::model::{Line, LineKind, Metadata, Mlyr, Source, VoiceId, VoiceRole};

use super::ValidationError;

const MAX_VOICES: usize = 4;

impl Mlyr {
    /// Validates the document against all MLYR invariants, metadata bounds,
    /// voice rules, segment bounds, and line concurrency constraints.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.metadata.is_empty() && self.lyrics.lines.is_empty() {
            return Err(ValidationError::EmptyDocument);
        }

        self.metadata.validate()?;

        let mut prev_sort_key: Option<(Duration, Duration)> = None;
        let mut prev2_main_end: Option<Duration> = None;
        let mut prev1_main: Option<(Duration, Duration)> = None;
        let mut prev_bg_end: Option<Duration> = None;

        for line in &self.lyrics.lines {
            validate_line_voice(&self.metadata, &line.voice)?;
            line.validate_local()?;

            let line_end = line.offset + line.duration;
            if let Some(track_length) = self.metadata.length {
                if line_end > track_length {
                    return Err(ValidationError::LineExceedsTrackLength {
                        kind: line.kind,
                        offset: line.offset,
                        line_end,
                        track_length,
                    });
                }
            }

            let key = (line.offset, line.duration);
            if let Some(prev_key) = prev_sort_key {
                if key < prev_key {
                    return Err(ValidationError::LinesOutOfOrder);
                }
            }
            prev_sort_key = Some(key);

            match line.kind {
                LineKind::Main => {
                    validate_main_step(prev2_main_end, prev1_main, line.offset, line_end)?;
                    prev2_main_end = prev1_main.map(|(_, end)| end);
                    prev1_main = Some((line.offset, line_end));
                }
                LineKind::Background => {
                    validate_bg_step(prev_bg_end, line.offset)?;
                    prev_bg_end = Some(line_end);
                }
            }
        }

        Ok(())
    }
}

impl Metadata {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_optional_text("title", self.title.as_deref())?;
        validate_optional_text("artist", self.artist.as_deref())?;
        validate_optional_text("album", self.album.as_deref())?;

        if let Some(length) = self.length {
            if length < Duration::from_millis(1) {
                return Err(ValidationError::NonPositiveTrackLength);
            }
        }

        for source in &self.sources {
            source.validate()?;
        }

        let mut seen: HashMap<&VoiceId, VoiceRole> = HashMap::with_capacity(self.voices.len());
        for voice in &self.voices {
            for singer in &voice.singers {
                if singer.trim().is_empty() {
                    return Err(ValidationError::EmptyVoiceSinger {
                        id: voice.id.clone(),
                    });
                }
            }

            if let Some(&existing) = seen.get(&voice.id) {
                if existing != voice.role {
                    return Err(ValidationError::ConflictingVoiceRole {
                        id: voice.id.clone(),
                        existing,
                        conflicting: voice.role,
                    });
                }
            } else {
                seen.insert(&voice.id, voice.role);
            }
        }

        if seen.len() > MAX_VOICES {
            return Err(ValidationError::TooManyVoices { count: seen.len() });
        }

        Ok(())
    }
}

impl Source {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.kind.trim().is_empty() {
            return Err(ValidationError::EmptySourceType);
        }

        let has_name = self.name.as_deref().is_some_and(|n| !n.trim().is_empty());
        let has_content = self
            .content
            .as_deref()
            .is_some_and(|c| !c.trim().is_empty());

        if !has_name && !has_content {
            return Err(ValidationError::EmptySourceAttribution);
        }

        Ok(())
    }
}

impl Line {
    /// Validates internal consistency of a single line (duration > 0, no overflow,
    /// segments monotonic and bounded).
    pub fn validate_local(&self) -> Result<(), ValidationError> {
        if self.duration < Duration::from_millis(1) {
            return Err(ValidationError::NonPositiveDuration {
                kind: self.kind,
                offset: self.offset,
            });
        }

        if self.offset.checked_add(self.duration).is_none() {
            return Err(ValidationError::TimestampOverflow {
                kind: self.kind,
                offset: self.offset,
                duration: self.duration,
            });
        }

        let mut last_val = Duration::ZERO;
        for seg in &self.segments {
            if seg.offset < last_val {
                return Err(ValidationError::SegmentOffsetDecreasing {
                    line_offset: self.offset,
                    previous: last_val,
                    current: seg.offset,
                });
            }
            if seg.offset > self.duration {
                return Err(ValidationError::SegmentOutOfBounds {
                    line_offset: self.offset,
                    duration: self.duration,
                    segment_offset: seg.offset,
                });
            }
            last_val = seg.offset;
        }

        Ok(())
    }
}

fn validate_optional_text(field: &'static str, value: Option<&str>) -> Result<(), ValidationError> {
    if let Some(s) = value {
        if s.trim().is_empty() {
            return Err(ValidationError::EmptyMetadataField { field });
        }
    }
    Ok(())
}

fn validate_line_voice(metadata: &Metadata, voice: &VoiceId) -> Result<(), ValidationError> {
    if metadata.voices.is_empty() {
        if !voice.is_default() {
            return Err(ValidationError::UnknownVoice { id: voice.clone() });
        }
    } else if !metadata.voices.iter().any(|v| &v.id == voice) {
        return Err(ValidationError::UnknownVoice { id: voice.clone() });
    }
    Ok(())
}

fn validate_main_step(
    prev2_end: Option<Duration>,
    prev1: Option<(Duration, Duration)>,
    curr_offset: Duration,
    curr_end: Duration,
) -> Result<(), ValidationError> {
    if let Some((prev_offset, prev_end)) = prev1 {
        if curr_offset < prev_end && curr_end < prev_end {
            return Err(ValidationError::MainLineNested {
                prev_offset,
                prev_end,
                curr_offset,
                curr_end,
            });
        }
    }

    if let Some(p2_end) = prev2_end {
        if curr_offset < p2_end {
            return Err(ValidationError::TooManyActiveMainLines {
                prev2_end: p2_end,
                curr_offset,
            });
        }
    }

    Ok(())
}

fn validate_bg_step(
    prev_end: Option<Duration>,
    curr_offset: Duration,
) -> Result<(), ValidationError> {
    if let Some(p_end) = prev_end {
        if curr_offset < p_end {
            return Err(ValidationError::BgLineOverlap {
                prev_end: p_end,
                curr_offset,
            });
        }
    }
    Ok(())
}
