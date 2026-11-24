use serde::{Deserialize, Deserializer};

use crate::format::normalize::RawVoice;

pub(crate) fn is_zero_u8(val: &u8) -> bool {
    *val == 0
}

pub(crate) mod duration_seconds {
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
        if !secs.is_finite() || secs < 0.0 {
            Err(serde::de::Error::custom(
                "Non-finite or negative durations are invalid",
            ))
        } else {
            Ok(Duration::from_secs_f64(secs))
        }
    }
}

pub(crate) mod duration_seconds_opt {
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

pub(crate) struct XmlVoiceDe(pub(crate) RawVoice);

impl<'de> Deserialize<'de> for XmlVoiceDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VoiceVisitor;
        impl<'de> serde::de::Visitor<'de> for VoiceVisitor {
            type Value = XmlVoiceDe;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("integer or string")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(XmlVoiceDe(RawVoice::Int(v)))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                if v >= 0 {
                    Ok(XmlVoiceDe(RawVoice::Int(v as u64)))
                } else {
                    Ok(XmlVoiceDe(RawVoice::Str(v.to_string())))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Ok(num) = v.parse::<u64>() {
                    Ok(XmlVoiceDe(RawVoice::Int(num)))
                } else {
                    Ok(XmlVoiceDe(RawVoice::Str(v.to_string())))
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Ok(num) = v.parse::<u64>() {
                    Ok(XmlVoiceDe(RawVoice::Int(num)))
                } else {
                    Ok(XmlVoiceDe(RawVoice::Str(v)))
                }
            }
        }
        deserializer.deserialize_any(VoiceVisitor)
    }
}
