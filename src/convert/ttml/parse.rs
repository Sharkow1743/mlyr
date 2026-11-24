use std::collections::HashMap;
use std::time::Duration;

use crate::ConversionError;
use crate::format::normalize::{RawLine, RawMetadata, RawMlyr, RawVoiceDecl, normalize_raw_mlyr};
use crate::model::{LineKind, Mlyr, Segment, Source, VoiceRole};

use super::time::{parse_ttml_time, resolve_end_or_dur};
use super::xml_dom::{XmlElement, XmlNode, as_element, parse_xml_root};

const MIN_LINE_DURATION: Duration = Duration::from_millis(1);
const DEFAULT_TAIL_DURATION: Duration = Duration::from_secs(5);

struct ParsedP {
    begin: Option<Duration>,
    end: Option<Duration>,
    voice: Option<String>,
    main_segments: Vec<(Option<Duration>, Option<Duration>, String)>,
    bg_items: Vec<ParsedBg>,
}

struct ParsedBg {
    begin: Option<Duration>,
    end: Option<Duration>,
    segments: Vec<(Option<Duration>, Option<Duration>, String)>,
}

pub(super) fn parse_ttml(input: &str) -> Result<Mlyr, ConversionError> {
    let root = parse_xml_root(input)?;
    if root.local_name() != "tt" {
        return Err(ConversionError::InvalidFormat {
            format_name: "Ttml",
            message: format!("Expected <tt> root element, found <{}>", root.name),
        });
    }

    let mut metadata = RawMetadata::default();
    let mut artists: Vec<String> = Vec::new();
    let mut songwriters: Vec<String> = Vec::new();
    let mut declared_voices: HashMap<String, usize> = HashMap::new();

    for head in root.children.iter().filter_map(as_element) {
        if head.local_name() == "head" {
            for meta_el in head.children.iter().filter_map(as_element) {
                if meta_el.local_name() == "metadata" {
                    parse_ttml_metadata(
                        meta_el,
                        &mut metadata,
                        &mut artists,
                        &mut songwriters,
                        &mut declared_voices,
                    );
                }
            }
        }
    }

    if metadata.artist.is_none() && !artists.is_empty() {
        let joined = artists.join(", ");
        if !joined.trim().is_empty() {
            metadata.artist = Some(joined);
        }
    }
    if !songwriters.is_empty() {
        metadata
            .sources
            .push(Source::with_name("songwriter", songwriters.join(", ")));
    }

    let body = root
        .children
        .iter()
        .filter_map(as_element)
        .find(|el| el.local_name() == "body")
        .ok_or_else(|| ConversionError::InvalidFormat {
            format_name: "Ttml",
            message: "Missing <body> element in TTML document".to_string(),
        })?;

    if let Some(dur_str) = body.attr("dur") {
        if let Some(dur) = parse_ttml_time(dur_str)? {
            if !dur.is_zero() {
                metadata.length = Some(dur);
            }
        }
    }

    let mut parsed_ps: Vec<(ParsedP, Option<Duration>)> = Vec::new();
    collect_body_paragraphs(body, None, None, &mut parsed_ps)?;

    for (p, _) in &parsed_ps {
        if let Some(ref agent_id) = p.voice {
            if !declared_voices.contains_key(agent_id) {
                let role = infer_role_from_id(agent_id, metadata.voices.len());
                let idx = metadata.voices.len();
                declared_voices.insert(agent_id.clone(), idx);
                metadata.voices.push(RawVoiceDecl {
                    id: agent_id.clone(),
                    role,
                    singer: None,
                });
            }
        }
    }

    let mut raw_main = Vec::new();
    let mut raw_bg = Vec::new();

    for (p, div_end) in parsed_ps {
        if !p.main_segments.is_empty() {
            let first_span_begin = p.main_segments.iter().find_map(|(b, _, _)| *b);
            let last_span_end = p.main_segments.iter().rev().find_map(|(_, e, _)| *e);

            let line_begin = p.begin.or(first_span_begin).unwrap_or(Duration::ZERO);
            let line_end = if !p.bg_items.is_empty() {
                last_span_end.or(p.end).or(div_end)
            } else {
                match (p.end, last_span_end) {
                    (Some(pe), Some(se)) => Some(pe.max(se)),
                    (Some(pe), None) => Some(pe),
                    (None, Some(se)) => Some(se),
                    (None, None) => div_end,
                }
            };

            let segments = build_segments(line_begin, p.main_segments);
            raw_main.push((line_begin, line_end, p.voice.clone(), segments));
        }

        for bg in p.bg_items {
            if bg.segments.is_empty() {
                continue;
            }
            let first_span_begin = bg.segments.iter().find_map(|(b, _, _)| *b);
            let last_span_end = bg.segments.iter().rev().find_map(|(_, e, _)| *e);

            let bg_begin = bg
                .begin
                .or(first_span_begin)
                .or(p.begin)
                .unwrap_or(Duration::ZERO);
            let bg_end = bg.end.or(last_span_end).or(p.end).or(div_end);

            let segments = build_segments(bg_begin, bg.segments);
            raw_bg.push((bg_begin, bg_end, p.voice.clone(), segments));
        }
    }

    raw_main.sort_by_key(|(b, _, _, _)| *b);
    raw_bg.sort_by_key(|(b, _, _, _)| *b);

    let mut raw_lines = Vec::with_capacity(raw_main.len() + raw_bg.len());
    finalize_track_lines(LineKind::Main, &raw_main, metadata.length, &mut raw_lines);
    finalize_track_lines(
        LineKind::Background,
        &raw_bg,
        metadata.length,
        &mut raw_lines,
    );

    let raw_mlyr = RawMlyr {
        metadata,
        lyrics_offset: None,
        lines: raw_lines,
    };

    Ok(normalize_raw_mlyr(raw_mlyr)?)
}

fn parse_ttml_metadata(
    meta_el: &XmlElement,
    metadata: &mut RawMetadata,
    artists: &mut Vec<String>,
    songwriters: &mut Vec<String>,
    declared_voices: &mut HashMap<String, usize>,
) {
    for child in meta_el.children.iter().filter_map(as_element) {
        match child.local_name() {
            "title" => {
                let text = child.text_content().trim().to_string();
                if !text.is_empty() && metadata.title.is_none() {
                    metadata.title = Some(text);
                }
            }
            "agent" => {
                if let Some(id) = child.attr_exact_or_prefixed("xml", "id").map(str::trim) {
                    if !id.is_empty() {
                        let role_attr = child.attr("type").unwrap_or("person").trim();
                        let role = match role_attr {
                            "group" => VoiceRole::Group,
                            _ => infer_role_from_id(id, metadata.voices.len()),
                        };
                        let singer = child
                            .children
                            .iter()
                            .filter_map(as_element)
                            .find(|c| c.local_name() == "name")
                            .map(|c| c.text_content().trim().to_string())
                            .filter(|s| !s.is_empty());

                        if let Some(&idx) = declared_voices.get(id) {
                            if let Some(name) = singer {
                                if metadata.voices[idx].singer.is_none() {
                                    metadata.voices[idx].singer = Some(name);
                                }
                            }
                        } else {
                            let idx = metadata.voices.len();
                            declared_voices.insert(id.to_string(), idx);
                            metadata.voices.push(RawVoiceDecl {
                                id: id.to_string(),
                                role,
                                singer,
                            });
                        }
                    }
                }
            }
            "meta" => {
                if let (Some(key), Some(val)) = (child.attr("key"), child.attr("value")) {
                    let val = val.trim();
                    if val.is_empty() {
                        continue;
                    }
                    match key {
                        "musicName" => metadata.title = Some(val.to_string()),
                        "artists" => artists.push(val.to_string()),
                        "album" => metadata.album = Some(val.to_string()),
                        "ttmlAuthorGithubLogin" | "ttmlAuthorGithub" => {
                            metadata.sources.push(Source::with_name(key, val));
                        }
                        "isrc" | "appleMusicId" | "ncmMusicId" | "qqMusicId" | "spotifyId" => {
                            metadata.sources.push(Source::new(
                                "id",
                                Some(key.to_string()),
                                Some(val.to_string()),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            "iTunesMetadata" | "metadata" => {
                for sub in child.children.iter().filter_map(as_element) {
                    if sub.local_name() == "songwriters" {
                        for sw in sub.children.iter().filter_map(as_element) {
                            if sw.local_name() == "songwriter" {
                                let name = sw.text_content().trim().to_string();
                                if !name.is_empty() {
                                    songwriters.push(name);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn infer_role_from_id(id: &str, existing_count: usize) -> VoiceRole {
    match id {
        "v1000" | "group" => VoiceRole::Group,
        "v2" => VoiceRole::Secondary,
        "v3" => VoiceRole::Tertiary,
        "v1" | "default" => VoiceRole::Primary,
        _ => match existing_count {
            0 => VoiceRole::Primary,
            1 => VoiceRole::Secondary,
            2 => VoiceRole::Tertiary,
            _ => VoiceRole::Group,
        },
    }
}

fn collect_body_paragraphs(
    el: &XmlElement,
    inherited_begin: Option<Duration>,
    inherited_end: Option<Duration>,
    out: &mut Vec<(ParsedP, Option<Duration>)>,
) -> Result<(), ConversionError> {
    for child in el.children.iter().filter_map(as_element) {
        match child.local_name() {
            "div" => {
                let div_begin = child
                    .attr("begin")
                    .map(parse_ttml_time)
                    .transpose()?
                    .flatten()
                    .or(inherited_begin);
                let div_end = resolve_end_or_dur(child, div_begin)?.or(inherited_end);
                collect_body_paragraphs(child, div_begin, div_end, out)?;
            }
            "p" => {
                let p = parse_paragraph(child, inherited_begin)?;
                if !p.main_segments.is_empty() || !p.bg_items.is_empty() {
                    out.push((p, inherited_end));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_paragraph(
    p_el: &XmlElement,
    inherited_begin: Option<Duration>,
) -> Result<ParsedP, ConversionError> {
    let begin = p_el
        .attr("begin")
        .map(parse_ttml_time)
        .transpose()?
        .flatten()
        .or(inherited_begin);
    let end = resolve_end_or_dur(p_el, begin)?;
    let voice = p_el
        .attr_exact_or_prefixed("ttm", "agent")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let mut main_segments = Vec::new();
    let mut bg_items = Vec::new();
    collect_inline_nodes(&p_el.children, begin, &mut main_segments, &mut bg_items)?;

    if main_segments
        .iter()
        .all(|(b, e, t)| b.is_none() && e.is_none() && t.trim().is_empty())
    {
        main_segments.clear();
    } else {
        if let Some(first) = main_segments.first_mut() {
            if first.0.is_none() && first.1.is_none() && first.2.contains('\n') {
                first.2 = first.2.trim_start().to_string();
            }
        }
        if let Some(last) = main_segments.last_mut() {
            if last.0.is_none() && last.1.is_none() && last.2.contains('\n') {
                last.2 = last.2.trim_end().to_string();
            }
        }
        main_segments.retain(|(b, e, t)| b.is_some() || e.is_some() || !t.is_empty());
    }

    Ok(ParsedP {
        begin,
        end,
        voice,
        main_segments,
        bg_items,
    })
}

fn collect_inline_nodes(
    nodes: &[XmlNode],
    parent_begin: Option<Duration>,
    segments: &mut Vec<(Option<Duration>, Option<Duration>, String)>,
    bg_items: &mut Vec<ParsedBg>,
) -> Result<(), ConversionError> {
    for node in nodes {
        match node {
            XmlNode::Text(raw_text) => {
                let text = if raw_text.contains('\n') || raw_text.contains('\r') {
                    if raw_text.trim().is_empty() {
                        continue;
                    }
                    normalize_multiline_xml_text(raw_text)
                } else {
                    raw_text.clone()
                };

                if text.is_empty() {
                    continue;
                }
                if text.chars().all(|c| c == ' ') {
                    if let Some(prev) = segments.last_mut() {
                        prev.2.push_str(&text);
                        continue;
                    }
                }
                segments.push((None, None, text));
            }
            XmlNode::Element(el) => {
                if el.local_name() == "br" {
                    if let Some(prev) = segments.last_mut() {
                        prev.2.push(' ');
                    } else {
                        segments.push((None, None, " ".to_string()));
                    }
                    continue;
                }
                if el.local_name() != "span" {
                    continue;
                }

                let role = el.attr_exact_or_prefixed("ttm", "role").unwrap_or("");
                if matches!(role, "x-translation" | "x-roman") {
                    continue;
                }
                let ruby_role = el.attr_exact_or_prefixed("tts", "ruby").unwrap_or("");
                if matches!(ruby_role, "text" | "delimiter") {
                    continue;
                }

                if matches!(role, "x-bg" | "lyric:background") {
                    let bg_begin = el
                        .attr("begin")
                        .map(parse_ttml_time)
                        .transpose()?
                        .flatten()
                        .or(parent_begin);
                    let bg_end = resolve_end_or_dur(el, bg_begin)?;
                    let mut bg_segs = Vec::new();
                    let mut nested_bgs = Vec::new();
                    collect_inline_nodes(&el.children, bg_begin, &mut bg_segs, &mut nested_bgs)?;
                    bg_segs.retain(|(b, e, t)| b.is_some() || e.is_some() || !t.is_empty());
                    if !bg_segs.is_empty() {
                        bg_items.push(ParsedBg {
                            begin: bg_begin,
                            end: bg_end,
                            segments: bg_segs,
                        });
                    }
                    bg_items.extend(nested_bgs);
                    continue;
                }

                let span_begin = el.attr("begin").map(parse_ttml_time).transpose()?.flatten();
                let span_end = resolve_end_or_dur(el, span_begin.or(parent_begin))?;

                let has_child_spans = el
                    .children
                    .iter()
                    .filter_map(as_element)
                    .any(|c| c.local_name() == "span");

                if has_child_spans && span_begin.is_none() {
                    collect_inline_nodes(&el.children, parent_begin, segments, bg_items)?;
                } else {
                    let span_text = extract_span_text(el);
                    if !span_text.is_empty() || span_begin.is_some() {
                        if let Some(last) = segments.last_mut() {
                            if last.0.is_none() && last.1.is_none() && last.2.trim().is_empty() {
                                let prefix = std::mem::take(&mut last.2);
                                segments.pop();
                                segments.push((
                                    span_begin,
                                    span_end,
                                    format!("{prefix}{span_text}"),
                                ));
                                continue;
                            }
                        }
                        segments.push((span_begin, span_end, span_text));
                    }
                }
            }
        }
    }
    Ok(())
}

fn extract_span_text(el: &XmlElement) -> String {
    let mut out = String::new();
    for child in &el.children {
        match child {
            XmlNode::Text(t) => out.push_str(t),
            XmlNode::Element(sub) => {
                let role = sub.attr_exact_or_prefixed("ttm", "role").unwrap_or("");
                let ruby = sub.attr_exact_or_prefixed("tts", "ruby").unwrap_or("");
                if matches!(
                    role,
                    "x-translation" | "x-roman" | "x-bg" | "lyric:background"
                ) || matches!(ruby, "text" | "delimiter")
                {
                    continue;
                }
                if sub.local_name() == "br" {
                    out.push(' ');
                } else {
                    out.push_str(&extract_span_text(sub));
                }
            }
        }
    }
    out
}

fn normalize_multiline_xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ' {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

fn build_segments(
    line_begin: Duration,
    raw_segs: Vec<(Option<Duration>, Option<Duration>, String)>,
) -> Vec<Segment> {
    let mut result: Vec<Segment> = Vec::with_capacity(raw_segs.len());
    let mut last_offset = Duration::ZERO;

    for (begin_opt, _, text) in raw_segs {
        if let Some(begin) = begin_opt {
            let rel = begin.saturating_sub(line_begin).max(last_offset);
            last_offset = rel;
            result.push(Segment { offset: rel, text });
        } else if let Some(prev) = result.last_mut() {
            prev.text.push_str(&text);
        } else {
            result.push(Segment {
                offset: Duration::ZERO,
                text,
            });
        }
    }
    result
}

fn finalize_track_lines(
    kind: LineKind,
    items: &[(Duration, Option<Duration>, Option<String>, Vec<Segment>)],
    track_length: Option<Duration>,
    out: &mut Vec<RawLine>,
) {
    for (i, (offset, explicit_end, voice, segments)) in items.iter().enumerate() {
        let last_seg_offset = segments.last().map(|s| s.offset).unwrap_or(Duration::ZERO);
        let duration = if let Some(end) = explicit_end {
            end.saturating_sub(*offset)
                .max(MIN_LINE_DURATION)
                .max(last_seg_offset)
        } else if let Some((next_offset, _, _, _)) = items.get(i + 1) {
            let delta = next_offset.saturating_sub(*offset);
            if delta.is_zero() {
                MIN_LINE_DURATION.max(last_seg_offset)
            } else {
                delta.max(last_seg_offset)
            }
        } else {
            match track_length {
                Some(len) if len > *offset => (len - *offset)
                    .min(DEFAULT_TAIL_DURATION)
                    .max(MIN_LINE_DURATION)
                    .max(last_seg_offset),
                _ => DEFAULT_TAIL_DURATION.max(last_seg_offset),
            }
        };

        out.push(RawLine {
            kind,
            offset: *offset,
            duration,
            voice: voice.clone(),
            segments: segments.clone(),
        });
    }
}
