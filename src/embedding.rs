use super::definition::*;
use std::path::Path;

// Import lofty metadata capabilities
use lofty::config::WriteOptions;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag, TagExt};

impl Mlyr {
    /// Embeds the XML representation of this `Mlyr` instance directly into the metadata
    /// of the specified audio file inside the standard `Comment` field.
    pub fn embed_to_file<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Serialize this Mlyr instance into its formatted XML lyrics string
        let xml_lyrics = self.to_xml()?;

        // 2. Open the audio file and read existing tags
        let mut tagged_file = Probe::open(&file_path)?.read()?;

        // 3. Determine or initialize the appropriate tag format for the container
        let tag = match tagged_file.primary_tag_mut() {
            Some(primary) => primary,
            None => {
                if let Some(first) = tagged_file.first_tag_mut() {
                    first
                } else {
                    let tag_type = tagged_file.primary_tag_type();
                    tagged_file.insert_tag(Tag::new(tag_type));
                    tagged_file
                        .primary_tag_mut()
                        .ok_or("Failed to initialize a tag for this file type")?
                }
            }
        };

        // 4. Save the XML structure to the standard Comment metadata field
        tag.insert_text(ItemKey::Comment, xml_lyrics);

        // 5. Save the modified metadata back to the target file path
        tag.save_to_path(&file_path, WriteOptions::default())?;

        Ok(())
    }

    /// Extracts and deserializes an `Mlyr` instance from the standard `Comment` field
    /// of the specified audio file.
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Open the audio file and read existing tags
        let tagged_file = Probe::open(&file_path)?.read()?;

        // 2. Retrieve the primary tag or fall back to the first available tag
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
            .ok_or("No tags found in this audio file")?;

        // 3. Retrieve the string from the Comment metadata field
        let xml_lyrics = tag
            .get_string(ItemKey::Comment)
            .ok_or("No comment field found in the audio metadata")?;

        // 4. Deserialize the XML content back into an Mlyr instance
        let mlyr = Self::from_xml(xml_lyrics)?;

        Ok(mlyr)
    }
}
