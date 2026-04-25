use crate::keyboard::Keyboard;
use crate::mouse::{Mouse, MouseEvent};

pub enum InputEvent {
    Key(u8),
    Click { x: usize, y: usize },
}

pub struct Input {
    pub kb: Keyboard,
    pub mouse: Option<Mouse>,
}

impl Input {
    pub fn read(&mut self) -> InputEvent {
        // For now, keyboard-only blocking read
        // Mouse events are polled separately
        InputEvent::Key(self.kb.read_key())
    }

    pub fn click_col_row(&self, x: usize, y: usize) -> (usize, usize) {
        let col = x / pst_framebuffer::font::GLYPH_WIDTH;
        let row = y / pst_framebuffer::font::GLYPH_HEIGHT;
        (col, row)
    }
}
