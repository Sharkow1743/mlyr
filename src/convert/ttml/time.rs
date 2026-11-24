use std::time::Duration;

use crate::ConversionError;

use super::xml_dom::XmlElement;

pub(super) fn resolve_end_or_dur(
    el: &XmlElement,
    begin: Option<Duration>,
) -> Result<Option<Duration>, ConversionError> {
    if let Some(end_str) = el.attr("end") {
        if let Some(end) = parse_ttml_time(end_str)? {
            return Ok(Some(end));
        }
    }
    if let Some(dur_str) = el.attr("dur") {
        if let Some(dur) = parse_ttml_time(dur_str)? {
            if let Some(b) = begin {
                return Ok(Some(b.saturating_add(dur)));
            }
        }
    }
    Ok(None)
}

/// Parses W3C TTML & Apple Music time expressions:
/// - Offset time: `432.25s`, `250ms`, `5m`, `0.5h`
/// - Clock time: `HH:MM:SS.fff`, `MM:SS.fff`, `SS.fff`, `HH:MM:SS:frames`
pub(super) fn parse_ttml_time(input: &str) -> Result<Option<Duration>, ConversionError> {
    let s = input.trim();
    if s.is_empty() {
        return Ok(None);
    }

    if let Some(num_str) = s.strip_suffix("ms") {
        return parse_metric_ms(num_str, 1.0, input).map(Some);
    }
    if let Some(num_str) = s.strip_suffix('s') {
        return parse_metric_ms(num_str, 1_000.0, input).map(Some);
    }
    if let Some(num_str) = s.strip_suffix('m') {
        return parse_metric_ms(num_str, 60_000.0, input).map(Some);
    }
    if let Some(num_str) = s.strip_suffix('h') {
        return parse_metric_ms(num_str, 3_600_000.0, input).map(Some);
    }

    let parts: Vec<&str> = s.split(':').collect();
    let (hours, mins, sec_frac): (u64, u64, &str) = match parts.len() {
        1 => (0, 0, parts[0]),
        2 => (
            0,
            parts[0].parse().map_err(|_| invalid_ttml_time(input))?,
            parts[1],
        ),
        3 => (
            parts[0].parse().map_err(|_| invalid_ttml_time(input))?,
            parts[1].parse().map_err(|_| invalid_ttml_time(input))?,
            parts[2],
        ),
        4 => {
            let h: u64 = parts[0].parse().map_err(|_| invalid_ttml_time(input))?;
            let m: u64 = parts[1].parse().map_err(|_| invalid_ttml_time(input))?;
            let sec: u64 = parts[2].parse().map_err(|_| invalid_ttml_time(input))?;
            let frames: u64 = parts[3].parse().map_err(|_| invalid_ttml_time(input))?;
            let ms = ((frames as f64 / 30.0) * 1_000.0).round() as u64;
            return Ok(Some(Duration::from_millis(
                h * 3_600_000 + m * 60_000 + sec * 1_000 + ms,
            )));
        }
        _ => return Err(invalid_ttml_time(input)),
    };

    let (secs_f, millis) = if let Some((sec_s, frac_s)) = sec_frac.split_once('.') {
        let sec: u64 = if sec_s.is_empty() {
            0
        } else {
            sec_s.parse().map_err(|_| invalid_ttml_time(input))?
        };
        let ms = parse_frac_millis(frac_s).ok_or_else(|| invalid_ttml_time(input))?;
        (sec, ms)
    } else {
        let sec: u64 = sec_frac.parse().map_err(|_| invalid_ttml_time(input))?;
        (sec, 0)
    };

    let total_ms = hours
        .checked_mul(3_600_000)
        .and_then(|v| v.checked_add(mins * 60_000))
        .and_then(|v| v.checked_add(secs_f * 1_000))
        .and_then(|v| v.checked_add(millis))
        .ok_or_else(|| invalid_ttml_time(input))?;

    Ok(Some(Duration::from_millis(total_ms)))
}

pub(super) fn format_ttml_time(d: Duration) -> String {
    let total_ms = u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
    let hours = total_ms / 3_600_000;
    let mins = (total_ms % 3_600_000) / 60_000;
    let secs = (total_ms % 60_000) / 1_000;
    let ms = total_ms % 1_000;
    if hours > 0 {
        format!("{hours:02}:{mins:02}:{secs:02}.{ms:03}")
    } else {
        format!("{mins:02}:{secs:02}.{ms:03}")
    }
}

fn parse_metric_ms(num_str: &str, scale_ms: f64, full: &str) -> Result<Duration, ConversionError> {
    let val: f64 = num_str
        .trim()
        .parse()
        .map_err(|_| invalid_ttml_time(full))?;
    if !val.is_finite() || val < 0.0 {
        return Err(invalid_ttml_time(full));
    }
    Ok(Duration::from_millis((val * scale_ms).round() as u64))
}

fn parse_frac_millis(frac: &str) -> Option<u64> {
    if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let b = frac.as_bytes();
    Some(match b.len() {
        1 => u64::from(b[0] - b'0') * 100,
        2 => u64::from(b[0] - b'0') * 100 + u64::from(b[1] - b'0') * 10,
        3 => u64::from(b[0] - b'0') * 100 + u64::from(b[1] - b'0') * 10 + u64::from(b[2] - b'0'),
        _ => {
            let base =
                u64::from(b[0] - b'0') * 100 + u64::from(b[1] - b'0') * 10 + u64::from(b[2] - b'0');
            let carry = u64::from(b[3] >= b'5');
            base + carry
        }
    })
}

fn invalid_ttml_time(input: &str) -> ConversionError {
    ConversionError::InvalidFormat {
        format_name: "Ttml",
        message: format!("Invalid TTML timestamp '{input}'"),
    }
}
