mod lrc;
mod richsync;
mod xml;

use std::time::Duration;

use super::ValidationError;
use crate::Mlyr;

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("LRC parse error: {0}")]
    Lrc(String),
    #[error("JSON processing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("XML Deserialization error: {0}")]
    XmlDeserialization(#[from] quick_xml::de::DeError),
    #[error("XML Serialization error: {0}")]
    XmlSerialization(#[from] quick_xml::se::SeError),
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
