use std::time::Duration;

use super::{Source, Sources};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub length: Option<Duration>,
    pub sources: Sources,
}

impl Metadata {
    pub fn new(song_name: &str, artist: &str) -> Self {
        Self {
            title: Some(song_name.to_string()),
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
        self.sources.list.push(Source {
            name: name.to_string(),
            url,
        });
        self
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.length.is_none()
            && self.sources.is_empty()
    }
}
