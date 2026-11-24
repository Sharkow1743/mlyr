use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::model::{Line, Lyrics, Metadata, Mlyr, ParallelLine, Segment};
use crate::validation::ValidationError;

pub(crate) struct RawMlyr {
    pub(crate) metadata: Option<Metadata>,
    pub(crate) lyrics: RawLyrics,
}

pub(crate) struct RawLyrics {
    pub(crate) lines: Vec<RawLine>,
}

pub(crate) struct RawLine {
    pub(crate) offset: Option<Duration>,
    pub(crate) voice: Option<RawVoice>,
    pub(crate) segments: Vec<RawSegment>,
    pub(crate) parallel: Option<RawParallelLine>,
}

pub(crate) struct RawParallelLine {
    pub(crate) offset: Option<Duration>,
    pub(crate) voice: Option<RawVoice>,
    pub(crate) segments: Vec<RawSegment>,
}

pub(crate) struct RawSegment {
    pub(crate) offset: Option<Duration>,
    pub(crate) text: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum RawVoice {
    Int(u64),
    Str(String),
}

impl TryFrom<RawMlyr> for Mlyr {
    type Error = ValidationError;

    fn try_from(raw: RawMlyr) -> Result<Self, Self::Error> {
        let unique_voices: HashSet<RawVoice> = raw
            .lyrics
            .lines
            .iter()
            .filter_map(|item| item.voice.clone())
            .collect();

        if unique_voices.len() >= 255 {
            return Err(Self::Error::TooManyUniqueVoices);
        }

        let mut voice_map = HashMap::new();
        let mut next_voice_id = 1u8;

        let mut get_voice_id = |raw_voice: Option<RawVoice>| -> u8 {
            match raw_voice {
                Some(voice) => *voice_map.entry(voice).or_insert_with(|| {
                    let id = next_voice_id;
                    next_voice_id = next_voice_id.saturating_add(1);
                    id
                }),
                None => 0,
            }
        };

        let mut lines = Vec::with_capacity(raw.lyrics.lines.len());
        for rl in raw.lyrics.lines {
            let segments: Vec<Segment> = rl
                .segments
                .into_iter()
                .map(|s| Segment {
                    offset: s.offset.unwrap_or(Duration::ZERO),
                    text: s.text,
                })
                .collect();

            let parallel = rl.parallel.map(|rp| {
                let p_segments: Vec<Segment> = rp
                    .segments
                    .into_iter()
                    .map(|s| Segment {
                        offset: s.offset.unwrap_or(Duration::ZERO),
                        text: s.text,
                    })
                    .collect();

                ParallelLine {
                    offset: rp.offset,
                    voice: get_voice_id(rp.voice),
                    segments: p_segments,
                }
            });

            lines.push(Line {
                offset: rl.offset,
                voice: get_voice_id(rl.voice),
                segments,
                parallel,
            });
        }

        lines.sort_by(|a, b| match (a.offset, b.offset) {
            (Some(oa), Some(ob)) => oa.cmp(&ob),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        });

        let mlyr = Mlyr {
            metadata: raw.metadata.unwrap_or_default(),
            lyrics: Lyrics { lines },
        };

        mlyr.validate()?;
        Ok(mlyr)
    }
}
