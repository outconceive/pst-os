use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    Heading(u8),
    Bold,
    Italic,
    Code,
    Strikethrough,
    ClearFormat,
    UnorderedList,
    OrderedList,
    Quote,
    CodeBlock,
    HorizontalRule,
    Indent,
    Dedent,
    Link,
    Undo,
    Redo,
    Export,
    Save,
    DarkMode,
    Quit,
    None,
}

#[derive(Clone, Copy)]
pub struct ToolbarButton {
    pub label: &'static str,
    pub action: ToolbarAction,
    pub width: usize,
    pub icon: Option<&'static [u8; 24]>,
}

pub const SEPARATOR: ToolbarButton = ToolbarButton {
    label: "", action: ToolbarAction::None, width: 6, icon: None,
};

const ICON_BOLD: [u8; 24] = [
    0b00000000, 0b00000000,
    0b01111100, 0b01111110,
    0b01100110, 0b01100110,
    0b01100110, 0b01100110,
    0b01111100, 0b01111110,
    0b01100110, 0b01100110,
    0b01100110, 0b01100110,
    0b01111100, 0b01111110,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

const ICON_ITALIC: [u8; 24] = [
    0b00000000, 0b00000000,
    0b00011110, 0b00011110,
    0b00001100, 0b00001100,
    0b00001100, 0b00011000,
    0b00011000, 0b00011000,
    0b00011000, 0b00110000,
    0b00110000, 0b00110000,
    0b01111000, 0b01111000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

const ICON_CODE: [u8; 24] = [
    0b00000000, 0b00000000,
    0b00100000, 0b00000100,
    0b01000000, 0b00000010,
    0b10000000, 0b00000001,
    0b10000000, 0b00000001,
    0b01000000, 0b00000010,
    0b00100000, 0b00000100,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

const ICON_STRIKE: [u8; 24] = [
    0b00000000, 0b00000000,
    0b00111100, 0b01000010,
    0b01000000, 0b01000000,
    0b00111100, 0b11111110,
    0b00000010, 0b00000010,
    0b01000010, 0b00111100,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

const ICON_UNDO: [u8; 24] = [
    0b00000000, 0b00000000,
    0b00010000, 0b00001000,
    0b00100000, 0b00111110,
    0b01000010, 0b00000010,
    0b00000010, 0b00000100,
    0b00001000, 0b00010000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

const ICON_REDO: [u8; 24] = [
    0b00000000, 0b00000000,
    0b00001000, 0b00010000,
    0b00000100, 0b01111100,
    0b01000000, 0b01000010,
    0b01000000, 0b00100000,
    0b00010000, 0b00001000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

const ICON_SAVE: [u8; 24] = [
    0b00000000, 0b00000000,
    0b01111110, 0b01011010,
    0b01000010, 0b01000010,
    0b01000010, 0b01000010,
    0b01000010, 0b01000010,
    0b01111110, 0b01011110,
    0b01000010, 0b01111110,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

pub fn default_toolbar() -> Vec<ToolbarButton> {
    let mut buttons = Vec::new();

    buttons.push(ToolbarButton { label: "H1", action: ToolbarAction::Heading(1), width: 24, icon: None });
    buttons.push(ToolbarButton { label: "H2", action: ToolbarAction::Heading(2), width: 24, icon: None });
    buttons.push(ToolbarButton { label: "H3", action: ToolbarAction::Heading(3), width: 24, icon: None });
    buttons.push(SEPARATOR);
    buttons.push(ToolbarButton { label: "B", action: ToolbarAction::Bold, width: 22, icon: Some(&ICON_BOLD) });
    buttons.push(ToolbarButton { label: "I", action: ToolbarAction::Italic, width: 22, icon: Some(&ICON_ITALIC) });
    buttons.push(ToolbarButton { label: "<>", action: ToolbarAction::Code, width: 22, icon: Some(&ICON_CODE) });
    buttons.push(ToolbarButton { label: "S", action: ToolbarAction::Strikethrough, width: 22, icon: Some(&ICON_STRIKE) });
    buttons.push(ToolbarButton { label: "Tx", action: ToolbarAction::ClearFormat, width: 22, icon: None });
    buttons.push(SEPARATOR);
    buttons.push(ToolbarButton { label: "UL", action: ToolbarAction::UnorderedList, width: 22, icon: None });
    buttons.push(ToolbarButton { label: "OL", action: ToolbarAction::OrderedList, width: 22, icon: None });
    buttons.push(ToolbarButton { label: "\"", action: ToolbarAction::Quote, width: 22, icon: None });
    buttons.push(ToolbarButton { label: "{}", action: ToolbarAction::CodeBlock, width: 22, icon: None });
    buttons.push(ToolbarButton { label: "--", action: ToolbarAction::HorizontalRule, width: 22, icon: None });
    buttons.push(SEPARATOR);
    buttons.push(ToolbarButton { label: ">", action: ToolbarAction::Indent, width: 18, icon: None });
    buttons.push(ToolbarButton { label: "<", action: ToolbarAction::Dedent, width: 18, icon: None });
    buttons.push(SEPARATOR);
    buttons.push(ToolbarButton { label: "Lnk", action: ToolbarAction::Link, width: 26, icon: None });
    buttons.push(SEPARATOR);
    buttons.push(ToolbarButton { label: "Un", action: ToolbarAction::Undo, width: 22, icon: Some(&ICON_UNDO) });
    buttons.push(ToolbarButton { label: "Re", action: ToolbarAction::Redo, width: 22, icon: Some(&ICON_REDO) });
    buttons.push(SEPARATOR);
    buttons.push(ToolbarButton { label: "Sv", action: ToolbarAction::Save, width: 22, icon: Some(&ICON_SAVE) });
    buttons.push(SEPARATOR);
    buttons.push(ToolbarButton { label: "DK", action: ToolbarAction::DarkMode, width: 22, icon: None });
    buttons.push(ToolbarButton { label: "X", action: ToolbarAction::Quit, width: 22, icon: None });

    buttons
}

pub fn hit_test(buttons: &[ToolbarButton], click_x: usize) -> ToolbarAction {
    let mut x: usize = 4;
    for btn in buttons {
        if btn.action == ToolbarAction::None {
            x += btn.width;
            continue;
        }
        if click_x >= x && click_x < x + btn.width {
            return btn.action;
        }
        x += btn.width + 2;
    }
    ToolbarAction::None
}
