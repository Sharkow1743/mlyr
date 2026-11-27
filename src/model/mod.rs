mod line;
mod lyrics;
mod metadata;
mod segment;
mod source;
mod sync_kind;
mod voice;

pub use line::{Line, LineKind};
pub use lyrics::Lyrics;
pub use metadata::Metadata;
pub use segment::Segment;
pub use source::Source;
pub use sync_kind::SyncKind;
pub use voice::{Voice, VoiceId, VoiceRole};

use std::time::Duration;

use crate::fetch::{Scorer, ScorerExtensions, TrackQuery};
use crate::{MlyrBuilder, SignedTimestamp};

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

    pub fn metadata_ref(&self) -> &Metadata {
        &self.metadata
    }

    pub fn lyrics(&self) -> &Lyrics {
        &self.lyrics
    }

    pub fn lines(&self) -> &[Line] {
        &self.lyrics.lines
    }

    pub fn into_builder(self) -> MlyrBuilder {
        MlyrBuilder::from_mlyr(self)
    }

    pub fn sync_kind(&self) -> SyncKind {
        if self.lyrics.lines.is_empty() {
            return SyncKind::None;
        }
        let is_word_synced = self
            .lyrics
            .lines
            .iter()
            .any(|l| l.segments.len() > 1 || l.segments.iter().any(|s| !s.offset.is_zero()));
        if is_word_synced {
            SyncKind::Word
        } else {
            SyncKind::Line
        }
    }

    pub fn score<S: Scorer + ?Sized>(&self, query: &TrackQuery, scorer: &S) -> f32 {
        scorer.score(
            std::slice::from_ref(query),
            self,
            None,
            &ScorerExtensions::new(),
        )
    }

    pub fn score_with<S: Scorer + ?Sized>(
        &self,
        queries: &[TrackQuery],
        scorer: &S,
        provider_id: Option<&str>,
        extensions: &ScorerExtensions,
    ) -> f32 {
        scorer.score(queries, self, provider_id, extensions)
    }

    /// Shifts all lyric line offsets, preserving relative spacing.
    /// Shifts are clamped to make sure line offsets don't become negative. Increases `metadata.length`, if needed.
    /// Returns clamped offset.
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

    pub fn default_voice(&self) -> VoiceId {
        self.metadata.default_voice()
    }
}
