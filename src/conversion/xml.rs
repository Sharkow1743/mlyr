use anyhow::Error;

use crate::define_lyrics_type;

use super::super::definition::*;
use super::ConversionError;

define_lyrics_type!(
    pub struct Xml(String);

    /// Mlyr XML format converter
    impl LyricsConverter {
        fn to_mlyr(xml: Self) -> Result<Mlyr, ConversionError> {
            let mlyr: Mlyr =
                quick_xml::de::from_str(&xml.0).map_err(|e| ConversionError::Parsing {
                    format_name: "Xml",
                    source: Error::from(e),
                })?;
            Ok(mlyr)
        }

        fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
            mlyr.validate()?;
            Ok(Xml(quick_xml::se::to_string(&mlyr).map_err(|e| {
                ConversionError::Parsing {
                    format_name: "Xml",
                    source: Error::from(e),
                }
            })?))
        }

        fn from_mlyr_formatted(mlyr: Mlyr) -> Result<Self, ConversionError> {
            mlyr.validate()?;

            let mut buffer = Vec::new();
            let mut writer = quick_xml::Writer::new_with_indent(&mut buffer, b'\t', 1);

            writer
                .write_serializable("mlyr", &mlyr)
                .map_err(|e| ConversionError::Parsing {
                    format_name: "Xml",
                    source: Error::from(e),
                })?;

            let xml_str = String::from_utf8(buffer).map_err(|e| ConversionError::Parsing {
                format_name: "Xml",
                source: Error::from(e),
            })?;

            Ok(Xml(xml_str))
        }
    }
);
