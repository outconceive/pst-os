use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::cursor::{Cursor, Selection};
use crate::line::Line;
use crate::history::{History, Operation};
use crate::styles::inline;

#[derive(Clone, Debug)]
pub struct Document {
    pub lines: Vec<Line>,
    pub cursor: Cursor,
    pub selection: Option<Selection>,
    pub history: History,
}

impl Document {
    pub fn new() -> Self {
        Self {
            lines: vec![Line::new()],
            cursor: Cursor::origin(),
            selection: None,
            history: History::new(500),
        }
    }

    pub fn from_lines(lines: Vec<Line>) -> Self {
        let lines = if lines.is_empty() { vec![Line::new()] } else { lines };
        Self { lines, cursor: Cursor::origin(), selection: None, history: History::new(500) }
    }

    pub fn from_text(text: &str) -> Self {
        let lines: Vec<Line> = if text.is_empty() {
            vec![Line::new()]
        } else {
            text.split('\n').map(|s| Line::with_content(s)).collect()
        };
        Self::from_lines(lines)
    }

    fn validate_cursor(&mut self) {
        if self.cursor.line >= self.lines.len() {
            self.cursor.line = self.lines.len() - 1;
        }
        let line_len = self.lines[self.cursor.line].len();
        if self.cursor.col > line_len {
            self.cursor.col = line_len;
        }
    }

    pub fn current_line(&self) -> &Line { &self.lines[self.cursor.line] }
    pub fn line_count(&self) -> usize { self.lines.len() }

    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
        self.validate_cursor();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
        }
    }

    pub fn move_cursor_right(&mut self) {
        let line_len = self.lines[self.cursor.line].len();
        if self.cursor.col < line_len {
            self.cursor.col += 1;
        } else if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let line_len = self.lines[self.cursor.line].len();
            if self.cursor.col > line_len { self.cursor.col = line_len; }
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            let line_len = self.lines[self.cursor.line].len();
            if self.cursor.col > line_len { self.cursor.col = line_len; }
        }
    }

    pub fn move_to_line_start(&mut self) { self.cursor.col = 0; }
    pub fn move_to_line_end(&mut self) { self.cursor.col = self.lines[self.cursor.line].len(); }

    pub fn set_selection(&mut self, selection: Selection) { self.selection = Some(selection); }
    pub fn clear_selection(&mut self) { self.selection = None; }

    pub fn get_selected_text(&self) -> Option<String> {
        let sel = self.selection?.normalized();
        if sel.is_collapsed() { return None; }

        if !sel.is_multiline() {
            let line = &self.lines[sel.start.line];
            let end = sel.end.col.min(line.len());
            let start = sel.start.col.min(end);
            return Some(String::from(&line.content[start..end]));
        }

        let mut result = String::new();
        let first = &self.lines[sel.start.line];
        let start_col = sel.start.col.min(first.len());
        result.push_str(&first.content[start_col..]);

        for i in (sel.start.line + 1)..sel.end.line {
            result.push('\n');
            result.push_str(&self.lines[i].content);
        }

        if sel.end.line > sel.start.line {
            result.push('\n');
            let last = &self.lines[sel.end.line];
            let end_col = sel.end.col.min(last.len());
            result.push_str(&last.content[..end_col]);
        }
        Some(result)
    }

    pub(crate) fn current_style_at_cursor(&self) -> char {
        let line = &self.lines[self.cursor.line];
        if self.cursor.col > 0 && self.cursor.col <= line.len() {
            line.get_style_at(self.cursor.col - 1)
        } else {
            inline::PLAIN
        }
    }

    // === Text operations ===

    pub fn insert_char(&mut self, ch: char) {
        if self.selection.is_some() { self.delete_selection(); }
        let style = self.current_style_at_cursor();
        let cursor = self.cursor;
        self.lines[cursor.line].insert_char(cursor.col, ch, style);
        self.cursor.col += 1;
        self.history.push(Operation::InsertChar { cursor, ch, style });
    }

    pub fn delete_char_before(&mut self) {
        if self.selection.is_some() { self.delete_selection(); return; }
        let cursor = self.cursor;

        if cursor.col > 0 {
            let line = &self.lines[cursor.line];
            let ch = line.content.chars().nth(cursor.col - 1).unwrap_or(' ');
            let style = line.get_style_at(cursor.col - 1);
            self.lines[cursor.line].delete_char(cursor.col - 1);
            self.cursor.col -= 1;
            self.history.push(Operation::DeleteChar {
                cursor: Cursor::new(cursor.line, cursor.col - 1), ch, style,
            });
        } else if cursor.line > 0 {
            let prev_len = self.lines[cursor.line - 1].len();
            let merged_line = self.lines[cursor.line].clone();
            self.merge_with_previous(cursor.line);
            self.history.push(Operation::MergeLine {
                line_idx: cursor.line, prev_len, merged_line,
            });
        }
    }

    pub fn delete_char_at(&mut self) {
        if self.selection.is_some() { self.delete_selection(); return; }
        let cursor = self.cursor;
        let line_len = self.lines[cursor.line].len();

        if cursor.col < line_len {
            let ch = self.lines[cursor.line].content.chars().nth(cursor.col).unwrap_or(' ');
            let style = self.lines[cursor.line].get_style_at(cursor.col);
            self.lines[cursor.line].delete_char(cursor.col);
            self.history.push(Operation::DeleteChar { cursor, ch, style });
        } else if cursor.line < self.lines.len() - 1 {
            let merged_line = self.lines[cursor.line + 1].clone();
            self.merge_with_previous(cursor.line + 1);
            self.history.push(Operation::MergeLine {
                line_idx: cursor.line + 1, prev_len: line_len, merged_line,
            });
        }
    }

    pub fn delete_selection(&mut self) {
        let sel = match self.selection.take() {
            Some(s) => s.normalized(),
            None => return,
        };
        if sel.is_collapsed() { return; }

        if !sel.is_multiline() {
            let line = &mut self.lines[sel.start.line];
            let start = sel.start.col.min(line.len());
            let end = sel.end.col.min(line.len());
            for _ in start..end { line.delete_char(start); }
            self.cursor = sel.start;
            return;
        }

        let end_col = sel.end.col.min(self.lines[sel.end.line].len());
        let remainder = String::from(&self.lines[sel.end.line].content[end_col..]);
        let remainder_styles = String::from(&self.lines[sel.end.line].styles[end_col..]);

        let remove_count = sel.end.line - sel.start.line;
        for _ in 0..remove_count { self.lines.remove(sel.start.line + 1); }

        let start_col = sel.start.col.min(self.lines[sel.start.line].len());
        self.lines[sel.start.line].content.truncate(start_col);
        self.lines[sel.start.line].styles.truncate(start_col);
        self.lines[sel.start.line].content.push_str(&remainder);
        self.lines[sel.start.line].styles.push_str(&remainder_styles);

        self.cursor = sel.start;
        self.validate_cursor();
    }

    pub fn insert_newline(&mut self) {
        if self.selection.is_some() { self.delete_selection(); }
        let cursor = self.cursor;
        let right = self.lines[cursor.line].split_at(cursor.col);
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.lines.insert(self.cursor.line, right);
        self.history.push(Operation::SplitLine { line_idx: cursor.line, col: cursor.col });
    }

    pub fn merge_with_previous(&mut self, line_idx: usize) {
        if line_idx == 0 || line_idx >= self.lines.len() { return; }
        let current = self.lines.remove(line_idx);
        let prev_len = self.lines[line_idx - 1].len();
        self.lines[line_idx - 1].append(&current);
        self.cursor.line = line_idx - 1;
        self.cursor.col = prev_len;
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 { out.push('\n'); }
            out.push_str(&line.content);
        }
        out
    }

    pub fn word_count(&self) -> usize {
        self.lines.iter()
            .map(|l| l.content.split_whitespace().count())
            .sum()
    }

    pub fn char_count(&self) -> usize {
        self.lines.iter().map(|l| l.content.len()).sum::<usize>()
            + self.lines.len().saturating_sub(1) // newlines
    }

    // === Formatting ===

    pub fn apply_bold(&mut self) {
        if let Some(sel) = self.selection {
            let sel = sel.normalized();
            if sel.is_collapsed() || sel.is_multiline() { return; }
            let line = &mut self.lines[sel.start.line];
            let start = sel.start.col.min(line.len());
            let end = sel.end.col.min(line.len());
            let mut chars: Vec<char> = line.styles.chars().collect();
            for ch in chars.iter_mut().skip(start).take(end - start) {
                *ch = inline::toggle_bold(*ch);
            }
            line.styles = chars.into_iter().collect();
        }
    }

    pub fn apply_italic(&mut self) {
        if let Some(sel) = self.selection {
            let sel = sel.normalized();
            if sel.is_collapsed() || sel.is_multiline() { return; }
            let line = &mut self.lines[sel.start.line];
            let start = sel.start.col.min(line.len());
            let end = sel.end.col.min(line.len());
            let mut chars: Vec<char> = line.styles.chars().collect();
            for ch in chars.iter_mut().skip(start).take(end - start) {
                *ch = inline::toggle_italic(*ch);
            }
            line.styles = chars.into_iter().collect();
        }
    }

    pub fn apply_code(&mut self) {
        if let Some(sel) = self.selection {
            let sel = sel.normalized();
            if sel.is_collapsed() || sel.is_multiline() { return; }
            let line = &mut self.lines[sel.start.line];
            let start = sel.start.col.min(line.len());
            let end = sel.end.col.min(line.len());
            let mut chars: Vec<char> = line.styles.chars().collect();
            for ch in chars.iter_mut().skip(start).take(end - start) {
                *ch = inline::toggle_code(*ch);
            }
            line.styles = chars.into_iter().collect();
        }
    }

    pub fn set_heading(&mut self, level: u8) {
        use crate::styles::block;
        let line = &mut self.lines[self.cursor.line];
        if line.meta.format == block::HEADING && line.meta.level == level {
            line.set_format(block::PLAIN, 0);
        } else {
            line.set_format(block::HEADING, level);
        }
    }

    pub fn set_list(&mut self) {
        use crate::styles::block;
        let line = &mut self.lines[self.cursor.line];
        if line.meta.format == block::LIST_UNORDERED {
            line.set_format(block::PLAIN, 0);
        } else {
            line.set_format(block::LIST_UNORDERED, 1);
        }
    }

    pub fn set_ordered_list(&mut self) {
        use crate::styles::block;
        let line = &mut self.lines[self.cursor.line];
        if line.meta.format == block::LIST_ORDERED {
            line.set_format(block::PLAIN, 0);
        } else {
            line.set_format(block::LIST_ORDERED, 1);
        }
    }

    pub fn set_quote(&mut self) {
        use crate::styles::block;
        let line = &mut self.lines[self.cursor.line];
        if line.meta.format == block::QUOTE {
            line.set_format(block::PLAIN, 0);
        } else {
            line.set_format(block::QUOTE, 0);
        }
    }

    pub fn indent(&mut self) {
        let line = &mut self.lines[self.cursor.line];
        if line.meta.is_list() {
            line.meta.level = line.meta.level.saturating_add(1).min(6);
        }
    }

    pub fn dedent(&mut self) {
        let line = &mut self.lines[self.cursor.line];
        if line.meta.is_list() && line.meta.level > 1 {
            line.meta.level -= 1;
        }
    }

    pub fn insert_divider(&mut self) {
        use crate::styles::block;
        let cursor = self.cursor;
        let right = self.lines[cursor.line].split_at(cursor.col);
        let mut hr = Line::new();
        hr.set_format(block::DIVIDER, 0);
        self.lines.insert(cursor.line + 1, hr);
        self.lines.insert(cursor.line + 2, right);
        self.cursor = Cursor::new(cursor.line + 2, 0);
    }

    pub fn clear_formatting(&mut self) {
        if let Some(sel) = self.selection {
            let sel = sel.normalized();
            if sel.is_collapsed() || sel.is_multiline() { return; }
            let line = &mut self.lines[sel.start.line];
            let start = sel.start.col.min(line.len());
            let end = sel.end.col.min(line.len());
            line.apply_style_range(start, end, inline::PLAIN);
        }
    }

    pub fn clear_block_format(&mut self) {
        use crate::styles::block;
        self.lines[self.cursor.line].set_format(block::PLAIN, 0);
    }

    // === Undo/Redo ===

    pub fn undo(&mut self) -> bool {
        let op = match self.history.undo() {
            Some(op) => op,
            None => return false,
        };
        self.apply_undo(&op);
        true
    }

    pub fn redo(&mut self) -> bool {
        let op = match self.history.redo() {
            Some(op) => op,
            None => return false,
        };
        self.apply_redo(&op);
        true
    }

    fn apply_undo(&mut self, op: &Operation) {
        match op {
            Operation::InsertChar { cursor, .. } => {
                self.set_cursor(*cursor);
                self.lines[cursor.line].delete_char(cursor.col);
            }
            Operation::DeleteChar { cursor, ch, style, .. } => {
                self.lines[cursor.line].insert_char(cursor.col, *ch, *style);
                self.set_cursor(Cursor::new(cursor.line, cursor.col + 1));
            }
            Operation::InsertLine { line_idx, .. } => {
                if *line_idx < self.lines.len() { self.lines.remove(*line_idx); }
            }
            Operation::DeleteLine { line_idx, line } => {
                let idx = (*line_idx).min(self.lines.len());
                self.lines.insert(idx, line.clone());
            }
            Operation::SplitLine { line_idx, col } => {
                if *line_idx + 1 < self.lines.len() {
                    let next = self.lines.remove(*line_idx + 1);
                    self.lines[*line_idx].append(&next);
                    self.set_cursor(Cursor::new(*line_idx, *col));
                }
            }
            Operation::MergeLine { line_idx, prev_len, merged_line } => {
                self.lines[*line_idx - 1].content.truncate(*prev_len);
                self.lines[*line_idx - 1].styles.truncate(*prev_len);
                self.lines.insert(*line_idx, merged_line.clone());
                self.set_cursor(Cursor::new(*line_idx, 0));
            }
            Operation::SetLineMeta { line_idx, old_meta, .. } => {
                if let Some(line) = self.lines.get_mut(*line_idx) {
                    line.meta = old_meta.clone();
                }
            }
            Operation::ApplyStyle { line_idx, start, old_styles, .. } => {
                if let Some(line) = self.lines.get_mut(*line_idx) {
                    let mut chars: Vec<char> = line.styles.chars().collect();
                    for (i, ch) in old_styles.chars().enumerate() {
                        if *start + i < chars.len() { chars[*start + i] = ch; }
                    }
                    line.styles = chars.into_iter().collect();
                }
            }
            Operation::Batch { ops } => {
                for op in ops.iter().rev() { self.apply_undo(op); }
            }
        }
    }

    fn apply_redo(&mut self, op: &Operation) {
        match op {
            Operation::InsertChar { cursor, ch, style, .. } => {
                self.lines[cursor.line].insert_char(cursor.col, *ch, *style);
                self.set_cursor(Cursor::new(cursor.line, cursor.col + 1));
            }
            Operation::DeleteChar { cursor, .. } => {
                self.set_cursor(*cursor);
                self.lines[cursor.line].delete_char(cursor.col);
            }
            Operation::InsertLine { line_idx, line } => {
                let idx = (*line_idx).min(self.lines.len());
                self.lines.insert(idx, line.clone());
            }
            Operation::DeleteLine { line_idx, .. } => {
                if *line_idx < self.lines.len() { self.lines.remove(*line_idx); }
            }
            Operation::SplitLine { line_idx, col } => {
                let right = self.lines[*line_idx].split_at(*col);
                self.lines.insert(*line_idx + 1, right);
                self.set_cursor(Cursor::new(*line_idx + 1, 0));
            }
            Operation::MergeLine { line_idx, prev_len, .. } => {
                if *line_idx < self.lines.len() {
                    let current = self.lines.remove(*line_idx);
                    self.lines[*line_idx - 1].append(&current);
                    self.set_cursor(Cursor::new(*line_idx - 1, *prev_len));
                }
            }
            Operation::SetLineMeta { line_idx, new_meta, .. } => {
                if let Some(line) = self.lines.get_mut(*line_idx) {
                    line.meta = new_meta.clone();
                }
            }
            Operation::ApplyStyle { line_idx, start, new_styles, .. } => {
                if let Some(line) = self.lines.get_mut(*line_idx) {
                    let mut chars: Vec<char> = line.styles.chars().collect();
                    for (i, ch) in new_styles.chars().enumerate() {
                        if *start + i < chars.len() { chars[*start + i] = ch; }
                    }
                    line.styles = chars.into_iter().collect();
                }
            }
            Operation::Batch { ops } => {
                for op in ops { self.apply_redo(op); }
            }
        }
    }
}

impl Default for Document {
    fn default() -> Self { Self::new() }
}
