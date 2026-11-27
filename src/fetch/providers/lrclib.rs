use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;

use crate::convert::Lrc;
use crate::fetch::{FetchContext, FetchError, FetchedMetadata, Key, RawCandidate, TrackQuery};
use crate::model::Source;

use super::{LyricsProvider, append_query, parse_retry_after, read_error_body, read_json_response};

const DEFAULT_BASE_URL: &str = "https://lrclib.net";

pub const KEY_LRCLIB_ID_U64: Key<u64> = Key::new("lrclib_id");
pub const KEY_LRCLIB_ID_STR: Key<String> = Key::new("lrclib_id");

#[derive(Debug, Clone)]
pub struct LrclibProvider {
    base_url: String,
    use_search_fallback: bool,
}

impl Default for LrclibProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LrclibProvider {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            use_search_fallback: true,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_search_fallback(mut self, enabled: bool) -> Self {
        self.use_search_fallback = enabled;
        self
    }

    fn record_to_candidate(&self, rec: LrclibRecord) -> Option<RawCandidate> {
        if rec.instrumental {
            return None;
        }
        let synced = rec.synced_lyrics?;
        if synced.trim().is_empty() {
            return None;
        }

        let length = rec
            .duration
            .filter(|d| d.is_finite() && *d > 0.0)
            .map(|d| Duration::from_millis((d * 1_000.0).round() as u64));

        let fetched_meta = FetchedMetadata {
            title: rec.track_name.or(rec.name),
            artist: rec.artist_name,
            album: rec.album_name,
            length,
            sources: vec![Source::new(
                "id",
                Some("lrclib".to_string()),
                Some(rec.id.to_string()),
            )],
        };

        Some(
            RawCandidate::new(synced)
                .with_source_url(format!("{}/api/get/{}", self.base_url, rec.id))
                .with_fetched_meta(fetched_meta)
                .with_format_hint("lrc"),
        )
    }

    async fn fetch_by_id(
        &self,
        id: &str,
        ctx: &FetchContext,
    ) -> Result<Option<RawCandidate>, FetchError> {
        if !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Ok(None);
        }

        let url = format!("{}/api/get/{id}", self.base_url);
        let resp = ctx.get(&url).send().await?;
        let status = resp.status();

        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status == StatusCode::TOO_MANY_REQUESTS
            || (status == StatusCode::SERVICE_UNAVAILABLE && parse_retry_after(&resp).is_some())
        {
            return Err(FetchError::RateLimited {
                retry_after: parse_retry_after(&resp),
            });
        }
        if !status.is_success() {
            return Err(FetchError::InvalidResponse {
                status: Some(status.as_u16()),
                message: read_error_body(resp).await,
            });
        }

        let rec: LrclibRecord = read_json_response(resp, "LRCLIB response").await?;
        Ok(self.record_to_candidate(rec))
    }

    async fn fetch_get(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Option<RawCandidate>, FetchError> {
        let mut params: Vec<(&str, String)> = vec![
            ("track_name", query.title.clone()),
            ("artist_name", query.artist.clone()),
        ];
        if let Some(ref album) = query.album {
            if !album.trim().is_empty() {
                params.push(("album_name", album.clone()));
            }
        }
        if let Some(dur) = query.duration {
            let secs = dur.as_secs_f64().round() as u64;
            if secs > 0 {
                params.push(("duration", secs.to_string()));
            }
        }

        let url = append_query(&format!("{}/api/get", self.base_url), &params);
        let resp = ctx.get(&url).send().await?;
        let status = resp.status();

        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status == StatusCode::TOO_MANY_REQUESTS
            || (status == StatusCode::SERVICE_UNAVAILABLE && parse_retry_after(&resp).is_some())
        {
            return Err(FetchError::RateLimited {
                retry_after: parse_retry_after(&resp),
            });
        }
        if !status.is_success() {
            return Err(FetchError::InvalidResponse {
                status: Some(status.as_u16()),
                message: read_error_body(resp).await,
            });
        }

        let rec: LrclibRecord = read_json_response(resp, "LRCLIB response").await?;
        Ok(self.record_to_candidate(rec))
    }

    async fn fetch_search(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        if !query.title.trim().is_empty() && !query.artist.trim().is_empty() {
            // First attempt with album if available
            if let Some(ref album) = query.album {
                if !album.trim().is_empty() {
                    let params = vec![
                        ("track_name", query.title.clone()),
                        ("artist_name", query.artist.clone()),
                        ("album_name", album.clone()),
                    ];
                    let hits = self.execute_search(&params, ctx).await?;
                    if !hits.is_empty() {
                        return Ok(hits);
                    }
                }
            }

            // Fallback: search without album
            let params = vec![
                ("track_name", query.title.clone()),
                ("artist_name", query.artist.clone()),
            ];
            let hits = self.execute_search(&params, ctx).await?;
            if !hits.is_empty() {
                return Ok(hits);
            }
        }

        let q = format!("{} {}", query.artist, query.title)
            .trim()
            .to_string();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        self.execute_search(&[("q", q)], ctx).await
    }

    async fn execute_search(
        &self,
        params: &[(&str, String)],
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let url = append_query(&format!("{}/api/search", self.base_url), params);
        let resp = ctx.get(&url).send().await?;
        let status = resp.status();

        if status == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if status == StatusCode::TOO_MANY_REQUESTS
            || (status == StatusCode::SERVICE_UNAVAILABLE && parse_retry_after(&resp).is_some())
        {
            return Err(FetchError::RateLimited {
                retry_after: parse_retry_after(&resp),
            });
        }
        if !status.is_success() {
            return Err(FetchError::InvalidResponse {
                status: Some(status.as_u16()),
                message: read_error_body(resp).await,
            });
        }

        let records: Vec<LrclibRecord> = read_json_response(resp, "LRCLIB search response").await?;

        Ok(records
            .into_iter()
            .filter_map(|rec| self.record_to_candidate(rec))
            .collect())
    }
}

impl LyricsProvider for LrclibProvider {
    type Format = Lrc;

    fn id(&self) -> &str {
        "lrclib"
    }

    async fn fetch(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let lrclib_id = query
            .extensions
            .get(KEY_LRCLIB_ID_U64)
            .map(u64::to_string)
            .or_else(|| query.extensions.get(KEY_LRCLIB_ID_STR).cloned());

        if let Some(id) = lrclib_id {
            if let Some(hit) = self.fetch_by_id(&id, ctx).await? {
                return Ok(vec![hit]);
            }
        }

        if !query.title.trim().is_empty() && !query.artist.trim().is_empty() {
            if let Some(hit) = self.fetch_get(query, ctx).await? {
                return Ok(vec![hit]);
            }
        }

        if self.use_search_fallback {
            return self.fetch_search(query, ctx).await;
        }

        Ok(Vec::new())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibRecord {
    id: u64,
    name: Option<String>,
    track_name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
    duration: Option<f64>,
    #[serde(default)]
    instrumental: bool,
    synced_lyrics: Option<String>,
    #[allow(dead_code)]
    plain_lyrics: Option<String>,
}
