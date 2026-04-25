#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod ansi;

use alloc::string::String;
use alloc::vec::Vec;
use pst_markout::vnode::{VNode, VElement};

pub fn render(markout: &str, cols: usize, rows: usize) -> String {
    let lines = pst_markout::parse::parse(markout);
    let vdom = pst_markout::render::render(&lines);
    to_ansi(&vdom, cols, rows)
}

pub fn to_ansi(node: &VNode, cols: usize, _rows: usize) -> String {
    let mut out = String::new();
    let mut ctx = RenderCtx { col: 0, row: 0, cols, indent: 0 };
    render_node(&mut out, node, &mut ctx);
    out.push_str(ansi::RESET);
    out.push_str("\r\n");
    out
}

struct RenderCtx {
    col: usize,
    row: usize,
    cols: usize,
    indent: usize,
}

fn render_node(out: &mut String, node: &VNode, ctx: &mut RenderCtx) {
    match node {
        VNode::Text(t) => {
            let text = t.content.trim();
            if !text.is_empty() {
                out.push_str(text);
                ctx.col += text.len();
            }
        }
        VNode::Element(el) => {
            let class = el.attrs.get("class").map(|s| s.as_str()).unwrap_or("");

            if let Some(style) = el.attrs.get("style") {
                if style.contains("position:absolute") {
                    let (x, y) = parse_position(style);
                    out.push_str(&ansi::cursor_to(ctx.row + y, ctx.indent + x));
                    ctx.col = ctx.indent + x;
                    for child in &el.children {
                        render_node(out, child, ctx);
                    }
                    return;
                }
                if style.contains("position:relative") {
                    let saved_row = ctx.row;
                    for child in &el.children {
                        render_node(out, child, ctx);
                    }
                    ctx.row = ctx.row.max(saved_row + 1);
                    newline(out, ctx);
                    return;
                }
            }

            if class.contains("mc-editor") {
                let features = el.attrs.get("data-features").map(|s| s.as_str()).unwrap_or("");
                pad_indent(out, ctx);
                out.push_str("\x1b[7m");
                for feat in features.split(',') {
                    let label = match feat {
                        "bold" => " B ",
                        "italic" => " I ",
                        "underline" => " U ",
                        "code" => " <> ",
                        "heading" => " H ",
                        "list" => " • ",
                        "quote" => " \" ",
                        _ => continue,
                    };
                    out.push_str(label);
                }
                out.push_str("\x1b[0m");
                newline(out, ctx);
                pad_indent(out, ctx);
                out.push_str(ansi::DIM);
                for _ in 0..ctx.cols.saturating_sub(ctx.indent * 2) { out.push('_'); }
                out.push_str(ansi::RESET);
                newline(out, ctx);
                newline(out, ctx);
                return;
            }

            if class.contains("mc-app") {
                for child in &el.children {
                    render_node(out, child, ctx);
                }
                return;
            }

            if class.contains("mc-card") {
                render_card(out, el, ctx);
                return;
            }

            if class.contains("mc-row") {
                pad_indent(out, ctx);
                for child in &el.children {
                    render_node(out, child, ctx);
                }
                newline(out, ctx);
                return;
            }

            if class.contains("mc-button") {
                render_button(out, el, ctx);
                return;
            }

            if class.contains("mc-input") {
                render_input(out, el, ctx);
                return;
            }

            if class.contains("mc-checkbox") {
                render_checkbox(out, el, ctx);
                return;
            }

            if class.contains("mc-radio") {
                out.push_str("( ) ");
                ctx.col += 4;
                return;
            }

            if class.contains("mc-select") {
                out.push_str(ansi::DIM);
                out.push_str("[______ v]");
                out.push_str(ansi::RESET);
                ctx.col += 10;
                return;
            }

            if class.contains("mc-textarea") {
                out.push_str(ansi::DIM);
                out.push_str("[");
                for _ in 0..24 { out.push('_'); }
                out.push_str("]\r\n");
                pad_indent(out, ctx);
                out.push_str("[");
                for _ in 0..24 { out.push('_'); }
                out.push(']');
                out.push_str(ansi::RESET);
                ctx.col += 26;
                return;
            }

            if class.contains("mc-link") {
                let label = text_content(node);
                out.push_str("\x1b[4;34m"); // underline + blue
                out.push_str(label.trim());
                out.push_str(ansi::RESET);
                ctx.col += label.trim().len();
                return;
            }

            if class.contains("mc-image") {
                out.push_str(ansi::DIM);
                out.push_str("[img]");
                out.push_str(ansi::RESET);
                ctx.col += 5;
                return;
            }

            if class.contains("mc-pill") {
                let label = text_content(node);
                out.push_str(&ansi::bg(55, 65, 81));
                out.push(' ');
                out.push_str(label.trim());
                out.push(' ');
                out.push_str(ansi::RESET);
                ctx.col += label.trim().len() + 2;
                return;
            }

            if class.contains("mc-badge") {
                let label = text_content(node);
                out.push_str(&ansi::bg(239, 68, 68));
                out.push_str(&ansi::fg(255, 255, 255));
                out.push_str(label.trim());
                out.push_str(ansi::RESET);
                ctx.col += label.trim().len();
                return;
            }

            if class.contains("mc-progress") {
                let label = text_content(node);
                let pct: usize = label.trim().parse().unwrap_or(50);
                let bar_w = 20;
                let filled = (bar_w * pct) / 100;
                out.push('[');
                for i in 0..bar_w {
                    if i < filled { out.push_str("█"); } else { out.push_str("░"); }
                }
                out.push(']');
                ctx.col += bar_w + 2;
                return;
            }

            if class.contains("mc-sparkline") {
                out.push_str(ansi::DIM);
                out.push_str("▁▃▅▇▅▃▁▂▄▆");
                out.push_str(ansi::RESET);
                ctx.col += 11;
                return;
            }

            if class.contains("mc-spacer") {
                out.push_str("  ");
                ctx.col += 2;
                return;
            }

            if class.contains("mc-divider") {
                pad_indent(out, ctx);
                let w = ctx.cols.saturating_sub(ctx.indent * 2);
                for _ in 0..w { out.push('─'); }
                ctx.col = ctx.indent + w;
                newline(out, ctx);
                return;
            }

            if class.contains("mc-label") {
                let content = text_content(node);
                out.push_str(&content);
                ctx.col += content.len();
                return;
            }

            for child in &el.children {
                render_node(out, child, ctx);
            }
        }
    }
}

fn render_card(out: &mut String, el: &VElement, ctx: &mut RenderCtx) {
    let inner_w = ctx.cols.saturating_sub(ctx.indent * 2 + 2);

    // Top border
    pad_indent(out, ctx);
    out.push('┌');
    for _ in 0..inner_w { out.push('─'); }
    out.push('┐');
    newline(out, ctx);

    ctx.indent += 1;
    for child in &el.children {
        // Left border
        out.push_str(&ansi::cursor_to(ctx.row, ctx.indent - 1));
        out.push('│');
        ctx.col = ctx.indent;

        render_node(out, child, ctx);

        // Right border
        out.push_str(&ansi::cursor_to(ctx.row, ctx.indent + inner_w));
        out.push('│');
        newline(out, ctx);
    }
    ctx.indent -= 1;

    // Bottom border
    pad_indent(out, ctx);
    out.push('└');
    for _ in 0..inner_w { out.push('─'); }
    out.push('┘');
    newline(out, ctx);
}

fn render_button(out: &mut String, el: &VElement, ctx: &mut RenderCtx) {
    let label = text_content(&VNode::Element(el.clone()));
    let class = el.attrs.get("class").map(|s| s.as_str()).unwrap_or("");

    let (r, g, b) = style_to_rgb(class);
    out.push_str(&ansi::bg(r, g, b));
    out.push_str(&ansi::fg(255, 255, 255));
    out.push_str(ansi::BOLD);
    out.push_str(" ");
    out.push_str(label.trim());
    out.push_str(" ");
    out.push_str(ansi::RESET);
    ctx.col += label.trim().len() + 2;
}

fn style_to_rgb(class: &str) -> (u8, u8, u8) {
    if class.contains("mc-primary") { (59, 130, 246) }
    else if class.contains("mc-secondary") { (107, 114, 128) }
    else if class.contains("mc-danger") { (239, 68, 68) }
    else if class.contains("mc-warning") { (245, 158, 11) }
    else if class.contains("mc-info") { (6, 182, 212) }
    else if class.contains("mc-dark") { (30, 30, 30) }
    else if class.contains("mc-light") { (229, 231, 235) }
    else if class.contains("mc-ghost") { (55, 65, 81) }
    else { (59, 130, 246) }
}

fn render_input(out: &mut String, _el: &VElement, ctx: &mut RenderCtx) {
    out.push_str(ansi::DIM);
    out.push('[');
    for _ in 0..18 { out.push('_'); }
    out.push(']');
    out.push_str(ansi::RESET);
    ctx.col += 20;
}

fn render_checkbox(out: &mut String, _el: &VElement, ctx: &mut RenderCtx) {
    out.push_str("[ ] ");
    ctx.col += 4;
}

fn text_content(node: &VNode) -> String {
    match node {
        VNode::Text(t) => t.content.clone(),
        VNode::Element(el) => {
            let mut s = String::new();
            for child in &el.children {
                s.push_str(&text_content(child));
            }
            s
        }
    }
}

fn pad_indent(out: &mut String, ctx: &mut RenderCtx) {
    for _ in 0..ctx.indent { out.push(' '); }
    ctx.col = ctx.indent;
}

fn newline(out: &mut String, ctx: &mut RenderCtx) {
    out.push_str("\r\n");
    ctx.row += 1;
    ctx.col = 0;
}

fn parse_position(style: &str) -> (usize, usize) {
    let mut x = 0usize;
    let mut y = 0usize;
    for part in style.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("left:") {
            x = parse_px(val) / 8; // px to approximate columns
        } else if let Some(val) = part.strip_prefix("top:") {
            y = parse_px(val) / 16; // px to approximate rows
        }
    }
    (x, y)
}

fn parse_px(s: &str) -> usize {
    let s = s.trim().trim_end_matches("px");
    if let Some(dot) = s.find('.') {
        s[..dot].parse().unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_renders() {
        let output = render("| Hello World", 80, 24);
        assert!(output.contains("Hello World"));
    }

    #[test]
    fn test_card_has_box_drawing() {
        let output = render("@card\n| Inside\n@end card", 80, 24);
        assert!(output.contains('┌'));
        assert!(output.contains('┘'));
        assert!(output.contains("Inside"));
    }

    #[test]
    fn test_button_has_ansi_color() {
        let output = render("| {button:go \"Click\"}", 80, 24);
        assert!(output.contains("Click"));
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_input_renders_field() {
        let output = render("| {input:name}", 80, 24);
        assert!(output.contains('['));
        assert!(output.contains(']'));
    }

    #[test]
    fn test_divider_renders_line() {
        let output = render("| {divider:sep}", 80, 24);
        assert!(output.contains('─'));
    }

    #[test]
    fn test_parametric_renders() {
        let input = "\
@parametric
| {label:title \"Dashboard\"}
| {input:search center-x:title gap-y:16}
@end parametric";
        let output = render(input, 80, 24);
        assert!(output.contains("Dashboard"));
    }

    #[test]
    fn test_full_document() {
        let input = "\
@card
| Parallel String Theory OS
@parametric
| {label:title \"PST OS v0.1\"}
| {label:arch \"x86_64 / seL4\" center-x:title gap-y:8}
@end parametric
| One primitive. One loop. One OS.
@end card";
        let output = render(input, 80, 24);
        assert!(output.contains('┌'));
        assert!(output.contains("PST OS v0.1"));
        assert!(output.contains("One primitive"));
    }
}
