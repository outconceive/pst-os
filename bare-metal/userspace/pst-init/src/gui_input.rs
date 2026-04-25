use alloc::string::String;
use pst_framebuffer::Color;

const FIELD_W: usize = 250;
const FIELD_H: usize = 24;
const TAB_W: usize = 6;
const SCREEN_W: usize = 640;

#[derive(Clone, Copy, PartialEq)]
pub enum InputType {
    Text,
    Password,
    Checkbox,
}

pub struct InputField {
    pub x: usize,
    pub y: usize,
    pub input_type: InputType,
    pub label: &'static str,
    pub value: String,
    pub checked: bool,
    pub focused: bool,
}

impl InputField {
    pub fn text(x: usize, y: usize, label: &'static str) -> Self {
        Self { x, y, input_type: InputType::Text, label, value: String::new(), checked: false, focused: false }
    }

    pub fn password(x: usize, y: usize, label: &'static str) -> Self {
        Self { x, y, input_type: InputType::Password, label, value: String::new(), checked: false, focused: false }
    }

    pub fn checkbox(x: usize, y: usize, label: &'static str) -> Self {
        Self { x, y, input_type: InputType::Checkbox, label, value: String::new(), checked: false, focused: false }
    }

    pub fn tab_color(&self) -> Color {
        match self.input_type {
            InputType::Text     => Color::rgb(59, 130, 246),  // blue
            InputType::Password => Color::rgb(239, 68, 68),   // red
            InputType::Checkbox => Color::rgb(16, 185, 129),  // green
        }
    }

    pub fn contains(&self, mx: usize, my: usize) -> bool {
        mx >= self.x && mx < self.x + FIELD_W && my >= self.y && my < self.y + FIELD_H
    }

    pub fn handle_key(&mut self, ch: u8) {
        match self.input_type {
            InputType::Checkbox => {
                if ch == b' ' || ch == b'\n' { self.checked = !self.checked; }
            }
            _ => {
                if ch == 0x08 { self.value.pop(); }
                else if ch >= 0x20 && ch < 0x80 { self.value.push(ch as char); }
            }
        }
    }

    pub fn draw(&self, fb_vaddr: u64) {
        if fb_vaddr == 0 { return; }
        let vga = fb_vaddr as *mut u8;

        let bg = if self.focused { Color::rgb(60, 60, 65) } else { Color::rgb(50, 50, 55) };
        let border = if self.focused { Color::rgb(80, 80, 90) } else { Color::rgb(45, 45, 50) };

        // Field background
        fill_rect(vga, self.x, self.y, FIELD_W, FIELD_H, bg);

        // Top/bottom border
        fill_rect(vga, self.x, self.y, FIELD_W, 1, border);
        fill_rect(vga, self.x, self.y + FIELD_H - 1, FIELD_W, 1, border);

        // Colored left tab
        let tc = self.tab_color();
        fill_rect(vga, self.x, self.y, TAB_W, FIELD_H, tc);
        // Tab highlight
        fill_rect(vga, self.x, self.y, TAB_W, 1, lighten(tc));

        // Label (above or to the right depending on type)
        let label_x = self.x + FIELD_W + 8;
        draw_str(vga, label_x, self.y + 6, self.label, Color::rgb(180, 180, 180));

        // Content area
        let text_x = self.x + TAB_W + 6;
        let text_y = self.y + 6;

        match self.input_type {
            InputType::Text => {
                draw_str(vga, text_x, text_y, &self.value, Color::WHITE);
                if self.focused {
                    let cx = text_x + self.value.len() * 8;
                    fill_rect(vga, cx, self.y + 4, 2, FIELD_H - 8, Color::WHITE);
                }
            }
            InputType::Password => {
                let dots: String = (0..self.value.len()).map(|_| '*').collect();
                draw_str(vga, text_x, text_y, &dots, Color::WHITE);
                if self.focused {
                    let cx = text_x + self.value.len() * 8;
                    fill_rect(vga, cx, self.y + 4, 2, FIELD_H - 8, Color::WHITE);
                }
            }
            InputType::Checkbox => {
                // Checkbox box
                let bx = text_x;
                let by = self.y + 4;
                let bs = 16;
                fill_rect(vga, bx, by, bs, bs, Color::rgb(40, 40, 45));
                fill_rect(vga, bx, by, bs, 1, Color::rgb(70, 70, 75));
                fill_rect(vga, bx, by + bs - 1, bs, 1, Color::rgb(30, 30, 35));
                if self.checked {
                    // Checkmark — two diagonal lines
                    let cc = Color::rgb(50, 255, 120);
                    for i in 0..4 {
                        fill_rect(vga, bx + 3 + i, by + 7 + i, 2, 2, cc);
                    }
                    for i in 0..7 {
                        fill_rect(vga, bx + 6 + i, by + 10 - i, 2, 2, cc);
                    }
                }
                if self.focused {
                    fill_rect(vga, bx - 1, by - 1, bs + 2, 1, Color::WHITE);
                    fill_rect(vga, bx - 1, by + bs, bs + 2, 1, Color::WHITE);
                }
            }
        }
    }
}

fn fill_rect(vga: *mut u8, x: usize, y: usize, w: usize, h: usize, c: Color) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < SCREEN_W && py < 480 {
                let off = (py * SCREEN_W + px) * 4;
                unsafe {
                    *vga.add(off) = c.b;
                    *vga.add(off + 1) = c.g;
                    *vga.add(off + 2) = c.r;
                    *vga.add(off + 3) = 0xFF;
                }
            }
        }
    }
}

fn draw_str(vga: *mut u8, x: usize, y: usize, s: &str, fg: Color) {
    let mut cx = x;
    for ch in s.bytes() {
        let glyph = pst_framebuffer::font::glyph(ch);
        for gy in 0..pst_framebuffer::font::GLYPH_HEIGHT {
            let bits = glyph[gy];
            for gx in 0..pst_framebuffer::font::GLYPH_WIDTH {
                if bits & (0x80 >> gx) != 0 {
                    let px = cx + gx;
                    let py = y + gy;
                    if px < SCREEN_W && py < 480 {
                        let off = (py * SCREEN_W + px) * 4;
                        unsafe {
                            *vga.add(off) = fg.b;
                            *vga.add(off + 1) = fg.g;
                            *vga.add(off + 2) = fg.r;
                        }
                    }
                }
            }
        }
        cx += pst_framebuffer::font::GLYPH_WIDTH;
    }
}

fn lighten(c: Color) -> Color {
    Color::rgb(c.r.saturating_add(40), c.g.saturating_add(40), c.b.saturating_add(40))
}
