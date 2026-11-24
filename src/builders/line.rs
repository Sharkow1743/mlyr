use std::time::Duration;

use crate::model::{Line, ParallelLine, Segment};
use crate::validation::ValidationError;

use super::IntoOffset;

pub struct LineBuilder {
    offset: Option<Duration>,
    voice: u8,
    segments: Vec<Segment>,
    parallel: Option<ParallelLine>,
}

impl LineBuilder {
    pub fn new<T: IntoOffset>(offset: T) -> Self {
        Self {
            offset: offset.into_offset(),
            voice: 0,
            segments: Vec::new(),
            parallel: None,
        }
    }

    pub fn voice(mut self, voice: u8) -> Self {
        self.voice = voice;
        self
    }

    pub fn add_segment<T: IntoOffset>(
        mut self,
        offset: T,
        text: String,
    ) -> Result<Self, ValidationError> {
        let current_val = offset.into_offset().unwrap_or(Duration::ZERO);
        if let Some(last_seg) = self.segments.last() {
            let last_val = last_seg.offset;
            if current_val < last_val {
                return Err(ValidationError::SegmentOffsetDecreasing {
                    line_offset: self.offset,
                    previous: last_val,
                    current: current_val,
                });
            }
        }
        self.segments.push(Segment {
            offset: current_val,
            text,
        });
        Ok(self)
    }

    pub fn parallel(mut self, parallel: ParallelLine) -> Self {
        self.parallel = Some(parallel);
        self
    }

    pub fn build(self) -> Result<Line, ValidationError> {
        let line = Line {
            offset: self.offset,
            voice: self.voice,
            segments: self.segments,
            parallel: self.parallel,
        };
        line.validate_local()?;
        Ok(line)
    }
}
