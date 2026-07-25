use std::time::Duration;

use crate::{
    Line, Lyrics, Metadata, Mlyr, ParallelLine, Segment,
    definition::{RawMlyr, RawVoice},
};
use serde::{Deserialize, Deserializer};

impl Mlyr {
    pub fn metadata(&self) -> Option<&Metadata> {
        if self.metadata.is_empty() {
            None
        } else {
            Some(&self.metadata)
        }
    }

    pub fn lyrics(&self) -> &Lyrics {
        &self.lyrics
    }

    /// Adjusts line offsets in the lyrics by the given duration delta
    /// Timestamps that would become negative are clamped to zero
    pub fn shift_timestamps(&mut self, delta: Duration, forward: bool) {
        if let Some(ref mut len) = self.metadata.length {
            if forward {
                *len += delta;
            } else {
                *len = len.checked_sub(delta).unwrap_or(Duration::ZERO);
            }
        }

        for line in &mut self.lyrics.lines {
            if let Some(ref mut offset) = line.offset {
                if forward {
                    *offset += delta;
                } else {
                    *offset = offset.checked_sub(delta).unwrap_or(Duration::ZERO);
                }
            }

            if let Some(ref mut parallel) = line.parallel {
                if let Some(ref mut offset) = parallel.offset {
                    if forward {
                        *offset += delta;
                    } else {
                        *offset = offset.checked_sub(delta).unwrap_or(Duration::ZERO);
                    }
                }
            }
        }
    }
}

impl Line {
    /// Returns the absolute end time of this line (including parallel vocals)
    /// Returns None if the line has no offset
    pub fn absolute_end_time(&self) -> Option<Duration> {
        let line_offset = self.offset?;

        let main_end = line_offset
            + self
                .segments
                .last()
                .map(|s| s.offset)
                .unwrap_or(Duration::ZERO);

        let parallel_end = self.parallel.as_ref().map_or(Duration::ZERO, |p| {
            line_offset
                + p.offset.unwrap_or(Duration::ZERO)
                + p.segments
                    .last()
                    .map(|s| s.offset)
                    .unwrap_or(Duration::ZERO)
        });

        Some(std::cmp::max(main_end, parallel_end))
    }
}

//----------------------------

impl<'de> Deserialize<'de> for RawVoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VoiceVisitor;
        impl<'de> serde::de::Visitor<'de> for VoiceVisitor {
            type Value = RawVoice;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("integer or string")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(RawVoice::Int(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                if v >= 0 {
                    Ok(RawVoice::Int(v as u64))
                } else {
                    Ok(RawVoice::Str(v.to_string()))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Ok(num) = v.parse::<u64>() {
                    Ok(RawVoice::Int(num))
                } else {
                    Ok(RawVoice::Str(v.to_string()))
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Ok(num) = v.parse::<u64>() {
                    Ok(RawVoice::Int(num))
                } else {
                    Ok(RawVoice::Str(v))
                }
            }
        }
        deserializer.deserialize_any(VoiceVisitor)
    }
}

impl TryFrom<RawMlyr> for Mlyr {
    type Error = super::ValidationError;

    fn try_from(raw: RawMlyr) -> Result<Self, Self::Error> {
        let unique_voices: std::collections::HashSet<RawVoice> = raw
            .lyrics
            .lines
            .iter()
            .filter_map(|item| item.voice.clone())
            .collect();

        if unique_voices.len() >= 255 {
            return Err(Self::Error::TooManyUniqueVoices);
        }

        let mut voice_map = std::collections::HashMap::new();
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

        // Normalize sorting chronologically
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
