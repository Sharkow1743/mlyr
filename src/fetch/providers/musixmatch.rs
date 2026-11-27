use std::sync::Arc;
use std::time::Duration;

use futures::lock::Mutex;
use reqwest::StatusCode;
use serde_json::Value;

#[cfg(feature = "lrc")]
use crate::convert::Lrc;
#[cfg(feature = "richsync")]
use crate::convert::Richsync;
use crate::convert::{ConversionError, LyricsConverter};
use crate::fetch::{FetchContext, FetchError, FetchedMetadata, Key, RawCandidate, TrackQuery};
use crate::model::{Mlyr, Source};

use super::{LyricsProvider, append_query, parse_retry_after, read_error_body, read_json_response};

const DEFAULT_INTERNAL_BASE_URL: &str = "https://apic.musixmatch.com/ws/1.1";
const DEFAULT_INTERNAL_APP_ID: &str = "android-player-v1.0";
const DEFAULT_API_BASE_URL: &str = "https://api.musixmatch.com/ws/1.1";

pub const KEY_ISRC: Key<String> = Key::new("isrc");
pub const KEY_MXM_TRACK_ID: Key<String> = Key::new("musixmatch_track_id");
pub const KEY_COMMONTRACK_ID: Key<String> = Key::new("commontrack_id");
pub const KEY_MXM_TRACK_ID_U64: Key<u64> = Key::new("musixmatch_track_id");
pub const KEY_COMMONTRACK_ID_U64: Key<u64> = Key::new("commontrack_id");

#[derive(Debug, Clone)]
pub enum MusixmatchAuth {
    Anonymous { app_id: String },
    UserToken { token: String, app_id: String },
    ApiKey(String),
}

#[derive(Debug, Clone)]
struct CachedSession {
    token: String,
    cookie: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MusixmatchProvider {
    auth: MusixmatchAuth,
    base_url: String,
    cached_session: Arc<Mutex<Option<CachedSession>>>,
}

impl Default for MusixmatchProvider {
    fn default() -> Self {
        Self::new()
    }
}

enum MethodOutcome {
    Ok(Value),
    NotFound,
    RenewToken { hint: Option<String> },
}

impl MusixmatchProvider {
    pub fn new() -> Self {
        Self {
            auth: MusixmatchAuth::Anonymous {
                app_id: DEFAULT_INTERNAL_APP_ID.to_string(),
            },
            base_url: DEFAULT_INTERNAL_BASE_URL.to_string(),
            cached_session: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_user_token(token: impl Into<String>) -> Self {
        Self {
            auth: MusixmatchAuth::UserToken {
                token: token.into(),
                app_id: DEFAULT_INTERNAL_APP_ID.to_string(),
            },
            base_url: DEFAULT_INTERNAL_BASE_URL.to_string(),
            cached_session: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            auth: MusixmatchAuth::ApiKey(api_key.into()),
            base_url: DEFAULT_API_BASE_URL.to_string(),
            cached_session: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        match &mut self.auth {
            MusixmatchAuth::Anonymous { app_id: slot }
            | MusixmatchAuth::UserToken { app_id: slot, .. } => {
                *slot = app_id.into();
            }
            MusixmatchAuth::ApiKey(_) => {}
        }
        self
    }

    fn app_id(&self) -> &str {
        match &self.auth {
            MusixmatchAuth::Anonymous { app_id } | MusixmatchAuth::UserToken { app_id, .. } => {
                app_id
            }
            MusixmatchAuth::ApiKey(_) => DEFAULT_INTERNAL_APP_ID,
        }
    }

    fn base_params(&self, params: &mut Vec<(&str, String)>, token: Option<&str>) {
        params.push(("format", "json".to_string()));
        match &self.auth {
            MusixmatchAuth::Anonymous { app_id } => {
                params.push(("app_id", app_id.clone()));
                if let Some(tok) = token {
                    params.push(("usertoken", tok.to_string()));
                }
            }
            MusixmatchAuth::UserToken { token, app_id } => {
                params.push(("app_id", app_id.clone()));
                params.push(("usertoken", token.clone()));
            }
            MusixmatchAuth::ApiKey(key) => {
                params.push(("apikey", key.clone()));
            }
        }
    }

    fn append_query_params(
        &self,
        query: &TrackQuery,
        include_album_and_duration: bool,
        params: &mut Vec<(&str, String)>,
    ) {
        if let Some(track_id) = query
            .extensions
            .get(KEY_MXM_TRACK_ID_U64)
            .map(|id| id.to_string())
            .or_else(|| query.extensions.get(KEY_MXM_TRACK_ID).cloned())
        {
            params.push(("track_id", track_id));
        }
        if let Some(ct_id) = query
            .extensions
            .get(KEY_COMMONTRACK_ID_U64)
            .map(|id| id.to_string())
            .or_else(|| query.extensions.get(KEY_COMMONTRACK_ID).cloned())
        {
            params.push(("commontrack_id", ct_id));
        }
        if let Some(isrc) = query.extensions.get(KEY_ISRC) {
            params.push(("track_isrc", isrc.clone()));
        }
        if !query.title.trim().is_empty() {
            params.push(("q_track", query.title.clone()));
        }
        if !query.artist.trim().is_empty() {
            params.push(("q_artist", query.artist.clone()));
        }
        if include_album_and_duration {
            if let Some(ref album) = query.album {
                if !album.trim().is_empty() {
                    params.push(("q_album", album.clone()));
                }
            }
            if let Some(dur) = query.duration {
                let secs = dur.as_secs_f64().round() as u64;
                if secs > 0 {
                    params.push(("q_duration", secs.to_string()));
                }
            }
        }
    }

    async fn ensure_anonymous_session(
        &self,
        ctx: &FetchContext,
        failed_token: Option<&str>,
    ) -> Result<CachedSession, FetchError> {
        let mut guard = self.cached_session.lock().await;
        if let Some(ref session) = *guard {
            if let Some(failed) = failed_token {
                if session.token != failed {
                    return Ok(session.clone());
                }
            } else {
                return Ok(session.clone());
            }
        }

        let session = self.mint_anonymous_token(ctx).await?;
        *guard = Some(session.clone());
        Ok(session)
    }

    async fn mint_anonymous_token(&self, ctx: &FetchContext) -> Result<CachedSession, FetchError> {
        let params = vec![
            ("app_id", self.app_id().to_string()),
            ("user_language", "en".to_string()),
            ("format", "json".to_string()),
        ];

        let (outcome, cookie) = self
            .call_method_raw("token.get", &params, None, ctx)
            .await?;

        let body = match outcome {
            MethodOutcome::Ok(body) => body,
            MethodOutcome::NotFound => {
                return Err(FetchError::InvalidResponse {
                    status: Some(404),
                    message: "Musixmatch token.get returned 404".to_string(),
                });
            }
            MethodOutcome::RenewToken { hint } => {
                return Err(FetchError::InvalidResponse {
                    status: Some(401),
                    message: format!(
                        "Musixmatch token.get returned 401{}",
                        hint.map(|h| format!(" (hint: {h})")).unwrap_or_default()
                    ),
                });
            }
        };

        let token = body
            .get("user_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");

        if is_degenerate_token(token) {
            return Err(FetchError::InvalidResponse {
                status: Some(200),
                message: format!(
                    "Musixmatch token.get returned degenerate token from {}",
                    self.base_url
                ),
            });
        }

        Ok(CachedSession {
            token: token.to_string(),
            cookie,
        })
    }

    async fn call_method_raw(
        &self,
        method: &str,
        params: &[(&str, String)],
        cookie: Option<&str>,
        ctx: &FetchContext,
    ) -> Result<(MethodOutcome, Option<String>), FetchError> {
        let url = append_query(&format!("{}/{method}", self.base_url), params);
        let mut req = ctx.get(&url);
        if let Some(c) = cookie {
            if !ctx.headers().contains_key(reqwest::header::COOKIE) {
                req = req.header(reqwest::header::COOKIE, c);
            }
        }
        let resp = req.send().await?;
        let http_status = resp.status();
        let set_cookie = extract_cookie_header(&resp);

        if http_status == StatusCode::TOO_MANY_REQUESTS {
            return Err(FetchError::RateLimited {
                retry_after: parse_retry_after(&resp),
            });
        }
        if !http_status.is_success() {
            return Err(FetchError::InvalidResponse {
                status: Some(http_status.as_u16()),
                message: read_error_body(resp).await,
            });
        }

        let root: Value = read_json_response(resp, "Musixmatch JSON").await?;
        let header = root.pointer("/message/header");
        let status_code = header
            .and_then(|h| h.get("status_code"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let hint = header
            .and_then(|h| h.get("hint"))
            .and_then(Value::as_str)
            .map(str::to_string);

        let outcome = match status_code {
            200 => MethodOutcome::Ok(
                root.pointer("/message/body")
                    .cloned()
                    .unwrap_or(Value::Null),
            ),
            404 => MethodOutcome::NotFound,
            401 => MethodOutcome::RenewToken { hint },
            429 => return Err(FetchError::RateLimited { retry_after: None }),
            code => {
                return Err(FetchError::InvalidResponse {
                    status: Some(code as u16),
                    message: format!(
                        "Musixmatch API status_code {code} for {method}{}",
                        hint.map(|h| format!(" (hint: {h})")).unwrap_or_default()
                    ),
                });
            }
        };

        Ok((outcome, set_cookie))
    }

    async fn call_method(
        &self,
        method: &str,
        params: &[(&str, String)],
        cookie: Option<&str>,
        ctx: &FetchContext,
    ) -> Result<Option<Value>, FetchError> {
        let (outcome, set_cookie) = self.call_method_raw(method, params, cookie, ctx).await?;
        if let Some(c) = set_cookie {
            let mut guard = self.cached_session.lock().await;
            if let Some(ref mut session) = *guard {
                session.cookie = Some(c);
            }
        }
        match outcome {
            MethodOutcome::Ok(body) => Ok(Some(body)),
            MethodOutcome::NotFound => Ok(None),
            MethodOutcome::RenewToken { hint } => Err(FetchError::InvalidResponse {
                status: Some(401),
                message: format!(
                    "Musixmatch unauthorized for {method}{}",
                    hint.map(|h| format!(" (hint: {h})")).unwrap_or_default()
                ),
            }),
        }
    }

    async fn fetch_macro(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let hits = self.fetch_macro_once(query, true, ctx).await?;
        if !hits.is_empty() {
            return Ok(hits);
        }

        let has_extra_filters = query.album.as_deref().is_some_and(|a| !a.trim().is_empty())
            || query.duration.is_some();
        if has_extra_filters && !query.title.trim().is_empty() && !query.artist.trim().is_empty() {
            let relaxed = self.fetch_macro_once(query, false, ctx).await?;
            if !relaxed.is_empty() {
                return Ok(relaxed);
            }
        }

        if !query.title.trim().is_empty() || !query.artist.trim().is_empty() {
            return self.fetch_search_fallback(query, ctx).await;
        }

        Ok(Vec::new())
    }

    async fn fetch_macro_once(
        &self,
        query: &TrackQuery,
        include_album_and_duration: bool,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let mut failed_token: Option<String> = None;

        for _attempt in 0..2 {
            let session = match &self.auth {
                MusixmatchAuth::Anonymous { .. } => Some(
                    self.ensure_anonymous_session(ctx, failed_token.as_deref())
                        .await?,
                ),
                MusixmatchAuth::UserToken { .. } => None,
                MusixmatchAuth::ApiKey(_) => unreachable!(),
            };

            let mut params = Vec::new();
            self.base_params(&mut params, session.as_ref().map(|s| s.token.as_str()));
            params.push(("namespace", "lyrics_richsynched".to_string()));
            params.push(("optional_calls", "track.richsync".to_string()));
            params.push(("subtitle_format", "lrc".to_string()));
            self.append_query_params(query, include_album_and_duration, &mut params);

            let cookie = session.as_ref().and_then(|s| s.cookie.as_deref());
            let (outcome, set_cookie) = self
                .call_method_raw("macro.subtitles.get", &params, cookie, ctx)
                .await?;

            if let Some(c) = set_cookie {
                let mut guard = self.cached_session.lock().await;
                if let Some(ref mut sess) = *guard {
                    sess.cookie = Some(c);
                }
            }

            match outcome {
                MethodOutcome::Ok(body) => {
                    let candidates = self.extract_macro_candidates(&body);
                    if !candidates.is_empty() {
                        return Ok(candidates);
                    }

                    // Inspect macro subcalls for errors
                    if let Some(err_status) = check_macro_subcalls(&body) {
                        match err_status {
                            MacroSubcallStatus::RateLimited => {
                                return Err(FetchError::RateLimited { retry_after: None });
                            }
                            MacroSubcallStatus::RenewToken => {
                                if matches!(self.auth, MusixmatchAuth::Anonymous { .. }) {
                                    failed_token = session.map(|s| s.token);
                                    continue;
                                }
                            }
                            MacroSubcallStatus::Other(code, hint) => {
                                return Err(FetchError::InvalidResponse {
                                    status: Some(code as u16),
                                    message: format!(
                                        "Musixmatch macro sub-call returned status_code {code}{}",
                                        hint.map(|h| format!(" (hint: {h})")).unwrap_or_default()
                                    ),
                                });
                            }
                        }
                    }

                    return Ok(Vec::new());
                }
                MethodOutcome::NotFound => return Ok(Vec::new()),
                MethodOutcome::RenewToken { hint } => {
                    if hint.as_deref() == Some("renew")
                        && matches!(self.auth, MusixmatchAuth::Anonymous { .. })
                    {
                        failed_token = session.map(|s| s.token);
                        continue;
                    }
                    return Err(FetchError::InvalidResponse {
                        status: Some(401),
                        message: format!(
                            "Musixmatch rejected request for macro.subtitles.get{}",
                            hint.map(|h| format!(" (hint: {h})")).unwrap_or_default()
                        ),
                    });
                }
            }
        }

        Ok(Vec::new())
    }

    async fn fetch_search_fallback(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let session = match &self.auth {
            MusixmatchAuth::Anonymous { .. } => {
                Some(self.ensure_anonymous_session(ctx, None).await?)
            }
            MusixmatchAuth::UserToken { .. } | MusixmatchAuth::ApiKey(_) => None,
        };
        let token = session.as_ref().map(|s| s.token.as_str());
        let cookie = session.as_ref().and_then(|s| s.cookie.as_deref());

        let mut params = Vec::new();
        self.base_params(&mut params, token);
        if !query.title.trim().is_empty() {
            params.push(("q_track", query.title.clone()));
        }
        if !query.artist.trim().is_empty() {
            params.push(("q_artist", query.artist.clone()));
        }
        params.push(("f_has_lyrics", "1".to_string()));
        params.push(("s_track_rating", "desc".to_string()));
        params.push(("page_size", "5".to_string()));

        let Some(body) = self
            .call_method("track.search", &params, cookie, ctx)
            .await?
        else {
            return Ok(Vec::new());
        };

        let Some(list) = body.get("track_list").and_then(Value::as_array) else {
            return Ok(Vec::new());
        };

        let tracks: Vec<&Value> = list
            .iter()
            .filter_map(|item| item.get("track"))
            .filter(|t| t.get("instrumental").and_then(Value::as_u64) != Some(1))
            .filter(|t| {
                t.get("has_subtitles").and_then(Value::as_u64) != Some(0)
                    || t.get("has_richsync").and_then(Value::as_u64) != Some(0)
            })
            .collect();

        for track in tracks.into_iter().take(3) {
            let cands = self
                .fetch_track_lyrics_by_id(track, token, cookie, ctx)
                .await?;
            if !cands.is_empty() {
                return Ok(cands);
            }
        }

        Ok(Vec::new())
    }

    fn extract_macro_candidates(&self, body: &Value) -> Vec<RawCandidate> {
        let Some(macro_calls) = body.get("macro_calls") else {
            return Vec::new();
        };

        let track = macro_calls.pointer("/matcher.track.get/message/body/track");
        if track
            .and_then(|t| t.get("instrumental"))
            .and_then(Value::as_u64)
            == Some(1)
        {
            return Vec::new();
        }

        let (fetched_meta, source_url) = extract_track_meta(track);
        let mut candidates = Vec::new();

        #[cfg(feature = "richsync")]
        if let Some(rich_body) = macro_calls
            .pointer("/track.richsync.get/message/body/richsync/richsync_body")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            let mut cand = RawCandidate::new(rich_body)
                .with_fetched_meta(fetched_meta.clone())
                .with_format_hint("richsync");
            if let Some(ref url) = source_url {
                cand = cand.with_source_url(url.clone());
            }
            candidates.push(cand);
        }

        #[cfg(feature = "lrc")]
        if let Some(sub_body) = macro_calls
            .pointer("/track.subtitles.get/message/body/subtitle_list/0/subtitle/subtitle_body")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            let mut cand = RawCandidate::new(sub_body)
                .with_fetched_meta(fetched_meta)
                .with_format_hint("lrc");
            if let Some(url) = source_url {
                cand = cand.with_source_url(url);
            }
            candidates.push(cand);
        }

        candidates
    }

    async fn fetch_api_key(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let mut match_params = Vec::new();
        self.base_params(&mut match_params, None);
        self.append_query_params(query, true, &mut match_params);

        let Some(track_body) = self
            .call_method("matcher.track.get", &match_params, None, ctx)
            .await?
        else {
            return Ok(Vec::new());
        };

        let Some(track) = track_body.get("track") else {
            return Ok(Vec::new());
        };
        if track.get("instrumental").and_then(Value::as_u64) == Some(1) {
            return Ok(Vec::new());
        }

        self.fetch_track_lyrics_by_id(track, None, None, ctx).await
    }

    async fn fetch_track_lyrics_by_id(
        &self,
        track: &Value,
        token: Option<&str>,
        cookie: Option<&str>,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        let commontrack_id = track
            .get("commontrack_id")
            .and_then(Value::as_u64)
            .map(|id| id.to_string());
        let track_id = track
            .get("track_id")
            .and_then(Value::as_u64)
            .map(|id| id.to_string());

        if commontrack_id.is_none() && track_id.is_none() {
            return Ok(Vec::new());
        }

        let (fetched_meta, source_url) = extract_track_meta(Some(track));
        let mut candidates = Vec::new();

        #[cfg(feature = "richsync")]
        if track.get("has_richsync").and_then(Value::as_u64) != Some(0) {
            let mut p = Vec::new();
            self.base_params(&mut p, token);
            if let Some(ref ct) = commontrack_id {
                p.push(("commontrack_id", ct.clone()));
            } else if let Some(ref tid) = track_id {
                p.push(("track_id", tid.clone()));
            }
            if let Some(body) = self
                .call_method("track.richsync.get", &p, cookie, ctx)
                .await?
            {
                if let Some(rich_body) = body
                    .pointer("/richsync/richsync_body")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                {
                    let mut cand = RawCandidate::new(rich_body)
                        .with_fetched_meta(fetched_meta.clone())
                        .with_format_hint("richsync");
                    if let Some(ref url) = source_url {
                        cand = cand.with_source_url(url.clone());
                    }
                    candidates.push(cand);
                }
            }
        }

        #[cfg(feature = "lrc")]
        if track.get("has_subtitles").and_then(Value::as_u64) != Some(0) {
            let mut p = Vec::new();
            self.base_params(&mut p, token);
            p.push(("subtitle_format", "lrc".to_string()));
            if let Some(ref ct) = commontrack_id {
                p.push(("commontrack_id", ct.clone()));
            } else if let Some(ref tid) = track_id {
                p.push(("track_id", tid.clone()));
            }
            if let Some(body) = self
                .call_method("track.subtitle.get", &p, cookie, ctx)
                .await?
            {
                if let Some(sub_body) = body
                    .pointer("/subtitle/subtitle_body")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                {
                    let mut cand = RawCandidate::new(sub_body)
                        .with_fetched_meta(fetched_meta)
                        .with_format_hint("lrc");
                    if let Some(url) = source_url {
                        cand = cand.with_source_url(url);
                    }
                    candidates.push(cand);
                }
            }
        }

        Ok(candidates)
    }
}

impl LyricsProvider for MusixmatchProvider {
    #[cfg(feature = "richsync")]
    type Format = Richsync;
    #[cfg(not(feature = "richsync"))]
    type Format = Lrc;

    fn id(&self) -> &str {
        "musixmatch"
    }

    async fn fetch(
        &self,
        query: &TrackQuery,
        ctx: &FetchContext,
    ) -> Result<Vec<RawCandidate>, FetchError> {
        match self.auth {
            MusixmatchAuth::Anonymous { .. } | MusixmatchAuth::UserToken { .. } => {
                self.fetch_macro(query, ctx).await
            }
            MusixmatchAuth::ApiKey(_) => self.fetch_api_key(query, ctx).await,
        }
    }

    fn parse(&self, raw: &RawCandidate) -> Result<Mlyr, ConversionError> {
        match raw.format_hint.as_deref() {
            #[cfg(feature = "richsync")]
            Some("richsync") => Richsync::to_mlyr(Richsync(raw.payload.clone())),
            #[cfg(not(feature = "richsync"))]
            Some("richsync") => Err(ConversionError::InvalidFormat {
                format_name: "Musixmatch",
                message: "Richsync support is disabled (enable the 'richsync' feature)".to_string(),
            }),
            #[cfg(feature = "lrc")]
            Some("lrc") => Lrc::to_mlyr(Lrc(raw.payload.clone())),
            #[cfg(not(feature = "lrc"))]
            Some("lrc") => Err(ConversionError::InvalidFormat {
                format_name: "Musixmatch",
                message: "LRC support is disabled (enable the 'lrc' feature)".to_string(),
            }),
            _ => Self::Format::to_mlyr(Self::Format::from(raw.payload.clone())),
        }
    }
}

enum MacroSubcallStatus {
    RateLimited,
    RenewToken,
    Other(u64, Option<String>),
}

fn check_macro_subcalls(body: &Value) -> Option<MacroSubcallStatus> {
    let macro_calls = body.get("macro_calls")?;
    let mut other_err = None;

    for call in [
        "matcher.track.get",
        "track.richsync.get",
        "track.subtitles.get",
    ] {
        let Some(header) = macro_calls
            .get(call)
            .and_then(|v| v.pointer("/message/header"))
        else {
            continue;
        };
        let code = header
            .get("status_code")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let hint = header
            .get("hint")
            .and_then(Value::as_str)
            .map(str::to_string);

        if code == 429 {
            return Some(MacroSubcallStatus::RateLimited);
        }
        if code == 401 && hint.as_deref() == Some("renew") {
            return Some(MacroSubcallStatus::RenewToken);
        }
        if code != 0 && code != 200 && code != 404 && other_err.is_none() {
            other_err = Some((code, hint));
        }
    }

    other_err.map(|(code, hint)| MacroSubcallStatus::Other(code, hint))
}

fn extract_cookie_header(resp: &reqwest::Response) -> Option<String> {
    let pairs: Vec<&str> = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|s| s.split(';').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if pairs.is_empty() {
        None
    } else {
        Some(pairs.join("; "))
    }
}

fn is_degenerate_token(token: &str) -> bool {
    let trimmed = token.trim();
    let Some(first) = trimmed.bytes().next() else {
        return true;
    };
    trimmed.bytes().all(|b| b == first)
}

fn extract_track_meta(track: Option<&Value>) -> (FetchedMetadata, Option<String>) {
    let Some(t) = track else {
        return (FetchedMetadata::default(), None);
    };

    let title = t
        .get("track_name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let artist = t
        .get("artist_name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let album = t
        .get("album_name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let length = t
        .get("track_length")
        .and_then(Value::as_u64)
        .filter(|&s| s > 0)
        .map(Duration::from_secs);
    let source_url = t
        .get("track_share_url")
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut sources = Vec::new();
    if let Some(id) = t.get("commontrack_id").and_then(Value::as_u64) {
        sources.push(Source::new(
            "id",
            Some("musixmatch_commontrack".to_string()),
            Some(id.to_string()),
        ));
    } else if let Some(id) = t.get("track_id").and_then(Value::as_u64) {
        sources.push(Source::new(
            "id",
            Some("musixmatch".to_string()),
            Some(id.to_string()),
        ));
    }

    (
        FetchedMetadata {
            title,
            artist,
            album,
            length,
            sources,
        },
        source_url,
    )
}
