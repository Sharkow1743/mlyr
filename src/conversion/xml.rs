use super::super::definition::*;

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
