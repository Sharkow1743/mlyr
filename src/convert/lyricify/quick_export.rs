#[cfg(feature = "lrc")]
use crate::convert::Lrc;
use crate::model::Mlyr;
use crate::{ConversionError, LyricsConverter, define_lyrics_type};

use super::lines::LyricifyLines;
use super::syllable::LyricifySyllable;

define_lyrics_type!(
    /// Lyricify Quick Export container format (`.lqe`).
    pub struct LyricifyQuickExport;
);

impl LyricsConverter for LyricifyQuickExport {
    fn to_mlyr(lqe: Self) -> Result<Mlyr, ConversionError> {
        let content = lqe.0.strip_prefix('\u{FEFF}').unwrap_or(&lqe.0);

        let mut saw_header = false;
        let mut format_tag: Option<String> = None;
        let mut lyrics_payload = String::new();
        let mut in_lyrics = false;

        for raw in content.lines() {
            let trimmed = raw.trim();

            if !saw_header && trimmed.is_empty() {
                continue;
            }
            if trimmed.eq_ignore_ascii_case("[Lyricify Quick Export]") {
                saw_header = true;
                continue;
            }
            if trimmed.to_ascii_lowercase().starts_with("[version:") && trimmed.ends_with(']') {
                continue;
            }

            if let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if let Some((sec_name, sec_val)) = inner.split_once(':') {
                    match sec_name.trim().to_ascii_lowercase().as_str() {
                        "lyrics" => {
                            in_lyrics = true;
                            let val = sec_val.trim();
                            let fmt = val.split('@').next().unwrap_or(val).trim().to_string();
                            format_tag = Some(fmt);
                            continue;
                        }
                        "translation" | "pronunciation" => {
                            in_lyrics = false;
                            continue;
                        }
                        _ => {}
                    }
                }
            }

            if in_lyrics {
                lyrics_payload.push_str(raw);
                lyrics_payload.push('\n');
            }
        }

        let fmt = format_tag.ok_or_else(|| ConversionError::InvalidFormat {
            format_name: "LyricifyQuickExport",
            message: "Missing '[lyrics: ...]' section in Lyricify Quick Export".to_string(),
        })?;

        let fmt_lower = fmt.to_ascii_lowercase();
        if fmt_lower.contains("syllable") {
            LyricifySyllable::to_mlyr(LyricifySyllable(lyrics_payload))
        } else if fmt_lower.contains("lines") {
            LyricifyLines::to_mlyr(LyricifyLines(lyrics_payload))
        } else if fmt_lower.contains("lrc") {
            #[cfg(feature = "lrc")]
            {
                Lrc::to_mlyr(Lrc(lyrics_payload))
            }
            #[cfg(not(feature = "lrc"))]
            {
                Err(ConversionError::InvalidFormat {
                    format_name: "LyricifyQuickExport",
                    message: "LRC support is disabled (enable the 'lrc' feature)".to_string(),
                })
            }
        } else {
            Err(ConversionError::InvalidFormat {
                format_name: "LyricifyQuickExport",
                message: format!("Unsupported LQE embedded lyrics format '{fmt}'"),
            })
        }
    }

    fn from_mlyr(mlyr: Mlyr) -> Result<Self, ConversionError> {
        let lys = LyricifySyllable::from_mlyr(mlyr)?;
        let mut out =
            String::from("[Lyricify Quick Export]\n[version:1.0]\n[lyrics: Lyricify Syllable@]\n");
        out.push_str(&lys.0);
        Ok(LyricifyQuickExport(out))
    }
}
