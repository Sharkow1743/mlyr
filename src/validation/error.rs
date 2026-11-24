use std::time::Duration;

use crate::model::{LineKind, VoiceId, VoiceRole};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("MLYR document cannot be completely empty")]
    EmptyDocument,

    #[error("Metadata field '{field}' cannot be empty or whitespace-only")]
    EmptyMetadataField { field: &'static str },

    #[error("Track length must be > 0ms")]
    NonPositiveTrackLength,

    #[error("Source 'type' attribute cannot be empty")]
    EmptySourceType,

    #[error("Source must provide at least a non-empty 'name' or text content")]
    EmptySourceAttribution,

    #[error("Voice id cannot be empty")]
    EmptyVoiceId,

    #[error("Voice '{id}' cannot contain an empty singer name")]
    EmptyVoiceSinger { id: VoiceId },

    #[error("Cannot declare more than 4 unique voices (found {count})")]
    TooManyVoices { count: usize },

    #[error("Voice '{id}' declared with conflicting roles: {existing:?} vs {conflicting:?}")]
    ConflictingVoiceRole {
        id: VoiceId,
        existing: VoiceRole,
        conflicting: VoiceRole,
    },

    #[error("Line references undeclared voice '{id}'")]
    UnknownVoice { id: VoiceId },

    #[error("{kind:?} line at {offset:?} must have duration > 0ms")]
    NonPositiveDuration { kind: LineKind, offset: Duration },

    #[error("{kind:?} line at {offset:?} with duration {duration:?} overflows Duration")]
    TimestampOverflow {
        kind: LineKind,
        offset: Duration,
        duration: Duration,
    },

    #[error(
        "{kind:?} line at {offset:?} ends at {line_end:?}, exceeding track length {track_length:?}"
    )]
    LineExceedsTrackLength {
        kind: LineKind,
        offset: Duration,
        line_end: Duration,
        track_length: Duration,
    },

    #[error(
        "Segment offset ({current:?}) is less than the previous segment ({previous:?}) in line at {line_offset:?}"
    )]
    SegmentOffsetDecreasing {
        line_offset: Duration,
        previous: Duration,
        current: Duration,
    },

    #[error(
        "Segment offset ({segment_offset:?}) exceeds line duration ({duration:?}) in line at {line_offset:?}"
    )]
    SegmentOutOfBounds {
        line_offset: Duration,
        duration: Duration,
        segment_offset: Duration,
    },

    #[error("Lines are not sorted chronologically by (offset, duration)")]
    LinesOutOfOrder,

    #[error(
        "Main line ({curr_offset:?}..{curr_end:?}) is nested inside the previous main line ({prev_offset:?}..{prev_end:?})"
    )]
    MainLineNested {
        prev_offset: Duration,
        prev_end: Duration,
        curr_offset: Duration,
        curr_end: Duration,
    },

    #[error(
        "More than 2 main lines active simultaneously: line at {curr_offset:?} begins before line ending at {prev2_end:?}"
    )]
    TooManyActiveMainLines {
        prev2_end: Duration,
        curr_offset: Duration,
    },

    #[error(
        "Background line at {curr_offset:?} overlaps the previous background line ending at {prev_end:?}"
    )]
    BgLineOverlap {
        prev_end: Duration,
        curr_offset: Duration,
    },
}
