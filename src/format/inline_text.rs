use std::time::Duration;

use crate::model::Segment;

use super::FormatError;
use super::timestamp::{format_short_duration, parse_timestamp};

pub fn parse_inline_synced_text(input: &str) -> Result<Vec<Segment>, FormatError> {
    let mut segments = Vec::new();
    let mut current_offset: Option<Duration> = None;
    let mut current_text = String::new();

    let mut chars = input.char_indices();
    while let Some((idx, ch)) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some((_, '\\')) => current_text.push('\\'),
                Some((_, '{')) => current_text.push('{'),
                Some((_, other)) => {
                    return Err(FormatError::InvalidInlineText {
                        reason: format!("unsupported escape sequence '\\{other}' at byte {idx}"),
                    });
                }
                None => {
                    return Err(FormatError::InvalidInlineText {
                        reason: "trailing '\\' at end of inline text".to_string(),
                    });
                }
            },
            '{' => {
                if !current_text.is_empty() || current_offset.is_some() {
                    segments.push(Segment {
                        offset: current_offset.unwrap_or(Duration::ZERO),
                        text: std::mem::take(&mut current_text),
                    });
                }

                let start = idx + 1;
                let mut close_idx = None;
                for (j, c) in chars.by_ref() {
                    if c == '}' {
                        close_idx = Some(j);
                        break;
                    }
                }

                let end = close_idx.ok_or_else(|| FormatError::InvalidInlineText {
                    reason: format!("unclosed sync marker '{{' at byte {idx}"),
                })?;

                let ts = parse_timestamp(&input[start..end])?;
                current_offset = Some(ts);
            }
            other => current_text.push(other),
        }
    }

    if !current_text.is_empty() || current_offset.is_some() {
        segments.push(Segment {
            offset: current_offset.unwrap_or(Duration::ZERO),
            text: current_text,
        });
    }

    Ok(segments)
}

pub fn format_inline_synced_text(segments: &[Segment]) -> String {
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        let omit_marker =
            i == 0 && seg.offset.is_zero() && (!seg.text.is_empty() || segments.len() == 1);
        if !omit_marker {
            out.push('{');
            out.push_str(&format_short_duration(seg.offset));
            out.push('}');
        }
        push_escaped(&mut out, &seg.text);
    }
    out
}

fn push_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            c => out.push(c),
        }
    }
}
