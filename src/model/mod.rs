mod line;
mod lyrics;
mod metadata;
mod parallel_line;
mod segment;
mod source;

pub use line::Line;
pub use lyrics::Lyrics;
pub use metadata::Metadata;
pub use parallel_line::ParallelLine;
pub use segment::Segment;
pub use source::{Source, Sources};

use std::time::Duration;

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

    /// Adjusts line offsets in the lyrics by the given duration delta
    ///
    /// Timestamps that would become negative are clamped to zero
    pub fn shift_timestamps(&mut self, delta: Duration, forward: bool) {
        if let Some(ref mut len) = self.metadata.length {
            if forward {
                *len += delta;
            } else {
                *len = len.checked_sub(delta).unwrap_or(Duration::ZERO);
            }
        }

        for line in &mut self.lyrics.lines {
            if let Some(ref mut offset) = line.offset {
                if forward {
                    *offset += delta;
                } else {
                    *offset = offset.checked_sub(delta).unwrap_or(Duration::ZERO);
                }
            }

            if let Some(ref mut parallel) = line.parallel {
                if let Some(ref mut offset) = parallel.offset {
                    if forward {
                        *offset += delta;
                    } else {
                        *offset = offset.checked_sub(delta).unwrap_or(Duration::ZERO);
                    }
                }
            }
        }
    }
}
