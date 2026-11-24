mod parse;
mod serialize;
mod time;
mod xml_dom;

use crate::model::Mlyr;
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

define_lyrics_type!(
    /// W3C TTML, Apple Music TTML, boidu / BetterLyrics TTML, and AMLL TTML.
    pub struct Ttml;
);

impl LyricsConverter for Ttml {
    fn to_mlyr(ttml: Self) -> Result<Mlyr, ConversionError> {
        parse::parse_ttml(&ttml.0)
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        serialize::serialize_ttml(&mlyr, false).map(Ttml)
    }

    fn from_mlyr_formatted(mlyr: Mlyr) -> Result<Self, ConversionError> {
        serialize::serialize_ttml(&mlyr, true).map(Ttml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_apple_music_and_boidu_ttml_with_bg_and_spaces() {
        let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" xmlns:amll="http://www.example.com/ns/amll" itunes:timing="Word">
  <head>
    <metadata>
      <ttm:agent type="person" xml:id="v1"><ttm:name>Singer A</ttm:name></ttm:agent>
      <ttm:agent type="person" xml:id="v2"/>
      <amll:meta key="musicName" value="My Song"/>
      <amll:meta key="artists" value="Artist 1"/>
      <amll:meta key="artists" value="Artist 2"/>
    </metadata>
  </head>
  <body dur="03:00.000">
    <div begin="00:01.000" end="00:10.000">
      <p begin="00:01.000" end="00:06.000" ttm:agent="v1">
        <span begin="00:01.000" end="00:01.500">Hello</span> <span begin="00:01.500" end="00:02.500">world</span>
        <span ttm:role="x-bg" begin="00:02.500" end="00:04.000"><span begin="00:02.500" end="00:03.200">(echo)</span></span>
        <span ttm:role="x-translation">你好世界</span>
      </p>
      <p begin="00:03.000" end="00:07.000" ttm:agent="v2">
        <span begin="00:03.000" end="00:04.000">Next</span>
        <span ttm:role="lyric:background" begin="00:04.500" end="00:05.500">(bg2)</span>
      </p>
    </div>
  </body>
</tt>"#;

        let mlyr = Ttml::to_mlyr(Ttml(xml.to_string())).unwrap();
        let meta = mlyr.metadata().unwrap();
        assert_eq!(meta.title.as_deref(), Some("My Song"));
        assert_eq!(meta.artist.as_deref(), Some("Artist 1, Artist 2"));
        assert_eq!(meta.voices.len(), 2);
        assert_eq!(mlyr.lyrics().lines.len(), 4);
        assert_eq!(mlyr.lyrics().lines[0].segments[0].text, "Hello ");
        assert_eq!(mlyr.lyrics().lines[0].segments[1].text, "world");

        let exported = Ttml::from_mlyr_formatted(mlyr).unwrap();
        let roundtrip = Ttml::to_mlyr(exported).unwrap();
        assert_eq!(roundtrip.lyrics().lines.len(), 4);
    }
}
