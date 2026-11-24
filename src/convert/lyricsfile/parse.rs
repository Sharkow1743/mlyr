use std::time::Duration;

use crate::ConversionError;
use crate::format::SignedTimestamp;
use crate::format::normalize::{RawLine, RawMetadata, RawMlyr, normalize_raw_mlyr};
use crate::model::{LineKind, Mlyr, Segment};

use super::schema::{LyricsfileDoc, LyricsfileLine};

const MIN_LINE_DURATION: Duration = Duration::from_millis(1);
const DEFAULT_TAIL_DURATION: Duration = Duration::from_secs(5);

struct NormalizedLineInput {
    offset: Duration,
    explicit_end: Option<Duration>,
    segments: Vec<Segment>,
}

pub(super) fn parse_lyricsfile(input: &str) -> Result<Mlyr, ConversionError> {
    let doc: LyricsfileDoc =
        serde_yaml_ng::from_str(input).map_err(|e| ConversionError::Parsing {
            format_name: "Lyricsfile",
            source: Box::new(e),
        })?;

    if doc.version.trim() != "1.0" {
        return Err(ConversionError::InvalidFormat {
            format_name: "Lyricsfile",
            message: format!("Unsupported Lyricsfile version '{}'", doc.version),
        });
    }

    if doc.metadata.instrumental {
        let has_lines = !doc.lines.is_empty();
        let has_plain = doc.plain.as_deref().is_some_and(|p| !p.trim().is_empty());
        if has_lines || has_plain {
            return Err(ConversionError::InvalidFormat {
                format_name: "Lyricsfile",
                message: "Instrumental tracks must omit or leave 'lines' and 'plain' empty"
                    .to_string(),
            });
        }
    }

    let length = match doc.metadata.duration_ms {
        Some(ms) if ms < 0 => {
            return Err(ConversionError::InvalidFormat {
                format_name: "Lyricsfile",
                message: format!("metadata.duration_ms cannot be negative ({ms})"),
            });
        }
        Some(0) | None => None,
        Some(ms) => Some(Duration::from_millis(ms as u64)),
    };

    let lyrics_offset = doc.metadata.offset_ms.map(|ms| SignedTimestamp {
        negative: ms < 0,
        magnitude: Duration::from_millis(ms.unsigned_abs()),
    });

    let metadata = RawMetadata {
        title: clean_opt(doc.metadata.title),
        artist: clean_opt(doc.metadata.artist),
        album: clean_opt(doc.metadata.album),
        length,
        voices: Vec::new(),
        sources: Vec::new(),
    };

    let mut parsed_lines = Vec::with_capacity(doc.lines.len());
    for line in doc.lines {
        parsed_lines.push(parse_line(line)?);
    }
    parsed_lines.sort_by_key(|l| l.offset);

    let raw_lines = finalize_lines(&parsed_lines, metadata.length);
    let raw_mlyr = RawMlyr {
        metadata,
        lyrics_offset,
        lines: raw_lines,
    };

    Ok(normalize_raw_mlyr(raw_mlyr)?)
}

fn parse_line(line: LyricsfileLine) -> Result<NormalizedLineInput, ConversionError> {
    if line.start_ms < 0 {
        return Err(ConversionError::InvalidFormat {
            format_name: "Lyricsfile",
            message: format!("Line start_ms cannot be negative ({})", line.start_ms),
        });
    }
    if let Some(end_ms) = line.end_ms {
        if end_ms < line.start_ms {
            return Err(ConversionError::InvalidFormat {
                format_name: "Lyricsfile",
                message: format!(
                    "Line end_ms ({end_ms}) must be >= start_ms ({})",
                    line.start_ms
                ),
            });
        }
    }

    let line_start = Duration::from_millis(line.start_ms as u64);
    let mut explicit_end = line.end_ms.map(|ms| Duration::from_millis(ms as u64));

    let segments = if line.words.is_empty() {
        vec![Segment {
            offset: Duration::ZERO,
            text: line.text,
        }]
    } else {
        let mut words = line.words;
        words.sort_by_key(|w| w.start_ms);

        let mut segs = Vec::with_capacity(words.len());
        let mut max_word_end: Option<Duration> = None;

        for w in words {
            if w.start_ms < 0 {
                return Err(ConversionError::InvalidFormat {
                    format_name: "Lyricsfile",
                    message: format!("Word start_ms cannot be negative ({})", w.start_ms),
                });
            }
            if let Some(w_end) = w.end_ms {
                if w_end < w.start_ms {
                    return Err(ConversionError::InvalidFormat {
                        format_name: "Lyricsfile",
                        message: format!(
                            "Word end_ms ({w_end}) must be >= start_ms ({})",
                            w.start_ms
                        ),
                    });
                }
                let w_end_dur = Duration::from_millis(w_end as u64);
                max_word_end = Some(max_word_end.map_or(w_end_dur, |m| m.max(w_end_dur)));
            }

            let w_start = Duration::from_millis(w.start_ms as u64);
            segs.push(Segment {
                offset: w_start.saturating_sub(line_start),
                text: w.text,
            });
        }

        if explicit_end.is_none() {
            explicit_end = max_word_end;
        }
        segs
    };

    Ok(NormalizedLineInput {
        offset: line_start,
        explicit_end,
        segments,
    })
}

fn finalize_lines(parsed: &[NormalizedLineInput], length: Option<Duration>) -> Vec<RawLine> {
    let mut raw_lines = Vec::with_capacity(parsed.len());
    for (i, cur) in parsed.iter().enumerate() {
        let last_seg_offset = cur
            .segments
            .last()
            .map(|s| s.offset)
            .unwrap_or(Duration::ZERO);

        let duration = if let Some(end) = cur.explicit_end {
            end.saturating_sub(cur.offset)
                .max(MIN_LINE_DURATION)
                .max(last_seg_offset)
        } else if let Some(next) = parsed.get(i + 1) {
            let delta = next.offset.saturating_sub(cur.offset);
            if delta.is_zero() {
                MIN_LINE_DURATION.max(last_seg_offset)
            } else {
                delta.max(last_seg_offset)
            }
        } else {
            match length {
                Some(len) if len > cur.offset => (len - cur.offset)
                    .min(DEFAULT_TAIL_DURATION)
                    .max(MIN_LINE_DURATION)
                    .max(last_seg_offset),
                _ => DEFAULT_TAIL_DURATION.max(last_seg_offset),
            }
        };

        raw_lines.push(RawLine {
            kind: LineKind::Main,
            offset: cur.offset,
            duration,
            voice: None,
            segments: cur.segments.clone(),
        });
    }
    raw_lines
}

fn clean_opt(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}
