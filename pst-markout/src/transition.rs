use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::vnode::VNode;

pub struct Transition {
    pub name: String,
    pub target: String,
    pub property: String,
    pub from: f64,
    pub to: f64,
    pub duration: u32,
    pub curve: String,
    pub after: Option<String>,
    pub trigger: String,
    pub loop_mode: String,
}

pub fn parse_transition(config: &str) -> Option<Transition> {
    let mut name = String::new();
    let mut target = String::new();
    let mut property = String::new();
    let mut from = 0.0f64;
    let mut to = 1.0f64;
    let mut duration = 400u32;
    let mut curve = String::from("linear");
    let mut after = None;
    let mut trigger = String::from("render");
    let mut loop_mode = String::from("none");

    for token in config.split_whitespace() {
        if let Some(v) = token.strip_prefix("target:") { target = String::from(v); }
        else if let Some(v) = token.strip_prefix("property:") { property = String::from(v); }
        else if let Some(v) = token.strip_prefix("from:") { from = v.parse().unwrap_or(0.0); }
        else if let Some(v) = token.strip_prefix("to:") { to = v.parse().unwrap_or(1.0); }
        else if let Some(v) = token.strip_prefix("duration:") { duration = v.parse().unwrap_or(400); }
        else if let Some(v) = token.strip_prefix("curve:") { curve = String::from(v); }
        else if let Some(v) = token.strip_prefix("after:") { after = Some(String::from(v)); }
        else if let Some(v) = token.strip_prefix("trigger:") { trigger = String::from(v); }
        else if let Some(v) = token.strip_prefix("loop:") { loop_mode = String::from(v); }
        else if name.is_empty() { name = String::from(token); }
    }

    if target.is_empty() || property.is_empty() { return None; }
    Some(Transition { name, target, property, from, to, duration, curve, after, trigger, loop_mode })
}

pub fn transition_to_vnode(t: &Transition) -> VNode {
    let mut attrs = BTreeMap::new();
    attrs.insert(String::from("class"), String::from("mc-transition"));
    attrs.insert(String::from("data-transition"), t.name.clone());
    attrs.insert(String::from("data-target"), t.target.clone());
    attrs.insert(String::from("data-property"), t.property.clone());
    attrs.insert(String::from("data-from"), format!("{}", t.from));
    attrs.insert(String::from("data-to"), format!("{}", t.to));
    attrs.insert(String::from("data-duration"), format!("{}", t.duration));
    attrs.insert(String::from("data-curve"), t.curve.clone());
    attrs.insert(String::from("data-trigger"), t.trigger.clone());
    attrs.insert(String::from("data-loop"), t.loop_mode.clone());
    if let Some(ref a) = t.after {
        attrs.insert(String::from("data-after"), a.clone());
    }
    VNode::element_with_attrs("div", attrs, Vec::new())
}

pub fn apply_curve(t: f64, curve: &str) -> f64 {
    let t = if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t };
    match curve {
        "linear" => t,
        "ease-in" => t * t,
        "ease-out" => 1.0 - (1.0 - t) * (1.0 - t),
        "ease-in-out" => {
            if t < 0.5 { 2.0 * t * t } else { 1.0 - 2.0 * (1.0 - t) * (1.0 - t) }
        }
        "cubic" => t * t * t,
        "step" => if t >= 1.0 { 1.0 } else { 0.0 },
        "bounce" => {
            if t >= 1.0 { 1.0 }
            else {
                let v = 1.0 - (1.0 - t) * (1.0 - t);
                let wobble = (1.0 - t) * 0.3;
                (v + wobble).min(1.0)
            }
        }
        _ => t,
    }
}

pub fn interpolate(from: f64, to: f64, t: f64, curve: &str) -> f64 {
    from + (to - from) * apply_curve(t, curve)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transition() {
        let t = parse_transition("fade target:title property:opacity from:0 to:1 duration:500 curve:ease-out").unwrap();
        assert_eq!(t.name, "fade");
        assert_eq!(t.target, "title");
        assert_eq!(t.property, "opacity");
        assert_eq!(t.from, 0.0);
        assert_eq!(t.to, 1.0);
        assert_eq!(t.duration, 500);
        assert_eq!(t.curve, "ease-out");
    }

    #[test]
    fn test_parse_transition_with_after() {
        let t = parse_transition("t2 target:b property:opacity from:0 to:1 duration:500 after:t1").unwrap();
        assert_eq!(t.after, Some(String::from("t1")));
    }

    #[test]
    fn test_parse_transition_with_loop() {
        let t = parse_transition("pulse target:dot property:scale from:1 to:1.3 duration:800 loop:bounce").unwrap();
        assert_eq!(t.loop_mode, "bounce");
    }

    #[test]
    fn test_transition_to_vnode() {
        let t = parse_transition("fade target:title property:opacity from:0 to:1 duration:500 curve:linear").unwrap();
        let vdom = transition_to_vnode(&t);
        let html = crate::html::to_html(&vdom);
        assert!(html.contains("mc-transition"));
        assert!(html.contains("data-target=\"title\""));
        assert!(html.contains("data-property=\"opacity\""));
        assert!(html.contains("data-duration=\"500\""));
    }

    #[test]
    fn test_curves() {
        assert_eq!(apply_curve(0.0, "linear"), 0.0);
        assert_eq!(apply_curve(1.0, "linear"), 1.0);
        assert_eq!(apply_curve(0.5, "linear"), 0.5);
        assert_eq!(apply_curve(0.0, "ease-in"), 0.0);
        assert_eq!(apply_curve(1.0, "ease-in"), 1.0);
        assert!(apply_curve(0.5, "ease-in") < 0.5);
        assert!(apply_curve(0.5, "ease-out") > 0.5);
        assert_eq!(apply_curve(0.5, "step"), 0.0);
        assert_eq!(apply_curve(1.0, "step"), 1.0);
    }

    #[test]
    fn test_interpolate() {
        assert_eq!(interpolate(0.0, 100.0, 0.5, "linear"), 50.0);
        assert_eq!(interpolate(10.0, 20.0, 1.0, "linear"), 20.0);
        assert_eq!(interpolate(10.0, 20.0, 0.0, "linear"), 10.0);
    }
}
