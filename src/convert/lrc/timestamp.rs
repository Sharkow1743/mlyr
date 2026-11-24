use std::time::Duration;

pub(super) fn parse_lrc_length(s: &str) -> Option<Duration> {
    if let Some(d) = parse_lrc_timestamp(s) {
        return Some(d);
    }
    if let Ok(secs) = s.trim().parse::<f64>() {
        if secs.is_finite() && secs >= 0.0 {
            return Some(Duration::from_millis((secs * 1_000.0).round() as u64));
        }
    }
    None
}

pub(super) fn parse_lrc_timestamp(s: &str) -> Option<Duration> {
    let s = s.trim();
    let (mins_str, sec_part) = s.split_once(':')?;
    if mins_str.is_empty() || !mins_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mins: u64 = mins_str.parse().ok()?;

    // Support `.` or `:` between seconds and fractional seconds (SPL / legacy LRC compatibility)
    let (secs_str, frac_opt) = match sec_part
        .split_once('.')
        .or_else(|| sec_part.split_once(':'))
    {
        Some((sec, frac)) => (sec, Some(frac)),
        None => (sec_part, None),
    };

    if secs_str.is_empty() || secs_str.len() > 2 || !secs_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let secs: u64 = secs_str.parse().ok()?;
    if secs >= 60 {
        return None;
    }

    let millis: u64 = match frac_opt {
        None => 0,
        Some(frac) => {
            if frac.is_empty() || frac.len() > 6 || !frac.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let b = frac.as_bytes();
            match b.len() {
                1 => u64::from(b[0] - b'0') * 100,
                2 => u64::from(b[0] - b'0') * 100 + u64::from(b[1] - b'0') * 10,
                3 => {
                    u64::from(b[0] - b'0') * 100
                        + u64::from(b[1] - b'0') * 10
                        + u64::from(b[2] - b'0')
                }
                _ => {
                    let base = u64::from(b[0] - b'0') * 100
                        + u64::from(b[1] - b'0') * 10
                        + u64::from(b[2] - b'0');
                    let carry = u64::from(b[3] >= b'5');
                    base + carry
                }
            }
        }
    };

    let total_ms = mins
        .checked_mul(60_000)?
        .checked_add(secs * 1_000)?
        .checked_add(millis)?;
    Some(Duration::from_millis(total_ms))
}

pub(super) fn format_lrc_timestamp(duration: Duration) -> String {
    let total_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
    let total_secs = total_ms / 1_000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let ms = total_ms % 1_000;
    if ms % 10 == 0 {
        let centis = ms / 10;
        format!("{mins:02}:{secs:02}.{centis:02}")
    } else {
        format!("{mins:02}:{secs:02}.{ms:03}")
    }
}
