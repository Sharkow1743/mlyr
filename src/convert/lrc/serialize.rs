use std::time::Duration;

use crate::ConversionError;
use crate::model::Mlyr;

use super::timestamp::format_lrc_timestamp;

pub(super) fn serialize_lrc(mlyr: &Mlyr) -> Result<String, ConversionError> {
    mlyr.validate()?;
    let mut output = String::new();

    let meta = mlyr.metadata();
    if let Some(ref ti) = meta.and_then(|m| m.title.as_deref()) {
        output.push_str(&format!("[ti:{ti}]\n"));
    }
    if let Some(ref ar) = meta.and_then(|m| m.artist.as_deref()) {
        output.push_str(&format!("[ar:{ar}]\n"));
    }
    if let Some(ref al) = meta.and_then(|m| m.album.as_deref()) {
        output.push_str(&format!("[al:{al}]\n"));
    }
    if let Some(len) = meta.and_then(|m| m.length) {
        output.push_str(&format!("[length:{}]\n", format_lrc_timestamp(len)));
    }
    if let Some(m) = meta {
        for src in &m.sources {
            if matches!(src.kind.as_str(), "au" | "lr" | "by" | "re" | "ve" | "tool") {
                if let Some(val) = src.name.as_deref().or(src.content.as_deref()) {
                    output.push_str(&format!("[{}:{val}]\n", src.kind));
                }
            }
        }
    }

    let emit_voices = meta.is_some_and(|m| !m.voices.is_empty());

    for line in mlyr.lines() {
        let ts_str = format_lrc_timestamp(line.offset);
        output.push_str(&format!("[{ts_str}]"));

        let mut body = String::new();
        if emit_voices {
            body.push_str(line.voice.as_str());
            body.push(':');
        }

        let has_word_sync =
            line.segments.len() > 1 || line.segments.iter().any(|s| s.offset != Duration::ZERO);

        if has_word_sync {
            for seg in &line.segments {
                let abs_ts = line.offset + seg.offset;
                body.push_str(&format!("<{}>{}", format_lrc_timestamp(abs_ts), seg.text));
            }
            let last_abs_ts = line
                .segments
                .last()
                .map(|s| line.offset + s.offset)
                .unwrap_or(line.offset);
            if line.end() > last_abs_ts || line.segments.last().is_some_and(|s| !s.text.is_empty())
            {
                body.push_str(&format!("<{}>", format_lrc_timestamp(line.end())));
            }
        } else {
            for seg in &line.segments {
                body.push_str(&seg.text);
            }
        }

        if line.is_bg() {
            output.push_str(&format!("[bg:{body}]"));
        } else {
            output.push_str(&body);
        }
        output.push('\n');
    }

    Ok(output)
}
