use std::collections::HashMap;
use std::time::Duration;

use crate::ConversionError;
use crate::builders::MlyrBuilder;
use crate::format::SignedTimestamp;
use crate::model::{Line, LineKind, Mlyr, Segment, Source, Voice, VoiceId, VoiceRole};

use super::inline::{
    contains_inline_word_timestamp, extract_inline_bg_blocks, extract_voice_prefix,
    is_wrapped_in_parens, parse_inline_segments, register_voice_role, strip_bg_marker,
    strip_standalone_bg_bracket,
};
use super::timestamp::{parse_lrc_length, parse_lrc_timestamp};

const MIN_LINE_DURATION: Duration = Duration::from_millis(1);
const DEFAULT_TAIL_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct ParsedLrcEntry {
    kind: LineKind,
    offset: Duration,
    explicit_end: Option<Duration>,
    voice: Option<String>,
    segments: Vec<Segment>,
    parenthesized_bg_candidate: bool,
}

#[derive(Default)]
struct ParsedLrcMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    length: Option<Duration>,
    sources: Vec<Source>,
}

pub(super) fn parse_lrc(input: &str) -> Result<Mlyr, ConversionError> {
    let content = input.strip_prefix('\u{FEFF}').unwrap_or(input);

    let mut parsed_entries: Vec<ParsedLrcEntry> = Vec::new();
    let mut metadata = ParsedLrcMetadata::default();
    let mut lyrics_offset: Option<SignedTimestamp> = None;
    let mut voice_roles: HashMap<String, VoiceRole> = HashMap::new();

    for raw_line in content.lines() {
        let raw_line = raw_line.trim();
        if raw_line.is_empty() || raw_line.starts_with('#') || raw_line.starts_with("//") {
            continue;
        }

        if is_metadata_tag_line(raw_line) {
            apply_metadata_tag(raw_line, &mut metadata, &mut lyrics_offset);
            continue;
        }

        if let Some(bg_inner) = strip_standalone_bg_bracket(raw_line) {
            let (segments, first_ts, explicit_end) = parse_inline_segments(bg_inner, None);
            let offset = first_ts.ok_or_else(|| ConversionError::InvalidFormat {
                format_name: "Lrc",
                message: format!("Background tag lacks timestamp: '{raw_line}'"),
            })?;
            parsed_entries.push(ParsedLrcEntry {
                kind: LineKind::Background,
                offset,
                explicit_end,
                voice: None,
                segments,
                parenthesized_bg_candidate: false,
            });
            continue;
        }

        let (timestamps, remainder) = split_leading_timestamps(raw_line);
        if timestamps.is_empty() {
            return Err(ConversionError::InvalidFormat {
                format_name: "Lrc",
                message: format!("Unsynced LRC line is not allowed: '{raw_line}'"),
            });
        }

        if remainder.is_empty() {
            if timestamps.len() == 1 {
                if let Some(prev) = parsed_entries.last_mut() {
                    if prev.explicit_end.is_none() && timestamps[0] > prev.offset {
                        prev.explicit_end = Some(timestamps[0]);
                    }
                }
            }
            continue;
        }

        let (line_timestamps, body_after_line_ts) =
            if timestamps.len() > 1 && contains_inline_word_timestamp(remainder) {
                let first_bracket_end = raw_line.find(']').unwrap() + 1;
                (vec![timestamps[0]], raw_line[first_bracket_end..].trim())
            } else {
                (timestamps, remainder)
            };

        let (main_body, inline_bgs) = if line_timestamps.len() == 1 {
            extract_inline_bg_blocks(body_after_line_ts)
        } else {
            (body_after_line_ts.to_string(), Vec::new())
        };

        let (voice, main_body_no_voice) = extract_voice_prefix(main_body.trim());
        if let Some(ref vid) = voice {
            register_voice_role(&mut voice_roles, vid);
        }

        let main_trimmed = main_body_no_voice.trim();
        if !main_trimmed.is_empty() {
            if let Some(bg_inner) = strip_bg_marker(main_trimmed) {
                for &ts in &line_timestamps {
                    let (segments, first_ts, explicit_end) =
                        parse_inline_segments(bg_inner, Some(ts));
                    parsed_entries.push(ParsedLrcEntry {
                        kind: LineKind::Background,
                        offset: first_ts.unwrap_or(ts),
                        explicit_end,
                        voice: voice.clone(),
                        segments,
                        parenthesized_bg_candidate: false,
                    });
                }
            } else {
                for &ts in &line_timestamps {
                    let (segments, _, explicit_end) = parse_inline_segments(main_trimmed, Some(ts));
                    let full_text: String = segments.iter().map(|s| s.text.as_str()).collect();
                    parsed_entries.push(ParsedLrcEntry {
                        kind: LineKind::Main,
                        offset: ts,
                        explicit_end,
                        voice: voice.clone(),
                        segments,
                        parenthesized_bg_candidate: is_wrapped_in_parens(full_text.trim()),
                    });
                }
            }
        }

        let parent_ts = line_timestamps[0];
        for bg_inner in inline_bgs {
            let (segments, first_ts, explicit_end) =
                parse_inline_segments(&bg_inner, Some(parent_ts));
            parsed_entries.push(ParsedLrcEntry {
                kind: LineKind::Background,
                offset: first_ts.unwrap_or(parent_ts),
                explicit_end,
                voice: voice.clone(),
                segments,
                parenthesized_bg_candidate: false,
            });
        }
    }

    promote_parenthesized_backgrounds(&mut parsed_entries);
    parsed_entries.sort_by_key(|e| (e.offset, e.kind == LineKind::Background));

    let lines = finalize_durations(&parsed_entries, metadata.length)?;

    let mut builder = MlyrBuilder::new();
    if let Some(ti) = metadata.title {
        builder = builder.title(ti);
    }
    if let Some(ar) = metadata.artist {
        builder = builder.artist(ar);
    }
    if let Some(al) = metadata.album {
        builder = builder.album(al);
    }
    builder = builder
        .length_opt(metadata.length)
        .add_sources(metadata.sources);

    let mut voices: Vec<(String, VoiceRole)> = voice_roles.into_iter().collect();
    voices.sort_by(|a, b| a.0.cmp(&b.0));
    for (vid, role) in voices {
        builder = builder.add_voice(Voice::new(VoiceId::new(vid)?, role));
    }

    builder = builder.add_lines(lines);
    if let Some(offset) = lyrics_offset {
        builder = builder.shift(offset);
    }

    Ok(builder.build()?)
}

fn is_metadata_tag_line(line: &str) -> bool {
    if !(line.starts_with('[') && line.ends_with(']')) {
        return false;
    }
    let inner = &line[1..line.len() - 1];
    if inner.contains(']') || parse_lrc_timestamp(inner).is_some() {
        return false;
    }
    let Some((key, _)) = inner.split_once(':') else {
        return false;
    };
    let key = key.trim();
    !key.is_empty()
        && !key.eq_ignore_ascii_case("bg")
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn apply_metadata_tag(
    raw_line: &str,
    metadata: &mut ParsedLrcMetadata,
    lyrics_offset: &mut Option<SignedTimestamp>,
) {
    let tag_content = &raw_line[1..raw_line.len() - 1];
    let Some((raw_key, raw_val)) = tag_content.split_once(':') else {
        return;
    };
    let key = raw_key.trim().to_lowercase();
    let val = raw_val.trim().to_string();
    if val.is_empty() {
        return;
    }
    match key.as_str() {
        "ti" => metadata.title = Some(val),
        "ar" => metadata.artist = Some(val),
        "al" => metadata.album = Some(val),
        "length" => {
            if let Some(dur) = parse_lrc_length(&val) {
                if !dur.is_zero() {
                    metadata.length = Some(dur);
                }
            }
        }
        "offset" => {
            if let Ok(ms) = val.parse::<i64>() {
                *lyrics_offset = Some(SignedTimestamp {
                    negative: ms < 0,
                    magnitude: Duration::from_millis(ms.unsigned_abs()),
                });
            }
        }
        "au" | "lr" | "by" | "re" | "ve" | "tool" => {
            metadata.sources.push(Source::with_name(key, val));
        }
        _ => {}
    }
}

fn split_leading_timestamps(raw_line: &str) -> (Vec<Duration>, &str) {
    let mut idx = 0;
    let mut timestamps = Vec::new();
    while raw_line[idx..].starts_with('[') {
        if let Some(close_idx) = raw_line[idx..].find(']') {
            let ts_str = &raw_line[idx + 1..idx + close_idx];
            if let Some(duration) = parse_lrc_timestamp(ts_str) {
                timestamps.push(duration);
                idx += close_idx + 1;
                continue;
            }
        }
        break;
    }
    (timestamps, raw_line[idx..].trim())
}

fn promote_parenthesized_backgrounds(entries: &mut [ParsedLrcEntry]) {
    let mut prev_bg_end: Option<Duration> = None;
    for entry in entries.iter_mut() {
        if entry.kind == LineKind::Background {
            prev_bg_end = entry
                .explicit_end
                .or(Some(entry.offset + MIN_LINE_DURATION));
            continue;
        }
        if entry.parenthesized_bg_candidate
            && prev_bg_end.is_none_or(|prev_end| entry.offset >= prev_end)
        {
            entry.kind = LineKind::Background;
            prev_bg_end = entry
                .explicit_end
                .or(Some(entry.offset + MIN_LINE_DURATION));
        }
    }
}

fn finalize_durations(
    entries: &[ParsedLrcEntry],
    track_length: Option<Duration>,
) -> Result<Vec<Line>, ConversionError> {
    let mut main_indices = Vec::new();
    let mut bg_indices = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        match entry.kind {
            LineKind::Main => main_indices.push(idx),
            LineKind::Background => bg_indices.push(idx),
        }
    }

    let main_pos: HashMap<usize, usize> = main_indices
        .iter()
        .enumerate()
        .map(|(pos, &idx)| (idx, pos))
        .collect();
    let bg_pos: HashMap<usize, usize> = bg_indices
        .iter()
        .enumerate()
        .map(|(pos, &idx)| (idx, pos))
        .collect();

    let mut lines = Vec::with_capacity(entries.len());
    for (idx, entry) in entries.iter().enumerate() {
        let last_seg_offset = entry
            .segments
            .last()
            .map(|s| s.offset)
            .unwrap_or(Duration::ZERO);

        let next_same_kind_offset = match entry.kind {
            LineKind::Main => main_pos
                .get(&idx)
                .and_then(|&p| main_indices.get(p + 1))
                .map(|&next_i| entries[next_i].offset),
            LineKind::Background => bg_pos
                .get(&idx)
                .and_then(|&p| bg_indices.get(p + 1))
                .map(|&next_i| entries[next_i].offset),
        };

        let duration = if let Some(end) = entry.explicit_end {
            end.saturating_sub(entry.offset)
                .max(MIN_LINE_DURATION)
                .max(last_seg_offset)
        } else if let Some(next_offset) = next_same_kind_offset {
            let delta = next_offset.saturating_sub(entry.offset);
            if delta.is_zero() {
                MIN_LINE_DURATION.max(last_seg_offset)
            } else {
                delta.max(last_seg_offset)
            }
        } else {
            match track_length {
                Some(len) if len > entry.offset => (len - entry.offset)
                    .min(DEFAULT_TAIL_DURATION)
                    .max(MIN_LINE_DURATION)
                    .max(last_seg_offset),
                _ => DEFAULT_TAIL_DURATION.max(last_seg_offset),
            }
        };

        let voice = match entry.voice.as_deref() {
            Some(v) => VoiceId::new(v)?,
            None => VoiceId::default_id(),
        };

        lines.push(Line {
            kind: entry.kind,
            offset: entry.offset,
            duration,
            voice,
            segments: entry.segments.clone(),
        });
    }
    Ok(lines)
}
