use crate::format::normalize::resolve_default_voice;
use crate::model::{Line, Lyrics, Metadata, Mlyr};
use crate::validation::ValidationError;

pub struct MlyrBuilder {
    metadata: Metadata,
    lines: Vec<Line>,
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
        }
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
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

    /// Builds and validates [`Mlyr`] model
    pub fn build(self) -> Result<Mlyr, ValidationError> {
        let mut lines = self.lines;

        if !self.metadata.voices.is_empty() {
            let has_explicit_default = self.metadata.voices.iter().any(|v| v.id.is_default());
            if !has_explicit_default {
                let default_target = resolve_default_voice(&self.metadata.voices);
                for line in &mut lines {
                    if line.voice.is_default() {
                        line.voice = default_target.clone();
                    }
                }
            }
        }

        lines.sort_by_key(|l| (l.offset, l.duration));

        let mlyr = Mlyr {
            metadata: self.metadata,
            lyrics: Lyrics { lines },
        };
        mlyr.validate()?;
        Ok(mlyr)
    }
}
