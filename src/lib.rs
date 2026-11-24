pub mod builders;
pub mod convert;
pub mod embedding;
pub mod format;
pub mod model;
pub mod validation;

pub use builders::{IntoOffset, LineBuilder, MlyrBuilder};
pub use convert::{ConversionError, Lrc, LyricsConverter, Richsync};
pub use embedding::EmbeddingError;
pub use format::{FormatError, MlyrXml, SignedTimestamp};
pub use model::{
    Line, LineKind, Lyrics, Metadata, Mlyr, Segment, Source, Voice, VoiceId, VoiceRole,
};
pub use validation::ValidationError;
