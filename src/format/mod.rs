pub(crate) mod normalize;
pub mod xml;

pub use xml::MlyrXml;

use crate::validation::ValidationError;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Failed to parse MLYR XML: {source}")]
    XmlDeserialize {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to serialize MLYR XML: {source}")]
    XmlSerialize {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
