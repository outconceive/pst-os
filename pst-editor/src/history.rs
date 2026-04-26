use alloc::string::String;
use alloc::vec::Vec;

use crate::cursor::Cursor;
use crate::line::{Line, MetaLine};

#[derive(Clone, Debug)]
pub enum Operation {
    InsertChar { cursor: Cursor, ch: char, style: char },
    DeleteChar { cursor: Cursor, ch: char, style: char },
    InsertLine { line_idx: usize, line: Line },
    DeleteLine { line_idx: usize, line: Line },
    SplitLine { line_idx: usize, col: usize },
    MergeLine { line_idx: usize, prev_len: usize, merged_line: Line },
    SetLineMeta { line_idx: usize, old_meta: MetaLine, new_meta: MetaLine },
    ApplyStyle { line_idx: usize, start: usize, end: usize, old_styles: String, new_styles: String },
    Batch { ops: Vec<Operation> },
}

#[derive(Clone, Debug)]
pub struct History {
    pub undo_stack: Vec<Operation>,
    pub redo_stack: Vec<Operation>,
    max_size: usize,
}

impl History {
    pub fn new(max_size: usize) -> Self {
        Self { undo_stack: Vec::new(), redo_stack: Vec::new(), max_size }
    }

    pub fn push(&mut self, op: Operation) {
        self.redo_stack.clear();
        self.undo_stack.push(op);
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<Operation> {
        let op = self.undo_stack.pop()?;
        self.redo_stack.push(op.clone());
        Some(op)
    }

    pub fn redo(&mut self) -> Option<Operation> {
        let op = self.redo_stack.pop()?;
        self.undo_stack.push(op.clone());
        Some(op)
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
