use alloc::string::String;
use alloc::format;

use pst_editor::document::Document;
use pst_editor::cursor::Cursor;
use pst_editor::toolbar::{self, ToolbarAction, ToolbarButton};
use pst_editor::styles::{inline, block};
use pst_framebuffer::{Framebuffer, Color};

use crate::ps2::{self, Ps2, InputEvent};
use crate::serial_print;

const TOOLBAR_H: usize = 24;
const EDITOR_Y: usize = TOOLBAR_H + 2;
const LINE_H: usize = 14;
const GUTTER_W: usize = 40;
const TEXT_X: usize = GUTTER_W + 4;

pub enum EditorAction {
    Continue,
    Save,
    Quit,
}

pub struct Editor {
    pub doc: Document,
    pub filename: String,
    pub scroll_top: usize,
    toolbar: alloc::vec::Vec<ToolbarButton>,
    pub dark_mode: bool,
}

impl Editor {
    pub fn new(filename: &str) -> Self {
        Self {
            doc: Document::new(),
            filename: String::from(filename),
            scroll_top: 0,
            toolbar: toolbar::default_toolbar(),
            dark_mode: true,
        }
    }

    pub fn from_text(filename: &str, text: &str) -> Self {
        Self {
            doc: Document::from_text(text),
            filename: String::from(filename),
            scroll_top: 0,
            toolbar: toolbar::default_toolbar(),
            dark_mode: true,
        }
    }

    pub fn to_text(&self) -> String {
        self.doc.to_text()
    }

    pub fn handle_key(&mut self, ch: u8) -> EditorAction {
        match ch {
            0x1B => return EditorAction::Save,
            b'`' => return EditorAction::Quit,

            ps2::KEY_F1 => { self.doc.set_heading(1); }
            ps2::KEY_F2 => { self.doc.set_heading(2); }
            ps2::KEY_F3 => { self.doc.set_heading(3); }

            ps2::KEY_UP => self.doc.move_cursor_up(),
            ps2::KEY_DOWN => self.doc.move_cursor_down(),
            ps2::KEY_LEFT => self.doc.move_cursor_left(),
            ps2::KEY_RIGHT => self.doc.move_cursor_right(),

            b'\n' => { self.doc.insert_newline(); }

            0x08 => { self.doc.delete_char_before(); }

            0x7F => { self.doc.delete_char_at(); }

            b'\t' => {
                for _ in 0..4 { self.doc.insert_char(' '); }
            }

            // Ctrl+Z undo (scancode 0x1A in some mappings)
            0x1A => { self.doc.undo(); }
            // Ctrl+Y redo
            0x19 => { self.doc.redo(); }

            ch if ch >= 0x20 && ch < 0x80 => {
                self.doc.insert_char(ch as char);
            }

            _ => {}
        }

        self.ensure_cursor_visible();
        EditorAction::Continue
    }

    pub fn handle_toolbar_click(&mut self, x: usize) -> Option<EditorAction> {
        let action = toolbar::hit_test(&self.toolbar, x);
        match action {
            ToolbarAction::Heading(level) => self.doc.set_heading(level),
            ToolbarAction::Bold => self.doc.apply_bold(),
            ToolbarAction::Italic => self.doc.apply_italic(),
            ToolbarAction::Code => self.doc.apply_code(),
            ToolbarAction::Strikethrough => self.doc.apply_strikethrough(),
            ToolbarAction::ClearFormat => self.doc.clear_formatting(),
            ToolbarAction::UnorderedList => self.doc.set_list(),
            ToolbarAction::OrderedList => self.doc.set_ordered_list(),
            ToolbarAction::Quote => self.doc.set_quote(),
            ToolbarAction::CodeBlock => {}
            ToolbarAction::HorizontalRule => self.doc.insert_divider(),
            ToolbarAction::Indent => self.doc.indent(),
            ToolbarAction::Dedent => self.doc.dedent(),
            ToolbarAction::Link => {}
            ToolbarAction::Undo => { self.doc.undo(); }
            ToolbarAction::Redo => { self.doc.redo(); }
            ToolbarAction::Export => {}
            ToolbarAction::Save => return Some(EditorAction::Save),
            ToolbarAction::DarkMode => { self.dark_mode = !self.dark_mode; }
            ToolbarAction::Quit => return Some(EditorAction::Quit),
            ToolbarAction::None => {}
        }
        None
    }

    fn ensure_cursor_visible(&mut self) {
        if self.doc.cursor.line < self.scroll_top {
            self.scroll_top = self.doc.cursor.line;
            return;
        }
        // Count how many lines fit from scroll_top
        let mut y = EDITOR_Y;
        let mut i = self.scroll_top;
        while i < self.doc.lines.len() && y + self.line_height(i) < 476 {
            if i == self.doc.cursor.line { return; } // cursor is visible
            y += self.line_height(i);
            i += 1;
        }
        // Cursor is past visible area — scroll down
        self.scroll_top = self.doc.cursor.line;
    }

    fn bg(&self) -> Color {
        if self.dark_mode { Color::rgb(30, 30, 35) } else { Color::rgb(255, 255, 255) }
    }
    fn fg(&self) -> Color {
        if self.dark_mode { Color::rgb(210, 210, 210) } else { Color::rgb(60, 60, 60) }
    }
    fn gutter_bg(&self) -> Color {
        if self.dark_mode { Color::rgb(35, 35, 40) } else { Color::rgb(248, 248, 248) }
    }
    fn toolbar_bg(&self) -> Color {
        if self.dark_mode { Color::rgb(40, 40, 45) } else { Color::rgb(248, 248, 248) }
    }
    fn cursor_line_bg(&self) -> Color {
        if self.dark_mode { Color::rgb(45, 45, 55) } else { Color::rgb(245, 245, 245) }
    }
    fn border_color(&self) -> Color {
        if self.dark_mode { Color::rgb(60, 60, 65) } else { Color::rgb(220, 220, 220) }
    }
    fn gutter_fg(&self) -> Color {
        if self.dark_mode { Color::rgb(90, 90, 100) } else { Color::rgb(180, 180, 180) }
    }
    fn gutter_active_fg(&self) -> Color {
        if self.dark_mode { Color::rgb(180, 180, 190) } else { Color::rgb(100, 100, 100) }
    }

    fn line_height(&self, line_idx: usize) -> usize {
        let line = &self.doc.lines[line_idx];
        match line.meta.format {
            block::HEADING => match line.meta.level {
                1 => 52, // 16 * 3 + 4 padding
                2 => 36, // 16 * 2 + 4 padding
                3 => 20,
                _ => 18,
            },
            _ => LINE_H,
        }
    }

    fn char_scale(&self, line_idx: usize) -> usize {
        let line = &self.doc.lines[line_idx];
        if line.meta.format == block::HEADING {
            match line.meta.level {
                1 => 3,
                2 => 2,
                _ => 1,
            }
        } else {
            1
        }
    }

    pub fn render_to_fb(&self, fb: &mut Framebuffer) {
        fb.clear(self.bg());

        self.render_toolbar(fb);

        fb.draw_hline(0, TOOLBAR_H, 640, self.border_color());

        let mut y = EDITOR_Y;
        let mut i = self.scroll_top;
        while i < self.doc.lines.len() && y + 4 < 480 {
            let lh = self.line_height(i);
            self.render_line(fb, i, y);
            y += lh;
            i += 1;
        }
    }

    fn render_toolbar(&self, fb: &mut Framebuffer) {
        let tb_bg = self.toolbar_bg();
        fb.fill_rect(0, 0, 640, TOOLBAR_H, tb_bg);

        let mut x: usize = 4;
        let btn_h: usize = 20;
        let btn_y: usize = 2;
        let sep_color = self.border_color();

        for btn in &self.toolbar {
            if btn.action == ToolbarAction::None {
                fb.fill_rect(x + 2, btn_y + 2, 1, btn_h - 4, sep_color);
                x += btn.width;
                continue;
            }

            let is_active = self.is_button_active(btn.action);
            let is_dark_toggle = btn.action == ToolbarAction::DarkMode;

            let btn_bg = if is_dark_toggle && self.dark_mode {
                Color::rgb(59, 130, 246)
            } else if is_active {
                if self.dark_mode { Color::rgb(50, 60, 80) } else { Color::rgb(220, 230, 245) }
            } else {
                tb_bg
            };

            let btn_fg = if is_dark_toggle && self.dark_mode {
                Color::WHITE
            } else if is_active {
                Color::rgb(59, 130, 246)
            } else if self.dark_mode {
                Color::rgb(170, 170, 180)
            } else {
                Color::rgb(80, 80, 80)
            };

            fb.fill_rect(x, btn_y, btn.width, btn_h, btn_bg);

            if is_active {
                fb.draw_hline(x, btn_y + btn_h - 1, btn.width, Color::rgb(59, 130, 246));
            }

            let text_w = btn.label.len() * 8;
            let tx = x + (btn.width.saturating_sub(text_w)) / 2;
            let ty = btn_y + (btn_h - 10) / 2;
            fb.draw_text(tx, ty, btn.label, btn_fg, btn_bg);

            x += btn.width + 2;
        }
    }

    fn is_button_active(&self, action: ToolbarAction) -> bool {
        let line = &self.doc.lines[self.doc.cursor.line];
        let cursor_style = self.doc.current_style_at_cursor();
        match action {
            ToolbarAction::Heading(level) => {
                line.meta.format == block::HEADING && line.meta.level == level
            }
            ToolbarAction::Bold => inline::is_bold(cursor_style),
            ToolbarAction::Italic => inline::is_italic(cursor_style),
            ToolbarAction::Code => inline::is_code(cursor_style),
            ToolbarAction::Strikethrough => inline::is_strikethrough(cursor_style),
            ToolbarAction::UnorderedList => line.meta.format == block::LIST_UNORDERED,
            ToolbarAction::OrderedList => line.meta.format == block::LIST_ORDERED,
            ToolbarAction::Quote => line.meta.format == block::QUOTE,
            ToolbarAction::DarkMode => self.dark_mode,
            _ => false,
        }
    }

    fn render_line(&self, fb: &mut Framebuffer, line_idx: usize, y: usize) {
        let line = &self.doc.lines[line_idx];
        let is_cursor_line = line_idx == self.doc.cursor.line;

        let bg = if is_cursor_line { self.cursor_line_bg() } else { self.bg() };

        if is_cursor_line {
            fb.fill_rect(0, y, 640, LINE_H, bg);
        }

        let num = format!("{:>3}", line_idx + 1);
        let gutter_fg = if is_cursor_line { self.gutter_active_fg() } else { self.gutter_fg() };
        fb.draw_text(4, y + 2, &num, gutter_fg, bg);

        fb.fill_rect(GUTTER_W - 1, y, 1, LINE_H, self.border_color());

        let is_heading = line.meta.format == block::HEADING;
        let heading_level = line.meta.level;

        // Block format prefix (WYSIWYG — no markdown symbols)
        let mut text_x = TEXT_X;
        match line.meta.format {
            block::HEADING => {
                // No prefix — text renders larger below
            }
            block::LIST_UNORDERED => {
                let indent = (line.meta.level.saturating_sub(1) as usize) * 16;
                text_x += indent;
                let bullet_y = y + LINE_H / 2 - 1;
                fb.fill_rect(text_x + 2, bullet_y, 4, 4, self.fg());
                text_x += 12;
            }
            block::LIST_ORDERED => {
                let indent = (line.meta.level.saturating_sub(1) as usize) * 16;
                text_x += indent;
                fb.draw_text(text_x, y + 2, "1.", self.gutter_active_fg(), bg);
                text_x += 20;
            }
            block::QUOTE => {
                fb.fill_rect(text_x, y, 3, LINE_H, Color::rgb(59, 130, 246));
                text_x += 12;
            }
            block::DIVIDER => {
                let line_color = if self.dark_mode { Color::rgb(80, 80, 85) } else { Color::rgb(200, 200, 200) };
                fb.draw_hline(text_x, y + LINE_H / 2, 640 - text_x - 8, line_color);
                return;
            }
            _ => {}
        }

        // Compute selection range for this line
        let (sel_start, sel_end) = if let Some(sel) = self.doc.selection {
            let s = sel.normalized();
            if line_idx >= s.start.line && line_idx <= s.end.line {
                let start = if line_idx == s.start.line { s.start.col } else { 0 };
                let end = if line_idx == s.end.line { s.end.col } else { line.len() };
                (start, end)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        // Render content with per-character styles
        for (ci, ch) in line.content.chars().enumerate() {
            let style = line.get_style_at(ci);
            let in_selection = ci >= sel_start && ci < sel_end && sel_start != sel_end;

            let mut fg = if inline::is_code(style) {
                if self.dark_mode { Color::rgb(255, 120, 100) } else { Color::rgb(200, 50, 50) }
            } else if inline::is_link(style) {
                Color::rgb(59, 130, 246)
            } else if is_heading {
                if self.dark_mode { Color::rgb(240, 240, 245) } else { Color::rgb(20, 20, 20) }
            } else if inline::is_bold(style) {
                if self.dark_mode { Color::rgb(240, 240, 240) } else { Color::rgb(20, 20, 20) }
            } else {
                self.fg()
            };

            let char_bg = if in_selection {
                if self.dark_mode { Color::rgb(59, 100, 180) } else { Color::rgb(180, 210, 255) }
            } else if inline::is_code(style) {
                if self.dark_mode { Color::rgb(50, 40, 40) } else { Color::rgb(245, 240, 240) }
            } else {
                bg
            };

            if in_selection {
                fg = if self.dark_mode { Color::WHITE } else { Color::rgb(30, 30, 30) };
            }

            let cs = alloc::format!("{}", ch);
            let scale = self.char_scale(line_idx);
            let char_w = 8 * scale;
            let lh = self.line_height(line_idx);
            let draw_bold = inline::is_bold(style) || is_heading;
            let draw_italic = inline::is_italic(style);

            if scale > 1 {
                // Scaled rendering: draw glyph at NxN
                let glyph = pst_framebuffer::font::glyph(ch as u8);
                let gw = pst_framebuffer::font::GLYPH_WIDTH;
                let gh = pst_framebuffer::font::GLYPH_HEIGHT;
                let ty = y + (lh - gh * scale) / 2;

                // Fill background for scaled char
                fb.fill_rect(text_x, y, char_w, lh, char_bg);

                for gy in 0..gh {
                    for gx in 0..gw {
                        if glyph[gy] & (0x80 >> gx) != 0 {
                            let px = text_x + gx * scale;
                            let py = ty + gy * scale;
                            fb.fill_rect(px, py, scale, scale, fg);
                            // Faux bold at scale
                            if draw_bold {
                                fb.fill_rect(px + 1, py, scale, scale, fg);
                            }
                        }
                    }
                }
            } else {
                if draw_italic {
                    fb.draw_text(text_x + 1, y + 2, &cs, fg, char_bg);
                    fb.draw_text(text_x, y + 2 + 5, &cs, fg, char_bg);
                } else {
                    fb.draw_text(text_x, y + 2, &cs, fg, char_bg);
                }

                if draw_bold {
                    if draw_italic {
                        fb.draw_text(text_x + 2, y + 2, &cs, fg, char_bg);
                        fb.draw_text(text_x + 1, y + 2 + 5, &cs, fg, char_bg);
                    } else {
                        fb.draw_text(text_x + 1, y + 2, &cs, fg, char_bg);
                    }
                }
            }

            // Underline
            if inline::is_underline(style) || inline::is_link(style) {
                fb.draw_hline(text_x, y + lh - 3, char_w, fg);
            }

            // Strikethrough
            if inline::is_strikethrough(style) {
                fb.draw_hline(text_x, y + lh / 2, char_w, fg);
            }

            text_x += char_w;
        }

        // Cursor
        if is_cursor_line {
            let scale = self.char_scale(line_idx);
            let cursor_x = TEXT_X + self.cursor_visual_col() * 8 * scale;
            let lh = self.line_height(line_idx);
            fb.fill_rect(cursor_x, y + 1, 2, lh - 2, Color::rgb(59, 130, 246));
        }
    }

    fn cursor_visual_col(&self) -> usize {
        let line = &self.doc.lines[self.doc.cursor.line];
        let mut vcol = self.doc.cursor.col;

        match line.meta.format {
            block::HEADING => {} // WYSIWYG — no prefix
            block::LIST_UNORDERED => {
                let indent = (line.meta.level.saturating_sub(1) as usize) * 2;
                vcol += indent + 1; // bullet width in char units
            }
            block::LIST_ORDERED => {
                let indent = (line.meta.level.saturating_sub(1) as usize) * 2;
                vcol += indent + 2; // "1." width
            }
            block::QUOTE => { vcol += 1; }
            _ => {}
        }
        vcol
    }
}

fn screen_to_cursor(ed: &Editor, x: usize, y: usize) -> Option<Cursor> {
    if y < EDITOR_Y { return None; }

    let mut line_y = EDITOR_Y;
    let mut line_idx = ed.scroll_top;
    while line_idx < ed.doc.lines.len() {
        let lh = ed.line_height(line_idx);
        if y < line_y + lh {
            let scale = ed.char_scale(line_idx);
            let col = x.saturating_sub(TEXT_X) / (8 * scale);
            let col = col.min(ed.doc.lines[line_idx].len());
            return Some(Cursor::new(line_idx, col));
        }
        line_y += lh;
        line_idx += 1;
    }
    None
}

pub fn run_editor(ps2: &mut Ps2, fb_vaddr: u64, filename: &str, initial_text: Option<&str>) -> Option<String> {
    if fb_vaddr == 0 { return None; }

    use pst_editor::cursor::Selection;

    let mut ed = match initial_text {
        Some(text) => Editor::from_text(filename, text),
        None => Editor::new(filename),
    };

    let mut drag_anchor: Option<Cursor> = None;
    let mut mousedown_on_toolbar = false;
    let mut dragging_in_editor = false;

    loop {
        let mut fb = Framebuffer::new(640, 480);
        ed.render_to_fb(&mut fb);

        // Draw dashed selection rectangle if dragging anywhere in editor area
        if dragging_in_editor {
            if let Some((rx0, ry0, rx1, ry1)) = ps2.drag_rect() {
                draw_dashed_rect(&mut fb, rx0, ry0, rx1, ry1, &ed);
            }
        }

        let vga = fb_vaddr as *mut u8;
        unsafe { core::ptr::copy_nonoverlapping(fb.pixels.as_ptr(), vga, 640 * 4 * 480); }

        match ps2.read_event() {
            InputEvent::Key(ch) => {
                drag_anchor = None;
                dragging_in_editor = false;
                mousedown_on_toolbar = false;
                ed.doc.clear_selection();
                match ed.handle_key(ch) {
                    EditorAction::Save => return Some(ed.to_text()),
                    EditorAction::Quit => return None,
                    EditorAction::Continue => {}
                }
            }
            InputEvent::MouseDown { x, y } => {
                mousedown_on_toolbar = y < TOOLBAR_H;
                dragging_in_editor = y >= EDITOR_Y;
                if y >= EDITOR_Y {
                    ed.doc.clear_selection();
                    if let Some(cur) = screen_to_cursor(&ed, x, y) {
                        ed.doc.set_cursor(cur);
                        drag_anchor = Some(cur);
                    } else {
                        // Click below last line — snap to end of document
                        let last = ed.doc.lines.len() - 1;
                        let col = ed.doc.lines[last].len();
                        let cur = Cursor::new(last, col);
                        ed.doc.set_cursor(cur);
                        drag_anchor = Some(cur);
                    }
                }
            }
            InputEvent::MouseDrag { x, y } => {
                if let Some(anchor) = drag_anchor {
                    if y >= EDITOR_Y {
                        if let Some(cur) = screen_to_cursor(&ed, x, y) {
                            ed.doc.set_cursor(cur);
                            if anchor != cur {
                                ed.doc.set_selection(Selection::new(anchor, cur));
                            }
                        }
                    }
                }
            }
            InputEvent::Click { x, y } => {
                if y < TOOLBAR_H && mousedown_on_toolbar {
                    if let Some(action) = ed.handle_toolbar_click(x) {
                        match action {
                            EditorAction::Save => return Some(ed.to_text()),
                            EditorAction::Quit => return None,
                            _ => {}
                        }
                    }
                } else if y >= EDITOR_Y {
                    if drag_anchor.is_none() {
                        if let Some(cur) = screen_to_cursor(&ed, x, y) {
                            ed.doc.set_cursor(cur);
                            ed.doc.clear_selection();
                        }
                    }
                }
                drag_anchor = None;
                dragging_in_editor = false;
            }
            _ => {}
        }
    }
}

fn draw_dashed_rect(fb: &mut Framebuffer, x0: usize, y0: usize, x1: usize, y1: usize, _ed: &Editor) {
    let color = Color::rgb(59, 130, 246);
    let dash = 4;

    // Top
    for px in x0..=x1.min(639) {
        if ((px - x0) / dash) % 2 == 0 { fb.set_pixel(px, y0, color); }
    }
    // Bottom
    for px in x0..=x1.min(639) {
        if ((px - x0) / dash) % 2 == 0 { fb.set_pixel(px, y1.min(479), color); }
    }
    // Left
    for py in y0..=y1.min(479) {
        if ((py - y0) / dash) % 2 == 0 { fb.set_pixel(x0, py, color); }
    }
    // Right
    for py in y0..=y1.min(479) {
        if ((py - y0) / dash) % 2 == 0 { fb.set_pixel(x1.min(639), py, color); }
    }
}
