use std::collections::HashMap;
use std::time::Duration;

use crate::ConversionError;
use crate::model::{Line, Mlyr, VoiceRole};

use super::time::format_ttml_time;
use super::xml_dom::{escape_xml_attr, escape_xml_text};

pub(super) fn serialize_ttml(mlyr: &Mlyr, formatted: bool) -> Result<String, ConversionError> {
    mlyr.validate()?;

    let is_word_synced = mlyr
        .lyrics
        .lines
        .iter()
        .any(|l| l.segments.len() > 1 || l.segments.iter().any(|s| !s.offset.is_zero()));

    let timing_mode = if is_word_synced { "Word" } else { "Line" };

    let mut out = String::new();
    let nl = if formatted { "\n" } else { "" };
    let t1 = if formatted { "\t" } else { "" };
    let t2 = if formatted { "\t\t" } else { "" };
    let t3 = if formatted { "\t\t\t" } else { "" };

    out.push_str("<tt xmlns=\"http://www.w3.org/ns/ttml\" xmlns:ttm=\"http://www.w3.org/ns/ttml#metadata\" xmlns:itunes=\"http://music.apple.com/lyric-ttml-internal\" xmlns:amll=\"http://www.example.com/ns/amll\" itunes:timing=\"");
    out.push_str(timing_mode);
    out.push_str("\">");
    out.push_str(nl);

    if !mlyr.metadata.is_empty() {
        write_head(&mut out, mlyr, nl, t1, t2, t3);
    }

    out.push_str(&format!("{t1}<body"));
    if let Some(len) = mlyr.metadata.length {
        out.push_str(&format!(" dur=\"{}\"", format_ttml_time(len)));
    }
    out.push('>');
    out.push_str(nl);

    write_body_div(&mut out, mlyr, is_word_synced, nl, t2, t3);

    out.push_str(&format!("{t1}</body>{nl}</tt>"));
    if formatted {
        out.push('\n');
    }

    Ok(out)
}

fn write_head(out: &mut String, mlyr: &Mlyr, nl: &str, t1: &str, t2: &str, t3: &str) {
    out.push_str(&format!("{t1}<head>{nl}{t2}<metadata>{nl}"));

    for voice in &mlyr.metadata.voices {
        let role_str = match voice.role {
            VoiceRole::Group => "group",
            VoiceRole::Primary | VoiceRole::Secondary | VoiceRole::Tertiary => "person",
        };
        if voice.singers.is_empty() {
            out.push_str(&format!(
                "{t3}<ttm:agent type=\"{role_str}\" xml:id=\"{}\"/>{nl}",
                escape_xml_attr(voice.id.as_str())
            ));
        } else {
            out.push_str(&format!(
                "{t3}<ttm:agent type=\"{role_str}\" xml:id=\"{}\">",
                escape_xml_attr(voice.id.as_str())
            ));
            for singer in &voice.singers {
                out.push_str(&format!(
                    "<ttm:name type=\"full\">{}</ttm:name>",
                    escape_xml_text(singer)
                ));
            }
            out.push_str(&format!("</ttm:agent>{nl}"));
        }
    }

    if let Some(ref title) = mlyr.metadata.title {
        out.push_str(&format!(
            "{t3}<amll:meta key=\"musicName\" value=\"{}\"/>{nl}",
            escape_xml_attr(title)
        ));
    }
    if let Some(ref artist) = mlyr.metadata.artist {
        out.push_str(&format!(
            "{t3}<amll:meta key=\"artists\" value=\"{}\"/>{nl}",
            escape_xml_attr(artist)
        ));
    }
    if let Some(ref album) = mlyr.metadata.album {
        out.push_str(&format!(
            "{t3}<amll:meta key=\"album\" value=\"{}\"/>{nl}",
            escape_xml_attr(album)
        ));
    }
    for src in &mlyr.metadata.sources {
        if let Some(val) = src.name.as_deref().or(src.content.as_deref()) {
            let key = src
                .name
                .as_deref()
                .filter(|_| src.kind == "id")
                .unwrap_or(&src.kind);
            let attr_val = if src.kind == "id" {
                src.content.as_deref().unwrap_or(val)
            } else {
                val
            };
            out.push_str(&format!(
                "{t3}<amll:meta key=\"{}\" value=\"{}\"/>{nl}",
                escape_xml_attr(key),
                escape_xml_attr(attr_val)
            ));
        }
    }

    out.push_str(&format!("{t2}</metadata>{nl}{t1}</head>{nl}"));
}

fn write_body_div(
    out: &mut String,
    mlyr: &Mlyr,
    is_word_synced: bool,
    nl: &str,
    t2: &str,
    t3: &str,
) {
    let mut main_indices = Vec::new();
    let mut bg_indices = Vec::new();
    for (idx, line) in mlyr.lyrics.lines.iter().enumerate() {
        if line.is_bg() {
            bg_indices.push(idx);
        } else {
            main_indices.push(idx);
        }
    }

    let mut attached_bgs: HashMap<usize, Vec<&Line>> = HashMap::new();
    let mut orphan_bgs: Vec<&Line> = Vec::new();

    for bg_idx in bg_indices {
        let bg = &mlyr.lyrics.lines[bg_idx];
        let host = main_indices
            .iter()
            .copied()
            .rfind(|&m_idx| {
                let m = &mlyr.lyrics.lines[m_idx];
                m.offset <= bg.offset && bg.offset <= m.end()
            })
            .or_else(|| {
                main_indices.iter().copied().find(|&m_idx| {
                    let m = &mlyr.lyrics.lines[m_idx];
                    bg.offset <= m.offset && bg.end() >= m.offset
                })
            });

        if let Some(m_idx) = host {
            attached_bgs.entry(m_idx).or_default().push(bg);
        } else {
            orphan_bgs.push(bg);
        }
    }

    enum TtmlPItem<'a> {
        Main { line: &'a Line, bgs: Vec<&'a Line> },
        OrphanBg(&'a Line),
    }

    let mut p_items: Vec<(Duration, TtmlPItem<'_>)> = Vec::new();
    for m_idx in main_indices {
        let line = &mlyr.lyrics.lines[m_idx];
        let bgs = attached_bgs.remove(&m_idx).unwrap_or_default();
        p_items.push((line.offset, TtmlPItem::Main { line, bgs }));
    }
    for bg in orphan_bgs {
        p_items.push((bg.offset, TtmlPItem::OrphanBg(bg)));
    }
    p_items.sort_by_key(|(offset, _)| *offset);

    if p_items.is_empty() {
        return;
    }

    let div_begin = p_items.first().map(|(o, _)| *o).unwrap();
    let div_end = p_items
        .iter()
        .map(|(_, item)| match item {
            TtmlPItem::Main { line, bgs } => {
                let bg_max = bgs.iter().map(|b| b.end()).max().unwrap_or(Duration::ZERO);
                line.end().max(bg_max)
            }
            TtmlPItem::OrphanBg(bg) => bg.end(),
        })
        .max()
        .unwrap();

    out.push_str(&format!(
        "{t2}<div begin=\"{}\" end=\"{}\">{nl}",
        format_ttml_time(div_begin),
        format_ttml_time(div_end)
    ));

    let emit_agents = !mlyr.metadata.voices.is_empty();
    for (idx, (_, item)) in p_items.into_iter().enumerate() {
        let key = idx + 1;
        match item {
            TtmlPItem::Main { line, bgs } => {
                out.push_str(&format!(
                    "{t3}<p begin=\"{}\" end=\"{}\"",
                    format_ttml_time(line.offset),
                    format_ttml_time(line.end())
                ));
                if emit_agents {
                    out.push_str(&format!(
                        " ttm:agent=\"{}\"",
                        escape_xml_attr(line.voice.as_str())
                    ));
                }
                out.push_str(&format!(" itunes:key=\"L{key}\">"));

                for bg in bgs.iter().filter(|b| b.offset < line.offset) {
                    write_bg_span(out, bg, is_word_synced);
                }
                write_line_content(out, line, is_word_synced);
                for bg in bgs.iter().filter(|b| b.offset >= line.offset) {
                    write_bg_span(out, bg, is_word_synced);
                }

                out.push_str(&format!("</p>{nl}"));
            }
            TtmlPItem::OrphanBg(bg) => {
                out.push_str(&format!(
                    "{t3}<p begin=\"{}\" end=\"{}\"",
                    format_ttml_time(bg.offset),
                    format_ttml_time(bg.end())
                ));
                if emit_agents {
                    out.push_str(&format!(
                        " ttm:agent=\"{}\"",
                        escape_xml_attr(bg.voice.as_str())
                    ));
                }
                out.push_str(&format!(" itunes:key=\"L{key}\">"));
                write_bg_span(out, bg, is_word_synced);
                out.push_str(&format!("</p>{nl}"));
            }
        }
    }

    out.push_str(&format!("{t2}</div>{nl}"));
}

fn write_line_content(out: &mut String, line: &Line, is_word_synced: bool) {
    if is_word_synced {
        for (i, seg) in line.segments.iter().enumerate() {
            let seg_begin = line.offset + seg.offset;
            let seg_end = line
                .segments
                .get(i + 1)
                .map(|n| line.offset + n.offset)
                .unwrap_or_else(|| line.end());
            out.push_str(&format!(
                "<span begin=\"{}\" end=\"{}\">{}</span>",
                format_ttml_time(seg_begin),
                format_ttml_time(seg_end),
                escape_xml_text(&seg.text)
            ));
        }
    } else {
        let text: String = line.segments.iter().map(|s| s.text.as_str()).collect();
        out.push_str(&escape_xml_text(&text));
    }
}

fn write_bg_span(out: &mut String, bg: &Line, is_word_synced: bool) {
    out.push_str(&format!(
        "<span ttm:role=\"x-bg\" begin=\"{}\" end=\"{}\">",
        format_ttml_time(bg.offset),
        format_ttml_time(bg.end())
    ));
    write_line_content(out, bg, is_word_synced);
    out.push_str("</span>");
}
