use reqwest::StatusCode;
use serde::Deserialize;

use crate::convert::ConversionError;
#[cfg(feature = "lrc")]
use crate::convert::Lrc;
use crate::convert::LyricsConverter;
#[cfg(feature = "ttml")]
use crate::convert::Ttml;
use crate::fetch::{FetchContext, FetchError, FetchedMetadata, Key, RawCandidate, TrackQuery};
use crate::model::{Mlyr, Source};

use super::{LyricsProvider, append_query, parse_retry_after, read_error_body, read_json_response};

const DEFAULT_BASE_URL: &str = "https://unison.betterlyrics.org";

pub const KEY_YOUTUBE_ID: Key<String> = Key::new("youtube_id");
pub const KEY_ISRC: Key<String> = Key::new("isrc");
pub const KEY_SPOTIFY_ID: Key<String> = Key::new("spotify_id");
pub const KEY_APPLE_MUSIC_ID: Key<String> = Key::new("apple_music_id");
pub const KEY_TIDAL_ID: Key<String> = Key::new("tidal_id");

#[derive(Debug, Clone)]
pub struct UnisonProvider {
    base_url: String,
    use_search_fallback: bool,
}

impl Default for UnisonProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UnisonProvider {
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

    fn item_to_candidate(&self, item: UnisonLyric) -> Option<RawCandidate> {
        let lyrics = item.lyrics?;
        if lyrics.trim().is_empty() {
            return None;
        }
        if item
            .format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("plain"))
            || item
                .sync_type
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case("plain"))
        {
            return None;
        }

        let source_url = item
            .video_id
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(|vid| format!("{}/song/{vid}", self.base_url))
            .or_else(|| item.id.map(|id| format!("{}/lyrics/{id}", self.base_url)));

        let mut sources = Vec::new();
        if let Some(sub) = item.submitter {
            if let Some(name) = sub.display_name.filter(|n| !n.trim().is_empty()) {
                let curator_url = sub
                    .key_id
                    .filter(|k| !k.trim().is_empty())
                    .map(|k| format!("{}/curator/{k}", self.base_url));
                sources.push(Source::new("submitter", Some(name), curator_url));
            }
        }
        if let Some(isrc) = item.isrc.filter(|s| !s.trim().is_empty()) {
            sources.push(Source::new("id", Some("isrc".to_string()), Some(isrc)));
        }

        let fetched_meta = FetchedMetadata {
            title: item.song,
            artist: item.artist,
            album: item.album,
            length: None,
            sources,
        };

        let mut cand = RawCandidate::new(lyrics).with_fetched_meta(fetched_meta);
        if let Some(url) = source_url {
            cand = cand.with_source_url(url);
        }
        if let Some(fmt) = item.format {
            cand = cand.with_format_hint(fmt);
        }
        Some(cand)
    }

    async fn fetch_by_ids(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Option<RawCandidate>, FetchError> {
        for (key, param_name) in [
            (&KEY_YOUTUBE_ID, "v"),
            (&KEY_ISRC, "isrc"),
            (&KEY_SPOTIFY_ID, "spotify"),
            (&KEY_APPLE_MUSIC_ID, "apple_music"),
            (&KEY_TIDAL_ID, "tidal"),
        ] {
            if let Some(val) = query.extensions.get(*key) {
                let params = vec![(param_name, val.clone())];
                if let Some(hit) = self.request_single_lyric(&params, ctx).await? {
                    return Ok(Some(hit));
                }
            }
        }
        Ok(None)
    }

    async fn fetch_by_metadata(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Option<RawCandidate>, FetchError> {
        if query.title.trim().is_empty() || query.artist.trim().is_empty() {
            return Ok(None);
        }

        let has_extra = query.album.as_deref().is_some_and(|a| !a.trim().is_empty())
            || query.duration.is_some();

        if has_extra {
            let mut params: Vec<(&str, String)> = vec![
                ("song", query.title.clone()),
                ("artist", query.artist.clone()),
            ];
            if let Some(ref album) = query.album {
                if !album.trim().is_empty() {
                    params.push(("album", album.clone()));
                }
            }
            if let Some(dur) = query.duration {
                let secs = dur.as_secs_f64().round() as u64;
                if secs > 0 {
                    params.push(("duration", secs.to_string()));
                }
            }
            if let Some(hit) = self.request_single_lyric(&params, ctx).await? {
                return Ok(Some(hit));
            }
        }

        // Relaxed query: song + artist only
        let params = vec![
            ("song", query.title.clone()),
            ("artist", query.artist.clone()),
        ];
        self.request_single_lyric(&params, ctx).await
    }

    async fn request_single_lyric(
        &self,
        params: &[(&str, String)],
        ctx: &FetchContext,
    ) -> Result<Option<RawCandidate>, FetchError> {
        let url = append_query(&format!("{}/lyrics", self.base_url), params);
        let resp = ctx.get(&url).send().await?;
        let status = resp.status();

        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE {
            let retry_after = parse_retry_after(&resp);
            if status == StatusCode::TOO_MANY_REQUESTS || retry_after.is_some() {
                return Err(FetchError::RateLimited { retry_after });
            }
        }
        if !status.is_success() {
            return Err(FetchError::InvalidResponse {
                status: Some(status.as_u16()),
                message: read_error_body(resp).await,
            });
        }

        let envelope: UnisonEnvelope<UnisonLyric> =
            read_json_response(resp, "Unison response").await?;

        if !envelope.success {
            return Ok(None);
        }
        Ok(envelope.data.and_then(|d| self.item_to_candidate(d)))
    }

    async fn fetch_search(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let q = format!("{} {}", query.title, query.artist)
            .trim()
            .to_string();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let params = vec![("q", q)];

        let url = append_query(&format!("{}/lyrics/search", self.base_url), &params);
        let resp = ctx.get(&url).send().await?;
        let status = resp.status();

        if status == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE {
            let retry_after = parse_retry_after(&resp);
            if status == StatusCode::TOO_MANY_REQUESTS || retry_after.is_some() {
                return Err(FetchError::RateLimited { retry_after });
            }
        }
        if !status.is_success() {
            return Err(FetchError::InvalidResponse {
                status: Some(status.as_u16()),
                message: read_error_body(resp).await,
            });
        }

        let envelope: UnisonEnvelope<Vec<UnisonLyric>> =
            read_json_response(resp, "Unison search response").await?;

        if !envelope.success {
            return Ok(Vec::new());
        }
        Ok(envelope
            .data
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| self.item_to_candidate(item))
            .collect())
    }
}

impl LyricsProvider for UnisonProvider {
    #[cfg(feature = "ttml")]
    type Format = Ttml;
    #[cfg(not(feature = "ttml"))]
    type Format = Lrc;

    fn id(&self) -> &str {
        "unison"
    }

    async fn fetch(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        if let Some(hit) = self.fetch_by_ids(query, ctx).await? {
            return Ok(vec![hit]);
        }

        if let Some(hit) = self.fetch_by_metadata(query, ctx).await? {
            return Ok(vec![hit]);
        }

        if self.use_search_fallback {
            return self.fetch_search(query, ctx).await;
        }

        Ok(Vec::new())
    }

    fn parse(&self, raw: &RawCandidate) -> Result<Mlyr, ConversionError> {
        let hint = raw.format_hint.as_deref().map(str::to_ascii_lowercase);
        match hint.as_deref() {
            #[cfg(feature = "ttml")]
            Some("ttml") => Ttml::to_mlyr(Ttml(raw.payload.clone())),
            #[cfg(not(feature = "ttml"))]
            Some("ttml") => Err(ConversionError::InvalidFormat {
                format_name: "Unison",
                message: "TTML support is disabled (enable the 'ttml' feature)".to_string(),
            }),
            #[cfg(feature = "lrc")]
            Some("lrc") => Lrc::to_mlyr(Lrc(raw.payload.clone())),
            #[cfg(not(feature = "lrc"))]
            Some("lrc") => Err(ConversionError::InvalidFormat {
                format_name: "Unison",
                message: "LRC support is disabled (enable the 'lrc' feature)".to_string(),
            }),
            Some("plain") => Err(ConversionError::InvalidFormat {
                format_name: "Unison",
                message: "Plain unsynced lyrics are not supported".to_string(),
            }),
            _ => {
                #[cfg(feature = "ttml")]
                if raw.payload.trim_start().starts_with('<') {
                    return Ttml::to_mlyr(Ttml(raw.payload.clone()));
                }
                #[cfg(feature = "lrc")]
                {
                    Lrc::to_mlyr(Lrc(raw.payload.clone()))
                }
                #[cfg(not(feature = "lrc"))]
                {
                    Ttml::to_mlyr(Ttml(raw.payload.clone()))
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct UnisonEnvelope<T> {
    #[serde(default)]
    success: bool,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnisonLyric {
    id: Option<u64>,
    video_id: Option<String>,
    song: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    isrc: Option<String>,
    lyrics: Option<String>,
    format: Option<String>,
    sync_type: Option<String>,
    submitter: Option<UnisonSubmitter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnisonSubmitter {
    display_name: Option<String>,
    key_id: Option<String>,
}
