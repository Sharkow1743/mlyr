use std::time::Duration;

use super::{ParallelLine, Segment};

#[derive(Debug, PartialEq, Clone)]
pub struct Line {
    pub offset: Option<Duration>,
    pub voice: u8,
    pub segments: Vec<Segment>,
    pub parallel: Option<ParallelLine>,
}

impl Line {
    /// Returns the absolute end time of this line
    ///
    /// Returns `None` if the line has no offset
    pub fn absolute_end_time(&self) -> Option<Duration> {
        let line_offset = self.offset?;

        let main_end = line_offset
            + self
                .segments
                .last()
                .map(|s| s.offset)
                .unwrap_or(Duration::ZERO);

        let parallel_end = self.parallel.as_ref().map_or(Duration::ZERO, |p| {
            line_offset
                + p.offset.unwrap_or(Duration::ZERO)
                + p.segments
                    .last()
                    .map(|s| s.offset)
                    .unwrap_or(Duration::ZERO)
        });

        Some(std::cmp::max(main_end, parallel_end))
    }
}
