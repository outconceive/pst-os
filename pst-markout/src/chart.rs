use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::vnode::VNode;

pub fn render_pie(tag_config: &str, content_lines: &[&str]) -> VNode {
    let mut name = String::new();
    let mut radius = 80usize;
    let mut inner = 0usize;
    let mut start = 0usize;

    for token in tag_config.split_whitespace() {
        if let Some(v) = token.strip_prefix("radius:") { radius = v.parse().unwrap_or(80); }
        else if let Some(v) = token.strip_prefix("inner:") { inner = v.parse().unwrap_or(0); }
        else if let Some(v) = token.strip_prefix("start:") { start = v.parse().unwrap_or(0); }
        else if name.is_empty() { name = String::from(token); }
    }

    let slices = parse_slices(content_lines);
    let total: f64 = slices.iter().map(|s| s.value).sum();

    let mut children = Vec::new();
    let mut cumulative = start as f64;

    for slice in &slices {
        let pct = if total > 0.0 { slice.value / total * 100.0 } else { 0.0 };
        let start_deg = cumulative;
        let end_deg = cumulative + (slice.value / total * 360.0);

        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("class"), format!("mc-pie-slice mc-{}", slice.style));
        attrs.insert(String::from("data-key"), slice.key.clone());
        attrs.insert(String::from("data-value"), format!("{}", slice.value));
        attrs.insert(String::from("data-pct"), format!("{:.1}", pct));
        attrs.insert(String::from("data-start"), format!("{:.1}", start_deg));
        attrs.insert(String::from("data-end"), format!("{:.1}", end_deg));
        if slice.explode > 0 {
            attrs.insert(String::from("data-explode"), format!("{}", slice.explode));
        }

        children.push(VNode::element_with_attrs("div", attrs, vec![VNode::text(&slice.label)]));
        cumulative = end_deg;
    }

    let mut container_attrs = BTreeMap::new();
    container_attrs.insert(String::from("class"), format!("mc-pie mc-pie-{}", name));
    container_attrs.insert(String::from("data-radius"), format!("{}", radius));
    if inner > 0 {
        container_attrs.insert(String::from("data-inner"), format!("{}", inner));
    }
    container_attrs.insert(
        String::from("style"),
        format!("width:{}px;height:{}px", radius * 2, radius * 2),
    );

    VNode::element_with_attrs("div", container_attrs, children)
}

pub fn render_bar(tag_config: &str, content_lines: &[&str]) -> VNode {
    let mut name = String::new();
    let mut height = 200usize;
    let mut direction = String::from("vertical");
    let mut stacked = false;

    for token in tag_config.split_whitespace() {
        if let Some(v) = token.strip_prefix("height:") { height = v.parse().unwrap_or(200); }
        else if let Some(v) = token.strip_prefix("direction:") { direction = String::from(v); }
        else if token == "stacked:true" { stacked = true; }
        else if name.is_empty() { name = String::from(token); }
    }

    let bars = parse_bars(content_lines);
    let max_val = bars.iter().map(|b| b.value).fold(0.0f64, f64::max).max(1.0);

    let mut children = Vec::new();
    let bar_width = if !bars.is_empty() { 400 / bars.len() } else { 40 };

    for bar in &bars {
        let bar_h = (bar.value / max_val * height as f64) as usize;

        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("class"), format!("mc-bar mc-{}", bar.style));
        attrs.insert(String::from("data-key"), bar.key.clone());
        attrs.insert(String::from("data-value"), format!("{}", bar.value));
        if direction == "horizontal" {
            attrs.insert(String::from("style"), format!("width:{}px;height:{}px", bar_h, bar_width));
        } else {
            attrs.insert(String::from("style"), format!("width:{}px;height:{}px", bar_width, bar_h));
        }
        children.push(VNode::element_with_attrs("div", attrs, vec![VNode::text(&bar.label)]));
    }

    let mut container_attrs = BTreeMap::new();
    container_attrs.insert(String::from("class"), format!("mc-bar-chart mc-bar-chart-{}", name));
    container_attrs.insert(String::from("data-direction"), direction);
    container_attrs.insert(String::from("style"), format!("height:{}px", height));
    if stacked { container_attrs.insert(String::from("data-stacked"), String::from("true")); }

    VNode::element_with_attrs("div", container_attrs, children)
}

pub fn render_line(tag_config: &str, content_lines: &[&str]) -> VNode {
    let mut name = String::new();
    let mut width = 400usize;
    let mut height = 200usize;

    for token in tag_config.split_whitespace() {
        if let Some(v) = token.strip_prefix("width:") { width = v.parse().unwrap_or(400); }
        else if let Some(v) = token.strip_prefix("height:") { height = v.parse().unwrap_or(200); }
        else if name.is_empty() { name = String::from(token); }
    }

    let series_list = parse_series(content_lines);
    let mut children = Vec::new();

    for series in &series_list {
        let points_str: String = series.points.iter()
            .map(|p| format!("{}", p))
            .collect::<Vec<_>>()
            .join(",");

        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("class"), format!("mc-line-series mc-{}", series.style));
        attrs.insert(String::from("data-key"), series.key.clone());
        attrs.insert(String::from("data-points"), points_str);
        if series.fill { attrs.insert(String::from("data-fill"), String::from("true")); }
        children.push(VNode::element_with_attrs("div", attrs, vec![VNode::text(&series.label)]));
    }

    let mut container_attrs = BTreeMap::new();
    container_attrs.insert(String::from("class"), format!("mc-line-chart mc-line-chart-{}", name));
    container_attrs.insert(String::from("style"), format!("width:{}px;height:{}px", width, height));

    VNode::element_with_attrs("div", container_attrs, children)
}

struct Slice { key: String, value: f64, style: String, label: String, explode: usize }
struct Bar { key: String, value: f64, style: String, label: String }
struct Series { key: String, points: Vec<f64>, style: String, label: String, fill: bool }

fn parse_slices(lines: &[&str]) -> Vec<Slice> {
    let mut slices = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if let Some(inner) = extract_braces(trimmed) {
            let tokens = shell_split(&inner);
            if tokens.is_empty() { continue; }
            let (kind, binding) = split_kind_binding(&tokens[0]);
            if kind != "slice" { continue; }
            let key = binding.unwrap_or_default();
            let mut value = 0.0f64;
            let mut style = String::from("primary");
            let mut label = String::new();
            let mut explode = 0usize;
            for t in &tokens[1..] {
                if let Some(v) = t.strip_prefix("value:") { value = v.parse().unwrap_or(0.0); }
                else if let Some(v) = t.strip_prefix("explode:") { explode = v.parse().unwrap_or(0); }
                else if t.starts_with('"') && t.ends_with('"') { label = t[1..t.len()-1].to_string(); }
                else if is_style(t) { style = t.clone(); }
            }
            slices.push(Slice { key, value, style, label, explode });
        }
    }
    slices
}

fn parse_bars(lines: &[&str]) -> Vec<Bar> {
    let mut bars = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if let Some(inner) = extract_braces(trimmed) {
            let tokens = shell_split(&inner);
            if tokens.is_empty() { continue; }
            let (kind, binding) = split_kind_binding(&tokens[0]);
            if kind != "bar" { continue; }
            let key = binding.unwrap_or_default();
            let mut value = 0.0f64;
            let mut style = String::from("primary");
            let mut label = String::new();
            for t in &tokens[1..] {
                if let Some(v) = t.strip_prefix("value:") { value = v.parse().unwrap_or(0.0); }
                else if t.starts_with('"') && t.ends_with('"') { label = t[1..t.len()-1].to_string(); }
                else if is_style(t) { style = t.clone(); }
            }
            bars.push(Bar { key, value, style, label });
        }
    }
    bars
}

fn parse_series(lines: &[&str]) -> Vec<Series> {
    let mut series = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if let Some(inner) = extract_braces(trimmed) {
            let tokens = shell_split(&inner);
            if tokens.is_empty() { continue; }
            let (kind, binding) = split_kind_binding(&tokens[0]);
            if kind != "series" { continue; }
            let key = binding.unwrap_or_default();
            let mut points = Vec::new();
            let mut style = String::from("primary");
            let mut label = String::new();
            let mut fill = false;
            for t in &tokens[1..] {
                if let Some(v) = t.strip_prefix("points:") {
                    points = v.split(',').filter_map(|p| p.trim().parse().ok()).collect();
                } else if t.starts_with('"') && t.ends_with('"') {
                    label = t[1..t.len()-1].to_string();
                } else if t == "fill" {
                    fill = true;
                } else if is_style(t) {
                    style = t.clone();
                }
            }
            series.push(Series { key, points, style, label, fill });
        }
    }
    series
}

fn extract_braces(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let mut depth = 0;
    let mut end = start;
    for (i, c) in s[start..].char_indices() {
        if c == '{' { depth += 1; }
        if c == '}' { depth -= 1; if depth == 0 { end = start + i; break; } }
    }
    if depth == 0 { Some(s[start+1..end].to_string()) } else { None }
}

fn split_kind_binding(s: &str) -> (String, Option<String>) {
    if let Some(idx) = s.find(':') {
        let k = s[..idx].to_string();
        let b = s[idx+1..].to_string();
        (k, if b.is_empty() { None } else { Some(b) })
    } else {
        (s.to_string(), None)
    }
}

fn is_style(s: &str) -> bool {
    matches!(s, "primary" | "secondary" | "danger" | "warning" | "info"
        | "success" | "muted" | "dark" | "light")
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
    fn test_pie_chart() {
        let lines = vec![
            "{slice:rust value:45 primary \"Rust\"}",
            "{slice:js value:25 warning \"JS\"}",
        ];
        let vdom = render_pie("usage radius:80", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-pie"));
        assert!(html.contains("mc-pie-slice"));
        assert!(html.contains("Rust"));
        assert!(html.contains("data-value=\"45\""));
    }

    #[test]
    fn test_bar_chart() {
        let lines = vec![
            "{bar:a value:85 primary \"Rust\"}",
            "{bar:b value:60 warning \"JS\"}",
        ];
        let vdom = render_bar("langs height:200", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-bar-chart"));
        assert!(html.contains("mc-bar"));
        assert!(html.contains("Rust"));
    }

    #[test]
    fn test_line_chart() {
        let lines = vec![
            "{series:cpu points:20,45,30,60 primary \"CPU\"}",
        ];
        let vdom = render_line("metrics width:400 height:200", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-line-chart"));
        assert!(html.contains("data-points=\"20,45,30,60\""));
    }

    #[test]
    fn test_pie_normalization() {
        let lines = vec![
            "{slice:a value:340 primary \"A\"}",
            "{slice:b value:460 danger \"B\"}",
        ];
        let vdom = render_pie("votes radius:60", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-pct=\"42.5\""));
    }

    #[test]
    fn test_donut() {
        let lines = vec!["{slice:x value:100 primary \"X\"}"];
        let vdom = render_pie("d radius:80 inner:40", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("data-inner=\"40\""));
    }
}
