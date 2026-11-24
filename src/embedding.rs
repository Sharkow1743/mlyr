use std::path::Path;

use lofty::config::WriteOptions;
use lofty::error::{FileEncodingError, FileParseError};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag, TagExt};

use crate::format::FormatError;
use crate::model::Mlyr;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Audio file I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse audio file metadata: {0}")]
    Parse(#[from] FileParseError),

    #[error("Failed to write audio file metadata: {0}")]
    Encode(#[from] FileEncodingError),

    #[error("MLYR XML format error: {0}")]
    Format(#[from] FormatError),

    #[error("Failed to initialize a tag for this file type")]
    TagInitializationFailed,

    #[error("No tags found in this audio file")]
    NoTagsFound,

    #[error("No lyrics field found in the audio metadata")]
    MissingLyricsField,
}

impl Mlyr {
    /// Embeds the `Mlyr` object as XML into metadata of the specified audio file
    pub fn embed_to_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), EmbeddingError> {
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
                        .ok_or(EmbeddingError::TagInitializationFailed)?
                }
            }
        };

        tag.insert_text(ItemKey::Lyrics, xml_lyrics);

        tag.save_to_path(&file_path, WriteOptions::default())?;

        Ok(())
    }

    /// Retrieves and parses an `Mlyr` object from XML stored in the metadata of the specified audio file
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, EmbeddingError> {
        let tagged_file = Probe::open(&file_path)?.read()?;

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
            .ok_or(EmbeddingError::NoTagsFound)?;

        let xml_lyrics = tag
            .get_string(ItemKey::Lyrics)
            .ok_or(EmbeddingError::MissingLyricsField)?;

        Ok(Mlyr::from_xml(xml_lyrics)?)
    }
}
