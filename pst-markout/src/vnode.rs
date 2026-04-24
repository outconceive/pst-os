use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub enum VNode {
    Element(VElement),
    Text(VText),
}

#[derive(Debug, Clone)]
pub struct VElement {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<VNode>,
}

#[derive(Debug, Clone)]
pub struct VText {
    pub content: String,
}

impl VNode {
    pub fn text(content: &str) -> Self {
        VNode::Text(VText { content: String::from(content) })
    }

    pub fn element(tag: &str, children: Vec<VNode>) -> Self {
        VNode::Element(VElement {
            tag: String::from(tag),
            attrs: BTreeMap::new(),
            children,
        })
    }

    pub fn element_with_attrs(
        tag: &str,
        attrs: BTreeMap<String, String>,
        children: Vec<VNode>,
    ) -> Self {
        VNode::Element(VElement {
            tag: String::from(tag),
            attrs,
            children,
        })
    }

    pub fn tag(&self) -> Option<&str> {
        match self {
            VNode::Element(el) => Some(&el.tag),
            VNode::Text(_) => None,
        }
    }

    pub fn children(&self) -> &[VNode] {
        match self {
            VNode::Element(el) => &el.children,
            VNode::Text(_) => &[],
        }
    }
}
