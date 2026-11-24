use serde::Serialize;

use crate::format::inline_text::format_inline_synced_text;
use crate::format::timestamp::{format_position, format_short_duration};
use crate::model::{Line, LineKind, Lyrics, Metadata, Mlyr, Source, Voice, VoiceRole};

#[derive(Serialize)]
#[serde(rename = "mlyr")]
pub(crate) struct XmlMlyrSer<'a> {
    #[serde(skip_serializing_if = "XmlMetadataSer::is_empty")]
    metadata: XmlMetadataSer<'a>,
    #[serde(skip_serializing_if = "XmlLyricsSer::is_empty")]
    lyrics: XmlLyricsSer<'a>,
}

#[derive(Serialize)]
struct XmlMetadataSer<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artist: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    album: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<String>,
    #[serde(skip_serializing_if = "XmlVoicesSer::is_empty")]
    voices: XmlVoicesSer<'a>,
    #[serde(rename = "source", skip_serializing_if = "Vec::is_empty")]
    sources: Vec<XmlSourceSer<'a>>,
}

impl XmlMetadataSer<'_> {
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.length.is_none()
            && self.voices.is_empty()
            && self.sources.is_empty()
    }
}

#[derive(Serialize)]
struct XmlSourceSer<'a> {
    #[serde(rename = "@type")]
    kind: &'a str,
    #[serde(rename = "@name", skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(rename = "$text", skip_serializing_if = "Option::is_none")]
    content: Option<&'a str>,
}

#[derive(Serialize)]
struct XmlVoicesSer<'a> {
    #[serde(rename = "$value")]
    declarations: Vec<XmlVoiceDeclSer<'a>>,
}

impl XmlVoicesSer<'_> {
    fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
}

#[derive(Serialize)]
enum XmlVoiceDeclSer<'a> {
    #[serde(rename = "primary")]
    Primary(XmlVoiceBodySer<'a>),
    #[serde(rename = "secondary")]
    Secondary(XmlVoiceBodySer<'a>),
    #[serde(rename = "tertiary")]
    Tertiary(XmlVoiceBodySer<'a>),
    #[serde(rename = "group")]
    Group(XmlVoiceBodySer<'a>),
}

#[derive(Serialize)]
struct XmlVoiceBodySer<'a> {
    #[serde(rename = "@id")]
    id: &'a str,
    #[serde(rename = "$text", skip_serializing_if = "Option::is_none")]
    singer: Option<&'a str>,
}

#[derive(Serialize)]
struct XmlLyricsSer<'a> {
    #[serde(rename = "$value")]
    entries: Vec<XmlLyricsEntrySer<'a>>,
}

impl XmlLyricsSer<'_> {
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Serialize)]
enum XmlLyricsEntrySer<'a> {
    #[serde(rename = "line")]
    Line(XmlLineBodySer<'a>),
    #[serde(rename = "bg")]
    Bg(XmlLineBodySer<'a>),
}

#[derive(Serialize)]
struct XmlLineBodySer<'a> {
    #[serde(rename = "@offset", skip_serializing_if = "Option::is_none")]
    offset: Option<String>,
    #[serde(rename = "@duration")]
    duration: String,
    #[serde(rename = "@voice", skip_serializing_if = "Option::is_none")]
    voice: Option<&'a str>,
    #[serde(rename = "$text")]
    text: String,
}

impl<'a> From<&'a Mlyr> for XmlMlyrSer<'a> {
    fn from(mlyr: &'a Mlyr) -> Self {
        let omit_default_voice = mlyr.metadata.voices.is_empty();
        Self {
            metadata: XmlMetadataSer::from(&mlyr.metadata),
            lyrics: XmlLyricsSer::from_lyrics(&mlyr.lyrics, omit_default_voice),
        }
    }
}

impl<'a> From<&'a Metadata> for XmlMetadataSer<'a> {
    fn from(m: &'a Metadata) -> Self {
        Self {
            title: m.title.as_deref(),
            artist: m.artist.as_deref(),
            album: m.album.as_deref(),
            length: m.length.map(format_position),
            voices: XmlVoicesSer::from(&m.voices),
            sources: m.sources.iter().map(XmlSourceSer::from).collect(),
        }
    }
}

impl<'a> From<&'a Source> for XmlSourceSer<'a> {
    fn from(s: &'a Source) -> Self {
        Self {
            kind: &s.kind,
            name: s.name.as_deref(),
            content: s.content.as_deref(),
        }
    }
}

impl<'a> From<&'a Vec<Voice>> for XmlVoicesSer<'a> {
    fn from(voices: &'a Vec<Voice>) -> Self {
        let mut declarations = Vec::new();
        for voice in voices {
            let wrap = |body: XmlVoiceBodySer<'a>| match voice.role {
                VoiceRole::Primary => XmlVoiceDeclSer::Primary(body),
                VoiceRole::Secondary => XmlVoiceDeclSer::Secondary(body),
                VoiceRole::Tertiary => XmlVoiceDeclSer::Tertiary(body),
                VoiceRole::Group => XmlVoiceDeclSer::Group(body),
            };

            if voice.singers.is_empty() {
                declarations.push(wrap(XmlVoiceBodySer {
                    id: voice.id.as_str(),
                    singer: None,
                }));
            } else {
                for singer in &voice.singers {
                    declarations.push(wrap(XmlVoiceBodySer {
                        id: voice.id.as_str(),
                        singer: Some(singer.as_str()),
                    }));
                }
            }
        }
        Self { declarations }
    }
}

impl<'a> XmlLyricsSer<'a> {
    fn from_lyrics(lyrics: &'a Lyrics, omit_default_voice: bool) -> Self {
        let entries = lyrics
            .lines
            .iter()
            .map(|l| XmlLyricsEntrySer::from_line(l, omit_default_voice))
            .collect();
        Self { entries }
    }
}

impl<'a> XmlLyricsEntrySer<'a> {
    fn from_line(line: &'a Line, omit_default_voice: bool) -> Self {
        let voice = if omit_default_voice && line.voice.is_default() {
            None
        } else {
            Some(line.voice.as_str())
        };

        let offset = if line.offset.is_zero() {
            None
        } else {
            Some(format_position(line.offset))
        };

        let body = XmlLineBodySer {
            offset,
            duration: format_short_duration(line.duration),
            voice,
            text: format_inline_synced_text(&line.segments),
        };

        match line.kind {
            LineKind::Main => XmlLyricsEntrySer::Line(body),
            LineKind::Background => XmlLyricsEntrySer::Bg(body),
        }
    }
}
