use alloc::string::String;
use crate::vnode::VNode;

pub fn to_serial(node: &VNode, cols: usize) -> String {
    let mut out = String::new();
    let mut ctx = SerialCtx { col: 0, cols, indent: 0 };
    render_node(&mut out, node, &mut ctx);
    out.push('\n');
    out
}

struct SerialCtx {
    col: usize,
    cols: usize,
    indent: usize,
}

fn render_node(out: &mut String, node: &VNode, ctx: &mut SerialCtx) {
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
                if style.contains("position:absolute") || style.contains("position:relative") {
                    for child in &el.children {
                        render_node(out, child, ctx);
                    }
                    newline(out, ctx);
                    return;
                }
            }

            if class.contains("mc-app") {
                for child in &el.children {
                    render_node(out, child, ctx);
                }
                return;
            }

            if class.contains("mc-card") || class.contains("mc-section") || class.contains("mc-form") {
                pad(out, ctx);
                let w = ctx.cols.saturating_sub(ctx.indent * 2);
                out.push('+');
                for _ in 0..w.saturating_sub(2) { out.push('-'); }
                out.push('+');
                newline(out, ctx);

                ctx.indent += 2;
                for child in &el.children {
                    pad(out, ctx);
                    render_node(out, child, ctx);
                    newline(out, ctx);
                }
                ctx.indent -= 2;

                pad(out, ctx);
                out.push('+');
                for _ in 0..w.saturating_sub(2) { out.push('-'); }
                out.push('+');
                newline(out, ctx);
                return;
            }

            if class.contains("mc-row") {
                pad(out, ctx);
                for child in &el.children {
                    render_node(out, child, ctx);
                }
                newline(out, ctx);
                return;
            }

            if class.contains("mc-button") {
                let label = text_content(node);
                out.push('[');
                out.push_str(label.trim());
                out.push(']');
                ctx.col += label.trim().len() + 2;
                return;
            }

            if class.contains("mc-input") || class.contains("mc-input-password") {
                out.push('[');
                for _ in 0..16 { out.push('_'); }
                out.push(']');
                ctx.col += 18;
                return;
            }

            if class.contains("mc-checkbox") {
                out.push_str("[ ] ");
                ctx.col += 4;
                return;
            }

            if class.contains("mc-radio") {
                out.push_str("( ) ");
                ctx.col += 4;
                return;
            }

            if class.contains("mc-select") {
                out.push_str("[______ v]");
                ctx.col += 10;
                return;
            }

            if class.contains("mc-textarea") {
                out.push('[');
                for _ in 0..24 { out.push('_'); }
                out.push(']');
                ctx.col += 26;
                return;
            }

            if class.contains("mc-divider") {
                pad(out, ctx);
                let w = ctx.cols.saturating_sub(ctx.indent * 2);
                for _ in 0..w { out.push('-'); }
                newline(out, ctx);
                return;
            }

            if class.contains("mc-link") {
                let label = text_content(node);
                out.push_str(label.trim());
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
                    if i < filled { out.push('#'); } else { out.push('.'); }
                }
                out.push(']');
                ctx.col += bar_w + 2;
                return;
            }

            if class.contains("mc-spacer") {
                out.push(' ');
                ctx.col += 1;
                return;
            }

            if class.contains("mc-label") || class.contains("mc-pill") || class.contains("mc-badge") {
                let content = text_content(node);
                out.push_str(&content);
                ctx.col += content.len();
                return;
            }

            if class.contains("mc-image") {
                out.push_str("[img]");
                ctx.col += 5;
                return;
            }

            for child in &el.children {
                render_node(out, child, ctx);
            }
        }
    }
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

fn pad(out: &mut String, ctx: &mut SerialCtx) {
    for _ in 0..ctx.indent { out.push(' '); }
    ctx.col = ctx.indent;
}

fn newline(out: &mut String, ctx: &mut SerialCtx) {
    out.push('\n');
    ctx.col = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::render;

    fn serial(input: &str) -> String {
        let lines = parse::parse(input);
        let vdom = render::render(&lines);
        to_serial(&vdom, 80)
    }

    #[test]
    fn test_serial_label() {
        let out = serial("| Hello World");
        assert!(out.contains("Hello World"));
    }

    #[test]
    fn test_serial_button() {
        let out = serial("| {button:go \"Click\"}");
        assert!(out.contains("["));
        assert!(out.contains("Click"));
        assert!(out.contains("]"));
    }

    #[test]
    fn test_serial_input() {
        let out = serial("| {input:name}");
        assert!(out.contains("[________________]"));
    }

    #[test]
    fn test_serial_card() {
        let out = serial("@card\n| Inside\n@end card");
        assert!(out.contains('+'));
        assert!(out.contains('-'));
        assert!(out.contains("Inside"));
    }

    #[test]
    fn test_serial_divider() {
        let out = serial("| {divider:sep}");
        assert!(out.contains("---"));
    }

    #[test]
    fn test_serial_no_escape_codes() {
        let out = serial("| {button:go \"Save\" primary}");
        assert!(!out.contains("\x1b["));
    }

    #[test]
    fn test_serial_checkbox() {
        let out = serial("| {checkbox:agree}");
        assert!(out.contains("[ ]"));
    }
}
