use std::time::Duration;

use crate::format::normalize::{RawLine, RawMetadata, RawMlyr, normalize_raw_mlyr};
use crate::model::{LineKind, Mlyr, Segment};
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

use super::common::{
    MIN_LINE_DURATION, append_lyricify_metadata, is_wrapped_in_parens, try_parse_lyricify_metadata,
};

define_lyrics_type!(
    /// Lyricify Lines format (`.lyl`).
    pub struct LyricifyLines;
);

impl LyricsConverter for LyricifyLines {
    fn to_mlyr(lyl: Self) -> Result<Mlyr, ConversionError> {
        let content = lyl.0.strip_prefix('\u{FEFF}').unwrap_or(&lyl.0);
        let mut metadata = RawMetadata::default();
        let mut raw_lines = Vec::new();
        let mut prev_bg_end: Option<Duration> = None;

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.eq_ignore_ascii_case("[type:LyricifyLines]") {
                continue;
            }
            if try_parse_lyricify_metadata(line, &mut metadata) == Some(true) {
                continue;
            }

            if let Some(rest) = line.strip_prefix('[') {
                if let Some((range_str, text)) = rest.split_once(']') {
                    if let Some((start_s, end_s)) = range_str.split_once(',') {
                        if let (Ok(start_ms), Ok(end_ms)) =
                            (start_s.trim().parse::<u64>(), end_s.trim().parse::<u64>())
                        {
                            let offset = Duration::from_millis(start_ms);
                            let end = Duration::from_millis(end_ms);
                            let duration = end.saturating_sub(offset).max(MIN_LINE_DURATION);

                            let is_paren_bg = is_wrapped_in_parens(text.trim())
                                && prev_bg_end.is_none_or(|pe| offset >= pe);

                            let kind = if is_paren_bg {
                                prev_bg_end = Some(offset + duration);
                                LineKind::Background
                            } else {
                                LineKind::Main
                            };

                            raw_lines.push(RawLine {
                                kind,
                                offset,
                                duration,
                                voice: None,
                                segments: vec![Segment {
                                    offset: Duration::ZERO,
                                    text: text.to_string(),
                                }],
                            });
                            continue;
                        }
                    }
                }
            }

            return Err(ConversionError::InvalidFormat {
                format_name: "LyricifyLines",
                message: format!("Invalid Lyricify Lines entry: '{line}'"),
            });
        }

        let raw_mlyr = RawMlyr {
            metadata,
            lyrics_offset: None,
            lines: raw_lines,
        };
        Ok(normalize_raw_mlyr(raw_mlyr)?)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        mlyr.validate()?;
        let mut out = String::from("[type:LyricifyLines]\n");
        append_lyricify_metadata(&mut out, &mlyr);

        for line in &mlyr.lyrics.lines {
            let start_ms = line.offset.as_millis();
            let end_ms = line.end().as_millis();
            let mut text: String = line.segments.iter().map(|s| s.text.as_str()).collect();
            if line.is_bg() && !is_wrapped_in_parens(text.trim()) {
                text = format!("({text})");
            }
            out.push_str(&format!("[{start_ms},{end_ms}]{text}\n"));
        }

        Ok(LyricifyLines(out))
    }
}
