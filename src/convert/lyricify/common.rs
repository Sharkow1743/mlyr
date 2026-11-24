use std::time::Duration;

use crate::format::normalize::RawMetadata;
use crate::model::{Mlyr, Source};

pub(super) const MIN_LINE_DURATION: Duration = Duration::from_millis(1);

pub(super) fn try_parse_lyricify_metadata(line: &str, metadata: &mut RawMetadata) -> Option<bool> {
    if !(line.starts_with('[') && line.ends_with(']')) {
        return None;
    }
    let inner = &line[1..line.len() - 1];
    if inner.contains(']') {
        return None;
    }
    let (key, val) = inner.split_once(':')?;
    let key_lower = key.trim().to_ascii_lowercase();
    let val_trimmed = val.trim().to_string();

    match key_lower.as_str() {
        "ti" | "title" if !val_trimmed.is_empty() => {
            metadata.title = Some(val_trimmed);
            Some(true)
        }
        "ar" | "artist" if !val_trimmed.is_empty() => {
            metadata.artist = Some(val_trimmed);
            Some(true)
        }
        "al" | "album" if !val_trimmed.is_empty() => {
            metadata.album = Some(val_trimmed);
            Some(true)
        }
        "by" | "author" | "tool" if !val_trimmed.is_empty() => {
            metadata
                .sources
                .push(Source::with_name(key_lower, val_trimmed));
            Some(true)
        }
        "offset" => Some(true),
        _ => None,
    }
}

pub(super) fn append_lyricify_metadata(out: &mut String, mlyr: &Mlyr) {
    if let Some(ref ti) = mlyr.metadata.title {
        out.push_str(&format!("[ti:{ti}]\n"));
    }
    if let Some(ref ar) = mlyr.metadata.artist {
        out.push_str(&format!("[ar:{ar}]\n"));
    }
    if let Some(ref al) = mlyr.metadata.album {
        out.push_str(&format!("[al:{al}]\n"));
    }
}

pub(super) fn is_wrapped_in_parens(s: &str) -> bool {
    (s.starts_with('(') && s.ends_with(')') && s.len() >= 2)
        || (s.starts_with('（') && s.ends_with('）') && s.chars().count() >= 2)
}
