use std::time::Duration;

use crate::ConversionError;
use crate::model::Mlyr;

use super::timestamp::format_lrc_timestamp;

pub(super) fn serialize_lrc(mlyr: &Mlyr) -> Result<String, ConversionError> {
    mlyr.validate()?;
    let mut output = String::new();

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
    for src in &mlyr.metadata.sources {
        if matches!(src.kind.as_str(), "au" | "lr" | "by" | "re" | "ve" | "tool") {
            if let Some(val) = src.name.as_deref().or(src.content.as_deref()) {
                output.push_str(&format!("[{}:{val}]\n", src.kind));
            }
        }
    }

    let emit_voices = !mlyr.metadata.voices.is_empty();

    for line in &mlyr.lyrics.lines {
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
