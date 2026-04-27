use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::vnode::VNode;

struct TableConfig {
    name: String,
    striped: bool,
    sortable: bool,
}

struct ColDef {
    key: String,
    header: String,
    col_type: String,
    width: usize,
    align: String,
    format: String,
    footer: String,
}

struct RowData {
    values: Vec<(String, String)>,
}

fn parse_table_config(config: &str) -> TableConfig {
    let mut name = String::new();
    let mut striped = false;
    let mut sortable = false;

    for token in config.split_whitespace() {
        if token == "striped:true" { striped = true; }
        else if token == "sortable:true" { sortable = true; }
        else if name.is_empty() && !token.contains(':') { name = String::from(token); }
        else if name.is_empty() {
            if let Some(n) = token.strip_prefix("name:") { name = String::from(n); }
            else if !token.contains(':') { name = String::from(token); }
        }
    }
    TableConfig { name, striped, sortable }
}

fn parse_col_def(line: &str) -> Option<ColDef> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("@col:")?;

    let mut key = String::new();
    let mut header = String::new();
    let mut col_type = String::from("text");
    let mut width = 0usize;
    let mut align = String::from("left");
    let mut format = String::new();
    let mut footer = String::new();

    let tokens = shell_split(rest);
    if tokens.is_empty() { return None; }
    key = tokens[0].clone();

    for token in &tokens[1..] {
        if token.starts_with("header:") {
            header = token[7..].trim_matches('"').to_string();
        } else if let Some(v) = token.strip_prefix("type:") {
            col_type = String::from(v);
        } else if let Some(v) = token.strip_prefix("width:") {
            width = v.parse().unwrap_or(0);
        } else if let Some(v) = token.strip_prefix("align:") {
            align = String::from(v);
        } else if let Some(v) = token.strip_prefix("format:") {
            format = String::from(v);
        } else if let Some(v) = token.strip_prefix("footer:") {
            footer = String::from(v);
        }
    }

    if header.is_empty() {
        header = key.clone();
    }

    Some(ColDef { key, header, col_type, width, align, format, footer })
}

fn parse_row(line: &str, col_count: usize) -> RowData {
    let trimmed = line.trim().strip_prefix("@row").unwrap_or(line.trim()).trim();
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in trimmed.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            values.push(parse_cell_value(current.trim()));
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        values.push(parse_cell_value(current.trim()));
    }

    while values.len() < col_count {
        values.push((String::new(), String::new()));
    }

    RowData { values }
}

fn parse_cell_value(s: &str) -> (String, String) {
    let trimmed = s.trim().trim_matches('"');
    let parts: Vec<&str> = trimmed.rsplitn(2, ' ').collect();
    if parts.len() == 2 {
        let maybe_style = parts[0];
        if is_style(maybe_style) {
            return (parts[1].to_string(), maybe_style.to_string());
        }
    }
    (trimmed.to_string(), String::new())
}

fn is_style(s: &str) -> bool {
    matches!(s, "primary" | "secondary" | "danger" | "warning" | "info"
        | "success" | "muted" | "dark" | "light")
}

fn compute_footer(values: &[f64], func: &str) -> String {
    if values.is_empty() { return String::new(); }
    match func {
        "sum" => {
            let sum: f64 = values.iter().sum();
            format_number(sum)
        }
        "count" => format!("{}", values.len()),
        "avg" => {
            let sum: f64 = values.iter().sum();
            format_number(sum / values.len() as f64)
        }
        "min" => {
            let min = values.iter().cloned().fold(f64::MAX, f64::min);
            format_number(min)
        }
        "max" => {
            let max = values.iter().cloned().fold(f64::MIN, f64::max);
            format_number(max)
        }
        _ => String::new(),
    }
}

fn format_number(n: f64) -> String {
    if n == (n as i64) as f64 { format!("{}", n as i64) }
    else { format!("{:.2}", n) }
}

pub fn render_table(tag_config: &str, content_lines: &[&str]) -> VNode {
    let cfg = parse_table_config(tag_config);
    let mut cols: Vec<ColDef> = Vec::new();
    let mut rows: Vec<RowData> = Vec::new();

    for line in content_lines {
        let trimmed = line.trim();
        if trimmed.starts_with("@col:") {
            if let Some(col) = parse_col_def(trimmed) {
                cols.push(col);
            }
        } else if trimmed.starts_with("@row") {
            rows.push(parse_row(trimmed, cols.len()));
        }
    }

    let mut table_attrs = BTreeMap::new();
    table_attrs.insert(String::from("class"), format!("mc-table mc-table-{}", cfg.name));
    if cfg.striped { table_attrs.insert(String::from("data-striped"), String::from("true")); }
    if cfg.sortable { table_attrs.insert(String::from("data-sortable"), String::from("true")); }

    let mut children = Vec::new();

    // Header row
    let mut header_cells = Vec::new();
    for col in &cols {
        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("class"), format!("mc-table-cell mc-table-header-cell mc-align-{}", col.align));
        if col.width > 0 {
            attrs.insert(String::from("style"), format!("width:{}ch", col.width));
        }
        attrs.insert(String::from("data-col"), col.key.clone());
        attrs.insert(String::from("data-type"), col.col_type.clone());
        header_cells.push(VNode::element_with_attrs("div", attrs, vec![VNode::text(&col.header)]));
    }
    let mut header_attrs = BTreeMap::new();
    header_attrs.insert(String::from("class"), String::from("mc-table-header mc-table-row"));
    children.push(VNode::element_with_attrs("div", header_attrs, header_cells));

    // Data rows
    let mut body_children = Vec::new();
    for (row_idx, row) in rows.iter().enumerate() {
        let mut row_cells = Vec::new();
        for (col_idx, col) in cols.iter().enumerate() {
            let (ref val, ref style) = if col_idx < row.values.len() {
                &row.values[col_idx]
            } else {
                &(String::new(), String::new())
            };

            let mut attrs = BTreeMap::new();
            let mut class = format!("mc-table-cell mc-align-{}", col.align);
            if !style.is_empty() {
                class.push_str(&format!(" mc-{}", style));
            }
            attrs.insert(String::from("class"), class);
            attrs.insert(String::from("data-col"), col.key.clone());

            let cell_content = render_cell_content(val, &col.col_type);
            row_cells.push(VNode::element_with_attrs("div", attrs, vec![cell_content]));
        }
        let mut row_attrs = BTreeMap::new();
        let mut row_class = String::from("mc-table-row");
        if cfg.striped && row_idx % 2 == 1 {
            row_class.push_str(" mc-table-row-alt");
        }
        row_attrs.insert(String::from("class"), row_class);
        body_children.push(VNode::element_with_attrs("div", row_attrs, row_cells));
    }
    let mut body_attrs = BTreeMap::new();
    body_attrs.insert(String::from("class"), String::from("mc-table-body"));
    children.push(VNode::element_with_attrs("div", body_attrs, body_children));

    // Footer
    let has_footer = cols.iter().any(|c| !c.footer.is_empty());
    if has_footer {
        let mut footer_cells = Vec::new();
        for (col_idx, col) in cols.iter().enumerate() {
            let mut attrs = BTreeMap::new();
            attrs.insert(String::from("class"), format!("mc-table-cell mc-table-footer-cell mc-align-{}", col.align));

            let footer_val = if !col.footer.is_empty() {
                let nums: Vec<f64> = rows.iter()
                    .filter_map(|r| r.values.get(col_idx).and_then(|(v, _)| v.parse::<f64>().ok()))
                    .collect();
                compute_footer(&nums, &col.footer)
            } else {
                String::new()
            };

            footer_cells.push(VNode::element_with_attrs("div", attrs, vec![VNode::text(&footer_val)]));
        }
        let mut footer_attrs = BTreeMap::new();
        footer_attrs.insert(String::from("class"), String::from("mc-table-footer mc-table-row"));
        children.push(VNode::element_with_attrs("div", footer_attrs, footer_cells));
    }

    VNode::element_with_attrs("div", table_attrs, children)
}

fn render_cell_content(value: &str, col_type: &str) -> VNode {
    match col_type {
        "progress" => {
            let pct: usize = value.parse().unwrap_or(0);
            let mut attrs = BTreeMap::new();
            attrs.insert(String::from("class"), String::from("mc-progress-inline"));
            attrs.insert(String::from("data-value"), format!("{}", pct));
            VNode::element_with_attrs("div", attrs, vec![VNode::text(value)])
        }
        "badge" => {
            let mut attrs = BTreeMap::new();
            attrs.insert(String::from("class"), String::from("mc-badge-inline"));
            VNode::element_with_attrs("span", attrs, vec![VNode::text(value)])
        }
        "pill" => {
            let mut attrs = BTreeMap::new();
            attrs.insert(String::from("class"), String::from("mc-pill-inline"));
            VNode::element_with_attrs("span", attrs, vec![VNode::text(value)])
        }
        "checkbox" => {
            let mut attrs = BTreeMap::new();
            attrs.insert(String::from("type"), String::from("checkbox"));
            if value == "true" {
                attrs.insert(String::from("checked"), String::from("checked"));
            }
            VNode::element_with_attrs("input", attrs, Vec::new())
        }
        "link" => {
            let mut attrs = BTreeMap::new();
            attrs.insert(String::from("class"), String::from("mc-link-inline"));
            attrs.insert(String::from("href"), String::from(value));
            VNode::element_with_attrs("a", attrs, vec![VNode::text(value)])
        }
        _ => VNode::text(value),
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
    if !current.is_empty() { parts.push(current); }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_table_config() {
        let cfg = parse_table_config("procs striped:true sortable:true");
        assert_eq!(cfg.name, "procs");
        assert!(cfg.striped);
        assert!(cfg.sortable);
    }

    #[test]
    fn test_parse_col_def() {
        let col = parse_col_def("@col:pid header:\"PID\" type:number width:6 align:right").unwrap();
        assert_eq!(col.key, "pid");
        assert_eq!(col.header, "PID");
        assert_eq!(col.col_type, "number");
        assert_eq!(col.width, 6);
        assert_eq!(col.align, "right");
    }

    #[test]
    fn test_parse_row() {
        let row = parse_row("@row 0, \"init\", \"running\" success, 12", 4);
        assert_eq!(row.values[0].0, "0");
        assert_eq!(row.values[1].0, "init");
        assert_eq!(row.values[2].0, "running");
        assert_eq!(row.values[2].1, "success");
        assert_eq!(row.values[3].0, "12");
    }

    #[test]
    fn test_render_table_structure() {
        let lines = vec![
            "@col:name header:\"Name\" type:text width:16",
            "@col:age header:\"Age\" type:number width:6 align:right",
            "@row \"Alice\", 30",
            "@row \"Bob\", 25",
        ];
        let vdom = render_table("people striped:true", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-table"));
        assert!(html.contains("mc-table-header"));
        assert!(html.contains("mc-table-body"));
        assert!(html.contains("Name"));
        assert!(html.contains("Age"));
        assert!(html.contains("Alice"));
        assert!(html.contains("Bob"));
    }

    #[test]
    fn test_striped_rows() {
        let lines = vec![
            "@col:x header:\"X\" type:text",
            "@row \"a\"",
            "@row \"b\"",
            "@row \"c\"",
        ];
        let vdom = render_table("t striped:true", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-table-row-alt"));
    }

    #[test]
    fn test_footer_sum() {
        let lines = vec![
            "@col:val header:\"Value\" type:number footer:sum",
            "@row 10",
            "@row 20",
            "@row 30",
        ];
        let vdom = render_table("t", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-table-footer"));
        assert!(html.contains("60"));
    }

    #[test]
    fn test_badge_cell() {
        let lines = vec![
            "@col:state header:\"State\" type:badge",
            "@row \"running\"",
        ];
        let vdom = render_table("t", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-badge-inline"));
    }
}
