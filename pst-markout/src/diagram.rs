use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::vnode::VNode;

#[derive(Debug, Clone, PartialEq)]
enum CellKind {
    Shape(ShapeType, String, String),
    Connector(String),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
enum ShapeType {
    Rectangle,
    Rounded,
    Diamond,
    Circle,
    Parallelogram,
    Subprocess,
}

pub fn render_diagram(tag_config: &str, content_lines: &[&str]) -> VNode {
    let mut name = String::new();
    let mut cols = 1usize;
    let mut rows = 1usize;

    for token in tag_config.split_whitespace() {
        if let Some(v) = token.strip_prefix("cols:") { cols = v.parse().unwrap_or(1); }
        else if let Some(v) = token.strip_prefix("rows:") { rows = v.parse().unwrap_or(1); }
        else if name.is_empty() { name = String::from(token); }
    }

    let mut grid: BTreeMap<(usize, usize), CellKind> = BTreeMap::new();

    for line in content_lines {
        let trimmed = line.trim();
        let mut rest = trimmed;
        while let Some(paren) = rest.find('(') {
            if let Some(close) = rest[paren..].find(')') {
                let coords = &rest[paren + 1..paren + close];
                let after = rest[paren + close + 1..].trim();
                if let Some((c, r)) = coords.split_once(',') {
                    let col: usize = c.trim().parse().unwrap_or(0);
                    let row: usize = r.trim().parse().unwrap_or(0);

                    let content_end = after.find('(').unwrap_or(after.len());
                    let content = after[..content_end].trim();

                    if !content.is_empty() {
                        let cell = parse_cell_content(content, col, row);
                        grid.insert((col, row), cell);
                    }
                }
                let next_start = paren + close + 1;
                let remaining = &rest[next_start..];
                match remaining.find('(') {
                    Some(p) => rest = &rest[next_start + p..],
                    None => break,
                }
            } else {
                break;
            }
        }
    }

    let cell_w = 120usize;
    let cell_h = 48usize;

    let mut container_attrs = BTreeMap::new();
    container_attrs.insert(String::from("class"), format!("mc-diagram mc-diagram-{}", name));
    container_attrs.insert(
        String::from("style"),
        format!("position:relative;width:{}px;height:{}px", cols * cell_w, rows * cell_h),
    );
    container_attrs.insert(String::from("data-cols"), format!("{}", cols));
    container_attrs.insert(String::from("data-rows"), format!("{}", rows));

    let mut children = Vec::new();

    for (&(col, row), cell) in &grid {
        let x = col * cell_w;
        let y = row * cell_h;

        let mut wrapper_attrs = BTreeMap::new();
        wrapper_attrs.insert(
            String::from("style"),
            format!("position:absolute;left:{}px;top:{}px;width:{}px;height:{}px", x, y, cell_w, cell_h),
        );
        wrapper_attrs.insert(String::from("data-cell"), format!("{},{}", col, row));

        match cell {
            CellKind::Shape(shape_type, label, style) => {
                let shape_class = match shape_type {
                    ShapeType::Rectangle => "mc-shape-rect",
                    ShapeType::Rounded => "mc-shape-rounded",
                    ShapeType::Diamond => "mc-shape-diamond",
                    ShapeType::Circle => "mc-shape-circle",
                    ShapeType::Parallelogram => "mc-shape-para",
                    ShapeType::Subprocess => "mc-shape-subprocess",
                };
                let mut class = format!("mc-diagram-shape {}", shape_class);
                if !style.is_empty() {
                    class.push_str(&format!(" mc-{}", style));
                }
                wrapper_attrs.insert(String::from("class"), class);
                wrapper_attrs.insert(String::from("data-type"), String::from("shape"));
                children.push(VNode::element_with_attrs("div", wrapper_attrs, vec![VNode::text(label)]));
            }
            CellKind::Connector(dir) => {
                wrapper_attrs.insert(String::from("class"), String::from("mc-diagram-connector"));
                wrapper_attrs.insert(String::from("data-type"), String::from("connector"));
                wrapper_attrs.insert(String::from("data-dir"), dir.clone());
                children.push(VNode::element_with_attrs("div", wrapper_attrs, vec![VNode::text(dir)]));
            }
            CellKind::Empty => {}
        }
    }

    VNode::element_with_attrs("div", container_attrs, children)
}

fn parse_cell_content(s: &str, col: usize, _row: usize) -> CellKind {
    let trimmed = s.trim();

    if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
        let label = trimmed[2..trimmed.len()-2].trim();
        let (label, style) = split_label_style(label);
        return CellKind::Shape(ShapeType::Subprocess, label, style);
    }
    if trimmed.starts_with("((") && trimmed.ends_with("))") {
        let label = trimmed[2..trimmed.len()-2].trim();
        let (label, style) = split_label_style(label);
        return CellKind::Shape(ShapeType::Circle, label, style);
    }
    if trimmed.starts_with("[/") && trimmed.ends_with("/]") {
        let label = trimmed[2..trimmed.len()-2].trim();
        let (label, style) = split_label_style(label);
        return CellKind::Shape(ShapeType::Parallelogram, label, style);
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let label = trimmed[1..trimmed.len()-1].trim();
        let (label, style) = split_label_style(label);
        return CellKind::Shape(ShapeType::Rectangle, label, style);
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let label = trimmed[1..trimmed.len()-1].trim();
        let (label, style) = split_label_style(label);
        return CellKind::Shape(ShapeType::Rounded, label, style);
    }
    if trimmed.starts_with('<') && trimmed.ends_with('>') {
        let label = trimmed[1..trimmed.len()-1].trim();
        let (label, style) = split_label_style(label);
        return CellKind::Shape(ShapeType::Diamond, label, style);
    }

    match trimmed {
        "-->" | "--" | "<--" | "<->" | "..>" | "<.." | "|" | "^" | "|^|" => {
            CellKind::Connector(String::from(trimmed))
        }
        _ => {
            if col % 2 == 1 {
                CellKind::Connector(String::from(trimmed))
            } else {
                CellKind::Empty
            }
        }
    }
}

fn split_label_style(s: &str) -> (String, String) {
    let parts: Vec<&str> = s.rsplitn(2, ' ').collect();
    if parts.len() == 2 && is_style(parts[0]) {
        (parts[1].to_string(), parts[0].to_string())
    } else {
        (s.to_string(), String::new())
    }
}

fn is_style(s: &str) -> bool {
    matches!(s, "primary" | "secondary" | "danger" | "warning" | "info"
        | "success" | "muted" | "dark" | "light")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rectangle() {
        let cell = parse_cell_content("[Start]", 0, 0);
        assert_eq!(cell, CellKind::Shape(ShapeType::Rectangle, String::from("Start"), String::new()));
    }

    #[test]
    fn test_parse_diamond() {
        let cell = parse_cell_content("<Error?>", 2, 0);
        assert_eq!(cell, CellKind::Shape(ShapeType::Diamond, String::from("Error?"), String::new()));
    }

    #[test]
    fn test_parse_connector() {
        let cell = parse_cell_content("-->", 1, 0);
        assert_eq!(cell, CellKind::Connector(String::from("-->")));
    }

    #[test]
    fn test_parse_styled_shape() {
        let cell = parse_cell_content("[Done success]", 0, 0);
        assert_eq!(cell, CellKind::Shape(ShapeType::Rectangle, String::from("Done"), String::from("success")));
    }

    #[test]
    fn test_render_diagram() {
        let lines = vec![
            "(0,0) [Start]  (1,0) -->  (2,0) [End]",
        ];
        let vdom = render_diagram("flow cols:3 rows:1", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-diagram"));
        assert!(html.contains("mc-shape-rect"));
        assert!(html.contains("mc-diagram-connector"));
        assert!(html.contains("Start"));
        assert!(html.contains("End"));
    }

    #[test]
    fn test_subprocess_and_circle() {
        let cell1 = parse_cell_content("[[Sub]]", 0, 0);
        assert_eq!(cell1, CellKind::Shape(ShapeType::Subprocess, String::from("Sub"), String::new()));
        let cell2 = parse_cell_content("((DB))", 0, 0);
        assert_eq!(cell2, CellKind::Shape(ShapeType::Circle, String::from("DB"), String::new()));
    }
}
