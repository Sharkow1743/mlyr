use std::time::Duration;

use crate::format::normalize::{RawLine, RawLyrics, RawMlyr, RawSegment};
use crate::model::{Metadata, Mlyr, Sources};
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

define_lyrics_type!(
    pub struct Lrc;
);

impl LyricsConverter for Lrc {
    fn to_mlyr(lrc: Self) -> Result<Mlyr, ConversionError> {
        let mut raw_lines = Vec::new();
        let mut metadata = Metadata {
            title: None,
            artist: None,
            album: None,
            length: None,
            sources: Sources::default(),
        };

        for raw_line in lrc.0.lines() {
            let raw_line = raw_line.trim();
            if raw_line.is_empty() {
                continue;
            }

            if raw_line.starts_with('[') && raw_line.contains(':') {
                if let Some(close_bracket) = raw_line.find(']') {
                    let tag_content = &raw_line[1..close_bracket];
                    if let Some(colon_idx) = tag_content.find(':') {
                        if parse_lrc_timestamp(tag_content).is_none() {
                            let key = tag_content[..colon_idx].trim().to_lowercase();
                            let val = tag_content[colon_idx + 1..].trim().to_string();
                            match key.as_str() {
                                "ti" => metadata.title = Some(val),
                                "ar" => metadata.artist = Some(val),
                                "al" => metadata.album = Some(val),
                                "length" => {
                                    if let Some(dur) = parse_lrc_timestamp(&val) {
                                        metadata.length = Some(dur);
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }
                    }
                }
            }

            let mut text_start_idx = 0;
            let mut timestamps = Vec::new();

            while raw_line[text_start_idx..].starts_with('[') {
                if let Some(close_idx) = raw_line[text_start_idx..].find(']') {
                    let ts_str = &raw_line[text_start_idx + 1..text_start_idx + close_idx];
                    if let Some(duration) = parse_lrc_timestamp(ts_str) {
                        timestamps.push(duration);
                    }
                    text_start_idx += close_idx + 1;
                } else {
                    break;
                }
            }

            let lyrics_text = raw_line[text_start_idx..].trim();
            if timestamps.is_empty() {
                if !lyrics_text.is_empty() {
                    raw_lines.push(RawLine {
                        offset: None,
                        voice: None,
                        segments: vec![RawSegment {
                            offset: None,
                            text: lyrics_text.to_string(),
                        }],
                        parallel: None,
                    });
                }
                continue;
            }

            for &ts in &timestamps {
                let mut segments = Vec::new();
                let mut current_segment_text = String::new();
                let mut last_segment_offset: Option<Duration> = None;

                let mut cursor = 0;
                let lyrics_chars: Vec<char> = lyrics_text.chars().collect();

                while cursor < lyrics_chars.len() {
                    if lyrics_chars[cursor] == '<' {
                        let sub_str: String = lyrics_chars[cursor..].iter().collect();
                        if let Some(close_tag_idx) = sub_str.find('>') {
                            let inner_ts_str = &sub_str[1..close_tag_idx];
                            if let Some(absolute_seg_ts) = parse_lrc_timestamp(inner_ts_str) {
                                if !current_segment_text.is_empty() || last_segment_offset.is_some()
                                {
                                    segments.push(RawSegment {
                                        offset: last_segment_offset,
                                        text: current_segment_text.clone(),
                                    });
                                    current_segment_text.clear();
                                }

                                let relative_offset = if absolute_seg_ts >= ts {
                                    absolute_seg_ts - ts
                                } else {
                                    Duration::ZERO
                                };
                                last_segment_offset = Some(relative_offset);
                                cursor += close_tag_idx + 1;
                                continue;
                            }
                        }
                    }
                    current_segment_text.push(lyrics_chars[cursor]);
                    cursor += 1;
                }

                if !current_segment_text.is_empty() || last_segment_offset.is_some() {
                    segments.push(RawSegment {
                        offset: last_segment_offset,
                        text: current_segment_text,
                    });
                }

                if !segments.is_empty() && segments.iter().all(|s| s.offset.is_none()) {
                    segments[0].offset = Some(Duration::ZERO);
                }

                raw_lines.push(RawLine {
                    offset: Some(ts),
                    voice: None,
                    segments,
                    parallel: None,
                });
            }
        }

        let raw_mlyr = RawMlyr {
            metadata: if metadata.title.is_some()
                || metadata.artist.is_some()
                || metadata.album.is_some()
                || metadata.length.is_some()
            {
                Some(metadata)
            } else {
                None
            },
            lyrics: RawLyrics { lines: raw_lines },
        };

        let mlyr = Mlyr::try_from(raw_mlyr)?;
        Ok(mlyr)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        mlyr.validate()?;
        let mut output = String::new();

        if !mlyr.metadata.is_empty() {
            if let Some(ref ti) = mlyr.metadata.title {
                output.push_str(&format!("[ti:{ti}]\n"));
            }
            if let Some(ref ar) = mlyr.metadata.artist {
                output.push_str(&format!("[ar:{ar}]\n"));
            }
            if let Some(ref al) = mlyr.metadata.album {
                output.push_str(&format!("[al:{al}]\n"));
            }
            if let Some(len) = mlyr.metadata.length {
                output.push_str(&format!("[length:{}]\n", format_lrc_timestamp(len)));
            }
        }

        for line in &mlyr.lyrics.lines {
            if let Some(line_offset) = line.offset {
                let ts_str = format_lrc_timestamp(line_offset);
                output.push_str(&format!("[{ts_str}]"));

                let has_word_sync = line.segments.iter().any(|s| s.offset != Duration::ZERO);

                if has_word_sync {
                    for seg in &line.segments {
                        let absolute_seg_ts = line_offset + seg.offset;
                        output.push_str(&format!(
                            "<{}>{}",
                            format_lrc_timestamp(absolute_seg_ts),
                            seg.text
                        ));
                    }
                } else {
                    for seg in &line.segments {
                        output.push_str(&seg.text);
                    }
                }
                output.push('\n');
            } else {
                for seg in &line.segments {
                    output.push_str(&seg.text);
                }
                output.push('\n');
            }
        }

        Ok(Lrc(output))
    }
}

fn parse_lrc_timestamp(s: &str) -> Option<Duration> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let mins: u64 = parts[0].trim().parse().ok()?;

    let sec_parts: Vec<&str> = if parts[1].contains(',') {
        parts[1].split(',').collect()
    } else {
        parts[1].split('.').collect()
    };
    if sec_parts.is_empty() {
        return None;
    }
    let secs: u64 = sec_parts[0].trim().parse().ok()?;

    let mut millis: u64 = 0;
    if sec_parts.len() > 1 {
        let frac_str = sec_parts[1].trim();
        let len = frac_str.len();
        if len > 0 {
            let parsed_frac: u64 = frac_str.parse().ok()?;
            if len == 1 {
                millis = parsed_frac * 100;
            } else if len == 2 {
                millis = parsed_frac * 10;
            } else if len == 3 {
                millis = parsed_frac;
            } else {
                millis = parsed_frac / 10u64.pow((len - 3) as u32);
            }
        }
    }

    Some(Duration::from_millis(
        mins * 60 * 1000 + secs * 1000 + millis,
    ))
}

fn format_lrc_timestamp(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let centis = (duration.subsec_millis() / 10) % 100;
    format!("{mins:02}:{secs:02}.{centis:02}")
}
