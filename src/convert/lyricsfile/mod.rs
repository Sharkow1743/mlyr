mod parse;
mod schema;
mod serialize;

use crate::model::Mlyr;
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

define_lyrics_type!(
    /// Open YAML-based Lyricsfile 1.0 format (`.lyricsfile.yaml`).
    pub struct Lyricsfile;
);

impl LyricsConverter for Lyricsfile {
    fn to_mlyr(lyricsfile: Self) -> Result<Mlyr, ConversionError> {
        parse::parse_lyricsfile(&lyricsfile.0)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        serialize::serialize_lyricsfile(&mlyr).map(Lyricsfile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_lyricsfile_word_sync() {
        let yaml = "\
version: '1.0'
metadata:
  title: 'Song Title'
  artist: 'Artist Name'
  duration_ms: 245000
lines:
  - text: 'Hello world'
    start_ms: 12000
    end_ms: 15500
    words:
      - text: 'Hello '
        start_ms: 12000
        end_ms: 13500
      - text: 'world'
        start_ms: 13500
        end_ms: 15500
";
        let mlyr = Lyricsfile::to_mlyr(Lyricsfile(yaml.to_string())).unwrap();
        assert_eq!(mlyr.lyrics().lines[0].segments.len(), 2);
        let exported = Lyricsfile::from_mlyr(mlyr.clone()).unwrap();
        let reimported = Lyricsfile::to_mlyr(exported).unwrap();
        assert_eq!(mlyr, reimported);
    }
}
