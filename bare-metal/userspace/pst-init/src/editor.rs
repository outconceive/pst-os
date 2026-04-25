use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::keyboard;

pub struct Editor {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub filename: String,
    pub dirty: bool,
}

impl Editor {
    pub fn new(filename: &str) -> Self {
        Self {
            lines: alloc::vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            filename: String::from(filename),
            dirty: false,
        }
    }

    pub fn from_text(filename: &str, text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            alloc::vec![String::new()]
        } else {
            text.lines().map(String::from).collect()
        };
        Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            filename: String::from(filename),
            dirty: false,
        }
    }

    pub fn handle_key(&mut self, ch: u8) -> EditorAction {
        match ch {
            0x1B => return EditorAction::Save, // Esc = save & exit
            b'`' => return EditorAction::Quit, // backtick = quit without saving

            keyboard::KEY_UP => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                }
            }
            keyboard::KEY_DOWN => {
                if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                }
            }
            keyboard::KEY_LEFT => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                }
            }
            keyboard::KEY_RIGHT => {
                if self.cursor_col < self.lines[self.cursor_row].len() {
                    self.cursor_col += 1;
                } else if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }

            b'\n' => {
                let line = &self.lines[self.cursor_row];
                let rest = String::from(&line[self.cursor_col..]);
                self.lines[self.cursor_row] = String::from(&line[..self.cursor_col]);
                self.cursor_row += 1;
                self.lines.insert(self.cursor_row, rest);
                self.cursor_col = 0;
                self.dirty = true;
            }

            0x08 => { // backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.lines[self.cursor_row].remove(self.cursor_col);
                    self.dirty = true;
                } else if self.cursor_row > 0 {
                    let removed = self.lines.remove(self.cursor_row);
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                    self.lines[self.cursor_row].push_str(&removed);
                    self.dirty = true;
                }
            }

            b'\t' => {
                let line = &mut self.lines[self.cursor_row];
                line.insert_str(self.cursor_col, "    ");
                self.cursor_col += 4;
                self.dirty = true;
            }

            ch if ch < 0x80 => {
                self.lines[self.cursor_row].insert(self.cursor_col, ch as char);
                self.cursor_col += 1;
                self.dirty = true;
            }

            _ => {}
        }

        EditorAction::Continue
    }

    pub fn to_text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn render(&self) -> String {
        let mut out = String::new();

        out.push_str("\x1b[2J\x1b[H");

        // Title bar
        out.push_str("\x1b[7m");
        let status = if self.dirty { " [modified]" } else { "" };
        let title = format!(" {} {}  Ln {}, Col {}  Esc=save `=quit ",
            self.filename, status, self.cursor_row + 1, self.cursor_col + 1);
        out.push_str(&title);
        let pad = 80usize.saturating_sub(title.len());
        for _ in 0..pad { out.push(' '); }
        out.push_str("\x1b[0m\r\n");

        // Content
        let visible_start = if self.cursor_row > 20 { self.cursor_row - 20 } else { 0 };
        let visible_end = (visible_start + 22).min(self.lines.len());

        for i in visible_start..visible_end {
            // Line number
            out.push_str("\x1b[2m");
            let num = format!("{:>4} ", i + 1);
            out.push_str(&num);
            out.push_str("\x1b[0m");

            let line = &self.lines[i];

            if i == self.cursor_row {
                // Render line with cursor
                let before = &line[..self.cursor_col.min(line.len())];
                out.push_str(before);

                // Cursor character
                out.push_str("\x1b[7m");
                if self.cursor_col < line.len() {
                    let ch = line.as_bytes()[self.cursor_col];
                    out.push(ch as char);
                } else {
                    out.push(' ');
                }
                out.push_str("\x1b[0m");

                if self.cursor_col + 1 < line.len() {
                    out.push_str(&line[self.cursor_col + 1..]);
                }
            } else {
                out.push_str(line);
            }

            out.push_str("\r\n");
        }

        // Bottom bar
        let total = self.lines.len();
        out.push_str("\x1b[2m");
        out.push_str(&format!("  {} lines | {}", total, if self.filename.ends_with(".md") { "Markout" } else { "Text" }));
        out.push_str("\x1b[0m\r\n");

        out
    }
}

pub enum EditorAction {
    Continue,
    Save,
    Quit,
}
