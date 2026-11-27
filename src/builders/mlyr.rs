use std::collections::HashMap;
use std::time::Duration;

use crate::format::SignedTimestamp;
use crate::model::{Line, Lyrics, Metadata, Mlyr, Source, Voice, VoiceId};
use crate::validation::ValidationError;

pub struct MlyrBuilder {
    metadata: Metadata,
    lines: Vec<Line>,
    shift: Option<SignedTimestamp>,
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
            shift: None,
        }
    }

    pub fn from_mlyr(mlyr: Mlyr) -> Self {
        Self {
            metadata: mlyr.metadata,
            lines: mlyr.lyrics.lines,
            shift: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.metadata.title = Some(title.into());
        self
    }

    pub fn artist(mut self, artist: impl Into<String>) -> Self {
        self.metadata.artist = Some(artist.into());
        self
    }

    pub fn album(mut self, album: impl Into<String>) -> Self {
        self.metadata.album = Some(album.into());
        self
    }

    pub fn length(mut self, length: Duration) -> Self {
        self.metadata.length = Some(length);
        self
    }

    pub fn length_opt(mut self, length: Option<Duration>) -> Self {
        self.metadata.length = length;
        self
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }

    pub fn add_source(mut self, source: Source) -> Self {
        self.metadata.sources.push(source);
        self
    }

    pub fn add_sources(mut self, sources: impl IntoIterator<Item = Source>) -> Self {
        self.metadata.sources.extend(sources);
        self
    }

    pub fn add_voice(mut self, voice: Voice) -> Self {
        self.metadata.voices.push(voice);
        self
    }

    pub fn add_voices(mut self, voices: impl IntoIterator<Item = Voice>) -> Self {
        self.metadata.voices.extend(voices);
        self
    }

    pub fn add_line(mut self, line: Line) -> Self {
        self.lines.push(line);
        self
    }

    pub fn add_lines(mut self, lines: impl IntoIterator<Item = Line>) -> Self {
        self.lines.extend(lines);
        self
    }

    pub fn lines_mut(&mut self) -> &mut Vec<Line> {
        &mut self.lines
    }

    pub fn shift(mut self, shift: impl Into<SignedTimestamp>) -> Self {
        self.shift = Some(shift.into());
        self
    }

    pub fn retain_lines(&mut self, f: impl FnMut(&Line) -> bool) {
        self.lines.retain(f);
    }

    /// Builds, normalizes and validates [`Mlyr`] model
    pub fn build(self) -> Result<Mlyr, ValidationError> {
        let mut lines = self.lines;
        let mut metadata = self.metadata;

        // Deduplicate and merge voices by VoiceId
        if !metadata.voices.is_empty() {
            let mut merged_voices: Vec<Voice> = Vec::new();
            let mut index_by_id: HashMap<VoiceId, usize> = HashMap::new();

            for voice in metadata.voices {
                if let Some(&idx) = index_by_id.get(&voice.id) {
                    let existing = &mut merged_voices[idx];
                    if existing.role != voice.role {
                        return Err(ValidationError::ConflictingVoiceRole {
                            id: voice.id,
                            existing: existing.role,
                            conflicting: voice.role,
                        });
                    }
                    for singer in voice.singers {
                        if !existing.singers.contains(&singer) {
                            existing.singers.push(singer);
                        }
                    }
                } else {
                    index_by_id.insert(voice.id.clone(), merged_voices.len());
                    merged_voices.push(voice);
                }
            }
            metadata.voices = merged_voices;
        }

        // Resolve default voices
        if !metadata.voices.is_empty() {
            let has_explicit_default = metadata.voices.iter().any(|v| v.id.is_default());
            if !has_explicit_default {
                let default_target = metadata.default_voice();
                for line in &mut lines {
                    if line.voice.is_default() {
                        line.voice = default_target.clone();
                    }
                }
            }
        }

        // Apply global track shift
        if let Some(shift) = self.shift {
            if !lines.is_empty() && !shift.magnitude.is_zero() {
                if shift.negative {
                    let min_offset = lines
                        .iter()
                        .map(|l| l.offset)
                        .min()
                        .unwrap_or(Duration::ZERO);
                    let effective = shift.magnitude.min(min_offset);
                    for line in &mut lines {
                        line.offset -= effective;
                    }
                } else {
                    for line in &mut lines {
                        line.offset += shift.magnitude;
                    }
                }
            }
        }

        lines.sort_by_key(|l| (l.offset, l.duration));

        let mlyr = Mlyr {
            metadata,
            lyrics: Lyrics { lines },
        };
        mlyr.validate()?;
        Ok(mlyr)
    }
}

impl From<Mlyr> for MlyrBuilder {
    fn from(mlyr: Mlyr) -> Self {
        Self::from_mlyr(mlyr)
    }
}
