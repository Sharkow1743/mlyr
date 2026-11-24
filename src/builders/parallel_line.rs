use std::time::Duration;

use crate::model::{ParallelLine, Segment};
use crate::validation::ValidationError;

use super::IntoOffset;

pub struct ParallelLineBuilder {
    offset: Option<Duration>,
    voice: u8,
    segments: Vec<Segment>,
}

impl ParallelLineBuilder {
    pub fn new<T: IntoOffset>(offset: T) -> Self {
        Self {
            offset: offset.into_offset(),
            voice: 0,
            segments: Vec::new(),
        }
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

    pub fn voice(mut self, voice: u8) -> Self {
        self.voice = voice;
        self
    }

    pub fn build(self) -> Result<ParallelLine, ValidationError> {
        let p_line = ParallelLine {
            offset: self.offset,
            voice: self.voice,
            segments: self.segments,
        };

        p_line.validate_local()?;
        Ok(p_line)
    }
}
