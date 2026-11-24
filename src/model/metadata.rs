use std::time::Duration;

use super::{Source, Voice};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub length: Option<Duration>,
    pub voices: Vec<Voice>,
    pub sources: Vec<Source>,
}

impl Metadata {
    pub fn new(title: &str, artist: &str) -> Self {
        Self {
            title: Some(title.to_string()),
            artist: Some(artist.to_string()),
            album: None,
            length: None,
            voices: Vec::new(),
            sources: Vec::new(),
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

    pub fn voice(mut self, voice: Voice) -> Self {
        self.voices.push(voice);
        self
    }

    pub fn source(mut self, source: Source) -> Self {
        self.sources.push(source);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.length.is_none()
            && self.voices.is_empty()
            && self.sources.is_empty()
    }
}
