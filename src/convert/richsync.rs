use std::time::Duration;

use crate::format::normalize::{RawLine, RawMetadata, RawMlyr, normalize_raw_mlyr};
use crate::model::{LineKind, Mlyr, Segment};
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

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
    /// Musixmatch's internal JSON-based format.
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
            let line_end = secs_to_duration(mxm_line.te, "te")?;

            if line_end <= line_offset {
                return Err(ConversionError::InvalidFormat {
                    format_name: "Richsync",
                    message: format!(
                        "Line end 'te' ({}) must be strictly greater than start 'ts' ({})",
                        mxm_line.te, mxm_line.ts
                    ),
                });
            }
            let duration = line_end - line_offset;

            let segments = if mxm_line.l.is_empty() && !mxm_line.x.is_empty() {
                vec![Segment {
                    offset: Duration::ZERO,
                    text: mxm_line.x,
                }]
            } else {
                let mut segs = Vec::with_capacity(mxm_line.l.len());
                for word in mxm_line.l {
                    let word_offset = secs_to_duration(word.o, "o")?;
                    segs.push(Segment {
                        offset: word_offset,
                        text: word.c,
                    });
                }
                segs
            };

            raw_lines.push(RawLine {
                kind: LineKind::Main,
                offset: line_offset,
                duration,
                voice: None,
                segments,
            });
        }

        let raw_mlyr = RawMlyr {
            metadata: RawMetadata::default(),
            lyrics_offset: None,
            lines: raw_lines,
        };

        Ok(normalize_raw_mlyr(raw_mlyr)?)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        mlyr.validate()?;

        let mut mxm_lines = Vec::with_capacity(mlyr.lyrics.lines.len());

        for line in &mlyr.lyrics.lines {
            let ts = line.offset.as_secs_f64();
            let te = line.end().as_secs_f64();

            let mut word_list = Vec::with_capacity(line.segments.len());
            let mut full_text = String::new();

            for seg in &line.segments {
                word_list.push(RichSyncWord {
                    c: seg.text.clone(),
                    o: seg.offset.as_secs_f64(),
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
    Ok(Duration::from_millis((secs * 1_000.0).round() as u64))
}
