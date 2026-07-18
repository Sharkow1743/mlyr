use super::conversion::duration_seconds_opt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename = "mlyr")]
pub struct Mlyr {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) metadata: Option<Metadata>,
    pub(crate) lyrics: Lyrics,
}

impl Mlyr {
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub fn lyrics(&self) -> &Lyrics {
        &self.lyrics
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_name: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Sources>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Sources {
    #[serde(rename = "source", default)]
    pub list: Vec<Source>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Source {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@url", skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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
    #[serde(rename = "@voice", default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
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
    #[serde(rename = "@voice")]
    pub voice: String,
    #[serde(rename = "segment", default)]
    pub segments: Vec<Segment>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Segment {
    #[serde(
        rename = "@offset",
        with = "duration_seconds_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub offset: Option<Duration>,
    #[serde(rename = "@text")]
    pub text: String,
}
