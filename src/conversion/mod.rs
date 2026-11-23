mod lrc;
mod richsync;
mod xml;

use std::time::Duration;

pub use lrc::Lrc;
pub use richsync::Richsync;
pub use xml::Xml;

use crate::{Mlyr, ValidationError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Parsing error for {format_name} format: {source}")]
    Parsing {
        format_name: &'static str,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid {format_name} format: {message}")]
    InvalidFormat {
        format_name: &'static str,
        message: String,
    },
}

pub mod duration_seconds_opt {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(val: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match val {
            Some(dur) => serializer.serialize_some(&dur.as_secs_f64()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<f64> = Option::deserialize(deserializer)?;
        match s {
            Some(secs) => {
                if !secs.is_finite() || secs < 0.0 {
                    Err(serde::de::Error::custom(
                        "Non-finite or negative durations are invalid",
                    ))
                } else {
                    Ok(Some(Duration::from_secs_f64(secs)))
                }
            }
            None => Ok(None),
        }
    }
}

/// Trait for conversion between external lyrics formats and the [`Mlyr`] IR
pub trait LyricsConverter<E = ConversionError>:
    From<String> + Into<String> + std::fmt::Display + Sized
{
    /// Converts this format to the [`Mlyr`] IR
    fn to_mlyr(lyrics: Self) -> Result<Mlyr, E>;

    /// Converts [`Mlyr`] IR to this format (minimized)
    fn from_mlyr(lyrics: Mlyr) -> Result<Self, E>;

    /// Converts [`Mlyr`] IR to this format with formatting
    /// Defaults to `from_mlyr`
    fn from_mlyr_formatted(lyrics: Mlyr) -> Result<Self, E> {
        Self::from_mlyr(lyrics)
    }

    /// Converts directly from this format into any other [`LyricsConverter`] via [`Mlyr`]
    fn convert_to<T: LyricsConverter<E>>(self) -> Result<T, E> {
        let mlyr = Self::to_mlyr(self)?;
        T::from_mlyr(mlyr)
    }

    /// Converts directly from this format into any formatted [`LyricsConverter`] via [`Mlyr`]
    fn convert_to_formatted<T: LyricsConverter<E>>(self) -> Result<T, E> {
        let mlyr = Self::to_mlyr(self)?;
        T::from_mlyr_formatted(mlyr)
    }
}

/// Generates a [`String`] wrapper implementing [`Display`](std::fmt::Display), [`AsRef<str>`], and [`From`] conversions
#[macro_export]
macro_rules! define_lyrics_type {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident;
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        $vis struct $name(pub String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

pub trait IntoOffset {
    fn into_offset(self) -> Option<Duration>;
}

impl IntoOffset for Duration {
    fn into_offset(self) -> Option<Duration> {
        Some(self)
    }
}

impl IntoOffset for f64 {
    fn into_offset(self) -> Option<Duration> {
        if self.is_finite() && self >= 0.0 {
            Some(Duration::from_secs_f64(self))
        } else {
            None
        }
    }
}

impl IntoOffset for f32 {
    fn into_offset(self) -> Option<Duration> {
        if self.is_finite() && self >= 0.0 {
            Some(Duration::from_secs_f64(self as f64))
        } else {
            None
        }
    }
}

impl IntoOffset for u64 {
    fn into_offset(self) -> Option<Duration> {
        Some(Duration::from_secs(self))
    }
}

impl IntoOffset for u32 {
    fn into_offset(self) -> Option<Duration> {
        Some(Duration::from_secs(self as u64))
    }
}

impl<T: IntoOffset> IntoOffset for Option<T> {
    fn into_offset(self) -> Option<Duration> {
        self.and_then(|v| v.into_offset())
    }
}

impl IntoOffset for () {
    fn into_offset(self) -> Option<Duration> {
        None
    }
}

pub(crate) fn is_zero_u8(val: &u8) -> bool {
    *val == 0
}
