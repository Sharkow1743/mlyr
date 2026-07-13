pub mod lrc;
pub mod richsync;

use super::definition::*;

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

impl Mlyr {
    /// Deserializes a MLYR payload from an XML string
    pub fn from_xml(xml_str: &str) -> Result<Self, quick_xml::de::DeError> {
        quick_xml::de::from_str(xml_str)
    }

    /// Serializes the struct back into an XML string with indentation
    pub fn to_xml_formatted(&self) -> Result<String, quick_xml::se::SeError> {
        let mut buffer = Vec::new();
        // Create an XML writer configured with 2-space indentation
        let mut writer = quick_xml::Writer::new_with_indent(&mut buffer, b'\t', 1);

        // Serialize the root "mlyr" element using the indentation settings
        writer.write_serializable("mlyr", self)?;

        let xml_str = String::from_utf8(buffer)
            .map_err(|e| <quick_xml::se::SeError as serde::ser::Error>::custom(e.to_string()))?;
        Ok(xml_str)
    }

    pub fn to_xml(&self) -> Result<String, quick_xml::se::SeError> {
        quick_xml::se::to_string(self)
    }
}
