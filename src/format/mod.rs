pub mod inline_text;
pub(crate) mod normalize;
pub mod timestamp;
#[cfg(feature = "xml")]
pub mod xml;

pub use timestamp::SignedTimestamp;
#[cfg(feature = "xml")]
pub use xml::MlyrXml;

use crate::validation::ValidationError;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Invalid timestamp '{input}': {reason}")]
    InvalidTimestamp { input: String, reason: &'static str },

    #[error("Invalid inline synced text: {reason}")]
    InvalidInlineText { reason: String },

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
