use std::time::Duration;

use serde::Deserialize;

use crate::format::FormatError;
use crate::format::inline_text::parse_inline_synced_text;
use crate::format::normalize::{RawLine, RawMetadata, RawMlyr, RawVoiceDecl};
use crate::format::timestamp::{SignedTimestamp, parse_timestamp};
use crate::model::{LineKind, Source, VoiceRole};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "mlyr")]
pub(crate) struct XmlMlyrDe {
    #[serde(default)]
    metadata: Option<XmlMetadataDe>,
    #[serde(default)]
    lyrics: Option<XmlLyricsDe>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct XmlMetadataDe {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    length: Option<String>,
    voices: Option<XmlVoicesDe>,
    #[serde(rename = "source", default)]
    sources: Vec<XmlSourceDe>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct XmlSourceDe {
    #[serde(rename = "@type")]
    kind: String,
    #[serde(rename = "@name")]
    name: Option<String>,
    #[serde(rename = "$text", default)]
    content: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct XmlVoicesDe {
    #[serde(rename = "$value", default)]
    declarations: Vec<XmlVoiceDeclDe>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum XmlVoiceDeclDe {
    #[serde(rename = "primary")]
    Primary(XmlVoiceBodyDe),
    #[serde(rename = "secondary")]
    Secondary(XmlVoiceBodyDe),
    #[serde(rename = "tertiary")]
    Tertiary(XmlVoiceBodyDe),
    #[serde(rename = "group")]
    Group(XmlVoiceBodyDe),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct XmlVoiceBodyDe {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "$text", default)]
    singer: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct XmlLyricsDe {
    #[serde(rename = "@offset")]
    offset: Option<String>,
    #[serde(rename = "$value", default)]
    entries: Vec<XmlLyricsEntryDe>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum XmlLyricsEntryDe {
    #[serde(rename = "line")]
    Line(XmlLineBodyDe),
    #[serde(rename = "bg")]
    Bg(XmlLineBodyDe),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct XmlLineBodyDe {
    #[serde(rename = "@offset")]
    offset: Option<String>,
    #[serde(rename = "@duration")]
    duration: String,
    #[serde(rename = "@voice")]
    voice: Option<String>,
    #[serde(rename = "$text", default)]
    text: String,
}

impl TryFrom<XmlMlyrDe> for RawMlyr {
    type Error = FormatError;

    fn try_from(xml: XmlMlyrDe) -> Result<Self, Self::Error> {
        let metadata = match xml.metadata {
            Some(m) => {
                let length = m.length.as_deref().map(parse_timestamp).transpose()?;
                let voices = m
                    .voices
                    .map(|v| {
                        v.declarations
                            .into_iter()
                            .map(|decl| match decl {
                                XmlVoiceDeclDe::Primary(b) => RawVoiceDecl {
                                    id: b.id,
                                    role: VoiceRole::Primary,
                                    singer: b.singer,
                                },
                                XmlVoiceDeclDe::Secondary(b) => RawVoiceDecl {
                                    id: b.id,
                                    role: VoiceRole::Secondary,
                                    singer: b.singer,
                                },
                                XmlVoiceDeclDe::Tertiary(b) => RawVoiceDecl {
                                    id: b.id,
                                    role: VoiceRole::Tertiary,
                                    singer: b.singer,
                                },
                                XmlVoiceDeclDe::Group(b) => RawVoiceDecl {
                                    id: b.id,
                                    role: VoiceRole::Group,
                                    singer: b.singer,
                                },
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let sources = m
                    .sources
                    .into_iter()
                    .map(|s| Source {
                        kind: s.kind,
                        name: s.name,
                        content: s.content,
                    })
                    .collect();

                RawMetadata {
                    title: m.title,
                    artist: m.artist,
                    album: m.album,
                    length,
                    voices,
                    sources,
                }
            }
            None => RawMetadata::default(),
        };

        let (lyrics_offset, lines) = match xml.lyrics {
            Some(l) => {
                let lyrics_offset = l
                    .offset
                    .as_deref()
                    .map(SignedTimestamp::parse)
                    .transpose()?;

                let mut raw_lines = Vec::with_capacity(l.entries.len());
                for entry in l.entries {
                    let (kind, body) = match entry {
                        XmlLyricsEntryDe::Line(b) => (LineKind::Main, b),
                        XmlLyricsEntryDe::Bg(b) => (LineKind::Background, b),
                    };

                    let offset = match body.offset.as_deref() {
                        Some(s) => parse_timestamp(s)?,
                        None => Duration::ZERO,
                    };
                    let duration = parse_timestamp(&body.duration)?;
                    let segments = parse_inline_synced_text(&body.text)?;

                    raw_lines.push(RawLine {
                        kind,
                        offset,
                        duration,
                        voice: body.voice,
                        segments,
                    });
                }

                (lyrics_offset, raw_lines)
            }
            None => (None, Vec::new()),
        };

        Ok(RawMlyr {
            metadata,
            lyrics_offset,
            lines,
        })
    }
}
