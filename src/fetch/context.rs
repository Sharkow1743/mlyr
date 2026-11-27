use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT};

const DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// Shared HTTP client configuration passed to [`LyricsProvider::fetch`](super::LyricsProvider::fetch).
#[derive(Debug, Clone)]
pub struct FetchContext {
    client: reqwest::Client,
    user_agent: String,
    headers: HeaderMap,
    timeout: Duration,
}

impl Default for FetchContext {
    fn default() -> Self {
        Self::new()
    }
}

impl FetchContext {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            headers: HeaderMap::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn request(
        &self,
        method: reqwest::Method,
        url: impl reqwest::IntoUrl,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .header(USER_AGENT, &self.user_agent)
            .headers(self.headers.clone())
            .timeout(self.timeout)
    }

    pub fn get(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.request(reqwest::Method::GET, url)
    }
}
