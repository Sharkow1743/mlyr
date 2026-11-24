mod inline;
mod parse;
mod serialize;
mod timestamp;

use crate::model::Mlyr;
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

define_lyrics_type!(
    /// Standard LRC, Enhanced LRC (eLRC / LRC A2), ESLyric, Salt Player Lyrics, Apple Music/Musixmatch eLRC, Walaoke LRC
    pub struct Lrc;
);

impl LyricsConverter for Lrc {
    fn to_mlyr(lrc: Self) -> Result<Mlyr, ConversionError> {
        parse::parse_lrc(&lrc.0)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        serialize::serialize_lrc(&mlyr).map(Lrc)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn parses_spl_eslyric_elrc_and_metadata() {
        let input = "\
// Comment
[ti:Test Song]
[ar:Tester]
[offset:500]
[awlrc:ignored]
[00:01.00][00:05.00]Repeated line
[00:10.00]v1:<00:10.00>Hello <00:10.50>world<00:11.00> [bg:<00:10.80>echo<00:11.50>]
[00:12.00]你[00:12.30]好[00:12.60]
";
        let mlyr = Lrc::to_mlyr(Lrc(input.to_string())).unwrap();
        assert_eq!(mlyr.metadata().unwrap().title.as_deref(), Some("Test Song"));
        assert_eq!(mlyr.lyrics().lines.len(), 5);
        assert_eq!(mlyr.lyrics().lines[0].offset, Duration::from_millis(1500));
        assert!(mlyr.lyrics().lines.iter().any(|l| l.is_bg()));
    }
}
