use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct LyricsfileDoc {
    pub(super) version: String,
    #[serde(default)]
    pub(super) metadata: LyricsfileMetadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) lines: Vec<LyricsfileLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) plain: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct LyricsfileMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) album: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) duration_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) offset_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) language: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) instrumental: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct LyricsfileLine {
    pub(super) text: String,
    pub(super) start_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) end_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) words: Vec<LyricsfileWord>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct LyricsfileWord {
    pub(super) text: String,
    pub(super) start_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) end_ms: Option<i64>,
}
