use super::super::definition::*;
use std::time::Duration;

// Support structures mapping directly to Musixmatch's Richsync JSON specification
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RichSyncLine {
    /// Line start time in seconds
    pub ts: f64,
    /// Line end time in seconds
    pub te: f64,
    /// List of word/character timings
    pub l: Vec<RichSyncWord>,
    /// Complete line text
    pub x: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RichSyncWord {
    /// Character or word text
    pub c: String,
    /// Time offset in seconds relative to line start time (ts)
    pub o: f64,
}

impl Mlyr {
    /// Creates an `Mlyr` instance from Musixmatch's proprietary Richsync JSON format.
    pub fn from_richsync(json_str: &str) -> Result<Self, serde_json::Error> {
        let mxm_lines: Vec<RichSyncLine> = serde_json::from_str(json_str)?;
        let mut lines = Vec::new();

        for mxm_line in mxm_lines {
            let offset = Some(Duration::from_secs_f64(mxm_line.ts));
            let segments: Vec<Segment> = mxm_line
                .l
                .into_iter()
                .map(|word| Segment {
                    offset: Some(Duration::from_secs_f64(word.o)),
                    text: word.c,
                })
                .collect();

            lines.push(Line {
                offset,
                voice: None,
                segments,
                parallel: None,
            });
        }

        Ok(Mlyr {
            metadata: None,
            lyrics: Lyrics { lines },
        })
    }

    /// Converts this `Mlyr` instance to Musixmatch's proprietary Richsync JSON format.
    pub fn to_richsync(&self) -> Result<String, serde_json::Error> {
        let mut mxm_lines = Vec::new();

        for line in &self.lyrics.lines {
            let ts = line.offset.unwrap_or(Duration::ZERO).as_secs_f64();
            let last_offset_sec = line
                .segments
                .last()
                .and_then(|s| s.offset)
                .unwrap_or(Duration::ZERO)
                .as_secs_f64();
            let te = ts + last_offset_sec;

            let mut word_list = Vec::new();
            let mut full_text = String::new();

            for seg in &line.segments {
                let offset_sec = seg.offset.unwrap_or(Duration::ZERO).as_secs_f64();
                word_list.push(RichSyncWord {
                    c: seg.text.clone(),
                    o: offset_sec,
                });
                full_text.push_str(&seg.text);
            }

            mxm_lines.push(RichSyncLine {
                ts,
                te,
                l: word_list,
                x: full_text,
            });
        }

        serde_json::to_string(&mxm_lines)
    }
}
