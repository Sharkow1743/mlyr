use std::time::Duration;

use super::{Segment, VoiceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineKind {
    #[default]
    Main,
    Background,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Line {
    pub kind: LineKind,
    pub offset: Duration,
    pub duration: Duration,
    pub voice: VoiceId,
    pub segments: Vec<Segment>,
}

impl Line {
    pub fn end(&self) -> Duration {
        self.offset + self.duration
    }

    pub fn is_bg(&self) -> bool {
        self.kind == LineKind::Background
    }
}
