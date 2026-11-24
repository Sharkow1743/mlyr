use crate::ConversionError;

#[derive(Debug, Clone)]
pub(super) enum XmlNode {
    Element(XmlElement),
    Text(String),
}

#[derive(Debug, Clone)]
pub(super) struct XmlElement {
    pub(super) name: String,
    pub(super) attrs: Vec<(String, String)>,
    pub(super) children: Vec<XmlNode>,
}

impl XmlElement {
    pub(super) fn local_name(&self) -> &str {
        local_name(&self.name)
    }

    pub(super) fn attr(&self, target: &str) -> Option<&str> {
        if let Some((_, v)) = self.attrs.iter().find(|(k, _)| k == target) {
            return Some(v.as_str());
        }
        let target_local = local_name(target);
        self.attrs
            .iter()
            .find(|(k, _)| local_name(k) == target_local)
            .map(|(_, v)| v.as_str())
    }

    pub(super) fn attr_exact_or_prefixed(&self, prefix_hint: &str, local: &str) -> Option<&str> {
        let prefixed = format!("{prefix_hint}:{local}");
        self.attrs
            .iter()
            .find(|(k, _)| k == &prefixed || k == local || local_name(k) == local)
            .map(|(_, v)| v.as_str())
    }

    pub(super) fn text_content(&self) -> String {
        let mut out = String::new();
        for child in &self.children {
            match child {
                XmlNode::Text(t) => out.push_str(t),
                XmlNode::Element(el) => {
                    if el.local_name() == "br" {
                        out.push(' ');
                    } else {
                        out.push_str(&el.text_content());
                    }
                }
            }
        }
        out
    }
}

pub(super) fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

pub(super) fn as_element(node: &XmlNode) -> Option<&XmlElement> {
    match node {
        XmlNode::Element(el) => Some(el),
        XmlNode::Text(_) => None,
    }
}

pub(super) fn parse_xml_root(input: &str) -> Result<XmlElement, ConversionError> {
    let s = input.strip_prefix('\u{FEFF}').unwrap_or(input);
    let mut cursor = 0;
    let mut stack: Vec<XmlElement> = Vec::new();
    let mut root: Option<XmlElement> = None;

    while cursor < s.len() {
        if let Some(lt_rel) = s[cursor..].find('<') {
            if lt_rel > 0 {
                let text = unescape_xml(&s[cursor..cursor + lt_rel]);
                if let Some(top) = stack.last_mut() {
                    top.children.push(XmlNode::Text(text));
                }
            }
            cursor += lt_rel;

            let rest = &s[cursor..];
            if rest.starts_with("<!--") {
                let end = rest
                    .find("-->")
                    .ok_or_else(|| xml_err("Unclosed XML comment"))?;
                cursor += end + 3;
            } else if rest.starts_with("<![CDATA[") {
                let end = rest
                    .find("]]>")
                    .ok_or_else(|| xml_err("Unclosed CDATA section"))?;
                let cdata = rest[9..end].to_string();
                if let Some(top) = stack.last_mut() {
                    top.children.push(XmlNode::Text(cdata));
                }
                cursor += end + 3;
            } else if rest.starts_with("<?") {
                let end = rest
                    .find("?>")
                    .ok_or_else(|| xml_err("Unclosed processing instruction"))?;
                cursor += end + 2;
            } else if rest.starts_with("<!") {
                let end = rest
                    .find('>')
                    .ok_or_else(|| xml_err("Unclosed XML declaration"))?;
                cursor += end + 1;
            } else if rest.starts_with("</") {
                let end = find_tag_end(rest).ok_or_else(|| xml_err("Unclosed end tag"))?;
                let close_name = rest[2..end].trim();
                let popped = stack
                    .pop()
                    .ok_or_else(|| xml_err(format!("Unexpected closing tag </{close_name}>")))?;
                if popped.name != close_name {
                    return Err(xml_err(format!(
                        "Mismatched XML tags: <{}> vs </{close_name}>",
                        popped.name
                    )));
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(XmlNode::Element(popped));
                } else {
                    root = Some(popped);
                }
                cursor += end + 1;
            } else {
                let end = find_tag_end(rest).ok_or_else(|| xml_err("Unclosed start tag"))?;
                let tag_body = &rest[1..end];
                let self_closing = tag_body.ends_with('/');
                let tag_inner = tag_body.strip_suffix('/').unwrap_or(tag_body).trim();
                let (name, attrs) = parse_tag_header(tag_inner)?;
                let elem = XmlElement {
                    name,
                    attrs,
                    children: Vec::new(),
                };
                if self_closing {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlNode::Element(elem));
                    } else {
                        root = Some(elem);
                    }
                } else {
                    stack.push(elem);
                }
                cursor += end + 1;
            }
        } else {
            let text = unescape_xml(&s[cursor..]);
            if let Some(top) = stack.last_mut() {
                top.children.push(XmlNode::Text(text));
            }
            break;
        }
    }

    if !stack.is_empty() {
        return Err(xml_err("Unclosed XML elements at end of document"));
    }

    root.ok_or_else(|| xml_err("No root element found in TTML document"))
}

pub(super) fn escape_xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

pub(super) fn escape_xml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

fn find_tag_end(s: &str) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (i, b) in s.bytes().enumerate() {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => return Some(i),
            None => {}
        }
    }
    None
}

fn parse_tag_header(s: &str) -> Result<(String, Vec<(String, String)>), ConversionError> {
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let name = s[..i].to_string();
    if name.is_empty() {
        return Err(xml_err("Empty XML tag name"));
    }

    let mut attrs = Vec::new();
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key = s[key_start..i].to_string();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            return Err(xml_err(format!("Malformed attribute '{key}' in <{name}>")));
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || (bytes[i] != b'"' && bytes[i] != b'\'') {
            return Err(xml_err(format!(
                "Attribute '{key}' value must be quoted in <{name}>"
            )));
        }
        let q = bytes[i];
        i += 1;
        let val_start = i;
        while i < bytes.len() && bytes[i] != q {
            i += 1;
        }
        if i >= bytes.len() {
            return Err(xml_err(format!("Unclosed attribute value for '{key}'")));
        }
        let val = unescape_xml(&s[val_start..i]);
        attrs.push((key, val));
        i += 1;
    }

    Ok((name, attrs))
}

fn unescape_xml(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s.as_bytes()[i] == b'&' {
            if let Some(semi_rel) = s[i..].find(';') {
                let entity = &s[i + 1..i + semi_rel];
                let decoded = match entity {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                        u32::from_str_radix(&entity[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                    }
                    _ if entity.starts_with('#') => {
                        entity[1..].parse::<u32>().ok().and_then(char::from_u32)
                    }
                    _ => None,
                };
                if let Some(ch) = decoded {
                    out.push(ch);
                    i += semi_rel + 1;
                    continue;
                }
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn xml_err(msg: impl Into<String>) -> ConversionError {
    ConversionError::InvalidFormat {
        format_name: "Ttml",
        message: msg.into(),
    }
}
