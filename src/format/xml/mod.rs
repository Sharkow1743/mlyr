mod dto;
mod serde_helpers;

use std::str::FromStr;

use crate::convert::{ConversionError, LyricsConverter};
use crate::define_lyrics_type;
use crate::format::normalize::RawMlyr;
use crate::model::Mlyr;

use super::FormatError;
use dto::{XmlMlyrDe, XmlMlyrSer};

define_lyrics_type!(
    /// Canonical MLYR XML representation
    pub struct MlyrXml;
);

impl Mlyr {
    /// Parses a MLYR XML document into a validated [`Mlyr`] model
    pub fn from_xml(xml: &str) -> Result<Self, FormatError> {
        let raw_de: XmlMlyrDe =
            quick_xml::de::from_str(xml).map_err(|e| FormatError::XmlDeserialize {
                source: Box::new(e),
            })?;
        let raw = RawMlyr::from(raw_de);
        Ok(Mlyr::try_from(raw)?)
    }

    /// Serializes this [`Mlyr`] model into compact MLYR XML
    pub fn to_xml(&self) -> Result<String, FormatError> {
        self.validate()?;
        let dto = XmlMlyrSer::from(self);
        quick_xml::se::to_string(&dto).map_err(|e| FormatError::XmlSerialize {
            source: Box::new(e),
        })
    }

    /// Serializes this [`Mlyr`] model into formatted MLYR XML
    pub fn to_xml_formatted(&self) -> Result<String, FormatError> {
        self.validate()?;
        let dto = XmlMlyrSer::from(self);

        let mut buffer = Vec::new();
        let mut writer = quick_xml::Writer::new_with_indent(&mut buffer, b'\t', 1);

        writer
            .write_serializable("mlyr", &dto)
            .map_err(|e| FormatError::XmlSerialize {
                source: Box::new(e),
            })?;

        String::from_utf8(buffer).map_err(|e| FormatError::XmlSerialize {
            source: Box::new(e),
        })
    }
}

impl FromStr for Mlyr {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_xml(s)
    }
}

impl LyricsConverter for MlyrXml {
    fn to_mlyr(xml: Self) -> Result<Mlyr, ConversionError> {
        Ok(Mlyr::from_xml(&xml.0)?)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        Ok(MlyrXml(mlyr.to_xml()?))
    }

    fn from_mlyr_formatted(mlyr: Mlyr) -> Result<Self, ConversionError> {
        Ok(MlyrXml(mlyr.to_xml_formatted()?))
    }
}
