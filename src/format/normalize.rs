#[cfg(any(feature = "convert", feature = "xml"))]
use std::collections::HashMap;
#[cfg(any(feature = "convert", feature = "xml"))]
use std::time::Duration;

#[cfg(any(feature = "convert", feature = "xml"))]
use crate::model::{Line, LineKind, Lyrics, Metadata, Mlyr, Segment, Source};
use crate::model::{Voice, VoiceId, VoiceRole};
#[cfg(any(feature = "convert", feature = "xml"))]
use crate::validation::ValidationError;

#[cfg(any(feature = "convert", feature = "xml"))]
use super::{FormatError, SignedTimestamp};

#[cfg(any(feature = "convert", feature = "xml"))]
#[allow(dead_code)]
pub(crate) struct RawMlyr {
    pub(crate) metadata: RawMetadata,
    pub(crate) lyrics_offset: Option<SignedTimestamp>,
    pub(crate) lines: Vec<RawLine>,
}

#[cfg(any(feature = "convert", feature = "xml"))]
#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct RawMetadata {
    pub(crate) title: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) length: Option<Duration>,
    pub(crate) voices: Vec<RawVoiceDecl>,
    pub(crate) sources: Vec<Source>,
}

#[cfg(any(feature = "convert", feature = "xml"))]
#[allow(dead_code)]
pub(crate) struct RawVoiceDecl {
    pub(crate) id: String,
    pub(crate) role: VoiceRole,
    pub(crate) singer: Option<String>,
}

#[cfg(any(feature = "convert", feature = "xml"))]
#[allow(dead_code)]
pub(crate) struct RawLine {
    pub(crate) kind: LineKind,
    pub(crate) offset: Duration,
    pub(crate) duration: Duration,
    pub(crate) voice: Option<String>,
    pub(crate) segments: Vec<Segment>,
}

pub(crate) fn resolve_default_voice(voices: &[Voice]) -> VoiceId {
    if let Some(explicit) = voices.iter().find(|v| v.id.is_default()) {
        return explicit.id.clone();
    }
    if let Some(first_primary) = voices.iter().find(|v| v.role == VoiceRole::Primary) {
        return first_primary.id.clone();
    }
    if let Some(first_any) = voices.first() {
        return first_any.id.clone();
    }
    VoiceId::default_id()
}

#[cfg(any(feature = "convert", feature = "xml"))]
#[allow(dead_code)]
pub(crate) fn normalize_raw_mlyr(raw: RawMlyr) -> Result<Mlyr, FormatError> {
    let voices = build_voice_table(raw.metadata.voices)?;
    let default_target = resolve_default_voice(&voices);

    let metadata = Metadata {
        title: raw.metadata.title,
        artist: raw.metadata.artist,
        album: raw.metadata.album,
        length: raw.metadata.length,
        voices,
        sources: raw.metadata.sources,
    };

    let mut lines = Vec::with_capacity(raw.lines.len());
    for rl in raw.lines {
        let voice = match rl.voice {
            Some(v) => {
                let vid = VoiceId::new(v)?;
                if vid.is_default() && !metadata.voices.iter().any(|decl| decl.id.is_default()) {
                    default_target.clone()
                } else {
                    vid
                }
            }
            None => default_target.clone(),
        };

        lines.push(Line {
            kind: rl.kind,
            offset: rl.offset,
            duration: rl.duration,
            voice,
            segments: rl.segments,
        });
    }

    if let Some(global) = raw.lyrics_offset {
        if !lines.is_empty() && !global.magnitude.is_zero() {
            if global.negative {
                let min_offset = lines
                    .iter()
                    .map(|l| l.offset)
                    .min()
                    .unwrap_or(Duration::ZERO);
                let shift = global.magnitude.min(min_offset);
                for line in &mut lines {
                    line.offset -= shift;
                }
            } else {
                for line in &mut lines {
                    line.offset += global.magnitude;
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

#[cfg(any(feature = "convert", feature = "xml"))]
fn build_voice_table(decls: Vec<RawVoiceDecl>) -> Result<Vec<Voice>, ValidationError> {
    let mut voices: Vec<Voice> = Vec::new();
    let mut index_by_id: HashMap<VoiceId, usize> = HashMap::new();

    for decl in decls {
        let id = VoiceId::new(decl.id)?;
        let singer = decl
            .singer
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        if let Some(&idx) = index_by_id.get(&id) {
            let existing = &mut voices[idx];
            if existing.role != decl.role {
                return Err(ValidationError::ConflictingVoiceRole {
                    id,
                    existing: existing.role,
                    conflicting: decl.role,
                });
            }
            if let Some(name) = singer {
                existing.singers.push(name);
            }
        } else {
            if voices.len() >= 4 {
                return Err(ValidationError::TooManyVoices {
                    count: voices.len() + 1,
                });
            }
            let mut voice = Voice::new(id.clone(), decl.role);
            if let Some(name) = singer {
                voice.singers.push(name);
            }
            index_by_id.insert(id, voices.len());
            voices.push(voice);
        }
    }

    Ok(voices)
}
