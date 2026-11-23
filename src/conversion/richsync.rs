use crate::{LyricsConverter, define_lyrics_type};

use super::super::definition::*;
use super::ConversionError;
use std::time::Duration;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RichSyncLine {
    pub ts: f64,
    pub te: f64,
    pub l: Vec<RichSyncWord>,
    pub x: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RichSyncWord {
    pub c: String,
    pub o: f64,
}

define_lyrics_type!(
	/// Musixmatch's internal JSON-based format
    pub struct Richsync;
);

impl LyricsConverter for Richsync {
    fn to_mlyr(richsync: Self) -> Result<Mlyr, ConversionError> {
        let mxm_lines: Vec<RichSyncLine> =
            serde_json::from_str(&richsync.0).map_err(|e| ConversionError::Parsing {
                format_name: "Richsync",
                source: Box::new(e),
            })?;
        let mut raw_lines = Vec::with_capacity(mxm_lines.len());

        for mxm_line in mxm_lines {
            let line_offset = secs_to_duration(mxm_line.ts, "ts")?;
            let mut segments = Vec::with_capacity(mxm_line.l.len());

            for word in mxm_line.l {
                let word_offset = secs_to_duration(word.o, "o")?;
                segments.push(RawSegment {
                    offset: Some(word_offset),
                    text: word.c,
                });
            }

            raw_lines.push(RawLine {
                offset: Some(line_offset),
                voice: None,
                segments,
                parallel: None,
            });
        }

        let raw_mlyr = RawMlyr {
            metadata: None,
            lyrics: RawLyrics { lines: raw_lines },
        };

        let mlyr = Mlyr::try_from(raw_mlyr)?;
        Ok(mlyr)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        mlyr.validate()?;

        let mut mxm_lines = Vec::new();

        for line in &mlyr.lyrics.lines {
            let ts = line.offset.unwrap_or(Duration::ZERO).as_secs_f64();
            let last_offset_sec = line
                .segments
                .last()
                .map(|s| s.offset)
                .unwrap_or(Duration::ZERO)
                .as_secs_f64();
            let te = ts + last_offset_sec;

            let mut word_list = Vec::new();
            let mut full_text = String::new();

            for seg in &line.segments {
                let offset_sec = seg.offset.as_secs_f64();
                word_list.push(RichSyncWord {
                    c: seg.text.clone(),
                    o: offset_sec,
                });
                full_text.push_str(&seg.text);
            }

            mxm_lines.push(RichSyncLine {
                ts,
                te,
                l: word_list,
                x: full_text,
            });
        }

        let json = serde_json::to_string(&mxm_lines).map_err(|e| ConversionError::Parsing {
            format_name: "Richsync",
            source: Box::new(e),
        })?;

        Ok(Richsync(json))
    }
}

fn secs_to_duration(secs: f64, field: &str) -> Result<Duration, ConversionError> {
    if !secs.is_finite() || secs < 0.0 {
        return Err(ConversionError::InvalidFormat {
            format_name: "Richsync",
            message: format!("Invalid non-finite or negative '{field}' value: {secs}"),
        });
    }
    Ok(Duration::from_secs_f64(secs))
}
