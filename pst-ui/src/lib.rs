#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ComponentType {
    Label,
    Input,
    Password,
    Button,
    Checkbox,
    Divider,
}

pub struct UiRow {
    pub component: ComponentType,
    pub state_key: String,
    pub value: String,
    pub label: String,
    pub focus: bool,
    pub hover: bool,
    pub enabled: bool,
    pub tab_order: usize,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub style: String,
    pub validate: String,
}

impl UiRow {
    fn new(comp: ComponentType, key: &str, label: &str, tab: usize) -> Self {
        let (w, h) = match comp {
            ComponentType::Input | ComponentType::Password => (200, 22),
            ComponentType::Button => (label.len() * 8 + 24, 28),
            ComponentType::Checkbox => (22, 22),
            ComponentType::Divider => (400, 8),
            ComponentType::Label => (label.len() * 8, 14),
        };
        Self {
            component: comp, state_key: String::from(key),
            value: String::new(), label: String::from(label),
            focus: false, hover: false, enabled: true,
            tab_order: tab, x: 0, y: 0, w, h,
            style: String::new(),
            validate: String::new(),
        }
    }
}

pub struct UiState {
    pub rows: Vec<UiRow>,
    pub focused: Option<usize>,
}

impl UiState {
    pub fn new() -> Self {
        Self { rows: Vec::new(), focused: None }
    }

    pub fn from_markout(markout: &str) -> Self {
        let lines = pst_markout::parse::parse(markout);
        let vdom = pst_markout::render::render(&lines);
        let mut state = Self::new();
        state.extract_components(&vdom, 0);
        state.layout();
        if !state.rows.is_empty() {
            state.focused = Some(0);
            state.rows[0].focus = true;
        }
        state
    }

    fn extract_components(&mut self, node: &pst_markout::vnode::VNode, tab: usize) {
        match node {
            pst_markout::vnode::VNode::Text(_) => {}
            pst_markout::vnode::VNode::Element(el) => {
                let class = el.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
                let bind = el.attrs.get("data-bind").map(|s| s.as_str()).unwrap_or("");

                let validate_str = el.attrs.get("data-validate").map(|s| s.as_str()).unwrap_or("");

                if class.contains("mc-input") && !class.contains("mc-input-password") {
                    let tab = self.rows.len();
                    let mut row = UiRow::new(ComponentType::Input, bind, bind, tab);
                    row.validate = String::from(validate_str);
                    self.rows.push(row);
                } else if class.contains("mc-input-password") {
                    let tab = self.rows.len();
                    let mut row = UiRow::new(ComponentType::Password, bind, bind, tab);
                    row.validate = String::from(validate_str);
                    self.rows.push(row);
                } else if class.contains("mc-checkbox") {
                    let tab = self.rows.len();
                    self.rows.push(UiRow::new(ComponentType::Checkbox, bind, bind, tab));
                } else if class.contains("mc-button") {
                    let tab = self.rows.len();
                    let label = text_content(node);
                    let mut row = UiRow::new(ComponentType::Button, bind, &label, tab);
                    if class.contains("mc-primary") { row.style = String::from("primary"); }
                    if class.contains("mc-danger") { row.style = String::from("danger"); }
                    self.rows.push(row);
                } else if class.contains("mc-divider") {
                    self.rows.push(UiRow::new(ComponentType::Divider, "", "", usize::MAX));
                } else if class.contains("mc-label") {
                    let label = text_content(node);
                    self.rows.push(UiRow::new(ComponentType::Label, bind, &label, usize::MAX));
                }

                for child in &el.children {
                    self.extract_components(child, tab);
                }
            }
        }
    }

    fn layout(&mut self) {
        let start_x = 32;
        let mut cy = 80;
        let mut i = 0;
        while i < self.rows.len() {
            let row = &self.rows[i];
            // Inline: if next row is a label, put it beside this component
            let is_field = matches!(row.component,
                ComponentType::Input | ComponentType::Password | ComponentType::Checkbox);

            self.rows[i].x = start_x;
            self.rows[i].y = cy;

            if is_field && i + 1 < self.rows.len() && self.rows[i + 1].component == ComponentType::Label {
                let field_w = self.rows[i].w;
                let field_h = self.rows[i].h;
                self.rows[i + 1].x = start_x + field_w + 8;
                self.rows[i + 1].y = cy + 4;
                cy += field_h + 6;
                i += 2;
            } else {
                cy += self.rows[i].h + 6;
                i += 1;
            }
        }
    }

    pub fn handle_key(&mut self, ch: u8) -> Option<String> {
        let idx = match self.focused {
            Some(i) => i,
            None => return None,
        };

        match self.rows[idx].component {
            ComponentType::Input | ComponentType::Password => {
                if ch == 0x08 { self.rows[idx].value.pop(); }
                else if ch >= 0x20 && ch < 0x80 { self.rows[idx].value.push(ch as char); }
            }
            ComponentType::Checkbox => {
                if ch == b' ' || ch == b'\n' {
                    let v = if self.rows[idx].value == "true" { "false" } else { "true" };
                    self.rows[idx].value = String::from(v);
                }
            }
            ComponentType::Button => {
                if ch == b' ' || ch == b'\n' {
                    return Some(self.rows[idx].state_key.clone());
                }
            }
            _ => {}
        }
        None
    }

    pub fn handle_click(&mut self, mx: usize, my: usize) -> Option<String> {
        let mut hit = None;
        for (i, row) in self.rows.iter().enumerate() {
            if mx >= row.x && mx < row.x + row.w && my >= row.y && my < row.y + row.h {
                hit = Some((i, row.component, row.state_key.clone()));
                break;
            }
        }
        let (i, comp, key) = match hit { Some(h) => h, None => return None };
        self.set_focus(i);
        match comp {
            ComponentType::Checkbox => {
                let v = if self.rows[i].value == "true" { "false" } else { "true" };
                self.rows[i].value = String::from(v);
                None
            }
            ComponentType::Button => Some(key),
            _ => None,
        }
    }

    pub fn handle_hover(&mut self, mx: usize, my: usize) {
        for row in &mut self.rows {
            row.hover = mx >= row.x && mx < row.x + row.w && my >= row.y && my < row.y + row.h;
        }
    }

    pub fn tab_next(&mut self) {
        let focusable: Vec<usize> = self.rows.iter().enumerate()
            .filter(|(_, r)| r.tab_order != usize::MAX && r.enabled)
            .map(|(i, _)| i)
            .collect();
        if focusable.is_empty() { return; }

        let cur = self.focused.unwrap_or(0);
        let pos = focusable.iter().position(|&i| i == cur).unwrap_or(0);
        let next = focusable[(pos + 1) % focusable.len()];
        self.set_focus(next);
    }

    pub fn set_focus(&mut self, idx: usize) {
        if let Some(old) = self.focused { self.rows[old].focus = false; }
        self.rows[idx].focus = true;
        self.focused = Some(idx);
    }

    pub fn validate(&self) -> Vec<(String, String)> {
        let mut errors = Vec::new();
        for row in &self.rows {
            if row.validate.is_empty() { continue; }
            for rule in row.validate.split(',') {
                let err = match rule {
                    "required" => {
                        if row.value.is_empty() { Some(format!("{} is required", row.state_key)) } else { None }
                    }
                    "email" => {
                        if !row.value.is_empty() && !row.value.contains('@') {
                            Some(format!("{} must be a valid email", row.state_key))
                        } else { None }
                    }
                    _ if rule.starts_with("min:") => {
                        if let Ok(min) = rule[4..].parse::<usize>() {
                            if row.value.len() < min {
                                Some(format!("{} must be at least {} characters", row.state_key, min))
                            } else { None }
                        } else { None }
                    }
                    _ if rule.starts_with("max:") => {
                        if let Ok(max) = rule[4..].parse::<usize>() {
                            if row.value.len() > max {
                                Some(format!("{} must be at most {} characters", row.state_key, max))
                            } else { None }
                        } else { None }
                    }
                    _ => None,
                };
                if let Some(msg) = err {
                    errors.push((row.state_key.clone(), msg));
                }
            }
        }
        errors
    }

    pub fn get_value(&self, key: &str) -> &str {
        for row in &self.rows {
            if row.state_key == key { return &row.value; }
        }
        ""
    }
}

fn text_content(node: &pst_markout::vnode::VNode) -> String {
    match node {
        pst_markout::vnode::VNode::Text(t) => t.content.clone(),
        pst_markout::vnode::VNode::Element(el) => {
            let mut s = String::new();
            for child in &el.children { s.push_str(&text_content(child)); }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_markout() {
        let ui = UiState::from_markout("| {input:name}  Name\n| {button:go \"Submit\" primary}");
        assert_eq!(ui.rows.len(), 3); // input, label, button
        assert_eq!(ui.rows[0].component, ComponentType::Input);
        assert_eq!(ui.rows[0].state_key, "name");
    }

    #[test]
    fn test_handle_key() {
        let mut ui = UiState::from_markout("| {input:name}");
        ui.handle_key(b'a');
        ui.handle_key(b'b');
        assert_eq!(ui.get_value("name"), "ab");
    }

    #[test]
    fn test_tab_focus() {
        let mut ui = UiState::from_markout("| {input:a}\n| {input:b}");
        assert!(ui.rows[0].focus);
        ui.tab_next();
        assert!(ui.rows[1].focus);
        assert!(!ui.rows[0].focus);
    }

    #[test]
    fn test_checkbox_toggle() {
        let mut ui = UiState::from_markout("| {checkbox:ok}");
        let idx = ui.rows.iter().position(|r| r.component == ComponentType::Checkbox).unwrap();
        ui.set_focus(idx);
        ui.handle_key(b' ');
        assert_eq!(ui.get_value("ok"), "true");
        ui.handle_key(b' ');
        assert_eq!(ui.get_value("ok"), "false");
    }

    #[test]
    fn test_click_button() {
        let mut ui = UiState::from_markout("| {button:submit \"Go\" primary}");
        let action = ui.handle_click(ui.rows[0].x + 5, ui.rows[0].y + 5);
        assert_eq!(action.as_deref(), Some("submit"));
    }

    #[test]
    fn test_password_masking() {
        let mut ui = UiState::from_markout("| {password:pw}");
        ui.handle_key(b'x');
        ui.handle_key(b'y');
        assert_eq!(ui.get_value("pw"), "xy");
    }

    #[test]
    fn test_validate_required() {
        let mut ui = UiState::from_markout("| {input:email validate:required,email}");
        let errors = ui.validate();
        assert!(errors.iter().any(|(_, msg)| msg.contains("required")));

        ui.handle_key(b'a');
        let errors = ui.validate();
        assert!(errors.iter().any(|(_, msg)| msg.contains("email")));

        // Fix it
        let idx = ui.rows.iter().position(|r| r.component == ComponentType::Input).unwrap();
        ui.rows[idx].value = String::from("a@b.com");
        let errors = ui.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_min() {
        let mut ui = UiState::from_markout("| {password:pw validate:min:8}");
        ui.handle_key(b'a'); ui.handle_key(b'b'); ui.handle_key(b'c');
        let errors = ui.validate();
        assert!(errors.iter().any(|(_, msg)| msg.contains("at least 8")));
    }

    #[test]
    fn test_layout_positions() {
        let ui = UiState::from_markout("| {input:a}\n| {input:b}");
        assert!(ui.rows[0].y < ui.rows[1].y);
        assert_eq!(ui.rows[0].x, ui.rows[1].x);
    }
}
