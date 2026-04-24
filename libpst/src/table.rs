use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;

pub const TOMBSTONE: u8 = 0xFF;
pub const EMPTY: u8 = 0x00;

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data: Vec<u8>,
}

impl Column {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            data: Vec::new(),
        }
    }

    pub fn get(&self, pos: usize) -> Option<u8> {
        self.data.get(pos).copied()
    }

    pub fn set(&mut self, pos: usize, value: u8) {
        if pos < self.data.len() {
            self.data[pos] = value;
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_tombstoned(&self, pos: usize) -> bool {
        self.data.get(pos) == Some(&TOMBSTONE)
    }
}

#[derive(Debug)]
pub struct ParallelTable {
    pub columns: Vec<Column>,
    len: usize,
    live_count: usize,
    tombstone_count: usize,
}

impl ParallelTable {
    pub fn new(column_names: &[&str]) -> Self {
        let columns = column_names.iter().map(|n| Column::new(n)).collect();
        Self {
            columns,
            len: 0,
            live_count: 0,
            tombstone_count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn live_count(&self) -> usize {
        self.live_count
    }

    pub fn tombstone_count(&self) -> usize {
        self.tombstone_count
    }

    pub fn append(&mut self, values: &[u8]) -> usize {
        let pos = self.len;
        for (i, col) in self.columns.iter_mut().enumerate() {
            let v = values.get(i).copied().unwrap_or(EMPTY);
            col.data.push(v);
        }
        self.len += 1;
        self.live_count += 1;
        pos
    }

    pub fn tombstone(&mut self, pos: usize) {
        if pos >= self.len {
            return;
        }
        if self.columns[0].is_tombstoned(pos) {
            return;
        }
        for col in &mut self.columns {
            col.data[pos] = TOMBSTONE;
        }
        self.live_count -= 1;
        self.tombstone_count += 1;
    }

    pub fn is_live(&self, pos: usize) -> bool {
        pos < self.len && !self.columns[0].is_tombstoned(pos)
    }

    pub fn get(&self, col_index: usize, pos: usize) -> Option<u8> {
        self.columns.get(col_index).and_then(|c| c.get(pos))
    }

    pub fn set(&mut self, col_index: usize, pos: usize, value: u8) {
        if let Some(col) = self.columns.get_mut(col_index) {
            col.set(pos, value);
        }
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    pub fn compact(&mut self) -> Vec<(usize, usize)> {
        let mut remap = Vec::new();
        let mut write = 0;

        for read in 0..self.len {
            if self.columns[0].is_tombstoned(read) {
                continue;
            }
            if write != read {
                for col in &mut self.columns {
                    col.data[write] = col.data[read];
                }
            }
            remap.push((read, write));
            write += 1;
        }

        for col in &mut self.columns {
            col.data.truncate(write);
        }

        self.len = write;
        self.live_count = write;
        self.tombstone_count = 0;
        remap
    }

    pub fn scan<F>(&self, col_index: usize, predicate: F) -> Vec<usize>
    where
        F: Fn(u8) -> bool,
    {
        let col = match self.columns.get(col_index) {
            Some(c) => c,
            None => return Vec::new(),
        };
        let mut results = Vec::new();
        for i in 0..self.len {
            if !self.columns[0].is_tombstoned(i) {
                if let Some(&v) = col.data.get(i) {
                    if predicate(v) {
                        results.push(i);
                    }
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_get() {
        let mut t = ParallelTable::new(&["state", "owner"]);
        let pos = t.append(&[b'R', 1]);
        assert_eq!(pos, 0);
        assert_eq!(t.get(0, 0), Some(b'R'));
        assert_eq!(t.get(1, 0), Some(1));
        assert_eq!(t.len(), 1);
        assert_eq!(t.live_count(), 1);
    }

    #[test]
    fn test_tombstone() {
        let mut t = ParallelTable::new(&["state"]);
        t.append(&[b'R']);
        t.append(&[b'S']);
        t.tombstone(0);
        assert!(!t.is_live(0));
        assert!(t.is_live(1));
        assert_eq!(t.live_count(), 1);
        assert_eq!(t.tombstone_count(), 1);
    }

    #[test]
    fn test_compact() {
        let mut t = ParallelTable::new(&["state", "owner"]);
        t.append(&[b'R', 1]);
        t.append(&[b'S', 2]);
        t.append(&[b'R', 3]);
        t.tombstone(1);

        let remap = t.compact();
        assert_eq!(t.len(), 2);
        assert_eq!(t.live_count(), 2);
        assert_eq!(t.tombstone_count(), 0);
        assert_eq!(t.get(0, 0), Some(b'R'));
        assert_eq!(t.get(1, 0), Some(1));
        assert_eq!(t.get(0, 1), Some(b'R'));
        assert_eq!(t.get(1, 1), Some(3));
        assert_eq!(remap, vec![(0, 0), (2, 1)]);
    }

    #[test]
    fn test_scan() {
        let mut t = ParallelTable::new(&["state"]);
        t.append(&[b'R']);
        t.append(&[b'S']);
        t.append(&[b'R']);
        t.tombstone(0);

        let running = t.scan(0, |v| v == b'R');
        assert_eq!(running, vec![2]);
    }

    #[test]
    fn test_double_tombstone_no_panic() {
        let mut t = ParallelTable::new(&["state"]);
        t.append(&[b'R']);
        t.tombstone(0);
        t.tombstone(0);
        assert_eq!(t.tombstone_count(), 1);
    }

    #[test]
    fn test_column_index() {
        let t = ParallelTable::new(&["state", "affinity", "owner"]);
        assert_eq!(t.column_index("affinity"), Some(1));
        assert_eq!(t.column_index("missing"), None);
    }
}
