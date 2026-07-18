mod lrc;
mod richsync;
mod xml;

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
