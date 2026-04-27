use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// Component type characters (matches Outconceive UI)
pub const LABEL: char         = 'L';
pub const TEXT_INPUT: char    = 'I';
pub const PASSWORD: char      = 'P';
pub const BUTTON: char        = 'B';
pub const CHECKBOX: char      = 'C';
pub const RADIO: char         = 'R';
pub const SELECT: char        = 'S';
pub const TEXTAREA: char      = 'T';
pub const IMAGE: char         = 'G';
pub const LINK: char          = 'K';
pub const DIVIDER: char       = 'D';
pub const SPACER: char        = '_';
pub const PILL: char          = 'W';
pub const BADGE: char         = 'J';
pub const PROGRESS: char      = 'Q';
pub const SPARKLINE: char     = 'Z';
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
    pub cols: BTreeMap<usize, (u8, u8)>,
    pub responsive: BTreeMap<usize, Vec<(String, u8, u8)>>,
    pub validates: BTreeMap<usize, String>,
    pub animates: BTreeMap<usize, String>,
    pub hrefs: BTreeMap<usize, String>,
    pub popovers: BTreeMap<usize, String>,
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
            cols: BTreeMap::new(),
            responsive: BTreeMap::new(),
            validates: BTreeMap::new(),
            animates: BTreeMap::new(),
            hrefs: BTreeMap::new(),
            popovers: BTreeMap::new(),
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
            cols: BTreeMap::new(),
            responsive: BTreeMap::new(),
            validates: BTreeMap::new(),
            animates: BTreeMap::new(),
            hrefs: BTreeMap::new(),
            popovers: BTreeMap::new(),
        }
    }

    pub fn each_start(key: &str) -> Self {
        Self {
            content: String::new(), components: String::new(),
            state_keys: String::new(), styles: String::new(),
            line_type: LineType::EachStart,
            tag: Some(String::from(key)),
            config: None,
            constraints: BTreeMap::new(),
            cols: BTreeMap::new(),
            responsive: BTreeMap::new(),
            validates: BTreeMap::new(),
            animates: BTreeMap::new(),
            hrefs: BTreeMap::new(),
            popovers: BTreeMap::new(),
        }
    }

    pub fn each_end() -> Self {
        Self {
            content: String::new(), components: String::new(),
            state_keys: String::new(), styles: String::new(),
            line_type: LineType::EachEnd,
            tag: None, config: None,
            constraints: BTreeMap::new(),
            cols: BTreeMap::new(),
            responsive: BTreeMap::new(),
            validates: BTreeMap::new(),
            animates: BTreeMap::new(),
            hrefs: BTreeMap::new(),
            popovers: BTreeMap::new(),
        }
    }

    pub fn is_each_start(&self) -> bool { self.line_type == LineType::EachStart }
    pub fn is_each_end(&self) -> bool { self.line_type == LineType::EachEnd }

    pub fn container_end(tag: &str) -> Self {
        Self {
            content: String::new(), components: String::new(),
            state_keys: String::new(), styles: String::new(),
            line_type: LineType::ContainerEnd,
            tag: Some(String::from(tag)),
            config: None,
            constraints: BTreeMap::new(),
            cols: BTreeMap::new(),
            responsive: BTreeMap::new(),
            validates: BTreeMap::new(),
            animates: BTreeMap::new(),
            hrefs: BTreeMap::new(),
            popovers: BTreeMap::new(),
        }
    }
}

pub fn parse(input: &str) -> Vec<Line> {
    let mut lines = Vec::new();

    for raw in input.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() { continue; }

        if trimmed == "@end each" {
            lines.push(Line::each_end());
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@end ") {
            lines.push(Line::container_end(rest.trim()));
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@each:") {
            lines.push(Line::each_start(rest.trim()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("@each ") {
            lines.push(Line::each_start(rest.trim()));
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
    let mut cols: BTreeMap<usize, (u8, u8)> = BTreeMap::new();
    let mut responsive_map: BTreeMap<usize, Vec<(String, u8, u8)>> = BTreeMap::new();
    let mut validates_map: BTreeMap<usize, String> = BTreeMap::new();
    let mut animates_map: BTreeMap<usize, String> = BTreeMap::new();
    let mut hrefs_map: BTreeMap<usize, String> = BTreeMap::new();
    let mut popovers_map: BTreeMap<usize, String> = BTreeMap::new();

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
                if let Some(c) = comp.col {
                    cols.insert(pos, c);
                }
                if !comp.responsive.is_empty() {
                    responsive_map.insert(pos, comp.responsive);
                }
                if let Some(v) = comp.validate {
                    validates_map.insert(pos, v);
                }
                if let Some(a) = comp.animate {
                    animates_map.insert(pos, a);
                }
                if let Some(h) = comp.href {
                    hrefs_map.insert(pos, h);
                }
                if let Some(p) = comp.popover {
                    popovers_map.insert(pos, p);
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
    line.cols = cols;
    line.responsive = responsive_map;
    line.validates = validates_map;
    line.animates = animates_map;
    line.hrefs = hrefs_map;
    line.popovers = popovers_map;
    line
}

struct ParsedComponent {
    comp_char: char,
    binding: Option<String>,
    label: String,
    style: Option<String>,
    width: usize,
    constraints: Vec<String>,
    col: Option<(u8, u8)>,
    responsive: Vec<(String, u8, u8)>,
    validate: Option<String>,
    animate: Option<String>,
    href: Option<String>,
    popover: Option<String>,
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
        "radio" => RADIO,
        "select" => SELECT,
        "textarea" => TEXTAREA,
        "image" => IMAGE,
        "link" => LINK,
        "label" => LABEL,
        "divider" => DIVIDER,
        "spacer" => SPACER,
        "pill" => PILL,
        "badge" => BADGE,
        "progress" => PROGRESS,
        "sparkline" => SPARKLINE,
        _ if binding.is_some() => LABEL,
        _ => return None,
    };

    let mut label = String::new();
    let mut style = None;
    let mut comp_constraints = Vec::new();
    let mut col = None;
    let mut responsive = Vec::new();
    let mut validate = None;
    let mut animate = None;
    let mut href = None;
    let mut popover = None;

    for part in &parts[1..] {
        if part.starts_with('"') && part.ends_with('"') && part.len() >= 2 {
            label = part[1..part.len() - 1].to_string();
        } else if part.starts_with("href:") || part.starts_with("href=") {
            href = Some(String::from(&part[5..]));
        } else if part.starts_with("route:") {
            href = Some(format!("route:{}", &part[6..]));
        } else if part.starts_with("fetch:") {
            href = Some(format!("fetch:{}", &part[6..]));
        } else if part.starts_with("popover:") {
            popover = Some(part[8..].trim_matches('"').to_string());
        } else if part.starts_with("animate:") {
            animate = Some(String::from(&part[8..]));
        } else if part.starts_with("validate:") {
            validate = Some(String::from(&part[9..]));
        } else if part.starts_with("col-") {
            col = parse_col(part);
        } else if part.starts_with("sm:") || part.starts_with("md:") || part.starts_with("lg:") || part.starts_with("xl:") {
            if let Some(r) = parse_responsive_col(part) {
                responsive.push(r);
            }
        } else if is_constraint_token(part) {
            comp_constraints.push(part.clone());
        } else if is_style(part) {
            style = Some(part.clone());
        }
    }

    let default_width = match comp_char {
        TEXT_INPUT | PASSWORD => 10,
        TEXTAREA => 20,
        BUTTON => label.len().max(6),
        CHECKBOX | RADIO => 3,
        SELECT => 12,
        IMAGE => 8,
        LINK => label.len().max(4),
        PILL => label.len().max(3),
        BADGE => label.len().max(2),
        PROGRESS => 15,
        SPARKLINE => 10,
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
        col,
        responsive,
        validate,
        animate,
        href,
        popover,
    }, end))
}

fn parse_responsive_col(part: &str) -> Option<(String, u8, u8)> {
    let colon = part.find(':')?;
    let breakpoint = String::from(&part[..colon]);
    let col_part = &part[colon + 1..];
    let (n, total) = parse_col(col_part)?;
    Some((breakpoint, n, total))
}

fn parse_col(s: &str) -> Option<(u8, u8)> {
    let rest = s.strip_prefix("col-")?;
    if let Some(bracket) = rest.find('[') {
        let n: u8 = rest[..bracket].parse().ok()?;
        let total: u8 = rest[bracket + 1..].trim_end_matches(']').parse().ok()?;
        if n > 0 && total > 0 && n <= total { Some((n, total)) } else { None }
    } else {
        let n: u8 = rest.parse().ok()?;
        if n > 0 && n <= 12 { Some((n, 12)) } else { None }
    }
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
    matches!(s, "primary" | "secondary" | "danger" | "warning" | "info"
        | "dark" | "light" | "outline" | "ghost")
    || (s.len() == 1 && s.as_bytes()[0] >= b'1' && s.as_bytes()[0] <= b'9')
}

fn style_char(s: &str) -> char {
    match s {
        "primary" => 'p',
        "secondary" => 's',
        "danger" => 'd',
        "warning" => 'w',
        "info" => 'i',
        "dark" => 'k',
        "light" => 'l',
        "outline" => 'o',
        "ghost" => 'g',
        "1" => '1', "2" => '2', "3" => '3',
        "4" => '4', "5" => '5', "6" => '6',
        "7" => '7', "8" => '8', "9" => '9',
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

    #[test]
    fn test_col_12_grid() {
        let lines = parse("| {input:name col-8}  {button:go \"Go\" col-4}");
        assert_eq!(lines[0].cols.len(), 2);
        let cols: Vec<&(u8, u8)> = lines[0].cols.values().collect();
        assert!(cols.contains(&&(8, 12)));
        assert!(cols.contains(&&(4, 12)));
    }

    #[test]
    fn test_col_custom_grid() {
        let lines = parse("| {input:name col-3[5]}");
        let cols: Vec<&(u8, u8)> = lines[0].cols.values().collect();
        assert_eq!(cols[0], &(3, 5));
    }

    #[test]
    fn test_responsive_col() {
        let lines = parse("| {input:name sm:col-12 md:col-6 lg:col-4}");
        let resp = &lines[0].responsive;
        assert!(!resp.is_empty());
        let vals: Vec<&Vec<(String, u8, u8)>> = resp.values().collect();
        let flat = &vals[0];
        assert!(flat.iter().any(|(bp, n, _)| bp == "sm" && *n == 12));
        assert!(flat.iter().any(|(bp, n, _)| bp == "md" && *n == 6));
        assert!(flat.iter().any(|(bp, n, _)| bp == "lg" && *n == 4));
    }

    #[test]
    fn test_responsive_flows_to_vnode() {
        use crate::render;
        let lines = parse("| {input:name sm:col-12 lg:col-6}");
        let vdom = render::render(&lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-responsive"));
        assert!(html.contains("sm:12,12"));
        assert!(html.contains("lg:6,12"));
    }

    #[test]
    fn test_href_parse() {
        let lines = parse("| {link:docs \"Docs\" href:/pst/docs}");
        let hrefs: Vec<&String> = lines[0].hrefs.values().collect();
        assert_eq!(hrefs.len(), 1);
        assert_eq!(hrefs[0], "/pst/docs");
    }

    #[test]
    fn test_route_parse() {
        let lines = parse("| {button:go \"Go\" route:home}");
        let hrefs: Vec<&String> = lines[0].hrefs.values().collect();
        assert_eq!(hrefs[0], "route:home");
    }

    #[test]
    fn test_fetch_parse() {
        let lines = parse("| {label:data fetch:/api/data}");
        let hrefs: Vec<&String> = lines[0].hrefs.values().collect();
        assert_eq!(hrefs[0], "fetch:/api/data");
    }

    #[test]
    fn test_popover_parse() {
        let lines = parse(r#"| {button:help "?" popover:"Click for help"}"#);
        let pops: Vec<&String> = lines[0].popovers.values().collect();
        assert_eq!(pops[0], "Click for help");
    }

    #[test]
    fn test_event_props_flow_to_vnode() {
        use crate::render;
        let lines = parse(r#"| {link:docs "Docs" href:/pst/docs popover:"Documentation"}"#);
        let vdom = render::render(&lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-href=\"/pst/docs\""));
        assert!(html.contains("data-popover=\"Documentation\""));
    }

    #[test]
    fn test_animate_parse() {
        let lines = parse("| {label:title \"Hello\" animate:fade}");
        let anims: Vec<&String> = lines[0].animates.values().collect();
        assert_eq!(anims.len(), 1);
        assert_eq!(anims[0], "fade");
    }

    #[test]
    fn test_animate_flows_to_vnode() {
        use crate::render;
        let lines = parse("| {button:go \"Go\" animate:slide}");
        let vdom = render::render(&lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-animate=\"slide\""));
    }

    #[test]
    fn test_validate_parse() {
        let lines = parse("| {input:email validate:required,email}");
        let vals: Vec<&String> = lines[0].validates.values().collect();
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0], "required,email");
    }

    #[test]
    fn test_validate_flows_to_vnode() {
        use crate::render;
        let lines = parse("| {input:pw validate:required,min:8}");
        let vdom = render::render(&lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-validate=\"required,min:8\""));
    }

    #[test]
    fn test_editor_parse() {
        let lines = parse("@editor bold italic code bind:notes\n| content\n@end editor");
        assert_eq!(lines[0].line_type, LineType::ContainerStart);
        assert_eq!(lines[0].tag, Some(String::from("editor")));
        assert_eq!(lines[0].config, Some(String::from("bold italic code bind:notes")));
        use crate::render;
        let vdom = render::render(&lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-editor"));
        assert!(html.contains("data-editor"));
        assert!(html.contains("data-features"));
        assert!(html.contains("data-bind"));
    }

    #[test]
    fn test_each_parse() {
        let lines = parse("@each:items\n| {label:name}\n@end each");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_type, LineType::EachStart);
        assert_eq!(lines[0].tag, Some(String::from("items")));
        assert_eq!(lines[2].line_type, LineType::EachEnd);
    }

    #[test]
    fn test_each_render_with_state() {
        use crate::state::{StateStore, StateValue};
        use crate::render;
        let mut state = StateStore::new();
        state.add_list_item("items", &[
            (String::from("name"), StateValue::Text(String::from("Apple"))),
        ]);
        state.add_list_item("items", &[
            (String::from("name"), StateValue::Text(String::from("Banana"))),
        ]);
        let lines = parse("@each:items\n| {label:name}\n@end each");
        let vdom = render::render_with_state(&lines, &state);
        let html = crate::html::to_html(&vdom);
        // Should have 2 rows rendered from the template
        assert!(html.contains("mc-row"));
    }

    #[test]
    fn test_container_config() {
        let lines = parse("@card padding:24,max-width:400px\n| Hello\n@end card");
        assert_eq!(lines[0].config, Some(String::from("padding:24,max-width:400px")));
        use crate::render;
        let vdom = render::render(&lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-config"));
    }

    #[test]
    fn test_col_flows_to_vnode() {
        use crate::render;
        let lines = parse("| {input:name col-6}");
        let vdom = render::render(&lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-col=\"6,12\""));
    }
}
