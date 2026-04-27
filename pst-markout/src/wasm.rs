use alloc::format;
use alloc::string::String;
use crate::vnode::VNode;

pub fn to_json(node: &VNode) -> String {
    let mut out = String::new();
    write_node(&mut out, node);
    out
}

fn write_node(out: &mut String, node: &VNode) {
    match node {
        VNode::Text(t) => {
            out.push_str("{\"t\":\"text\",\"v\":\"");
            out.push_str(&escape_json(&t.content));
            out.push_str("\"}");
        }
        VNode::Element(el) => {
            out.push_str("{\"t\":\"");
            out.push_str(&escape_json(&el.tag));
            out.push('"');

            if !el.attrs.is_empty() {
                out.push_str(",\"a\":{");
                let mut first = true;
                for (k, v) in &el.attrs {
                    if !first { out.push(','); }
                    first = false;
                    out.push('"');
                    out.push_str(&escape_json(k));
                    out.push_str("\":\"");
                    out.push_str(&escape_json(v));
                    out.push('"');
                }
                out.push('}');
            }

            if !el.children.is_empty() {
                out.push_str(",\"c\":[");
                for (i, child) in el.children.iter().enumerate() {
                    if i > 0 { out.push(','); }
                    write_node(out, child);
                }
                out.push(']');
            }

            out.push('}');
        }
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::render;

    fn json(input: &str) -> String {
        let lines = parse::parse(input);
        let vdom = render::render(&lines);
        to_json(&vdom)
    }

    #[test]
    fn test_json_label() {
        let out = json("| Hello");
        assert!(out.contains("\"t\":\"div\""));
        assert!(out.contains("\"t\":\"span\""));
        assert!(out.contains("\"t\":\"text\""));
        assert!(out.contains("Hello"));
    }

    #[test]
    fn test_json_button() {
        let out = json("| {button:go \"Click\"}");
        assert!(out.contains("\"t\":\"button\""));
        assert!(out.contains("Click"));
    }

    #[test]
    fn test_json_input() {
        let out = json("| {input:name}");
        assert!(out.contains("\"t\":\"input\""));
        assert!(out.contains("data-bind"));
        assert!(out.contains("name"));
    }

    #[test]
    fn test_json_attrs() {
        let out = json("| {button:go \"Save\" primary}");
        assert!(out.contains("mc-button"));
        assert!(out.contains("mc-primary"));
        assert!(out.contains("data-action"));
    }

    #[test]
    fn test_json_card() {
        let out = json("@card\n| Inside\n@end card");
        assert!(out.contains("mc-card"));
        assert!(out.contains("Inside"));
    }

    #[test]
    fn test_json_escaping() {
        let out = json("| He said \"hello\"");
        assert!(out.contains("\\\"hello\\\""));
    }

    #[test]
    fn test_json_valid_structure() {
        let out = json("| Hello");
        assert!(out.starts_with('{'));
        assert!(out.ends_with('}'));
        let open = out.chars().filter(|&c| c == '{').count();
        let close = out.chars().filter(|&c| c == '}').count();
        assert_eq!(open, close);
    }

    #[test]
    fn test_json_parametric() {
        let out = json("@parametric\n| {label:a \"X\"}\n| {label:b \"Y\" gap-y:8:a}\n@end parametric");
        assert!(out.contains("mc-parametric"));
        assert!(out.contains("position:absolute"));
    }
}
