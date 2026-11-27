#[cfg(feature = "fetch")]
mod context;
#[cfg(feature = "fetch")]
mod error;
pub mod extensions;
pub mod normalize;
#[cfg(feature = "fetch")]
mod pipeline;
#[cfg(feature = "fetch")]
pub mod providers;
pub mod scoring;
pub mod types;

#[cfg(feature = "fetch")]
pub use context::FetchContext;
#[cfg(feature = "fetch")]
pub use error::FetchError;
pub use extensions::{Key, ProviderExtensions, ScorerExtensions};
pub use normalize::{NoopNormalizer, QueryNormalizer, build_query_list};
#[cfg(feature = "fetch")]
pub use pipeline::{default_postprocess, fetch_stream, run_provider};
#[cfg(feature = "fetch")]
pub use providers::{DynLyricsProvider, LyricsProvider};
#[cfg(all(feature = "fetch", any(feature = "ttml", feature = "lrc")))]
pub use providers::{
    KEY_APPLE_MUSIC_ID, KEY_SPOTIFY_ID, KEY_TIDAL_ID, KEY_UNISON_ISRC, KEY_YOUTUBE_ID,
    UnisonProvider,
};
#[cfg(all(feature = "fetch", any(feature = "richsync", feature = "lrc")))]
pub use providers::{
    KEY_COMMONTRACK_ID, KEY_COMMONTRACK_ID_U64, KEY_ISRC, KEY_MXM_TRACK_ID, KEY_MXM_TRACK_ID_U64,
    MusixmatchAuth, MusixmatchProvider,
};
#[cfg(all(feature = "fetch", feature = "lrc"))]
pub use providers::{KEY_LRCLIB_ID_STR, KEY_LRCLIB_ID_U64, LrclibProvider};
pub use scoring::{DefaultScorer, Scorer};
pub use types::{FetchedMetadata, RawCandidate, TrackQuery};

use crate::model::Mlyr;

#[derive(Debug, Clone, PartialEq)]
pub struct ScoredCandidate {
    pub mlyr: Mlyr,
    pub score: f32,
    pub provider_id: String,
    pub source_url: Option<String>,
}
