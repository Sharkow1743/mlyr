mod line;
mod lyrics;
mod metadata;
mod segment;
mod source;
mod voice;

pub use line::{Line, LineKind};
pub use lyrics::Lyrics;
pub use metadata::Metadata;
pub use segment::Segment;
pub use source::Source;
pub use voice::{Voice, VoiceId, VoiceRole};

use std::time::Duration;

use crate::SignedTimestamp;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Mlyr {
    pub(crate) metadata: Metadata,
    pub(crate) lyrics: Lyrics,
}

impl Mlyr {
    pub fn metadata(&self) -> Option<&Metadata> {
        if self.metadata.is_empty() {
            None
        } else {
            Some(&self.metadata)
        }
    }

    pub fn lyrics(&self) -> &Lyrics {
        &self.lyrics
    }

    /// Shifts all lyric line offsets, preserving relative spacing
    ///
    /// Shifts are clamped to make sure line offsets don't become negative. Increases `metadata.length`, if needed.
    /// Returns clamped offset
    pub fn shift(&mut self, offset: impl Into<SignedTimestamp>) -> Duration {
        let offset = offset.into();
        if self.lyrics.lines.is_empty() || offset.magnitude.is_zero() {
            return Duration::ZERO;
        }

        if offset.negative {
            let min_offset = self
                .lyrics
                .lines
                .iter()
                .map(|l| l.offset)
                .min()
                .unwrap_or(Duration::ZERO);
            let effective = offset.magnitude.min(min_offset);
            for line in &mut self.lyrics.lines {
                line.offset -= effective;
            }
            effective
        } else {
            let mut max_end = Duration::ZERO;
            for line in &mut self.lyrics.lines {
                line.offset = line.offset.saturating_add(offset.magnitude);
                max_end = max_end.max(line.offset.saturating_add(line.duration));
            }
            if let Some(ref mut track_length) = self.metadata.length {
                *track_length = (*track_length).max(max_end);
            }
            offset.magnitude
        }
    }
}
