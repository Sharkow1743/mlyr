use std::time::Duration;

use super::{Source, Voice, VoiceId, VoiceRole};

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

    /// Resolves the default voice for this metadata according to rules:
    /// 1. Voice with explicit ID `"default"`
    /// 2. First `VoiceRole::Primary`
    /// 3. First declared voice
    /// 4. Global `"default"` VoiceId
    pub fn default_voice(&self) -> VoiceId {
        if let Some(explicit) = self.voices.iter().find(|v| v.id.is_default()) {
            return explicit.id.clone();
        }
        if let Some(first_primary) = self.voices.iter().find(|v| v.role == VoiceRole::Primary) {
            return first_primary.id.clone();
        }
        if let Some(first_any) = self.voices.first() {
            return first_any.id.clone();
        }
        VoiceId::default_id()
    }
}
