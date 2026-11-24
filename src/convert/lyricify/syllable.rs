use std::collections::HashMap;
use std::time::Duration;

use crate::format::normalize::{RawLine, RawMetadata, RawMlyr, RawVoiceDecl, normalize_raw_mlyr};
use crate::model::{LineKind, Mlyr, Segment, VoiceRole};
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

use super::common::{MIN_LINE_DURATION, append_lyricify_metadata, try_parse_lyricify_metadata};

define_lyrics_type!(
    /// Lyricify Syllable format (`.lys`).
    pub struct LyricifySyllable;
);

impl LyricsConverter for LyricifySyllable {
    fn to_mlyr(lys: Self) -> Result<Mlyr, ConversionError> {
        let content = lys.0.strip_prefix('\u{FEFF}').unwrap_or(&lys.0);
        let mut metadata = RawMetadata::default();
        let mut raw_lines = Vec::new();
        let mut used_duet = false;

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.eq_ignore_ascii_case("[type:LyricifySyllable]") {
                continue;
            }
            if try_parse_lyricify_metadata(line, &mut metadata) == Some(true) {
                continue;
            }

            let rest = line
                .strip_prefix('[')
                .ok_or_else(|| ConversionError::InvalidFormat {
                    format_name: "LyricifySyllable",
                    message: format!("Expected property tag '[0..8]' on line: '{line}'"),
                })?;
            let (prop_str, body) =
                rest.split_once(']')
                    .ok_or_else(|| ConversionError::InvalidFormat {
                        format_name: "LyricifySyllable",
                        message: format!("Unclosed property tag '[' on line: '{line}'"),
                    })?;
            let prop: u8 = prop_str
                .trim()
                .parse()
                .map_err(|_| ConversionError::InvalidFormat {
                    format_name: "LyricifySyllable",
                    message: format!("Invalid Lyricify Syllable property '{prop_str}'"),
                })?;

            if prop > 8 {
                return Err(ConversionError::InvalidFormat {
                    format_name: "LyricifySyllable",
                    message: format!("Lyricify Syllable property out of range (0..=8): {prop}"),
                });
            }

            let (kind, voice) = match prop {
                6..=8 => {
                    used_duet = true;
                    (LineKind::Background, Some("v2".to_string()))
                }
                3..=5 => {
                    used_duet = true;
                    (LineKind::Main, Some("v1".to_string()))
                }
                _ => (LineKind::Main, None),
            };

            let parsed_syllables = parse_lys_syllables(body);
            if parsed_syllables.is_empty() {
                continue;
            }

            let line_start = parsed_syllables[0].0;
            let mut line_end = line_start + MIN_LINE_DURATION;
            let mut segments = Vec::with_capacity(parsed_syllables.len());

            for (syl_start, syl_dur, text) in parsed_syllables {
                let rel_offset = syl_start.saturating_sub(line_start);
                line_end = line_end.max(syl_start + syl_dur);
                segments.push(Segment {
                    offset: rel_offset,
                    text,
                });
            }

            let duration = line_end.saturating_sub(line_start).max(MIN_LINE_DURATION);

            raw_lines.push(RawLine {
                kind,
                offset: line_start,
                duration,
                voice,
                segments,
            });
        }

        if used_duet {
            metadata.voices = vec![
                RawVoiceDecl {
                    id: "v1".to_string(),
                    role: VoiceRole::Primary,
                    singer: None,
                },
                RawVoiceDecl {
                    id: "v2".to_string(),
                    role: VoiceRole::Secondary,
                    singer: None,
                },
            ];
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
        let mut out = String::from("[type:LyricifySyllable]\n");
        append_lyricify_metadata(&mut out, &mlyr);

        let voice_roles: HashMap<&str, VoiceRole> = mlyr
            .metadata
            .voices
            .iter()
            .map(|v| (v.id.as_str(), v.role))
            .collect();

        for line in &mlyr.lyrics.lines {
            let prop: u8 = if line.is_bg() {
                7
            } else {
                match voice_roles.get(line.voice.as_str()) {
                    Some(VoiceRole::Secondary) => 5,
                    Some(VoiceRole::Primary) if !voice_roles.is_empty() => 4,
                    _ => 0,
                }
            };

            out.push_str(&format!("[{prop}]"));
            let line_start_ms = line.offset.as_millis();
            let line_end_ms = line.end().as_millis();

            for (i, seg) in line.segments.iter().enumerate() {
                let seg_start_ms = line_start_ms + seg.offset.as_millis();
                let seg_end_ms = line
                    .segments
                    .get(i + 1)
                    .map(|next| line_start_ms + next.offset.as_millis())
                    .unwrap_or(line_end_ms);
                let dur_ms = seg_end_ms.saturating_sub(seg_start_ms);
                out.push_str(&format!("{}({seg_start_ms},{dur_ms})", seg.text));
            }
            out.push('\n');
        }

        Ok(LyricifySyllable(out))
    }
}

/// Parses `syllable(start_ms,dur_ms)` tokens, handling literal parentheses inside lyric text.
fn parse_lys_syllables(body: &str) -> Vec<(Duration, Duration, String)> {
    let mut out = Vec::new();
    let mut text_start = 0;
    let bytes = body.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'(' {
            if let Some(close_rel) = body[i + 1..].find(')') {
                let inside = &body[i + 1..i + 1 + close_rel];
                if let Some((start_s, dur_s)) = inside.split_once(',') {
                    if let (Ok(start_ms), Ok(dur_ms)) =
                        (start_s.trim().parse::<u64>(), dur_s.trim().parse::<u64>())
                    {
                        let syl_text = body[text_start..i].to_string();
                        out.push((
                            Duration::from_millis(start_ms),
                            Duration::from_millis(dur_ms),
                            syl_text,
                        ));
                        i = i + 1 + close_rel + 1;
                        text_start = i;
                        continue;
                    }
                }
            }
        }
        i += body[i..].chars().next().map_or(1, |c| c.len_utf8());
    }

    out
}
