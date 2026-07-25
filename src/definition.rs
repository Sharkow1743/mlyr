use super::conversion::{duration_seconds, duration_seconds_opt, is_zero_u8};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(rename = "mlyr", try_from = "RawMlyr")]
pub struct Mlyr {
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub(crate) metadata: Metadata,
    pub(crate) lyrics: Lyrics,
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
