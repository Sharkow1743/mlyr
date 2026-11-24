#[cfg(feature = "lrc")]
mod lrc;
#[cfg(feature = "richsync")]
mod richsync;

#[cfg(feature = "lrc")]
pub use lrc::Lrc;
#[cfg(feature = "richsync")]
pub use richsync::Richsync;

use thiserror::Error;

use crate::format::FormatError;
use crate::model::Mlyr;
use crate::validation::ValidationError;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Format error: {0}")]
    Format(#[from] FormatError),

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

/// Trait for conversion between external lyrics formats and the [`Mlyr`] model
pub trait LyricsConverter<E = ConversionError>:
    From<String> + Into<String> + std::fmt::Display + Sized
{
    /// Converts this format to the [`Mlyr`] model
    fn to_mlyr(lyrics: Self) -> Result<Mlyr, E>;

    /// Converts [`Mlyr`] model to this format (minimized, if format supports)
    fn from_mlyr(lyrics: Mlyr) -> Result<Self, E>;

    /// Converts [`Mlyr`] model to this format with formatting
    ///
    /// Defaults to `from_mlyr`
    fn from_mlyr_formatted(lyrics: Mlyr) -> Result<Self, E> {
        Self::from_mlyr(lyrics)
    }

    /// Converts directly from this format into any other [`LyricsConverter`] via [`Mlyr`] model
    fn convert_to<T: LyricsConverter<E>>(self) -> Result<T, E> {
        let mlyr = Self::to_mlyr(self)?;
        T::from_mlyr(mlyr)
    }

    /// Converts directly from this format into any formatted [`LyricsConverter`] via [`Mlyr`] model
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
