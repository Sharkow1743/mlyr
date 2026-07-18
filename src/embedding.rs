use super::definition::*;
use std::path::Path;

// Import lofty metadata capabilities
use lofty::config::WriteOptions;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag, TagExt};

impl Mlyr {
    /// Embeds the `Mlyr` object as XML into metadata of the specified audio file
    pub fn embed_to_file<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let xml_lyrics = self.to_xml()?;

        let mut tagged_file = Probe::open(&file_path)?.read()?;

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

        tag.insert_text(ItemKey::Comment, xml_lyrics);

        tag.save_to_path(&file_path, WriteOptions::default())?;

        Ok(())
    }

    /// Retrieves and converts to `Mlyr` object XML from metadata of the specified auio file
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let tagged_file = Probe::open(&file_path)?.read()?;

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
            .ok_or("No tags found in this audio file")?;

        let xml_lyrics = tag
            .get_string(ItemKey::Comment)
            .ok_or("No comment field found in the audio metadata")?;

        let mlyr = Self::from_xml(xml_lyrics)?;

        Ok(mlyr)
    }
}
