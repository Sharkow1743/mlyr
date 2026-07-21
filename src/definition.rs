use super::conversion::{duration_seconds, duration_seconds_opt, is_zero_u8};
use serde::{Deserialize, Deserializer, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(rename = "mlyr", try_from = "RawMlyr")]
pub struct Mlyr {
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub(crate) metadata: Metadata,
    pub(crate) lyrics: Lyrics,
}

impl Mlyr {
    pub fn metadata(&self) -> Option<&Metadata> {
        if self.metadata.is_empty() {
            None
        } else {
            Some(&self.metadata)
        }
    }

    pub fn lyrics(&self) -> &Lyrics {
        &self.lyrics
    }

    /// Adjusts all timestamps in the lyrics by the given duration delta.
    /// Timestamps that would become negative are clamped to zero.
    pub fn shift_timestamps(&mut self, delta: Duration, forward: bool) {
        if let Some(ref mut len) = self.metadata.length {
            if forward {
                *len += delta;
            } else {
                *len = len.checked_sub(delta).unwrap_or(Duration::ZERO);
            }
        }

        for line in &mut self.lyrics.lines {
            if let Some(ref mut offset) = line.offset {
                if forward {
                    *offset += delta;
                } else {
                    *offset = offset.checked_sub(delta).unwrap_or(Duration::ZERO);
                }
            }

            if let Some(ref mut parallel) = line.parallel {
                if let Some(ref mut offset) = parallel.offset {
                    if forward {
                        *offset += delta;
                    } else {
                        *offset = offset.checked_sub(delta).unwrap_or(Duration::ZERO);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(
        with = "duration_seconds_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub length: Option<Duration>,
    #[serde(default, skip_serializing_if = "Sources::is_empty")]
    pub sources: Sources,
}

impl Metadata {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.length.is_none()
            && self.sources.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Sources {
    #[serde(rename = "source", default)]
    pub list: Vec<Source>,
}

impl Sources {
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Source {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@url", skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Lyrics {
    #[serde(rename = "line", default)]
    pub lines: Vec<Line>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Line {
    #[serde(
        rename = "@offset",
        with = "duration_seconds_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub offset: Option<Duration>,
    #[serde(rename = "@voice", default, skip_serializing_if = "is_zero_u8")]
    pub voice: u8,
    #[serde(rename = "segment", default)]
    pub segments: Vec<Segment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel: Option<ParallelLine>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ParallelLine {
    #[serde(
        rename = "@offset",
        with = "duration_seconds_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub offset: Option<Duration>,
    #[serde(rename = "@voice", default, skip_serializing_if = "is_zero_u8")]
    pub voice: u8,
    #[serde(rename = "segment", default)]
    pub segments: Vec<Segment>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Segment {
    #[serde(rename = "@offset", with = "duration_seconds")]
    pub offset: Duration,
    #[serde(rename = "@text")]
    pub text: String,
}

// Raw intermediate definitions for deserialization & voice/sorting normalization
#[derive(Deserialize)]
#[serde(rename = "mlyr")]
pub(crate) struct RawMlyr {
    pub(crate) metadata: Option<Metadata>,
    pub(crate) lyrics: RawLyrics,
}

#[derive(Deserialize)]
pub(crate) struct RawLyrics {
    #[serde(rename = "line", default)]
    pub(crate) lines: Vec<RawLine>,
}

#[derive(Deserialize)]
pub(crate) struct RawLine {
    #[serde(rename = "@offset", with = "duration_seconds_opt", default)]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@voice")]
    pub(crate) voice: Option<RawVoice>,
    #[serde(rename = "segment", default)]
    pub(crate) segments: Vec<RawSegment>,
    pub(crate) parallel: Option<RawParallelLine>,
}

#[derive(Deserialize)]
pub(crate) struct RawParallelLine {
    #[serde(rename = "@offset", with = "duration_seconds_opt", default)]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@voice")]
    pub(crate) voice: Option<RawVoice>,
    #[serde(rename = "segment", default)]
    pub(crate) segments: Vec<RawSegment>,
}

#[derive(Deserialize)]
pub(crate) struct RawSegment {
    #[serde(rename = "@offset", with = "duration_seconds_opt", default)]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@text")]
    pub(crate) text: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum RawVoice {
    Int(u64),
    Str(String),
}

impl<'de> Deserialize<'de> for RawVoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VoiceVisitor;
        impl<'de> serde::de::Visitor<'de> for VoiceVisitor {
            type Value = RawVoice;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("integer or string")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(RawVoice::Int(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                if v >= 0 {
                    Ok(RawVoice::Int(v as u64))
                } else {
                    Ok(RawVoice::Str(v.to_string()))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Ok(num) = v.parse::<u64>() {
                    Ok(RawVoice::Int(num))
                } else {
                    Ok(RawVoice::Str(v.to_string()))
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Ok(num) = v.parse::<u64>() {
                    Ok(RawVoice::Int(num))
                } else {
                    Ok(RawVoice::Str(v))
                }
            }
        }
        deserializer.deserialize_any(VoiceVisitor)
    }
}

impl TryFrom<RawMlyr> for Mlyr {
    type Error = super::ValidationError;

    fn try_from(raw: RawMlyr) -> Result<Self, Self::Error> {
        let mut voice_map = std::collections::HashMap::new();
        let mut next_voice_id = 1u8;

        let mut get_voice_id = |raw_voice: Option<RawVoice>| -> u8 {
            match raw_voice {
                Some(voice) => *voice_map.entry(voice).or_insert_with(|| {
                    let id = next_voice_id;
                    if next_voice_id < 255 {
                        next_voice_id += 1;
                    }
                    id
                }),
                None => 0,
            }
        };

        let mut lines = Vec::with_capacity(raw.lyrics.lines.len());
        for rl in raw.lyrics.lines {
            let mut segments: Vec<Segment> = rl
                .segments
                .into_iter()
                .map(|s| Segment {
                    offset: s.offset.unwrap_or(Duration::ZERO),
                    text: s.text,
                })
                .collect();
            segments.sort_by_key(|s| s.offset);

            let parallel = rl.parallel.map(|rp| {
                let mut p_segments: Vec<Segment> = rp
                    .segments
                    .into_iter()
                    .map(|s| Segment {
                        offset: s.offset.unwrap_or(Duration::ZERO),
                        text: s.text,
                    })
                    .collect();
                p_segments.sort_by_key(|s| s.offset);

                ParallelLine {
                    offset: rp.offset,
                    voice: get_voice_id(rp.voice),
                    segments: p_segments,
                }
            });

            lines.push(Line {
                offset: rl.offset,
                voice: get_voice_id(rl.voice),
                segments,
                parallel,
            });
        }

        // Normalize sorting chronologically
        lines.sort_by(|a, b| match (a.offset, b.offset) {
            (Some(oa), Some(ob)) => oa.cmp(&ob),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        });

        let mlyr = Mlyr {
            metadata: raw.metadata.unwrap_or_default(),
            lyrics: Lyrics { lines },
        };

        mlyr.validate()?;
        Ok(mlyr)
    }
}
