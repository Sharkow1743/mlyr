use std::time::Duration;

use thiserror::Error;

use crate::convert::ConversionError;
use crate::validation::ValidationError;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error(
        "Rate limited by provider{}",
        .retry_after
            .map(|d| format!(" (retry after {}s)", d.as_secs()))
            .unwrap_or_default()
    )]
    RateLimited { retry_after: Option<Duration> },

    #[error("Invalid response from provider (status {status:?}): {message}")]
    InvalidResponse {
        status: Option<u16>,
        message: String,
    },

    #[error("Failed to parse lyrics: {0}")]
    Parse(#[from] ConversionError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
}
