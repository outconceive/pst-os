use alloc::string::String;
use crate::vnode::VNode;

pub fn to_html(node: &VNode) -> String {
    match node {
        VNode::Text(t) => escape_html(&t.content),
        VNode::Element(el) => {
            let mut html = String::new();
            html.push('<');
            html.push_str(&el.tag);

            for (key, value) in &el.attrs {
                html.push(' ');
                html.push_str(key);
                html.push_str("=\"");
                html.push_str(&escape_attr(value));
                html.push('"');
            }

            if is_void(&el.tag) {
                html.push_str("/>");
                return html;
            }

            html.push('>');
            for child in &el.children {
                html.push_str(&to_html(child));
            }
            html.push_str("</");
            html.push_str(&el.tag);
            html.push('>');
            html
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

fn is_void(tag: &str) -> bool {
    matches!(tag, "br" | "hr" | "img" | "input" | "meta" | "link" | "col")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::render;

    #[test]
    fn test_label_to_html() {
        let lines = parse::parse("| Hello World");
        let vdom = render::render(&lines);
        let html = to_html(&vdom);
        assert!(html.contains("Hello World"));
        assert!(html.contains("mc-app"));
    }

    #[test]
    fn test_input_to_html() {
        let lines = parse::parse("| {input:name}");
        let vdom = render::render(&lines);
        let html = to_html(&vdom);
        assert!(html.contains("<input"));
        assert!(html.contains("type=\"text\""));
        assert!(html.contains("data-bind=\"name\""));
    }

    #[test]
    fn test_parametric_to_html() {
        let input = "\
@parametric
| {label:title \"Dashboard\"}
| {input:search center-x:title gap-y:16}
| {button:go \"Search\" primary gap-x:8:search center-y:search}
@end parametric";
        let lines = parse::parse(input);
        let vdom = render::render(&lines);
        let html = to_html(&vdom);

        assert!(html.contains("mc-parametric"));
        assert!(html.contains("position:absolute"));
        assert!(html.contains("data-parametric=\"title\""));
        assert!(html.contains("data-parametric=\"search\""));
        assert!(html.contains("data-parametric=\"go\""));
        assert!(html.contains("Dashboard"));
        assert!(html.contains("Search"));
    }

    #[test]
    fn test_full_markout_to_html_on_bare_metal() {
        let input = "\
@card
| Welcome to PST OS
@parametric
| {label:heading \"Parallel String Theory\"}
| {input:query center-x:heading gap-y:16}
| {button:run \"Execute\" primary center-x:heading gap-y:12:query}
@end parametric
| The thesis is proven.
@end card";

        let lines = parse::parse(input);
        let vdom = render::render(&lines);
        let html = to_html(&vdom);

        assert!(html.contains("mc-card"));
        assert!(html.contains("mc-parametric"));
        assert!(html.contains("Parallel String Theory"));
        assert!(html.contains("Execute"));
        assert!(html.contains("The thesis is proven."));
        assert!(html.contains("position:absolute"));
        assert!(html.contains("position:relative"));
    }
}
