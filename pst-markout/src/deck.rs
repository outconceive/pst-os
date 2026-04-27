use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::vnode::VNode;

pub fn render_deck(tag_config: &str, content_lines: &[&str]) -> VNode {
    let mut name = String::new();
    let mut theme = String::from("dark");
    let mut bg = String::new();
    let mut fg = String::new();

    for token in tag_config.split_whitespace() {
        if let Some(v) = token.strip_prefix("theme:") { theme = String::from(v); }
        else if let Some(v) = token.strip_prefix("bg:") { bg = String::from(v); }
        else if let Some(v) = token.strip_prefix("fg:") { fg = String::from(v); }
        else if name.is_empty() { name = String::from(token); }
    }

    let slides = parse_slides(content_lines);
    let total = slides.len();
    let mut children = Vec::new();

    for (idx, slide) in slides.iter().enumerate() {
        let mut attrs = BTreeMap::new();
        let mut class = format!("mc-slide mc-slide-{}", slide.key);
        if let Some(ref layout) = slide.layout {
            class.push_str(&format!(" mc-layout-{}", layout));
        }
        attrs.insert(String::from("class"), class);
        attrs.insert(String::from("data-slide"), format!("{}", idx));
        attrs.insert(String::from("data-key"), slide.key.clone());
        attrs.insert(String::from("data-transition"), slide.transition.clone());
        attrs.insert(String::from("data-duration"), format!("{}", slide.duration));
        if !slide.notes.is_empty() {
            attrs.insert(String::from("data-notes"), slide.notes.clone());
        }
        if let Some(ref bg) = slide.bg {
            attrs.insert(String::from("data-bg"), bg.clone());
        }
        if let Some(auto) = slide.auto_advance {
            attrs.insert(String::from("data-auto"), format!("{}", auto));
        }

        let parsed = crate::parse::parse(&slide.content_raw);
        let inner = crate::render::render(&parsed);
        let inner_children = match inner {
            VNode::Element(el) => el.children,
            _ => vec![inner],
        };

        children.push(VNode::element_with_attrs("section", attrs, inner_children));
    }

    let mut deck_attrs = BTreeMap::new();
    deck_attrs.insert(String::from("class"), format!("mc-deck mc-deck-{} mc-theme-{}", name, theme));
    deck_attrs.insert(String::from("data-total"), format!("{}", total));
    if !bg.is_empty() { deck_attrs.insert(String::from("data-bg"), bg); }
    if !fg.is_empty() { deck_attrs.insert(String::from("data-fg"), fg); }

    VNode::element_with_attrs("div", deck_attrs, children)
}

struct SlideData {
    key: String,
    transition: String,
    duration: u32,
    notes: String,
    layout: Option<String>,
    bg: Option<String>,
    auto_advance: Option<u32>,
    content_raw: String,
}

fn parse_slides(lines: &[&str]) -> Vec<SlideData> {
    let mut slides = Vec::new();
    let mut current: Option<SlideData> = None;
    let mut content_buf = String::new();

    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@slide:") {
            if let Some(mut prev) = current.take() {
                prev.content_raw = content_buf.clone();
                slides.push(prev);
                content_buf.clear();
            }

            let mut key = String::new();
            let mut transition = String::from("none");
            let mut duration = 400u32;
            let mut notes = String::new();
            let mut layout = None;
            let mut bg = None;
            let mut auto_advance = None;

            let tokens = shell_split(rest);
            if !tokens.is_empty() { key = tokens[0].clone(); }
            for t in &tokens[1..] {
                if let Some(v) = t.strip_prefix("transition:") { transition = String::from(v); }
                else if let Some(v) = t.strip_prefix("duration:") { duration = v.parse().unwrap_or(400); }
                else if let Some(v) = t.strip_prefix("notes:") { notes = v.trim_matches('"').to_string(); }
                else if let Some(v) = t.strip_prefix("layout:") { layout = Some(String::from(v)); }
                else if let Some(v) = t.strip_prefix("bg:") { bg = Some(String::from(v)); }
                else if let Some(v) = t.strip_prefix("auto:") { auto_advance = v.parse().ok(); }
            }

            current = Some(SlideData { key, transition, duration, notes, layout, bg, auto_advance, content_raw: String::new() });
        } else if current.is_some() {
            if !content_buf.is_empty() { content_buf.push('\n'); }
            content_buf.push_str(trimmed);
        }
    }

    if let Some(mut prev) = current.take() {
        prev.content_raw = content_buf;
        slides.push(prev);
    }

    slides
}

fn shell_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in input.chars() {
        if ch == '"' { in_quotes = !in_quotes; current.push(ch); }
        else if ch == ' ' && !in_quotes {
            if !current.is_empty() { parts.push(current.clone()); current.clear(); }
        } else { current.push(ch); }
    }
    if !current.is_empty() { parts.push(current); }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_slide() {
        let lines = vec![
            "@slide:intro transition:fade duration:500",
            "{label:title \"Hello\" primary}",
        ];
        let slides = parse_slides(&lines);
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].key, "intro");
        assert_eq!(slides[0].transition, "fade");
        assert_eq!(slides[0].duration, 500);
    }

    #[test]
    fn test_parse_multiple_slides() {
        let lines = vec![
            "@slide:a transition:fade",
            "{label:t \"Slide A\"}",
            "@slide:b transition:zoom",
            "{label:t \"Slide B\"}",
        ];
        let slides = parse_slides(&lines);
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].key, "a");
        assert_eq!(slides[1].key, "b");
        assert_eq!(slides[1].transition, "zoom");
    }

    #[test]
    fn test_render_deck() {
        let lines = vec![
            "@slide:intro transition:fade",
            "| {label:title \"PST OS\" primary}",
            "@slide:end transition:none",
            "| {label:t \"Thanks\"}",
        ];
        let vdom = render_deck("talk theme:dark", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-deck"));
        assert!(html.contains("mc-theme-dark"));
        assert!(html.contains("mc-slide"));
        assert!(html.contains("data-transition=\"fade\""));
        assert!(html.contains("data-total=\"2\""));
    }

    #[test]
    fn test_slide_notes() {
        let lines = vec![
            "@slide:demo transition:fade notes:\"Run the demo here\"",
            "| {label:t \"Demo\"}",
        ];
        let slides = parse_slides(&lines);
        assert_eq!(slides[0].notes, "Run the demo here");
    }
}
