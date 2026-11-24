use std::time::Duration;

use crate::model::{Line, Lyrics, Metadata, Mlyr};
use crate::validation::ValidationError;

pub struct MlyrBuilder {
    metadata: Metadata,
    lines: Vec<Line>,
    last_line_max_time: Option<Duration>,
}

impl Default for MlyrBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MlyrBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Metadata::default(),
            lines: Vec::new(),
            last_line_max_time: None,
        }
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn add_line(mut self, line: Line) -> Result<Self, ValidationError> {
        if let Some(line_offset) = line.offset {
            if let Some(prev_max) = self.last_line_max_time {
                if line_offset < prev_max {
                    return Err(ValidationError::LineOverlap {
                        current_line: line_offset,
                        prev_max_time: prev_max,
                    });
                }
            }

            let last_seg_offset = line
                .segments
                .last()
                .map(|s| s.offset)
                .unwrap_or(Duration::ZERO);
            let main_max_time = line_offset + last_seg_offset;

            let parallel_max_time = line.parallel.as_ref().map_or(Duration::ZERO, |p| {
                let last_p_seg_offset = p
                    .segments
                    .last()
                    .map(|s| s.offset)
                    .unwrap_or(Duration::ZERO);
                line_offset + p.offset.unwrap_or(Duration::ZERO) + last_p_seg_offset
            });

            self.last_line_max_time = Some(std::cmp::max(main_max_time, parallel_max_time));
        } else {
            self.last_line_max_time = None;
        }

        self.lines.push(line);
        Ok(self)
    }

    pub fn build(self) -> Result<Mlyr, ValidationError> {
        let mlyr = Mlyr {
            metadata: self.metadata,
            lyrics: Lyrics { lines: self.lines },
        };
        mlyr.validate()?;
        Ok(mlyr)
    }
}
