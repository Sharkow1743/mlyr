use std::fmt;
use std::ops::Deref;

use crate::validation::ValidationError;

pub const DEFAULT_VOICE_ID: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoiceId(String);

impl VoiceId {
    pub fn new(id: impl Into<String>) -> Result<Self, ValidationError> {
        let trimmed = id.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(ValidationError::EmptyVoiceId);
        }
        Ok(Self(trimmed))
    }

    pub fn default_id() -> Self {
        Self(DEFAULT_VOICE_ID.to_string())
    }

    pub fn is_default(&self) -> bool {
        self.0 == DEFAULT_VOICE_ID
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for VoiceId {
    fn default() -> Self {
        Self::default_id()
    }
}

impl AsRef<str> for VoiceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for VoiceId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for VoiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<&str> for VoiceId {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for VoiceId {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoiceRole {
    Primary,
    Secondary,
    Tertiary,
    Group,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    pub id: VoiceId,
    pub role: VoiceRole,
    pub singers: Vec<String>,
}

impl Voice {
    pub fn new(id: VoiceId, role: VoiceRole) -> Self {
        Self {
            id,
            role,
            singers: Vec::new(),
        }
    }

    pub fn with_singer(mut self, singer: impl Into<String>) -> Self {
        let trimmed = singer.into().trim().to_string();
        if !trimmed.is_empty() {
            self.singers.push(trimmed);
        }
        self
    }
}
