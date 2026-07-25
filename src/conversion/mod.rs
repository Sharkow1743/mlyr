mod lrc;
mod richsync;
mod xml;

pub use lrc::Lrc;
pub use richsync::Richsync;
pub use xml::Xml;

use std::time::Duration;

use super::ValidationError;
use crate::Mlyr;

use anyhow::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Parsing error for {format_name} format: {source}")]
    Parsing {
        format_name: &'static str,
        #[source]
        source: Error,
    },

    #[error("Unexpected error: {0}")]
    Other(#[from] Error),
}

pub mod duration_seconds {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(val: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(val.as_secs_f64())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = f64::deserialize(deserializer)?;
        if secs < 0.0 {
            Err(serde::de::Error::custom("Negative durations are invalid"))
        } else {
            Ok(Duration::from_secs_f64(secs))
        }
    }
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
                if secs < 0.0 {
                    Err(serde::de::Error::custom("Negative durations are invalid"))
                } else {
                    Ok(Some(Duration::from_secs_f64(secs)))
                }
            }
            None => Ok(None),
        }
    }
}

/// Trait for conversion between external lyrics formats and Mlyr format
pub trait LyricsConverter<E = ConversionError>:
    From<String> + Into<String> + std::fmt::Display
{
    /// Converts this fromat to the Mlyr format
    fn to_mlyr(lyrics: Self) -> Result<Mlyr, E>;
    /// Converts Mlyr format to this format
    /// Creates minimized version
    fn from_mlyr(lyrics: Mlyr) -> Result<Self, E>;
    /// Converts Mlyr format to this format
    /// Creates version with formatting, like indentation, comments, etc
    /// Defaults to output of `from_mlyr` method
    fn from_mlyr_formatted(lyrics: Mlyr) -> Result<Self, E> {
        Self::from_mlyr(lyrics)
    }
}

/// Macro to create a new lyrics type with LyricsConverter implementation
#[macro_export]
macro_rules! define_lyrics_type {
    (
        $vis:vis struct $name:ident($inner_type:ty);
        $(#[$meta:meta])*
        impl LyricsConverter {
            fn to_mlyr($param:ident: Self) -> Result<Mlyr, $error:ty> $to_impl:block
            fn from_mlyr($param2:ident: Mlyr) -> Result<Self, $error2:ty> $from_impl:block
            $(fn from_mlyr_formatted($param3:ident: Mlyr) -> Result<Self, $error3:ty> $formatted_impl:block)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name($inner_type);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl Into<String> for $name {
            fn into(self) -> String {
                self.0
            }
        }

        impl $crate::LyricsConverter<$error> for $name {
            fn to_mlyr($param: Self) -> Result<Mlyr, $error> {
                $to_impl
            }

            fn from_mlyr($param2: Mlyr) -> Result<Self, $error2> {
                $from_impl
            }

            $(
            fn from_mlyr_formatted($param3: Mlyr) -> Result<Self, $error3> {
                $formatted_impl
            }
            )?
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
        if self >= 0.0 {
            Some(Duration::from_secs_f64(self))
        } else {
            None
        }
    }
}

impl IntoOffset for f32 {
    fn into_offset(self) -> Option<Duration> {
        if self >= 0.0 {
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
