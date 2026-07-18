use super::super::definition::*;
use super::ConversionError;
use std::time::Duration;

impl Mlyr {
    /// Creates `Mlyr` object from LRC string
    pub fn from_lrc(lrc_text: &str) -> Result<Self, ConversionError> {
        let mut lines = Vec::new();
        let mut metadata = Metadata {
            song_name: None,
            artist: None,
            album: None,
            length: None,
            sources: None,
        };

        for raw_line in lrc_text.lines() {
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
                                "ti" => metadata.song_name = Some(val),
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
                    lines.push(Line {
                        offset: None,
                        voice: None,
                        segments: vec![Segment {
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
                                    segments.push(Segment {
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
                    segments.push(Segment {
                        offset: last_segment_offset,
                        text: current_segment_text,
                    });
                }

                if !segments.is_empty() && segments.iter().all(|s| s.offset.is_none()) {
                    segments[0].offset = Some(Duration::ZERO);
                }

                lines.push(Line {
                    offset: Some(ts),
                    voice: None,
                    segments,
                    parallel: None,
                });
            }
        }

        let mlyr = Mlyr {
            metadata: if metadata.song_name.is_some()
                || metadata.artist.is_some()
                || metadata.album.is_some()
                || metadata.length.is_some()
            {
                Some(metadata)
            } else {
                None
            },
            lyrics: Lyrics { lines },
        };

        mlyr.validate()?;
        Ok(mlyr)
    }

    /// Creates LRC string from `Mlyr` object
    pub fn to_lrc(&self) -> Result<String, ConversionError> {
        self.validate()?;

        let mut output = String::new();

        if let Some(ref meta) = self.metadata {
            if let Some(ref ti) = meta.song_name {
                output.push_str(&format!("[ti:{}]\n", ti));
            }
            if let Some(ref ar) = meta.artist {
                output.push_str(&format!("[ar:{}]\n", ar));
            }
            if let Some(ref al) = meta.album {
                output.push_str(&format!("[al:{}]\n", al));
            }
            if let Some(len) = meta.length {
                output.push_str(&format!("[length:{}]\n", format_lrc_timestamp(len)));
            }
        }

        for line in &self.lyrics.lines {
            if let Some(line_offset) = line.offset {
                let ts_str = format_lrc_timestamp(line_offset);
                output.push_str(&format!("[{}]", ts_str));

                let has_word_sync = line
                    .segments
                    .iter()
                    .any(|s| s.offset.is_some() && s.offset != Some(Duration::ZERO));

                if has_word_sync {
                    for seg in &line.segments {
                        if let Some(seg_offset) = seg.offset {
                            let absolute_seg_ts = line_offset + seg_offset;
                            output.push_str(&format!(
                                "<{}>{}",
                                format_lrc_timestamp(absolute_seg_ts),
                                seg.text
                            ));
                        } else {
                            output.push_str(&seg.text);
                        }
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

        Ok(output)
    }
}

fn parse_lrc_timestamp(s: &str) -> Option<Duration> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let mins: u64 = parts[0].trim().parse().ok()?;

    let sec_parts: Vec<&str> = parts[1].split('.').collect();
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
    format!("{:02}:{:02}.{:02}", mins, secs, centis)
}
