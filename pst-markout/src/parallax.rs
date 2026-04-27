use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::vnode::VNode;

pub fn render_parallax(tag_config: &str, content_lines: &[&str]) -> VNode {
    let mut name = String::new();
    for token in tag_config.split_whitespace() {
        if name.is_empty() { name = String::from(token); }
    }

    let layers = parse_layers(content_lines);
    let mut children = Vec::new();

    for (idx, layer) in layers.iter().enumerate() {
        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("class"), String::from("mc-parallax-layer"));
        attrs.insert(String::from("data-layer"), format!("{}", idx));
        attrs.insert(String::from("data-speed"), format!("{}", layer.speed));
        attrs.insert(
            String::from("style"),
            String::from("position:absolute;left:0;top:0;width:100%;height:100%"),
        );

        let parsed = crate::parse::parse(&layer.content_raw);
        let inner = crate::render::render(&parsed);
        let inner_children = match inner {
            VNode::Element(el) => el.children,
            _ => vec![inner],
        };

        children.push(VNode::element_with_attrs("div", attrs, inner_children));
    }

    let mut container_attrs = BTreeMap::new();
    container_attrs.insert(String::from("class"), format!("mc-parallax mc-parallax-{}", name));
    container_attrs.insert(String::from("style"), String::from("position:relative;overflow:hidden"));

    VNode::element_with_attrs("div", container_attrs, children)
}

struct Layer {
    speed: f64,
    content_raw: String,
}

fn parse_layers(lines: &[&str]) -> Vec<Layer> {
    let mut layers = Vec::new();
    let mut current_speed: Option<f64> = None;
    let mut content_buf = String::new();

    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@layer") {
            if let Some(speed) = current_speed.take() {
                layers.push(Layer { speed, content_raw: content_buf.clone() });
                content_buf.clear();
            }
            let mut speed = 1.0f64;
            for token in rest.split_whitespace() {
                if let Some(v) = token.strip_prefix("speed:") {
                    speed = v.parse().unwrap_or(1.0);
                }
            }
            current_speed = Some(speed);
        } else if current_speed.is_some() {
            if !content_buf.is_empty() { content_buf.push('\n'); }
            content_buf.push_str(trimmed);
        }
    }

    if let Some(speed) = current_speed.take() {
        layers.push(Layer { speed, content_raw: content_buf });
    }

    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_layers() {
        let lines = vec![
            "@layer speed:0.2",
            "| {label:bg \"Background\"}",
            "@layer speed:1.0",
            "| {label:fg \"Foreground\"}",
        ];
        let layers = parse_layers(&lines);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].speed, 0.2);
        assert_eq!(layers[1].speed, 1.0);
    }

    #[test]
    fn test_render_parallax() {
        let lines = vec![
            "@layer speed:0.3",
            "| {label:bg \"Stars\"}",
            "@layer speed:1.0",
            "| {label:fg \"Title\" primary}",
        ];
        let vdom = render_parallax("hero", &lines);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-parallax"));
        assert!(html.contains("mc-parallax-layer"));
        assert!(html.contains("data-speed=\"0.3\""));
        assert!(html.contains("data-speed=\"1\""));
    }

    #[test]
    fn test_fixed_layer() {
        let lines = vec!["@layer speed:0", "| {label:x \"Fixed\"}"];
        let layers = parse_layers(&lines);
        assert_eq!(layers[0].speed, 0.0);
    }
}
