mod common;
mod lines;
mod quick_export;
mod syllable;

pub use lines::LyricifyLines;
pub use quick_export::LyricifyQuickExport;
pub use syllable::LyricifySyllable;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LyricsConverter;

    #[test]
    fn roundtrips_lys_and_lqe() {
        let lys = "\
[type:LyricifySyllable]
[ti:Example]
[4]Hel(1000,300)lo (1300,400)world(1700,500)
[7](oooh)(1800,900)
";
        let mlyr = LyricifySyllable::to_mlyr(LyricifySyllable(lys.to_string())).unwrap();
        assert_eq!(mlyr.lyrics().lines.len(), 2);
        assert!(mlyr.lyrics().lines[1].is_bg());

        let lqe = LyricifyQuickExport::from_mlyr(mlyr.clone()).unwrap();
        let back = LyricifyQuickExport::to_mlyr(lqe).unwrap();
        assert_eq!(mlyr, back);
    }
}
