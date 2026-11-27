#[cfg(feature = "lrc")]
pub mod lrclib;
#[cfg(any(feature = "richsync", feature = "lrc"))]
pub mod musixmatch;
#[cfg(any(feature = "ttml", feature = "lrc"))]
pub mod unison;

#[cfg(feature = "lrc")]
pub use lrclib::{KEY_LRCLIB_ID_STR, KEY_LRCLIB_ID_U64, LrclibProvider};
#[cfg(any(feature = "richsync", feature = "lrc"))]
pub use musixmatch::{
    KEY_COMMONTRACK_ID, KEY_COMMONTRACK_ID_U64, KEY_ISRC, KEY_MXM_TRACK_ID, KEY_MXM_TRACK_ID_U64,
    MusixmatchAuth, MusixmatchProvider,
};
#[cfg(any(feature = "ttml", feature = "lrc"))]
pub use unison::{
    KEY_APPLE_MUSIC_ID, KEY_ISRC as KEY_UNISON_ISRC, KEY_SPOTIFY_ID, KEY_TIDAL_ID, KEY_YOUTUBE_ID,
    UnisonProvider,
};

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

use reqwest::Response;
use reqwest::header::RETRY_AFTER;
use serde::de::DeserializeOwned;

use crate::builders::MlyrBuilder;
use crate::convert::{ConversionError, LyricsConverter};
use crate::model::Mlyr;

use super::pipeline::default_postprocess;
use super::{FetchContext, FetchError, RawCandidate, TrackQuery};

pub trait LyricsProvider: Send + Sync {
    type Format: LyricsConverter;

    fn id(&self) -> &str;

    fn fetch(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> impl Future<Output = Result<Vec<RawCandidate>, FetchError>> + Send;

    fn preprocess(&self, _raw: &mut RawCandidate) {}

    fn parse(&self, raw: &RawCandidate) -> Result<Mlyr, ConversionError> {
        Self::Format::to_mlyr(Self::Format::from(raw.payload.clone()))
    }

    fn postprocess(&self, builder: &mut MlyrBuilder, raw: &RawCandidate) {
        default_postprocess(self.id(), builder, raw);
    }
}

pub trait DynLyricsProvider: Send + Sync {
    fn id(&self) -> &str;

    fn fetch_dyn<'a>(
        &'a self,
        query: &'a TrackQuery,
        ctx: &'a FetchContext,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RawCandidate>, FetchError>> + Send + 'a>>;

    fn preprocess(&self, raw: &mut RawCandidate);

    fn parse(&self, raw: &RawCandidate) -> Result<Mlyr, ConversionError>;

    fn postprocess(&self, builder: &mut MlyrBuilder, raw: &RawCandidate);
}

impl<P: LyricsProvider> DynLyricsProvider for P {
    fn id(&self) -> &str {
        LyricsProvider::id(self)
    }

    fn fetch_dyn<'a>(
        &'a self,
        query: &'a TrackQuery,
        ctx: &'a FetchContext,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RawCandidate>, FetchError>> + Send + 'a>> {
        Box::pin(LyricsProvider::fetch(self, query, ctx))
    }

    fn preprocess(&self, raw: &mut RawCandidate) {
        LyricsProvider::preprocess(self, raw)
    }

    fn parse(&self, raw: &RawCandidate) -> Result<Mlyr, ConversionError> {
        LyricsProvider::parse(self, raw)
    }

    fn postprocess(&self, builder: &mut MlyrBuilder, raw: &RawCandidate) {
        LyricsProvider::postprocess(self, builder, raw)
    }
}

pub(super) fn parse_retry_after(resp: &Response) -> Option<Duration> {
    let header_val = resp.headers().get(RETRY_AFTER)?.to_str().ok()?.trim();
    if let Ok(secs) = header_val.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }

    // Try RFC 2822 / HTTP-date string
    if let Ok(target) = http_date_to_system_time(header_val) {
        if let Ok(duration) = target.duration_since(SystemTime::now()) {
            return Some(duration);
        }
        return Some(Duration::ZERO);
    }
    None
}
/// Basic day-month-year RFC-9110 time parser
fn http_date_to_system_time(s: &str) -> Result<SystemTime, ()> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 5 {
        return Err(());
    }
    let (day_str, month_str, year_str, time_str) = if parts[0].ends_with(',') {
        (parts[1], parts[2], parts[3], parts[4])
    } else {
        (parts[0], parts[1], parts[2], parts[3])
    };

    let day: u64 = day_str.parse().map_err(|_| ())?;
    let year: u64 = year_str.parse().map_err(|_| ())?;
    let month = match month_str.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return Err(()),
    };

    let t_parts: Vec<&str> = time_str.split(':').collect();
    if t_parts.len() != 3 {
        return Err(());
    }
    let hour: u64 = t_parts[0].parse().map_err(|_| ())?;
    let min: u64 = t_parts[1].parse().map_err(|_| ())?;
    let sec: u64 = t_parts[2].parse().map_err(|_| ())?;

    if year < 1970 {
        return Err(());
    }
    let mut days = (year - 1970) * 365 + (year - 1969) / 4;
    let days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 0..(month - 1) as usize {
        days += days_in_months[m];
    }
    if month > 2 && (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)) {
        days += 1;
    }
    days += day - 1;

    let total_secs = days * 86_400 + hour * 3600 + min * 60 + sec;
    Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(total_secs))
}

pub(super) fn append_query(base: &str, params: &[(&str, String)]) -> String {
    if params.is_empty() {
        return base.to_string();
    }
    let mut out = String::from(base);
    out.push(if base.contains('?') { '&' } else { '?' });
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        push_percent_encoded(&mut out, k);
        out.push('=');
        push_percent_encoded(&mut out, v);
    }
    out
}

fn push_percent_encoded(out: &mut String, input: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0F) as usize] as char);
        }
    }
}

pub(super) async fn read_error_body(resp: Response) -> String {
    match resp.bytes().await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => String::new(),
    }
}

pub(super) async fn read_json_response<T: DeserializeOwned>(
    resp: Response,
    label: &str,
) -> Result<T, FetchError> {
    let status = resp.status();
    let bytes = resp.bytes().await?;
    serde_json::from_slice(&bytes).map_err(|e| FetchError::InvalidResponse {
        status: Some(status.as_u16()),
        message: format!("Failed to decode {label}: {e}"),
    })
}
