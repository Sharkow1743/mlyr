use super::conversion::IntoOffset;
use super::definition::*;
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
}

// Mlyr Builder
pub struct MlyrBuilder {
    metadata: Metadata,
    lines: Vec<Line>,
    last_line_max_time: Option<Duration>,
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

// Line Builder
pub struct LineBuilder {
    offset: Option<Duration>,
    voice: u8,
    segments: Vec<Segment>,
    parallel: Option<ParallelLine>,
}

impl LineBuilder {
    pub fn new<T: IntoOffset>(offset: T) -> Self {
        Self {
            offset: offset.into_offset(),
            voice: 0,
            segments: Vec::new(),
            parallel: None,
        }
    }

    pub fn voice(mut self, voice: u8) -> Self {
        self.voice = voice;
        self
    }

    pub fn add_segment<T: IntoOffset>(
        mut self,
        offset: T,
        text: String,
    ) -> Result<Self, ValidationError> {
        let current_val = offset.into_offset().unwrap_or(Duration::ZERO);
        if let Some(last_seg) = self.segments.last() {
            let last_val = last_seg.offset;
            if current_val < last_val {
                return Err(ValidationError::SegmentOffsetDecreasing {
                    line_offset: self.offset,
                    previous: last_val,
                    current: current_val,
                });
            }
        }
        self.segments.push(Segment {
            offset: current_val,
            text,
        });
        Ok(self)
    }

    pub fn parallel(mut self, parallel: ParallelLine) -> Self {
        self.parallel = Some(parallel);
        self
    }

    pub fn build(self) -> Line {
        Line {
            offset: self.offset,
            voice: self.voice,
            segments: self.segments,
            parallel: self.parallel,
        }
    }
}

// Parallel Line Builder
pub struct ParallelLineBuilder {
    offset: Option<Duration>,
    voice: u8,
    segments: Vec<Segment>,
}

impl ParallelLineBuilder {
    pub fn new<T: IntoOffset>(offset: T) -> Self {
        Self {
            offset: offset.into_offset(),
            voice: 0,
            segments: Vec::new(),
        }
    }

    pub fn add_segment<T: IntoOffset>(
        mut self,
        offset: T,
        text: String,
    ) -> Result<Self, ValidationError> {
        let current_val = offset.into_offset().unwrap_or(Duration::ZERO);
        if let Some(last_seg) = self.segments.last() {
            let last_val = last_seg.offset;
            if current_val < last_val {
                return Err(ValidationError::SegmentOffsetDecreasing {
                    line_offset: self.offset,
                    previous: last_val,
                    current: current_val,
                });
            }
        }
        self.segments.push(Segment {
            offset: current_val,
            text,
        });
        Ok(self)
    }

    pub fn voice(mut self, voice: u8) -> Self {
        self.voice = voice;
        self
    }

    pub fn build(self) -> ParallelLine {
        ParallelLine {
            offset: self.offset,
            voice: self.voice,
            segments: self.segments,
        }
    }
}

// Metadata Helper Constructors
impl Metadata {
    pub fn new(song_name: &str, artist: &str) -> Self {
        Self {
            song_name: Some(song_name.to_string()),
            artist: Some(artist.to_string()),
            album: None,
            length: None,
            sources: Sources::default(),
        }
    }

    pub fn album(mut self, album: &str) -> Self {
        self.album = Some(album.to_string());
        self
    }

    pub fn length(mut self, length: Duration) -> Self {
        self.length = Some(length);
        self
    }

    pub fn source(mut self, name: &str, url: Option<String>) -> Self {
        let new_source = Source {
            name: name.to_string(),
            url,
        };
        self.sources.list.push(new_source);
        self
    }
}

// Validation
impl Mlyr {
    /// Ensures that segments and lines go after each other
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut last_line_max_time: Option<Duration> = None;

        for line in &self.lyrics.lines {
            let mut last_val = Duration::ZERO;
            for (i, seg) in line.segments.iter().enumerate() {
                let current_val = seg.offset;
                if i > 0 && current_val < last_val {
                    return Err(ValidationError::SegmentOffsetDecreasing {
                        line_offset: line.offset,
                        previous: last_val,
                        current: current_val,
                    });
                }
                last_val = current_val;
            }

            if let Some(ref parallel) = line.parallel {
                let mut last_p_val = Duration::ZERO;
                for (i, seg) in parallel.segments.iter().enumerate() {
                    let current_val = seg.offset;
                    if i > 0 && current_val < last_p_val {
                        return Err(ValidationError::SegmentOffsetDecreasing {
                            line_offset: parallel.offset,
                            previous: last_p_val,
                            current: current_val,
                        });
                    }
                    last_p_val = current_val;
                }
            }

            if let Some(line_offset) = line.offset {
                if let Some(prev_max) = last_line_max_time {
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

                last_line_max_time = Some(std::cmp::max(main_max_time, parallel_max_time));
            } else {
                last_line_max_time = None;
            }
        }

        Ok(())
    }
}
