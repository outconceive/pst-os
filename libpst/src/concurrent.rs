use core::sync::atomic::{AtomicUsize, Ordering};
use core::cell::UnsafeCell;
use alloc::vec;
use alloc::vec::Vec;
use crate::table::{TOMBSTONE, EMPTY};

struct ConcurrentColumn {
    data: UnsafeCell<Vec<u8>>,
}

// Safety: ConcurrentTable guarantees non-overlapping writes via atomic row counter.
// Each append claims a unique slot before writing.
unsafe impl Sync for ConcurrentColumn {}
unsafe impl Send for ConcurrentColumn {}

impl ConcurrentColumn {
    fn new(capacity: usize) -> Self {
        Self { data: UnsafeCell::new(vec![EMPTY; capacity]) }
    }

    #[inline]
    unsafe fn read(&self, pos: usize) -> u8 {
        let ptr = (*self.data.get()).as_ptr();
        *ptr.add(pos)
    }

    #[inline]
    unsafe fn write(&self, pos: usize, val: u8) {
        let ptr = (*self.data.get()).as_mut_ptr();
        *ptr.add(pos) = val;
    }
}

pub struct ConcurrentTable {
    columns: Vec<ConcurrentColumn>,
    len: AtomicUsize,
    capacity: usize,
    live_count: AtomicUsize,
}

unsafe impl Sync for ConcurrentTable {}
unsafe impl Send for ConcurrentTable {}

impl ConcurrentTable {
    pub fn with_capacity(column_names: &[&str], capacity: usize) -> Self {
        let columns = column_names.iter().map(|_| ConcurrentColumn::new(capacity)).collect();
        Self {
            columns,
            len: AtomicUsize::new(0),
            capacity,
            live_count: AtomicUsize::new(0),
        }
    }

    /// Lock-free append. Returns the row position, or None if at capacity.
    /// Multiple threads can call this concurrently — each gets a unique slot.
    pub fn append(&self, values: &[u8]) -> Option<usize> {
        let pos = self.len.fetch_add(1, Ordering::AcqRel);
        if pos >= self.capacity {
            self.len.fetch_sub(1, Ordering::Release);
            return None;
        }
        for (i, col) in self.columns.iter().enumerate() {
            let v = values.get(i).copied().unwrap_or(EMPTY);
            // Safety: pos is unique per caller (atomic fetch_add guarantees non-overlapping slots)
            unsafe { col.write(pos, v); }
        }
        self.live_count.fetch_add(1, Ordering::Release);
        Some(pos)
    }

    pub fn get(&self, col_index: usize, pos: usize) -> Option<u8> {
        if pos >= self.len.load(Ordering::Acquire) { return None; }
        self.columns.get(col_index).map(|col| unsafe { col.read(pos) })
    }

    /// Tombstone a row. This is NOT concurrent-safe with other tombstone calls
    /// on the same row — designed for single-writer (watchdog/GC thread).
    pub fn tombstone(&self, pos: usize) {
        if pos >= self.len.load(Ordering::Acquire) { return; }
        if unsafe { self.columns[0].read(pos) } == TOMBSTONE { return; }
        for col in &self.columns {
            unsafe { col.write(pos, TOMBSTONE); }
        }
        self.live_count.fetch_sub(1, Ordering::Release);
    }

    pub fn is_live(&self, pos: usize) -> bool {
        if pos >= self.len.load(Ordering::Acquire) { return false; }
        let val = unsafe { self.columns[0].read(pos) };
        val != TOMBSTONE
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    pub fn live_count(&self) -> usize {
        self.live_count.load(Ordering::Acquire)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn scan<F>(&self, col_index: usize, predicate: F) -> Vec<usize>
    where
        F: Fn(u8) -> bool,
    {
        let len = self.len.load(Ordering::Acquire);
        let col = match self.columns.get(col_index) {
            Some(c) => c,
            None => return Vec::new(),
        };
        let first_col = &self.columns[0];
        let mut results = Vec::new();
        for i in 0..len {
            unsafe {
                if first_col.read(i) != TOMBSTONE && predicate(col.read(i)) {
                    results.push(i);
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
    fn test_concurrent_append() {
        let t = ConcurrentTable::with_capacity(&["state", "owner"], 64);
        let p0 = t.append(&[b'R', 1]).unwrap();
        let p1 = t.append(&[b'S', 2]).unwrap();
        assert_eq!(p0, 0);
        assert_eq!(p1, 1);
        assert_eq!(t.len(), 2);
        assert_eq!(t.live_count(), 2);
        assert_eq!(t.get(0, 0), Some(b'R'));
        assert_eq!(t.get(1, 1), Some(2));
    }

    #[test]
    fn test_concurrent_tombstone() {
        let t = ConcurrentTable::with_capacity(&["state"], 64);
        t.append(&[b'R']);
        t.append(&[b'S']);
        t.tombstone(0);
        assert!(!t.is_live(0));
        assert!(t.is_live(1));
        assert_eq!(t.live_count(), 1);
    }

    #[test]
    fn test_concurrent_capacity_limit() {
        let t = ConcurrentTable::with_capacity(&["state"], 2);
        assert!(t.append(&[b'R']).is_some());
        assert!(t.append(&[b'S']).is_some());
        assert!(t.append(&[b'B']).is_none());
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn test_concurrent_scan() {
        let t = ConcurrentTable::with_capacity(&["state"], 64);
        t.append(&[b'R']);
        t.append(&[b'S']);
        t.append(&[b'R']);
        t.tombstone(0);
        let running = t.scan(0, |v| v == b'R');
        assert_eq!(running, vec![2]);
    }

    #[test]
    fn test_concurrent_double_tombstone() {
        let t = ConcurrentTable::with_capacity(&["state"], 64);
        t.append(&[b'R']);
        t.tombstone(0);
        t.tombstone(0);
        assert_eq!(t.live_count(), 0);
    }

    #[test]
    fn test_concurrent_get_out_of_range() {
        let t = ConcurrentTable::with_capacity(&["state"], 64);
        assert_eq!(t.get(0, 0), None);
        assert_eq!(t.get(0, 100), None);
    }

    #[test]
    fn test_concurrent_append_is_ref_not_mut() {
        let t = ConcurrentTable::with_capacity(&["state"], 64);
        let r1 = &t;
        let r2 = &t;
        r1.append(&[b'R']);
        r2.append(&[b'S']);
        assert_eq!(t.len(), 2);
    }
}
