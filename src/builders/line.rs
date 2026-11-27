use std::time::Duration;

use crate::model::{Line, LineKind, Segment, VoiceId};
use crate::validation::ValidationError;

use super::IntoOffset;

pub struct LineBuilder {
    kind: LineKind,
    offset: Duration,
    duration: Duration,
    voice: VoiceId,
    segments: Vec<Segment>,
}

impl LineBuilder {
    pub fn new<O: IntoOffset, D: IntoOffset>(offset: O, duration: D) -> Self {
        Self {
            kind: LineKind::Main,
            offset: offset.into_offset().unwrap_or(Duration::ZERO),
            duration: duration.into_offset().unwrap_or(Duration::ZERO),
            voice: VoiceId::default_id(),
            segments: Vec::new(),
        }
    }

    pub fn main<O: IntoOffset, D: IntoOffset>(offset: O, duration: D) -> Self {
        Self::new(offset, duration)
    }

    pub fn background<O: IntoOffset, D: IntoOffset>(offset: O, duration: D) -> Self {
        Self {
            kind: LineKind::Background,
            offset: offset.into_offset().unwrap_or(Duration::ZERO),
            duration: duration.into_offset().unwrap_or(Duration::ZERO),
            voice: VoiceId::default_id(),
            segments: Vec::new(),
        }
    }

    pub fn kind(mut self, kind: LineKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn voice(mut self, voice: VoiceId) -> Self {
        self.voice = voice;
        self
    }

    pub fn add_segment<T: IntoOffset>(mut self, offset: T, text: impl Into<String>) -> Self {
        self.segments.push(Segment {
            offset: offset.into_offset().unwrap_or(Duration::ZERO),
            text: text.into(),
        });
        self
    }

    pub fn add_segments(mut self, segments: impl IntoIterator<Item = Segment>) -> Self {
        self.segments.extend(segments);
        self
    }

    pub fn build(self) -> Result<Line, ValidationError> {
        let line = Line {
            kind: self.kind,
            offset: self.offset,
            duration: self.duration,
            voice: self.voice,
            segments: self.segments,
        };
        line.validate_local()?;
        Ok(line)
    }
}
