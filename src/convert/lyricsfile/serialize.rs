use crate::ConversionError;
use crate::model::Mlyr;

use super::schema::{LyricsfileDoc, LyricsfileLine, LyricsfileMetadata, LyricsfileWord};

pub(super) fn serialize_lyricsfile(mlyr: &Mlyr) -> Result<String, ConversionError> {
    mlyr.validate()?;

    let metadata = LyricsfileMetadata {
        title: mlyr.metadata.title.clone(),
        artist: mlyr.metadata.artist.clone(),
        album: mlyr.metadata.album.clone(),
        duration_ms: mlyr.metadata.length.map(|d| d.as_millis() as i64),
        offset_ms: None,
        language: None,
        instrumental: mlyr.lyrics.lines.is_empty(),
    };

    let mut lines = Vec::with_capacity(mlyr.lyrics.lines.len());
    for line in &mlyr.lyrics.lines {
        let start_ms = line.offset.as_millis() as i64;
        let end_ms = line.end().as_millis() as i64;
        let text: String = line.segments.iter().map(|s| s.text.as_str()).collect();

        let is_word_synced =
            line.segments.len() > 1 || line.segments.iter().any(|s| !s.offset.is_zero());

        let words = if is_word_synced {
            line.segments
                .iter()
                .enumerate()
                .map(|(i, seg)| {
                    let w_start = start_ms + seg.offset.as_millis() as i64;
                    let w_end = line
                        .segments
                        .get(i + 1)
                        .map(|next| start_ms + next.offset.as_millis() as i64)
                        .unwrap_or(end_ms);
                    LyricsfileWord {
                        text: seg.text.clone(),
                        start_ms: w_start,
                        end_ms: Some(w_end),
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        lines.push(LyricsfileLine {
            text,
            start_ms,
            end_ms: Some(end_ms),
            words,
        });
    }

    let doc = LyricsfileDoc {
        version: "1.0".to_string(),
        metadata,
        lines,
        plain: None,
    };

    serde_yaml_ng::to_string(&doc).map_err(|e| ConversionError::Parsing {
        format_name: "Lyricsfile",
        source: Box::new(e),
    })
}
