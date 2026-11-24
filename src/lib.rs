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
#[cfg(feature = "lyricsfile")]
pub use convert::Lyricsfile;
#[cfg(feature = "richsync")]
pub use convert::Richsync;
#[cfg(feature = "ttml")]
pub use convert::Ttml;
#[cfg(feature = "convert")]
pub use convert::{ConversionError, LyricsConverter};
#[cfg(feature = "lyricify")]
pub use convert::{LyricifyLines, LyricifyQuickExport, LyricifySyllable};
#[cfg(feature = "embedding")]
pub use embedding::EmbeddingError;
#[cfg(feature = "xml")]
pub use format::MlyrXml;
pub use format::{FormatError, SignedTimestamp};
pub use model::{
    Line, LineKind, Lyrics, Metadata, Mlyr, Segment, Source, Voice, VoiceId, VoiceRole,
};
pub use validation::ValidationError;
