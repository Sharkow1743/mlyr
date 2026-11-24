use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::format::normalize::{RawLine, RawLyrics, RawMlyr, RawParallelLine, RawSegment};
use crate::model::{Line, Lyrics, Metadata, Mlyr, ParallelLine, Segment, Source, Sources};

use super::serde_helpers::{XmlVoiceDe, duration_seconds, duration_seconds_opt, is_zero_u8};

// deserialization DTOs

#[derive(Deserialize)]
#[serde(rename = "mlyr")]
pub(crate) struct XmlMlyrDe {
    #[serde(default)]
    pub(crate) metadata: Option<XmlMetadataDe>,
    pub(crate) lyrics: XmlLyricsDe,
}

#[derive(Deserialize, Default)]
pub(crate) struct XmlMetadataDe {
    pub(crate) title: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) album: Option<String>,
    #[serde(with = "duration_seconds_opt", default)]
    pub(crate) length: Option<Duration>,
    #[serde(default)]
    pub(crate) sources: XmlSourcesDe,
}

#[derive(Deserialize, Default)]
pub(crate) struct XmlSourcesDe {
    #[serde(rename = "source", default)]
    pub(crate) list: Vec<XmlSourceDe>,
}

#[derive(Deserialize)]
pub(crate) struct XmlSourceDe {
    #[serde(rename = "@name")]
    pub(crate) name: String,
    #[serde(rename = "@url")]
    pub(crate) url: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct XmlLyricsDe {
    #[serde(rename = "line", default)]
    pub(crate) lines: Vec<XmlLineDe>,
}

#[derive(Deserialize)]
pub(crate) struct XmlLineDe {
    #[serde(rename = "@offset", with = "duration_seconds_opt", default)]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@voice")]
    pub(crate) voice: Option<XmlVoiceDe>,
    #[serde(rename = "segment", default)]
    pub(crate) segments: Vec<XmlSegmentDe>,
    pub(crate) parallel: Option<XmlParallelLineDe>,
}

#[derive(Deserialize)]
pub(crate) struct XmlParallelLineDe {
    #[serde(rename = "@offset", with = "duration_seconds_opt", default)]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@voice")]
    pub(crate) voice: Option<XmlVoiceDe>,
    #[serde(rename = "segment", default)]
    pub(crate) segments: Vec<XmlSegmentDe>,
}

#[derive(Deserialize)]
pub(crate) struct XmlSegmentDe {
    #[serde(rename = "@offset", with = "duration_seconds_opt", default)]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@text")]
    pub(crate) text: String,
}

impl From<XmlMlyrDe> for RawMlyr {
    fn from(xml: XmlMlyrDe) -> Self {
        Self {
            metadata: xml.metadata.map(Into::into),
            lyrics: xml.lyrics.into(),
        }
    }
}

impl From<XmlMetadataDe> for Metadata {
    fn from(m: XmlMetadataDe) -> Self {
        Self {
            title: m.title,
            artist: m.artist,
            album: m.album,
            length: m.length,
            sources: Sources {
                list: m.sources.list.into_iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<XmlSourceDe> for Source {
    fn from(s: XmlSourceDe) -> Self {
        Self {
            name: s.name,
            url: s.url,
        }
    }
}

impl From<XmlLyricsDe> for RawLyrics {
    fn from(l: XmlLyricsDe) -> Self {
        Self {
            lines: l.lines.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<XmlLineDe> for RawLine {
    fn from(l: XmlLineDe) -> Self {
        Self {
            offset: l.offset,
            voice: l.voice.map(|v| v.0),
            segments: l.segments.into_iter().map(Into::into).collect(),
            parallel: l.parallel.map(Into::into),
        }
    }
}

impl From<XmlParallelLineDe> for RawParallelLine {
    fn from(p: XmlParallelLineDe) -> Self {
        Self {
            offset: p.offset,
            voice: p.voice.map(|v| v.0),
            segments: p.segments.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<XmlSegmentDe> for RawSegment {
    fn from(s: XmlSegmentDe) -> Self {
        Self {
            offset: s.offset,
            text: s.text,
        }
    }
}

// serialization DTOs

#[derive(Serialize)]
#[serde(rename = "mlyr")]
pub(crate) struct XmlMlyrSer<'a> {
    #[serde(skip_serializing_if = "XmlMetadataSer::is_empty")]
    pub(crate) metadata: XmlMetadataSer<'a>,
    pub(crate) lyrics: XmlLyricsSer<'a>,
}

#[derive(Serialize)]
pub(crate) struct XmlMetadataSer<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) artist: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) album: Option<&'a str>,
    #[serde(with = "duration_seconds_opt", skip_serializing_if = "Option::is_none")]
    pub(crate) length: Option<Duration>,
    #[serde(skip_serializing_if = "XmlSourcesSer::is_empty")]
    pub(crate) sources: XmlSourcesSer<'a>,
}

impl XmlMetadataSer<'_> {
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.length.is_none()
            && self.sources.is_empty()
    }
}

#[derive(Serialize)]
pub(crate) struct XmlSourcesSer<'a> {
    #[serde(rename = "source")]
    pub(crate) list: Vec<XmlSourceSer<'a>>,
}

impl XmlSourcesSer<'_> {
    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

#[derive(Serialize)]
pub(crate) struct XmlSourceSer<'a> {
    #[serde(rename = "@name")]
    pub(crate) name: &'a str,
    #[serde(rename = "@url", skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct XmlLyricsSer<'a> {
    #[serde(rename = "line")]
    pub(crate) lines: Vec<XmlLineSer<'a>>,
}

#[derive(Serialize)]
pub(crate) struct XmlLineSer<'a> {
    #[serde(
        rename = "@offset",
        with = "duration_seconds_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@voice", skip_serializing_if = "is_zero_u8")]
    pub(crate) voice: u8,
    #[serde(rename = "segment")]
    pub(crate) segments: Vec<XmlSegmentSer<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parallel: Option<XmlParallelLineSer<'a>>,
}

#[derive(Serialize)]
pub(crate) struct XmlParallelLineSer<'a> {
    #[serde(
        rename = "@offset",
        with = "duration_seconds_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) offset: Option<Duration>,
    #[serde(rename = "@voice", skip_serializing_if = "is_zero_u8")]
    pub(crate) voice: u8,
    #[serde(rename = "segment")]
    pub(crate) segments: Vec<XmlSegmentSer<'a>>,
}

#[derive(Serialize)]
pub(crate) struct XmlSegmentSer<'a> {
    #[serde(rename = "@offset", with = "duration_seconds")]
    pub(crate) offset: Duration,
    #[serde(rename = "@text")]
    pub(crate) text: &'a str,
}

impl<'a> From<&'a Mlyr> for XmlMlyrSer<'a> {
    fn from(mlyr: &'a Mlyr) -> Self {
        Self {
            metadata: XmlMetadataSer::from(&mlyr.metadata),
            lyrics: XmlLyricsSer::from(&mlyr.lyrics),
        }
    }
}

impl<'a> From<&'a Metadata> for XmlMetadataSer<'a> {
    fn from(m: &'a Metadata) -> Self {
        Self {
            title: m.title.as_deref(),
            artist: m.artist.as_deref(),
            album: m.album.as_deref(),
            length: m.length,
            sources: XmlSourcesSer::from(&m.sources),
        }
    }
}

impl<'a> From<&'a Sources> for XmlSourcesSer<'a> {
    fn from(s: &'a Sources) -> Self {
        Self {
            list: s.list.iter().map(XmlSourceSer::from).collect(),
        }
    }
}

impl<'a> From<&'a Source> for XmlSourceSer<'a> {
    fn from(s: &'a Source) -> Self {
        Self {
            name: &s.name,
            url: s.url.as_deref(),
        }
    }
}

impl<'a> From<&'a Lyrics> for XmlLyricsSer<'a> {
    fn from(l: &'a Lyrics) -> Self {
        Self {
            lines: l.lines.iter().map(XmlLineSer::from).collect(),
        }
    }
}

impl<'a> From<&'a Line> for XmlLineSer<'a> {
    fn from(l: &'a Line) -> Self {
        Self {
            offset: l.offset,
            voice: l.voice,
            segments: l.segments.iter().map(XmlSegmentSer::from).collect(),
            parallel: l.parallel.as_ref().map(XmlParallelLineSer::from),
        }
    }
}

impl<'a> From<&'a ParallelLine> for XmlParallelLineSer<'a> {
    fn from(p: &'a ParallelLine) -> Self {
        Self {
            offset: p.offset,
            voice: p.voice,
            segments: p.segments.iter().map(XmlSegmentSer::from).collect(),
        }
    }
}

impl<'a> From<&'a Segment> for XmlSegmentSer<'a> {
    fn from(s: &'a Segment) -> Self {
        Self {
            offset: s.offset,
            text: &s.text,
        }
    }
}
