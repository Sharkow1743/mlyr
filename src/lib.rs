pub mod builders;
#[cfg(feature = "convert")]
pub mod convert;
#[cfg(feature = "embedding")]
pub mod embedding;
pub mod format;
pub mod model;
pub mod validation;

pub use builders::{IntoOffset, LineBuilder, MlyrBuilder};
#[cfg(feature = "lrc")]
pub use convert::Lrc;
#[cfg(feature = "richsync")]
pub use convert::Richsync;
#[cfg(feature = "convert")]
pub use convert::{ConversionError, LyricsConverter};
#[cfg(feature = "embedding")]
pub use embedding::EmbeddingError;
#[cfg(feature = "xml")]
pub use format::MlyrXml;
pub use format::{FormatError, SignedTimestamp};
pub use model::{
    Line, LineKind, Lyrics, Metadata, Mlyr, Segment, Source, Voice, VoiceId, VoiceRole,
};
pub use validation::ValidationError;
