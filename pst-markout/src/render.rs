use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::parse::{self, Line, LineType};
use crate::vnode::VNode;

use libpst::constraint::{Constraint, GapValue};

pub fn render(lines: &[Line]) -> VNode {
    render_with_state(lines, &crate::state::StateStore::new())
}

pub fn render_with_state(lines: &[Line], state: &crate::state::StateStore) -> VNode {
    let mut root_children = Vec::new();
    let mut container_stack: Vec<(String, BTreeMap<String, String>, Vec<VNode>)> = Vec::new();
    let mut each_stack: Vec<(String, Vec<Line>)> = Vec::new();
    let mut parametric_stack: Vec<(BTreeMap<String, String>, Vec<Line>)> = Vec::new();

    for line in lines {
        // Collecting template lines inside @each
        if !each_stack.is_empty() {
            if line.is_each_end() {
                let (list_key, template_lines) = each_stack.pop().unwrap();
                let count = state.get_list_count(&list_key);
                for item_idx in 0..count {
                    let scope = format!("{}.{}", list_key, item_idx);
                    for tpl_line in &template_lines {
                        let mut scoped_line = tpl_line.clone();
                        // Replace state keys with scoped versions
                        let mut new_keys = String::new();
                        for ch in scoped_line.state_keys.chars() {
                            new_keys.push(ch);
                        }
                        let row = render_line(&scoped_line);
                        if let Some(parent) = container_stack.last_mut() {
                            parent.2.push(row);
                        } else {
                            root_children.push(row);
                        }
                    }
                }
                continue;
            } else if line.is_each_start() {
                let key = line.tag.as_deref().unwrap_or("").to_string();
                each_stack.push((key, Vec::new()));
            } else {
                each_stack.last_mut().unwrap().1.push(line.clone());
            }
            continue;
        }

        if line.is_each_start() {
            let key = line.tag.as_deref().unwrap_or("").to_string();
            each_stack.push((key, Vec::new()));
            continue;
        }

        // Collecting inside @parametric
        if !parametric_stack.is_empty() {
            if line.line_type == LineType::ContainerEnd {
                if line.tag.as_deref() == Some("parametric") {
                    let (attrs, collected) = parametric_stack.pop().unwrap();
                    let parametric_node = render_parametric(&collected, attrs);
                    if let Some(parent) = container_stack.last_mut() {
                        parent.2.push(parametric_node);
                    } else {
                        root_children.push(parametric_node);
                    }
                    continue;
                }
            }
            parametric_stack.last_mut().unwrap().1.push(line.clone());
            continue;
        }

        if line.line_type == LineType::ContainerStart {
            let tag = line.tag.as_deref().unwrap_or("div");

            if tag == "parametric" {
                let mut attrs = BTreeMap::new();
                attrs.insert(String::from("class"), String::from("mc-parametric"));
                parametric_stack.push((attrs, Vec::new()));
                continue;
            }

            if tag == "editor" {
                let mut attrs = BTreeMap::new();
                attrs.insert(String::from("class"), String::from("mc-editor"));
                attrs.insert(String::from("data-editor"), String::from("true"));
                if let Some(ref cfg) = line.config {
                    let (features, bind_key) = parse_editor_config(cfg);
                    if !features.is_empty() {
                        attrs.insert(String::from("data-features"), features.join(","));
                    }
                    if let Some(key) = bind_key {
                        attrs.insert(String::from("data-bind"), key);
                    }
                }
                container_stack.push((String::from(tag), attrs, Vec::new()));
                continue;
            }

            let mut attrs = BTreeMap::new();
            attrs.insert(String::from("class"), format!("mc-{}", tag));
            if let Some(ref cfg) = line.config {
                attrs.insert(String::from("data-config"), cfg.clone());
            }
            container_stack.push((String::from(tag), attrs, Vec::new()));
            continue;
        }

        if line.line_type == LineType::ContainerEnd {
            if let Some((tag, attrs, children)) = container_stack.pop() {
                let el_tag = semantic_tag(&tag);
                let container = VNode::element_with_attrs(el_tag, attrs, children);
                if let Some(parent) = container_stack.last_mut() {
                    parent.2.push(container);
                } else {
                    root_children.push(container);
                }
            }
            continue;
        }

        let row = render_line(line);
        if let Some(parent) = container_stack.last_mut() {
            parent.2.push(row);
        } else {
            root_children.push(row);
        }
    }

    let mut root_attrs = BTreeMap::new();
    root_attrs.insert(String::from("class"), String::from("mc-app"));
    VNode::element_with_attrs("div", root_attrs, root_children)
}

fn render_line(line: &Line) -> VNode {
    let mut children = Vec::new();
    let content: Vec<char> = line.content.chars().collect();
    let comps: Vec<char> = line.components.chars().collect();
    let keys: Vec<char> = line.state_keys.chars().collect();
    let styles: Vec<char> = line.styles.chars().collect();
    let len = content.len();

    if len == 0 {
        children.push(VNode::element("br", Vec::new()));
    } else {
        let mut i = 0;
        while i < len {
            let comp = *comps.get(i).unwrap_or(&parse::EMPTY);
            let start = i;
            i += 1;
            while i < len && comps.get(i) == Some(&comp) { i += 1; }

            let span_content: String = content[start..i].iter().collect();
            let span_key: String = keys[start..i.min(keys.len())].iter().collect::<String>()
                .trim_matches('_').to_string();
            let style_char = *styles.get(start).unwrap_or(&' ');

            let mut attrs = BTreeMap::new();
            let mut class = css_class(comp);
            let style_class = style_css_class(style_char);
            if !style_class.is_empty() {
                class.push(' ');
                class.push_str(style_class);
            }
            attrs.insert(String::from("class"), class);
            if !span_key.is_empty() {
                attrs.insert(String::from("data-bind"), span_key);
            }
            if let Some(&(span, total)) = line.cols.get(&start) {
                attrs.insert(String::from("data-col"), format!("{},{}", span, total));
            }
            if let Some(h) = line.hrefs.get(&start) {
                attrs.insert(String::from("data-href"), h.clone());
            }
            if let Some(p) = line.popovers.get(&start) {
                attrs.insert(String::from("data-popover"), p.clone());
            }
            if let Some(a) = line.animates.get(&start) {
                attrs.insert(String::from("data-animate"), a.clone());
            }
            if let Some(v) = line.validates.get(&start) {
                attrs.insert(String::from("data-validate"), v.clone());
            }
            if let Some(resp) = line.responsive.get(&start) {
                let val: String = resp.iter()
                    .map(|(bp, n, t)| format!("{}:{},{}", bp, n, t))
                    .collect::<Vec<_>>().join(";");
                attrs.insert(String::from("data-responsive"), val);
            }

            match comp {
                parse::TEXT_INPUT | parse::PASSWORD => {
                    let input_type = if comp == parse::PASSWORD { "password" } else { "text" };
                    attrs.insert(String::from("type"), String::from(input_type));
                    children.push(VNode::element_with_attrs("input", attrs, Vec::new()));
                }
                parse::BUTTON => {
                    let label = span_content.trim();
                    if let Some(key) = attrs.get("data-bind") {
                        attrs.insert(String::from("data-action"), key.clone());
                    }
                    children.push(VNode::element_with_attrs("button", attrs, vec![VNode::text(label)]));
                }
                parse::CHECKBOX => {
                    attrs.insert(String::from("type"), String::from("checkbox"));
                    children.push(VNode::element_with_attrs("input", attrs, Vec::new()));
                }
                parse::RADIO => {
                    attrs.insert(String::from("type"), String::from("radio"));
                    children.push(VNode::element_with_attrs("input", attrs, Vec::new()));
                }
                parse::SELECT => {
                    children.push(VNode::element_with_attrs("select", attrs, vec![VNode::text(span_content.trim())]));
                }
                parse::TEXTAREA => {
                    children.push(VNode::element_with_attrs("textarea", attrs, Vec::new()));
                }
                parse::IMAGE => {
                    let src = span_content.trim();
                    if !src.is_empty() { attrs.insert(String::from("src"), String::from(src)); }
                    children.push(VNode::element_with_attrs("img", attrs, Vec::new()));
                }
                parse::LINK => {
                    let label = span_content.trim();
                    children.push(VNode::element_with_attrs("a", attrs, vec![VNode::text(label)]));
                }
                parse::PILL => {
                    let label = span_content.trim();
                    children.push(VNode::element_with_attrs("span", attrs, vec![VNode::text(label)]));
                }
                parse::BADGE => {
                    let label = span_content.trim();
                    children.push(VNode::element_with_attrs("span", attrs, vec![VNode::text(label)]));
                }
                parse::PROGRESS => {
                    children.push(VNode::element_with_attrs("div", attrs, vec![VNode::text(span_content.trim())]));
                }
                parse::SPARKLINE => {
                    children.push(VNode::element_with_attrs("svg", attrs, Vec::new()));
                }
                parse::DIVIDER => {
                    children.push(VNode::element_with_attrs("hr", attrs, Vec::new()));
                }
                _ => {
                    children.push(VNode::element_with_attrs("span", attrs, vec![VNode::text(&span_content)]));
                }
            }
        }
    }

    let mut row_attrs = BTreeMap::new();
    row_attrs.insert(String::from("class"), String::from("mc-row"));
    VNode::element_with_attrs("div", row_attrs, children)
}

fn render_parametric(lines: &[Line], mut container_attrs: BTreeMap<String, String>) -> VNode {
    // Collect elements with their constraints
    let mut elements: Vec<(String, char, String, Vec<Constraint>)> = Vec::new();
    let mut anon = 0usize;

    for line in lines {
        let comps: Vec<char> = line.components.chars().collect();
        let keys: Vec<char> = line.state_keys.chars().collect();
        let content: Vec<char> = line.content.chars().collect();

        let mut i = 0;
        while i < comps.len() {
            let comp = comps[i];
            let start = i;
            i += 1;
            while i < comps.len() && comps[i] == comp { i += 1; }

            let name: String = keys[start..i.min(keys.len())].iter().collect::<String>()
                .trim_matches('_').to_string();
            let name = if name.is_empty() {
                let n = format!("_anon_{}", anon);
                anon += 1;
                n
            } else {
                name
            };

            let span_content: String = content[start..i.min(content.len())].iter().collect();

            let raw_constraints = line.constraints.get(&start)
                .cloned()
                .unwrap_or_default();
            let constraints: Vec<Constraint> = raw_constraints.iter()
                .filter_map(|s| parse_markout_constraint(s))
                .collect();

            elements.push((name, comp, span_content, constraints));
        }
    }

    // Solve using libpst
    use libpst::solver::{ConstrainedNode, topological_sort, CycleAction};

    let nodes: Vec<ConstrainedNode> = elements.iter().map(|(name, _, _, constraints)| {
        ConstrainedNode {
            name: name.clone(),
            constraints: constraints.clone(),
            priority: 0,
        }
    }).collect();

    let result = topological_sort(&nodes, CycleAction::Break);

    // Compute positions (simplified solver for spatial constraints)
    let mut solved: BTreeMap<String, (f64, f64, f64, f64)> = BTreeMap::new(); // x, y, w, h
    let mut prev_solved: Option<String> = None;

    for name in &result.order {
        let el = match elements.iter().find(|(n, _, _, _)| n == name) {
            Some(e) => e,
            None => continue,
        };

        let (_, comp, ref content, ref constraints) = *el;
        let (def_w, def_h) = intrinsic_size(comp, content.trim().len());
        let mut x = 0.0f64;
        let mut y = 0.0f64;
        let mut w = def_w;
        let mut h = def_h;

        let first_ref: Option<String> = constraints.iter()
            .flat_map(|c| c.references())
            .next()
            .map(|s| String::from(s));

        for c in constraints {
            match c {
                Constraint::Left(r) => { if let Some(&(rx, _, _, _)) = solved.get(r) { x = rx; } }
                Constraint::Right(r) => { if let Some(&(rx, _, rw, _)) = solved.get(r) { x = rx + rw - w; } }
                Constraint::Top(r) => { if let Some(&(_, ry, _, _)) = solved.get(r) { y = ry; } }
                Constraint::Bottom(r) => { if let Some(&(_, ry, _, rh)) = solved.get(r) { y = ry + rh - h; } }
                Constraint::CenterX(r) => { if let Some(&(rx, _, rw, _)) = solved.get(r) { x = rx + rw / 2.0 - w / 2.0; } }
                Constraint::CenterY(r) => { if let Some(&(_, ry, _, rh)) = solved.get(r) { y = ry + rh / 2.0 - h / 2.0; } }
                Constraint::GapX(gap, ref_opt) => {
                    let r = ref_opt.as_ref()
                        .or(first_ref.as_ref())
                        .or(prev_solved.as_ref());
                    if let Some(rr) = r.and_then(|n| solved.get(n.as_str())) {
                        x = rr.0 + rr.2 + gap.pixels;
                    }
                }
                Constraint::GapY(gap, ref_opt) => {
                    let r = ref_opt.as_ref()
                        .or(first_ref.as_ref())
                        .or(prev_solved.as_ref());
                    if let Some(rr) = r.and_then(|n| solved.get(n.as_str())) {
                        y = rr.1 + rr.3 + gap.pixels;
                    }
                }
                Constraint::MatchWidth(r) => { if let Some(&(_, _, rw, _)) = solved.get(r) { w = rw; } }
                Constraint::MatchHeight(r) => { if let Some(&(_, _, _, rh)) = solved.get(r) { h = rh; } }
                _ => {}
            }
        }

        // Stretch between left: and right:
        let left_ref = constraints.iter().find_map(|c| match c { Constraint::Left(r) => Some(r.as_str()), _ => None });
        let right_ref = constraints.iter().find_map(|c| match c { Constraint::Right(r) => Some(r.as_str()), _ => None });
        if let (Some(lr), Some(rr)) = (left_ref, right_ref) {
            if let (Some(l), Some(r)) = (solved.get(lr), solved.get(rr)) {
                x = l.0;
                w = (r.0 + r.2) - l.0;
            }
        }

        solved.insert(name.clone(), (x, y, w, h));
        prev_solved = Some(name.clone());
    }

    // Compute container bounds
    let mut max_x = 0.0f64;
    let mut max_y = 0.0f64;
    for (_, &(x, y, w, h)) in &solved {
        let right = x + w;
        let bottom = y + h;
        if right > max_x { max_x = right; }
        if bottom > max_y { max_y = bottom; }
    }

    container_attrs.insert(
        String::from("style"),
        format!("position:relative;width:{:.1}px;height:{:.1}px", max_x, max_y),
    );

    // Build children
    let mut children = Vec::new();
    for (name, comp, ref content, _) in &elements {
        if let Some(&(x, y, w, h)) = solved.get(name) {
            let mut wrapper_attrs = BTreeMap::new();
            wrapper_attrs.insert(
                String::from("style"),
                format!("position:absolute;left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px", x, y, w, h),
            );
            wrapper_attrs.insert(String::from("data-parametric"), name.clone());

            let mut inner_attrs = BTreeMap::new();
            inner_attrs.insert(String::from("class"), css_class(*comp));
            inner_attrs.insert(String::from("data-bind"), name.clone());

            let inner = match comp {
                &parse::TEXT_INPUT | &parse::PASSWORD => {
                    let t = if *comp == parse::PASSWORD { "password" } else { "text" };
                    inner_attrs.insert(String::from("type"), String::from(t));
                    VNode::element_with_attrs("input", inner_attrs, Vec::new())
                }
                &parse::BUTTON => {
                    inner_attrs.insert(String::from("data-action"), name.clone());
                    VNode::element_with_attrs("button", inner_attrs, vec![VNode::text(content.trim())])
                }
                &parse::DIVIDER => VNode::element_with_attrs("hr", inner_attrs, Vec::new()),
                _ => VNode::element_with_attrs("span", inner_attrs, vec![VNode::text(content.trim())]),
            };

            children.push(VNode::element_with_attrs("div", wrapper_attrs, vec![inner]));
        }
    }

    VNode::element_with_attrs("div", container_attrs, children)
}

fn parse_markout_constraint(s: &str) -> Option<Constraint> {
    if let Some(r) = s.strip_prefix("left:") { return Some(Constraint::Left(String::from(r))); }
    if let Some(r) = s.strip_prefix("right:") { return Some(Constraint::Right(String::from(r))); }
    if let Some(r) = s.strip_prefix("top:") { return Some(Constraint::Top(String::from(r))); }
    if let Some(r) = s.strip_prefix("bottom:") { return Some(Constraint::Bottom(String::from(r))); }
    if let Some(r) = s.strip_prefix("center-x:") { return Some(Constraint::CenterX(String::from(r))); }
    if let Some(r) = s.strip_prefix("center-y:") { return Some(Constraint::CenterY(String::from(r))); }
    if let Some(rest) = s.strip_prefix("gap-x:") { return parse_gap_constraint(rest, false); }
    if let Some(rest) = s.strip_prefix("gap-y:") { return parse_gap_constraint(rest, true); }
    if let Some(r) = s.strip_prefix("width:") { return Some(Constraint::MatchWidth(String::from(r))); }
    if let Some(r) = s.strip_prefix("height:") { return Some(Constraint::MatchHeight(String::from(r))); }
    None
}

fn parse_gap_constraint(rest: &str, vertical: bool) -> Option<Constraint> {
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    let gap = GapValue::from_str(parts[0])?;
    let reference = parts.get(1).map(|s| String::from(*s));
    if vertical {
        Some(Constraint::GapY(gap, reference))
    } else {
        Some(Constraint::GapX(gap, reference))
    }
}

fn intrinsic_size(comp: char, content_len: usize) -> (f64, f64) {
    match comp {
        parse::TEXT_INPUT | parse::PASSWORD => (200.0, 36.0),
        parse::BUTTON => ((content_len as f64 * 9.0).max(80.0), 36.0),
        parse::CHECKBOX => (20.0, 20.0),
        parse::DIVIDER => (400.0, 1.0),
        parse::SPACER => (0.0, 0.0),
        _ => ((content_len as f64 * 8.0).max(20.0), 24.0),
    }
}

fn css_class(comp: char) -> String {
    String::from(match comp {
        parse::LABEL => "mc-label",
        parse::TEXT_INPUT => "mc-input",
        parse::PASSWORD => "mc-input-password",
        parse::BUTTON => "mc-button",
        parse::CHECKBOX => "mc-checkbox",
        parse::RADIO => "mc-radio",
        parse::SELECT => "mc-select",
        parse::TEXTAREA => "mc-textarea",
        parse::IMAGE => "mc-image",
        parse::LINK => "mc-link",
        parse::DIVIDER => "mc-divider",
        parse::SPACER => "mc-spacer",
        parse::PILL => "mc-pill",
        parse::BADGE => "mc-badge",
        parse::PROGRESS => "mc-progress",
        parse::SPARKLINE => "mc-sparkline",
        _ => "mc-label",
    })
}

fn parse_editor_config(config: &str) -> (Vec<String>, Option<String>) {
    let valid = ["bold", "italic", "underline", "strikethrough", "code",
        "heading", "list", "ordered-list", "quote", "code-block",
        "link", "image", "divider", "hr"];
    let mut features = Vec::new();
    let mut bind_key = None;
    for token in config.split_whitespace() {
        if let Some(key) = token.strip_prefix("bind:") {
            bind_key = Some(String::from(key));
        } else if valid.contains(&token) {
            features.push(String::from(token));
        }
    }
    (features, bind_key)
}

fn style_css_class(c: char) -> &'static str {
    match c {
        'p' => "mc-primary",
        's' => "mc-secondary",
        'd' => "mc-danger",
        'w' => "mc-warning",
        'i' => "mc-info",
        'k' => "mc-dark",
        'l' => "mc-light",
        'o' => "mc-outline",
        'g' => "mc-ghost",
        '1' => "mc-size-1",
        '2' => "mc-size-2",
        '3' => "mc-size-3",
        '4' => "mc-size-4",
        '5' => "mc-size-5",
        '6' => "mc-size-6",
        '7' => "mc-size-7",
        '8' => "mc-size-8",
        '9' => "mc-size-9",
        _ => "",
    }
}

fn semantic_tag(tag: &str) -> &str {
    match tag {
        "nav" => "nav",
        "header" => "header",
        "footer" => "footer",
        "main" => "main",
        "section" => "section",
        "aside" => "aside",
        "form" => "form",
        _ => "div",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_render_label() {
        let lines = parse::parse("| Hello");
        let vdom = render(&lines);
        assert_eq!(vdom.tag(), Some("div"));
        assert_eq!(vdom.children().len(), 1);
    }

    #[test]
    fn test_render_container() {
        let lines = parse::parse("@card\n| Inside\n@end card");
        let vdom = render(&lines);
        let card = &vdom.children()[0];
        assert_eq!(card.tag(), Some("div"));
    }

    #[test]
    fn test_render_parametric() {
        let input = "@parametric\n| {label:title \"Dashboard\"}\n| {input:search center-x:title gap-y:16}\n@end parametric";
        let lines = parse::parse(input);
        let vdom = render(&lines);

        let parametric = &vdom.children()[0];
        assert_eq!(parametric.tag(), Some("div"));

        // Should have positioned children
        let children = parametric.children();
        assert!(children.len() >= 2);

        // Each child should have position:absolute in style
        if let VNode::Element(el) = &children[0] {
            let style = el.attrs.get("style").unwrap();
            assert!(style.contains("position:absolute"));
        }
    }
}
