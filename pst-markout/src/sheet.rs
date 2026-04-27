use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::vnode::VNode;

#[derive(Debug, Clone)]
enum CellValue {
    Empty,
    Text(String),
    Number(f64),
    Compute(ComputeOp),
}

#[derive(Debug, Clone)]
struct ComputeOp {
    op: String,
    direction: String,
    until: String,
    format: String,
}

pub fn render_sheet(tag_config: &str, content_lines: &[&str]) -> VNode {
    let mut name = String::new();
    let mut cols = 1usize;
    let mut rows = 1usize;

    for token in tag_config.split_whitespace() {
        if let Some(v) = token.strip_prefix("cols:") { cols = v.parse().unwrap_or(1); }
        else if let Some(v) = token.strip_prefix("rows:") { rows = v.parse().unwrap_or(1); }
        else if name.is_empty() { name = String::from(token); }
    }

    let mut grid: BTreeMap<(usize, usize), CellValue> = BTreeMap::new();
    let mut styles: BTreeMap<(usize, usize), String> = BTreeMap::new();

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
                        let (val, style) = parse_cell(content);
                        grid.insert((col, row), val);
                        if !style.is_empty() {
                            styles.insert((col, row), style);
                        }
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

    // Evaluate compute cells via simple dependency resolution
    let computed = evaluate_grid(&grid, cols, rows);

    // Build VNode table
    let mut table_rows = Vec::new();
    for r in 0..rows {
        let mut row_cells = Vec::new();
        for c in 0..cols {
            let key = (c, r);
            let display = computed.get(&key).cloned().unwrap_or_default();
            let mut attrs = BTreeMap::new();
            let mut class = String::from("mc-sheet-cell");
            if let Some(style) = styles.get(&key) {
                class.push_str(&format!(" mc-{}", style));
            }
            attrs.insert(String::from("class"), class);
            attrs.insert(String::from("data-cell"), format!("{},{}", c, r));
            row_cells.push(VNode::element_with_attrs("div", attrs, vec![VNode::text(&display)]));
        }
        let mut row_attrs = BTreeMap::new();
        row_attrs.insert(String::from("class"), String::from("mc-sheet-row"));
        table_rows.push(VNode::element_with_attrs("div", row_attrs, row_cells));
    }

    let mut container_attrs = BTreeMap::new();
    container_attrs.insert(String::from("class"), format!("mc-sheet mc-sheet-{}", name));
    container_attrs.insert(String::from("data-cols"), format!("{}", cols));
    container_attrs.insert(String::from("data-rows"), format!("{}", rows));

    VNode::element_with_attrs("div", container_attrs, table_rows)
}

fn parse_cell(s: &str) -> (CellValue, String) {
    let trimmed = s.trim();

    if trimmed.starts_with('{') && trimmed.contains("compute") {
        let inner = trimmed.trim_start_matches('{').trim_end_matches('}').trim();
        let rest = inner.strip_prefix("compute").unwrap_or(inner).trim();
        let mut op = String::new();
        let mut direction = String::new();
        let mut until = String::from("edge");
        let mut format = String::new();

        for token in rest.split_whitespace() {
            if let Some(v) = token.strip_prefix("until:") { until = String::from(v); }
            else if let Some(v) = token.strip_prefix("format:") { format = String::from(v); }
            else if token.contains(':') && op.is_empty() {
                let parts: Vec<&str> = token.splitn(2, ':').collect();
                op = parts[0].to_string();
                direction = parts[1].to_string();
            }
        }

        return (CellValue::Compute(ComputeOp { op, direction, until, format }), String::new());
    }

    // Check for trailing style
    let parts: Vec<&str> = trimmed.rsplitn(2, ' ').collect();
    if parts.len() == 2 && is_style(parts[0]) {
        let val = parse_value(parts[1]);
        return (val, parts[0].to_string());
    }

    let val = parse_value(trimmed);
    (val, String::new())
}

fn parse_value(s: &str) -> CellValue {
    let s = s.trim().trim_matches('"');
    if s.is_empty() { return CellValue::Empty; }
    if let Ok(n) = s.parse::<f64>() { return CellValue::Number(n); }
    CellValue::Text(String::from(s))
}

fn is_style(s: &str) -> bool {
    matches!(s, "primary" | "secondary" | "danger" | "warning" | "info"
        | "success" | "muted" | "bold" | "dark" | "light")
}

fn evaluate_grid(grid: &BTreeMap<(usize, usize), CellValue>, cols: usize, rows: usize) -> BTreeMap<(usize, usize), String> {
    let mut result: BTreeMap<(usize, usize), String> = BTreeMap::new();

    // First pass: resolve non-compute cells
    for (&key, val) in grid {
        match val {
            CellValue::Text(s) => { result.insert(key, s.clone()); }
            CellValue::Number(n) => { result.insert(key, format_number(*n)); }
            CellValue::Empty => { result.insert(key, String::new()); }
            CellValue::Compute(_) => {}
        }
    }

    // Second pass: resolve compute cells
    for (&(c, r), val) in grid {
        if let CellValue::Compute(ref op) = val {
            let computed = evaluate_compute(grid, &result, c, r, op, cols, rows);
            result.insert((c, r), computed);
        }
    }

    // Fill empty cells
    for r in 0..rows {
        for c in 0..cols {
            result.entry((c, r)).or_insert_with(String::new);
        }
    }

    result
}

fn evaluate_compute(
    grid: &BTreeMap<(usize, usize), CellValue>,
    resolved: &BTreeMap<(usize, usize), String>,
    col: usize, row: usize,
    op: &ComputeOp,
    cols: usize, rows: usize,
) -> String {
    let (dc, dr): (i32, i32) = match op.direction.as_str() {
        "up" | "all-up" => (0, -1),
        "down" => (0, 1),
        "left" | "all-left" => (-1, 0),
        "right" => (1, 0),
        "up-left" => (-1, -1),
        "up-right" => (1, -1),
        "down-left" => (-1, 1),
        "down-right" => (1, 1),
        _ => (0, -1),
    };

    let mut values: Vec<f64> = Vec::new();
    let mut c = col as i32 + dc;
    let mut r = row as i32 + dr;

    loop {
        if c < 0 || r < 0 || c >= cols as i32 || r >= rows as i32 { break; }
        let key = (c as usize, r as usize);

        let cell_val = grid.get(&key);
        match cell_val {
            Some(CellValue::Number(n)) => {
                values.push(*n);
            }
            Some(CellValue::Text(t)) => {
                if op.until == "text" { break; }
                if let Ok(n) = t.parse::<f64>() { values.push(n); }
            }
            Some(CellValue::Empty) | None => {
                if op.until == "empty" { break; }
            }
            Some(CellValue::Compute(_)) => {
                if let Some(s) = resolved.get(&key) {
                    if let Ok(n) = s.parse::<f64>() { values.push(n); }
                }
            }
        }

        c += dc;
        r += dr;
    }

    let result = match op.op.as_str() {
        "sum" => values.iter().sum::<f64>(),
        "product" => values.iter().product::<f64>(),
        "count" => values.len() as f64,
        "min" => values.iter().cloned().fold(f64::MAX, f64::min),
        "max" => values.iter().cloned().fold(f64::MIN, f64::max),
        "avg" => if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 },
        _ => 0.0,
    };

    format_number(result)
}

fn format_number(n: f64) -> String {
    if n == (n as i64) as f64 { format!("{}", n as i64) }
    else { format!("{:.2}", n) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number_cell() {
        let (val, _) = parse_cell("100");
        assert!(matches!(val, CellValue::Number(n) if n == 100.0));
    }

    #[test]
    fn test_parse_text_cell() {
        let (val, _) = parse_cell("\"Revenue\"");
        assert!(matches!(val, CellValue::Text(ref s) if s == "Revenue"));
    }

    #[test]
    fn test_parse_compute_cell() {
        let (val, _) = parse_cell("{compute sum:up until:text}");
        if let CellValue::Compute(op) = val {
            assert_eq!(op.op, "sum");
            assert_eq!(op.direction, "up");
            assert_eq!(op.until, "text");
        } else {
            panic!("expected Compute");
        }
    }

    #[test]
    fn test_evaluate_sum_up() {
        let lines = vec![
            "(0,0) \"Header\" primary  (1,0) \"Q1\" primary",
            "(0,1) \"Revenue\"         (1,1) 100",
            "(0,2) \"COGS\"            (1,2) -40",
            "(0,3) \"Profit\"          (1,3) {compute sum:up until:text}",
        ];
        let vdom = render_sheet("report cols:2 rows:4", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("60")); // 100 + (-40) = 60
    }

    #[test]
    fn test_render_sheet_structure() {
        let lines = vec![
            "(0,0) \"A\"  (1,0) 10",
            "(0,1) \"B\"  (1,1) 20",
        ];
        let vdom = render_sheet("test cols:2 rows:2", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-sheet"));
        assert!(html.contains("mc-sheet-row"));
        assert!(html.contains("mc-sheet-cell"));
    }

    #[test]
    fn test_styled_cell() {
        let (_, style) = parse_cell("\"Header\" primary");
        assert_eq!(style, "primary");
    }
}
