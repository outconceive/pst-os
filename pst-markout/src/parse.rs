use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// Component type characters
pub const LABEL: char         = 'L';
pub const TEXT_INPUT: char    = 'I';
pub const PASSWORD: char      = 'P';
pub const BUTTON: char        = 'B';
pub const CHECKBOX: char      = 'C';
pub const DIVIDER: char       = 'D';
pub const SPACER: char        = '_';
pub const EMPTY: char         = ' ';

#[derive(Debug, Clone)]
pub struct Line {
    pub content: String,
    pub components: String,
    pub state_keys: String,
    pub styles: String,
    pub line_type: LineType,
    pub tag: Option<String>,
    pub config: Option<String>,
    pub constraints: BTreeMap<usize, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineType {
    Content,
    ContainerStart,
    ContainerEnd,
    EachStart,
    EachEnd,
}

impl Line {
    pub fn content_row(content: &str, components: &str, keys: &str, styles: &str) -> Self {
        Self {
            content: String::from(content),
            components: String::from(components),
            state_keys: String::from(keys),
            styles: String::from(styles),
            line_type: LineType::Content,
            tag: None,
            config: None,
            constraints: BTreeMap::new(),
        }
    }

    pub fn container_start(tag: &str, config: Option<&str>) -> Self {
        Self {
            content: String::new(), components: String::new(),
            state_keys: String::new(), styles: String::new(),
            line_type: LineType::ContainerStart,
            tag: Some(String::from(tag)),
            config: config.map(String::from),
            constraints: BTreeMap::new(),
        }
    }

    pub fn container_end(tag: &str) -> Self {
        Self {
            content: String::new(), components: String::new(),
            state_keys: String::new(), styles: String::new(),
            line_type: LineType::ContainerEnd,
            tag: Some(String::from(tag)),
            config: None,
            constraints: BTreeMap::new(),
        }
    }
}

pub fn parse(input: &str) -> Vec<Line> {
    let mut lines = Vec::new();

    for raw in input.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() { continue; }

        if let Some(rest) = trimmed.strip_prefix("@end ") {
            lines.push(Line::container_end(rest.trim()));
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('@') {
            let (tag, config) = parse_directive(rest);
            let cfg = if config.is_empty() { None } else { Some(config.as_str()) };
            lines.push(Line::container_start(&tag, cfg));
            continue;
        }

        let content_str = if let Some(rest) = trimmed.strip_prefix("| ") {
            rest
        } else if trimmed == "|" {
            ""
        } else {
            trimmed
        };

        lines.push(parse_content_line(content_str));
    }

    lines
}

fn parse_directive(rest: &str) -> (String, String) {
    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
    let tag = String::from(parts[0]);
    let config = if parts.len() > 1 { String::from(parts[1]) } else { String::new() };
    (tag, config)
}

fn parse_content_line(input: &str) -> Line {
    let mut content = String::new();
    let mut components = String::new();
    let mut state_keys = String::new();
    let mut styles = String::new();
    let mut constraints: BTreeMap<usize, Vec<String>> = BTreeMap::new();

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            if let Some((comp, end)) = parse_component(&chars, i) {
                let pos = content.len();
                let len = comp.width.max(1);

                for _ in 0..len {
                    content.push(if comp.label.is_empty() { '_' } else { comp.label.chars().next().unwrap_or('_') });
                }
                // Better: use label chars then pad
                let label_chars: Vec<char> = comp.label.chars().collect();
                // Overwrite with actual label
                let content_bytes = unsafe { content.as_bytes_mut() };
                let start = pos;
                for (j, &ch) in label_chars.iter().enumerate() {
                    if start + j < content_bytes.len() {
                        // This is ASCII-safe for Markout labels
                        content_bytes[start + j] = ch as u8;
                    }
                }

                for _ in 0..len { components.push(comp.comp_char); }

                if let Some(ref key) = comp.binding {
                    let padded = pad_key(key, len);
                    state_keys.push_str(&padded);
                } else {
                    for _ in 0..len { state_keys.push('_'); }
                }

                if let Some(ref s) = comp.style {
                    let sc = style_char(s);
                    for _ in 0..len { styles.push(sc); }
                } else {
                    for _ in 0..len { styles.push(' '); }
                }

                if !comp.constraints.is_empty() {
                    constraints.insert(pos, comp.constraints);
                }

                i = end;
                continue;
            }
        }

        // Plain text = label
        content.push(chars[i]);
        components.push(LABEL);
        state_keys.push('_');
        styles.push(' ');
        i += 1;
    }

    let mut line = Line::content_row(&content, &components, &state_keys, &styles);
    line.constraints = constraints;
    line
}

struct ParsedComponent {
    comp_char: char,
    binding: Option<String>,
    label: String,
    style: Option<String>,
    width: usize,
    constraints: Vec<String>,
}

fn parse_component(chars: &[char], start: usize) -> Option<(ParsedComponent, usize)> {
    let mut end = start + 1;
    let mut depth = 1;
    while end < chars.len() && depth > 0 {
        if chars[end] == '{' { depth += 1; }
        if chars[end] == '}' { depth -= 1; }
        end += 1;
    }
    if depth != 0 { return None; }

    let inner: String = chars[start + 1..end - 1].iter().collect();
    let parts = shell_split(&inner);
    if parts.is_empty() { return None; }

    let first = &parts[0];
    let (kind, binding) = if let Some(idx) = first.find(':') {
        let k = &first[..idx];
        let b = &first[idx + 1..];
        (k.to_string(), if b.is_empty() { None } else { Some(b.to_string()) })
    } else {
        (first.clone(), None)
    };

    let comp_char = match kind.as_str() {
        "input" => TEXT_INPUT,
        "password" => PASSWORD,
        "button" => BUTTON,
        "checkbox" => CHECKBOX,
        "label" => LABEL,
        "divider" => DIVIDER,
        "spacer" => SPACER,
        _ => LABEL,
    };

    let mut label = String::new();
    let mut style = None;
    let mut comp_constraints = Vec::new();

    for part in &parts[1..] {
        if part.starts_with('"') && part.ends_with('"') && part.len() >= 2 {
            label = part[1..part.len() - 1].to_string();
        } else if is_constraint_token(part) {
            comp_constraints.push(part.clone());
        } else if is_style(part) {
            style = Some(part.clone());
        }
    }

    let default_width = match comp_char {
        TEXT_INPUT | PASSWORD => 10,
        BUTTON => label.len().max(6),
        CHECKBOX => 3,
        DIVIDER => 1,
        SPACER => 1,
        _ => label.len().max(binding.as_ref().map(|b| b.len()).unwrap_or(1)),
    };

    Some((ParsedComponent {
        comp_char,
        binding,
        label,
        style,
        width: default_width,
        constraints: comp_constraints,
    }, end))
}

fn is_constraint_token(s: &str) -> bool {
    s.starts_with("left:") || s.starts_with("right:") ||
    s.starts_with("top:") || s.starts_with("bottom:") ||
    s.starts_with("center-x:") || s.starts_with("center-y:") ||
    s.starts_with("gap-x:") || s.starts_with("gap-y:") ||
    s.starts_with("distribute-x:") || s.starts_with("distribute-y:") ||
    (s.starts_with("width:") && !s[6..].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true)) ||
    (s.starts_with("height:") && !s[7..].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true))
}

fn is_style(s: &str) -> bool {
    matches!(s, "primary" | "secondary" | "danger" | "warning" | "info" | "outline" | "ghost")
}

fn style_char(s: &str) -> char {
    match s {
        "primary" => 'p',
        "secondary" => 's',
        "danger" => 'd',
        "warning" => 'w',
        "info" => 'i',
        "outline" => 'o',
        "ghost" => 'g',
        _ => ' ',
    }
}

fn pad_key(key: &str, len: usize) -> String {
    let key_len = key.len();
    if key_len >= len {
        key[..len].to_string()
    } else {
        let pad = len - key_len;
        let left = pad / 2;
        let right = pad - left;
        let mut s = String::new();
        for _ in 0..left { s.push('_'); }
        s.push_str(key);
        for _ in 0..right { s.push('_'); }
        s
    }
}

fn shell_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
        } else if ch == ' ' && !in_quotes {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_label() {
        let lines = parse("| Hello World");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].line_type, LineType::Content);
        assert!(lines[0].components.chars().all(|c| c == LABEL));
    }

    #[test]
    fn test_parse_container() {
        let lines = parse("@card padding:16\n| Content\n@end card");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_type, LineType::ContainerStart);
        assert_eq!(lines[0].tag, Some(String::from("card")));
        assert_eq!(lines[0].config, Some(String::from("padding:16")));
        assert_eq!(lines[2].line_type, LineType::ContainerEnd);
    }

    #[test]
    fn test_parse_input() {
        let lines = parse("| {input:name}");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].components.contains(TEXT_INPUT));
        assert!(lines[0].state_keys.contains("name"));
    }

    #[test]
    fn test_parse_button() {
        let lines = parse("| {button:submit \"Save\" primary}");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].components.contains(BUTTON));
        assert!(lines[0].styles.contains('p'));
    }

    #[test]
    fn test_parse_parametric() {
        let lines = parse(
            "@parametric\n| {label:title \"Dashboard\"}\n| {input:search center-x:title gap-y:16}\n@end parametric"
        );
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].line_type, LineType::ContainerStart);
        assert_eq!(lines[0].tag, Some(String::from("parametric")));

        // search line should have constraints
        assert!(!lines[2].constraints.is_empty());
    }

    #[test]
    fn test_constraint_parsing() {
        let lines = parse("| {button:go \"Search\" primary right:search gap-x:8:search center-y:search}");
        assert_eq!(lines.len(), 1);
        let constraints: Vec<&Vec<String>> = lines[0].constraints.values().collect();
        assert!(!constraints.is_empty());
        let flat: Vec<&String> = constraints.into_iter().flatten().collect();
        assert!(flat.iter().any(|c| c.starts_with("right:")));
        assert!(flat.iter().any(|c| c.starts_with("gap-x:")));
        assert!(flat.iter().any(|c| c.starts_with("center-y:")));
    }
}
