use std::str::FromStr;
use std::time::Duration;

use super::FormatError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SignedTimestamp {
    pub negative: bool,
    pub magnitude: Duration,
}

impl SignedTimestamp {
    pub fn forward(magnitude: Duration) -> Self {
        Self {
            negative: false,
            magnitude,
        }
    }

    pub fn backward(magnitude: Duration) -> Self {
        Self {
            negative: true,
            magnitude,
        }
    }

    pub fn parse(input: &str) -> Result<Self, FormatError> {
        if let Some(rest) = input.strip_prefix('-') {
            Ok(Self {
                negative: true,
                magnitude: parse_timestamp(rest)?,
            })
        } else if let Some(rest) = input.strip_prefix('+') {
            Ok(Self {
                negative: false,
                magnitude: parse_timestamp(rest)?,
            })
        } else {
            Ok(Self {
                negative: false,
                magnitude: parse_timestamp(input)?,
            })
        }
    }
}

impl From<Duration> for SignedTimestamp {
    fn from(magnitude: Duration) -> Self {
        Self::forward(magnitude)
    }
}

impl FromStr for SignedTimestamp {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Parses an unsigned MLYR `Timestamp` (`<sec>[.<frac>]` or `<min>:[<sec>][.<frac>]`) into a millisecond-precision [`Duration`]
pub fn parse_timestamp(input: &str) -> Result<Duration, FormatError> {
    if input.is_empty() {
        return Err(FormatError::InvalidTimestamp {
            input: input.to_string(),
            reason: "timestamp cannot be empty",
        });
    }

    let (mins_str, rest) = match input.split_once(':') {
        Some((m, r)) => {
            if r.contains(':') {
                return Err(FormatError::InvalidTimestamp {
                    input: input.to_string(),
                    reason: "hours are not allowed (at most one ':')",
                });
            }
            if m.is_empty() || !m.bytes().all(|b| b.is_ascii_digit()) {
                return Err(FormatError::InvalidTimestamp {
                    input: input.to_string(),
                    reason: "minutes must contain one or more ASCII digits",
                });
            }
            (Some(m), r)
        }
        None => (None, input),
    };

    let (sec_str, frac_str) = match rest.split_once('.') {
        Some((s, f)) => {
            if f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit()) {
                return Err(FormatError::InvalidTimestamp {
                    input: input.to_string(),
                    reason: "fractional part after '.' must contain one or more ASCII digits",
                });
            }
            (s, Some(f))
        }
        None => (rest, None),
    };

    let secs: u64 = if mins_str.is_some() && sec_str.is_empty() {
        0
    } else {
        if sec_str.is_empty() || !sec_str.bytes().all(|b| b.is_ascii_digit()) {
            return Err(FormatError::InvalidTimestamp {
                input: input.to_string(),
                reason: "seconds must contain one or more ASCII digits",
            });
        }
        sec_str
            .parse::<u64>()
            .map_err(|_| FormatError::InvalidTimestamp {
                input: input.to_string(),
                reason: "seconds overflow",
            })?
    };

    if secs >= 60 {
        return Err(FormatError::InvalidTimestamp {
            input: input.to_string(),
            reason: "seconds must be strictly less than 60",
        });
    }

    let mins: u64 = match mins_str {
        Some(m) => m
            .parse::<u64>()
            .map_err(|_| FormatError::InvalidTimestamp {
                input: input.to_string(),
                reason: "minutes overflow",
            })?,
        None => 0,
    };

    let millis: u64 = match frac_str {
        None => 0,
        Some(f) => {
            let b = f.as_bytes();
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
        .checked_mul(60_000)
        .and_then(|v| v.checked_add(secs * 1_000))
        .and_then(|v| v.checked_add(millis))
        .ok_or_else(|| FormatError::InvalidTimestamp {
            input: input.to_string(),
            reason: "timestamp duration overflow",
        })?;

    Ok(Duration::from_millis(total_ms))
}

/// Formats a track-position timestamp (line offsets and song length) as `mm:ss[.fff]`
pub fn format_position(duration: Duration) -> String {
    let total_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
    let mins = total_ms / 60_000;
    let secs = (total_ms % 60_000) / 1_000;
    let ms = total_ms % 1_000;

    let mut out = format!("{mins:02}:{secs:02}");
    push_trimmed_millis(&mut out, ms);
    out
}

/// Formats a short relative duration (line durations and segment offsets) as `[mm:]ss[.fff]`
pub fn format_short_duration(duration: Duration) -> String {
    let total_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
    let mins = total_ms / 60_000;
    let secs = (total_ms % 60_000) / 1_000;
    let ms = total_ms % 1_000;

    let mut out = if mins == 0 {
        format!("{secs}")
    } else {
        format!("{mins}:{secs:02}")
    };
    push_trimmed_millis(&mut out, ms);
    out
}

fn push_trimmed_millis(out: &mut String, ms: u64) {
    if ms == 0 {
        return;
    }
    let buf = format!("{ms:03}");
    let trimmed = buf.trim_end_matches('0');
    out.push('.');
    out.push_str(trimmed);
}
