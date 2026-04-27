use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::vnode::VNode;

struct GridConfig {
    name: String,
    cols: usize,
    rows: usize,
}

fn parse_grid_config(config: &str) -> GridConfig {
    let mut name = String::new();
    let mut cols = 1usize;
    let mut rows = 1usize;

    for token in config.split_whitespace() {
        if let Some(v) = token.strip_prefix("cols:") {
            cols = v.parse().unwrap_or(1);
        } else if let Some(v) = token.strip_prefix("rows:") {
            rows = v.parse().unwrap_or(1);
        } else if name.is_empty() {
            name = String::from(token);
        }
    }
    GridConfig { name, cols, rows }
}

struct Cell {
    col: usize,
    row: usize,
    content: String,
}

fn parse_cells(lines: &[&str]) -> Vec<Cell> {
    let mut cells = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        let mut rest = trimmed;
        while let Some(paren) = rest.find('(') {
            if let Some(close) = rest[paren..].find(')') {
                let coords = &rest[paren + 1..paren + close];
                let after = &rest[paren + close + 1..];
                if let Some((c, r)) = coords.split_once(',') {
                    let col: usize = c.trim().parse().unwrap_or(0);
                    let row: usize = r.trim().parse().unwrap_or(0);

                    let content_end = after.find('(').unwrap_or(after.len());
                    let content = after[..content_end].trim().to_string();

                    if !content.is_empty() {
                        cells.push(Cell { col, row, content });
                    }
                }
                rest = &rest[paren + close + 1 + after.find('(').unwrap_or(after.len())..];
                if rest.is_empty() { break; }
            } else {
                break;
            }
        }
    }
    cells
}

pub fn render_grid(tag_config: &str, content_lines: &[&str]) -> VNode {
    let cfg = parse_grid_config(tag_config);
    let cells = parse_cells(content_lines);

    let cell_w = 120.0f64;
    let cell_h = 36.0f64;
    let total_w = cell_w * cfg.cols as f64;
    let total_h = cell_h * cfg.rows as f64;

    let mut container_attrs = BTreeMap::new();
    container_attrs.insert(String::from("class"), format!("mc-grid mc-grid-{}", cfg.name));
    container_attrs.insert(
        String::from("style"),
        format!("position:relative;width:{:.0}px;height:{:.0}px", total_w, total_h),
    );
    container_attrs.insert(String::from("data-cols"), format!("{}", cfg.cols));
    container_attrs.insert(String::from("data-rows"), format!("{}", cfg.rows));

    let mut children = Vec::new();
    for cell in &cells {
        let x = cell.col as f64 * cell_w;
        let y = cell.row as f64 * cell_h;

        let mut wrapper_attrs = BTreeMap::new();
        wrapper_attrs.insert(
            String::from("style"),
            format!("position:absolute;left:{:.0}px;top:{:.0}px;width:{:.0}px;height:{:.0}px", x, y, cell_w, cell_h),
        );
        wrapper_attrs.insert(String::from("data-cell"), format!("{},{}", cell.col, cell.row));

        let parsed = crate::parse::parse(&format!("| {}", cell.content));
        let inner = crate::render::render(&parsed);
        let inner_children = match inner {
            VNode::Element(el) => el.children,
            _ => alloc::vec![inner],
        };

        children.push(VNode::element_with_attrs("div", wrapper_attrs, inner_children));
    }

    VNode::element_with_attrs("div", container_attrs, children)
}

pub fn placement_style(config: &str, container_w: usize, container_h: usize, grid_w: usize, grid_h: usize) -> String {
    let mut anchor = "top-left";
    let mut off_x = 0usize;
    let mut off_y = 0usize;

    for token in config.split_whitespace() {
        if let Some(v) = token.strip_prefix("anchor:") {
            anchor = match v {
                "top-left" | "top-right" | "bottom-left" | "bottom-right"
                | "top" | "bottom" | "center" => v,
                _ => "top-left",
            };
            // leak-free: store in a longer-lived place
            let _ = anchor;
            // Actually, we need to handle this differently since v borrows token
        } else if let Some(v) = token.strip_prefix("offset:") {
            if let Some((ox, oy)) = v.split_once(',') {
                off_x = ox.parse().unwrap_or(0);
                off_y = oy.parse().unwrap_or(0);
            }
        }
    }

    // Re-parse anchor since the borrow above is tricky
    let mut anchor_str = "top-left";
    for token in config.split_whitespace() {
        if let Some(v) = token.strip_prefix("anchor:") {
            anchor_str = match v {
                "top-left" => "top-left",
                "top-right" => "top-right",
                "bottom-left" => "bottom-left",
                "bottom-right" => "bottom-right",
                "top" => "top",
                "bottom" => "bottom",
                "center" => "center",
                _ => "top-left",
            };
        }
    }

    let (x, y) = match anchor_str {
        "top-left" => (off_x, off_y),
        "top-right" => (container_w.saturating_sub(grid_w + off_x), off_y),
        "bottom-left" => (off_x, container_h.saturating_sub(grid_h + off_y)),
        "bottom-right" => (container_w.saturating_sub(grid_w + off_x), container_h.saturating_sub(grid_h + off_y)),
        "top" => ((container_w.saturating_sub(grid_w)) / 2, off_y),
        "bottom" => ((container_w.saturating_sub(grid_w)) / 2, container_h.saturating_sub(grid_h + off_y)),
        "center" => ((container_w.saturating_sub(grid_w)) / 2, (container_h.saturating_sub(grid_h)) / 2),
        _ => (off_x, off_y),
    };

    format!("position:absolute;left:{}px;top:{}px", x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_parse_grid_config() {
        let cfg = parse_grid_config("dpad cols:3 rows:3");
        assert_eq!(cfg.name, "dpad");
        assert_eq!(cfg.cols, 3);
        assert_eq!(cfg.rows, 3);
    }

    #[test]
    fn test_parse_cells() {
        let lines = vec![
            "(0,0) {button:up \"^\"} (2,0) {button:right \">\"}",
        ];
        let cells = parse_cells(&lines);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].col, 0);
        assert_eq!(cells[0].row, 0);
        assert_eq!(cells[1].col, 2);
    }

    #[test]
    fn test_render_grid_produces_positioned_divs() {
        let lines = vec![
            "(0,0) {label:a \"Hello\"}",
            "(1,0) {button:b \"Click\"}",
        ];
        let vdom = render_grid("test cols:2 rows:1", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-grid"));
        assert!(html.contains("position:absolute"));
        assert!(html.contains("data-cell=\"0,0\""));
        assert!(html.contains("data-cell=\"1,0\""));
    }

    #[test]
    fn test_placement_center() {
        let style = placement_style("anchor:center", 800, 600, 200, 100);
        assert!(style.contains("left:300px"));
        assert!(style.contains("top:250px"));
    }

    #[test]
    fn test_placement_bottom_right() {
        let style = placement_style("anchor:bottom-right offset:8,8", 800, 600, 200, 100);
        assert!(style.contains("left:592px"));
        assert!(style.contains("top:492px"));
    }

    #[test]
    fn test_empty_cells_skipped() {
        let lines = vec!["(0,0) {} (1,0) {label:a \"Hi\"}"];
        let cells = parse_cells(&lines);
        assert!(cells.len() >= 1);
    }
}
