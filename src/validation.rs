use std::time::Duration;

use crate::{Line, Mlyr, ParallelLine};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error(
        "Segment offset ({current:?}) is less than the previous segment ({previous:?}) in line starting at {line_offset:?}"
    )]
    SegmentOffsetDecreasing {
        line_offset: Option<Duration>,
        previous: Duration,
        current: Duration,
    },
    #[error(
        "Line offset ({current_line:?}) begins before the previous line group fully finished (previous active time bound: {prev_max_time:?})"
    )]
    LineOverlap {
        current_line: Duration,
        prev_max_time: Duration,
    },
    #[error("Lyrics cannot contain more than 255 unique voices")]
    TooManyUniqueVoices,
}

impl Mlyr {
    /// Ensures that segments and lines go after each other
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut last_line_max_time: Option<Duration> = None;

        for line in &self.lyrics.lines {
            line.validate_local()?;

            // Global overlap check
            if let Some(current_end) = line.absolute_end_time() {
                if let Some(prev_max) = last_line_max_time {
                    if let Some(line_start) = line.offset {
                        if line_start < prev_max {
                            return Err(ValidationError::LineOverlap {
                                current_line: line_start,
                                prev_max_time: prev_max,
                            });
                        }
                    }
                }
                last_line_max_time = Some(current_end);
            }
        }

        Ok(())
    }
}

impl Line {
    /// Validates internal consistency of a single line
    pub fn validate_local(&self) -> Result<(), ValidationError> {
        let mut last_val = Duration::ZERO;
        for seg in &self.segments {
            if seg.offset < last_val {
                return Err(ValidationError::SegmentOffsetDecreasing {
                    line_offset: self.offset,
                    previous: last_val,
                    current: seg.offset,
                });
            }
            last_val = seg.offset;
        }

        if let Some(ref parallel) = self.parallel {
            parallel.validate_local()?;
        }

        Ok(())
    }
}

impl ParallelLine {
    pub fn validate_local(&self) -> Result<(), ValidationError> {
        let mut last_p_val = Duration::ZERO;
        for seg in &self.segments {
            if seg.offset < last_p_val {
                return Err(ValidationError::SegmentOffsetDecreasing {
                    line_offset: self.offset,
                    previous: last_p_val,
                    current: seg.offset,
                });
            }
            last_p_val = seg.offset;
        }

        Ok(())
    }
}
