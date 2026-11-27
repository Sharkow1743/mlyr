use std::any::Any;
use std::time::Duration;

use crate::model::Source;

use super::{Key, ProviderExtensions};

#[derive(Debug, Clone, Default)]
pub struct TrackQuery {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    pub extensions: ProviderExtensions,
}

impl TrackQuery {
    pub fn new(title: impl Into<String>, artist: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            artist: artist.into(),
            album: None,
            duration: None,
            extensions: ProviderExtensions::new(),
        }
    }

    pub fn album(mut self, album: impl Into<String>) -> Self {
        self.album = Some(album.into());
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn with_extension<T: Any + Send + Sync>(mut self, key: Key<T>, value: T) -> Self {
        self.extensions.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FetchedMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub length: Option<Duration>,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Default)]
pub struct RawCandidate {
    pub payload: String,
    pub source_url: Option<String>,
    pub fetched_meta: FetchedMetadata,
    pub format_hint: Option<String>,
}

impl RawCandidate {
    pub fn new(payload: impl Into<String>) -> Self {
        Self {
            payload: payload.into(),
            source_url: None,
            fetched_meta: FetchedMetadata::default(),
            format_hint: None,
        }
    }

    pub fn with_source_url(mut self, url: impl Into<String>) -> Self {
        self.source_url = Some(url.into());
        self
    }

    pub fn with_fetched_meta(mut self, meta: FetchedMetadata) -> Self {
        self.fetched_meta = meta;
        self
    }

    pub fn with_format_hint(mut self, hint: impl Into<String>) -> Self {
        self.format_hint = Some(hint.into());
        self
    }
}
