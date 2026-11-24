use std::time::Duration;

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
