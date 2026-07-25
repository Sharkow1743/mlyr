use crate::define_lyrics_type;

use super::super::definition::*;
use super::ConversionError;
use std::time::Duration;

use anyhow::Error;

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
    pub struct Richsync(String);

    /// Musixmatch Richsync JSON converter
    impl LyricsConverter {
        fn to_mlyr(richsync: Self) -> Result<Mlyr, ConversionError> {
            let mxm_lines: Vec<RichSyncLine> =
                serde_json::from_str(&richsync.0).map_err(|e| ConversionError::Parsing {
                    format_name: "Richsync",
                    source: Error::from(e),
                })?;
            let mut raw_lines = Vec::with_capacity(mxm_lines.len());

            for mxm_line in mxm_lines {
                let offset = Some(Duration::from_secs_f64(mxm_line.ts));
                let segments: Vec<RawSegment> = mxm_line
                    .l
                    .into_iter()
                    .map(|word| RawSegment {
                        offset: Some(Duration::from_secs_f64(word.o)),
                        text: word.c,
                    })
                    .collect();

                raw_lines.push(RawLine {
                    offset,
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

            Ok(Richsync(serde_json::to_string(&mxm_lines).map_err(
                |e| ConversionError::Parsing {
                    format_name: "Richsync",
                    source: Error::from(e),
                },
            )?))
        }
    }
);
