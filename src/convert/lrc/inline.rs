use std::collections::HashMap;
use std::time::Duration;

use crate::model::{Segment, VoiceRole};

use super::timestamp::parse_lrc_timestamp;

pub(super) fn strip_standalone_bg_bracket(line: &str) -> Option<&str> {
    if line.starts_with('[') && line.ends_with(']') {
        let inner = &line[1..line.len() - 1];
        if let Some(rest) = inner
            .strip_prefix("bg:")
            .or_else(|| inner.strip_prefix("BG:"))
        {
            return Some(rest.trim());
        }
    }
    None
}

pub(super) fn strip_bg_marker(s: &str) -> Option<&str> {
    if let Some(inner) = strip_standalone_bg_bracket(s) {
        return Some(inner);
    }
    if let Some(rest) = s.strip_prefix("bg:").or_else(|| s.strip_prefix("BG:")) {
        return Some(rest.trim());
    }
    None
}

pub(super) fn extract_inline_bg_blocks(body: &str) -> (String, Vec<String>) {
    let mut main_out = String::with_capacity(body.len());
    let mut bg_blocks = Vec::new();
    let mut i = 0;
    let bytes = body.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'[' {
            let rest = &body[i + 1..];
            if rest.starts_with("bg:") || rest.starts_with("BG:") {
                if let Some(close_rel) = rest.find(']') {
                    let inner = rest[3..close_rel].trim().to_string();
                    bg_blocks.push(inner);
                    i += 1 + close_rel + 1;
                    continue;
                }
            }
        }
        let ch = body[i..].chars().next().unwrap();
        main_out.push(ch);
        i += ch.len_utf8();
    }

    (main_out, bg_blocks)
}

pub(super) fn extract_voice_prefix(body: &str) -> (Option<String>, &str) {
    // Walaoke duet prefixes: `M:`, `F:`, `D:`
    for prefix in ["M:", "F:", "D:"] {
        if let Some(rest) = body.strip_prefix(prefix) {
            if rest.is_empty()
                || rest.starts_with(' ')
                || rest.starts_with('<')
                || rest.starts_with('[')
            {
                let voice_id = match prefix {
                    "M:" => "v1",
                    "F:" => "v2",
                    _ => "v1000",
                };
                return (Some(voice_id.to_string()), rest.trim_start_matches(' '));
            }
        }
    }

    // eLRC voice prefixes: `v1:`, `v2:`, `v1000:`, etc.
    if let Some(rest_after_v) = body.strip_prefix('v').or_else(|| body.strip_prefix('V')) {
        if let Some(colon_idx) = rest_after_v.find(':') {
            let digits = &rest_after_v[..colon_idx];
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                let after_colon = &rest_after_v[colon_idx + 1..];
                let trimmed = after_colon.strip_prefix(' ').unwrap_or(after_colon);
                return (Some(format!("v{digits}")), trimmed);
            }
        }
    }

    (None, body)
}

pub(super) fn register_voice_role(map: &mut HashMap<String, VoiceRole>, id: &str) {
    map.entry(id.to_string()).or_insert_with(|| match id {
        "v1000" => VoiceRole::Group,
        "v2" => VoiceRole::Secondary,
        "v3" => VoiceRole::Tertiary,
        _ => VoiceRole::Primary,
    });
}

pub(super) fn contains_inline_word_timestamp(body: &str) -> bool {
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(rel) = body[i + 1..].find('>') {
                if parse_lrc_timestamp(&body[i + 1..i + 1 + rel]).is_some() {
                    return true;
                }
            }
        } else if bytes[i] == b'[' {
            if let Some(rel) = body[i + 1..].find(']') {
                if parse_lrc_timestamp(&body[i + 1..i + 1 + rel]).is_some() {
                    return true;
                }
            }
        }
        i += body[i..].chars().next().map_or(1, |c| c.len_utf8());
    }
    false
}

/// Parses inline `<ts>` (LRC A2 / eLRC) and `[ts]` (ESLyric / SPL) word timestamps.
/// Returns `(segments, first_timestamp_seen, trailing_explicit_end_timestamp)`.
pub(super) fn parse_inline_segments(
    body: &str,
    line_offset: Option<Duration>,
) -> (Vec<Segment>, Option<Duration>, Option<Duration>) {
    let mut raw_tokens: Vec<(Option<Duration>, String)> = Vec::new();
    let mut current_text = String::new();
    let mut current_ts: Option<Duration> = None;
    let mut first_ts: Option<Duration> = None;

    let mut i = 0;
    let bytes = body.as_bytes();

    while i < bytes.len() {
        let ch_byte = bytes[i];
        if ch_byte == b'<' || ch_byte == b'[' {
            let close_byte = if ch_byte == b'<' { b'>' } else { b']' };
            if let Some(close_rel) = body[i + 1..].find(close_byte as char) {
                let inner = &body[i + 1..i + 1 + close_rel];
                if let Some(ts) = parse_lrc_timestamp(inner) {
                    if first_ts.is_none() {
                        first_ts = Some(ts);
                    }
                    if !current_text.is_empty() || current_ts.is_some() {
                        raw_tokens.push((current_ts, std::mem::take(&mut current_text)));
                    }
                    current_ts = Some(ts);
                    i += 1 + close_rel + 1;
                    continue;
                }
            }
        }

        let ch = body[i..].chars().next().unwrap();
        current_text.push(ch);
        i += ch.len_utf8();
    }

    let mut explicit_end: Option<Duration> = None;
    if !current_text.is_empty() {
        raw_tokens.push((current_ts, current_text));
    } else if let Some(trailing_ts) = current_ts {
        if !raw_tokens.is_empty() {
            explicit_end = Some(trailing_ts);
        } else {
            raw_tokens.push((Some(trailing_ts), String::new()));
        }
    }

    let base = line_offset.or(first_ts).unwrap_or(Duration::ZERO);
    let segments = raw_tokens
        .into_iter()
        .map(|(ts_opt, text)| Segment {
            offset: ts_opt.unwrap_or(base).saturating_sub(base),
            text,
        })
        .collect();

    (segments, first_ts, explicit_end)
}

pub(super) fn is_wrapped_in_parens(s: &str) -> bool {
    (s.starts_with('(') && s.ends_with(')') && s.len() >= 2)
        || (s.starts_with('（') && s.ends_with('）') && s.chars().count() >= 2)
}
