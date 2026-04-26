#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

impl Cursor {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    pub fn origin() -> Self {
        Self { line: 0, col: 0 }
    }

    pub fn is_at_line_start(&self) -> bool {
        self.col == 0
    }

    pub fn is_at_line_end(&self, line_len: usize) -> bool {
        self.col >= line_len
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub start: Cursor,
    pub end: Cursor,
}

impl Selection {
    pub fn new(start: Cursor, end: Cursor) -> Self {
        Self { start, end }
    }

    pub fn normalized(&self) -> Self {
        if self.start.line > self.end.line
            || (self.start.line == self.end.line && self.start.col > self.end.col)
        {
            Self { start: self.end, end: self.start }
        } else {
            *self
        }
    }

    pub fn is_multiline(&self) -> bool {
        let n = self.normalized();
        n.start.line != n.end.line
    }

    pub fn get_line_range(&self) -> core::ops::RangeInclusive<usize> {
        let n = self.normalized();
        n.start.line..=n.end.line
    }

    pub fn is_collapsed(&self) -> bool {
        self.start == self.end
    }
}
