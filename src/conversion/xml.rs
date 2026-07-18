use super::super::definition::*;
use super::ConversionError;

impl Mlyr {
    /// Creates `Mlyr` object from XML string
    pub fn from_xml(xml_str: &str) -> Result<Self, ConversionError> {
        let mlyr: Self = quick_xml::de::from_str(xml_str)?;
        mlyr.validate()?;
        Ok(mlyr)
    }

    /// Creates XML string with indentation from `Mlyr` object
    pub fn to_xml_formatted(&self) -> Result<String, ConversionError> {
        self.validate()?;

        let mut buffer = Vec::new();
        let mut writer = quick_xml::Writer::new_with_indent(&mut buffer, b'\t', 1);

        writer.write_serializable("mlyr", self)?;

        let xml_str = String::from_utf8(buffer)
            .map_err(|e| <quick_xml::se::SeError as serde::ser::Error>::custom(e.to_string()))?;
        Ok(xml_str)
    }

    /// Creates XML string from `Mlyr` object
    pub fn to_xml(&self) -> Result<String, ConversionError> {
        self.validate()?;
        Ok(quick_xml::se::to_string(self)?)
    }
}
