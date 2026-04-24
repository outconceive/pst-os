#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use libpst::table::ParallelTable;
use libpst::offset::OffsetTable;

// Temperature tiers
pub const TIER_HOT: u8    = 0;  // Full resolution — every tick
pub const TIER_WARM: u8   = 1;  // Summarized — deltas only
pub const TIER_COLD: u8   = 2;  // Compressed — hashed segments
pub const TIER_FROZEN: u8 = 3;  // Archived or dropped

// Retention policies
pub const RETAIN_FOREVER: u64    = u64::MAX;
pub const RETAIN_SESSION: u64    = 0;  // Amnesic — forget on shutdown

// Column indices
const COL_ENTITY: usize   = 0;  // Which entity changed (process ID, file ID, etc.)
const COL_COLUMN: usize   = 1;  // Which column changed (state, perm, etc.)
const COL_OLD_VAL: usize  = 2;  // Previous value
const COL_NEW_VAL: usize  = 3;  // New value
const COL_TIER: usize     = 4;  // Temperature tier

#[derive(Debug, Clone)]
pub struct StateChange {
    pub tick: u64,
    pub entity: u8,
    pub column: u8,
    pub old_value: u8,
    pub new_value: u8,
}

#[derive(Debug)]
pub enum TimeError {
    NotFound,
    OutOfRange,
}

pub struct Timeline {
    events: ParallelTable,
    offsets: OffsetTable,
    ticks: Vec<u64>,
    current_tick: u64,
    retention: u64,
    hot_window: u64,
    warm_window: u64,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            events: ParallelTable::new(&["entity", "column", "old_val", "new_val", "tier"]),
            offsets: OffsetTable::new(),
            ticks: Vec::new(),
            current_tick: 0,
            retention: RETAIN_FOREVER,
            hot_window: 100,
            warm_window: 1000,
        }
    }

    pub fn with_retention(mut self, ticks: u64) -> Self {
        self.retention = ticks;
        self
    }

    pub fn with_hot_window(mut self, ticks: u64) -> Self {
        self.hot_window = ticks;
        self
    }

    pub fn with_warm_window(mut self, ticks: u64) -> Self {
        self.warm_window = ticks;
        self
    }

    pub fn advance(&mut self) {
        self.current_tick += 1;
    }

    pub fn tick(&self) -> u64 {
        self.current_tick
    }

    /// Record a state change — append to the time string.
    pub fn record(&mut self, entity: u8, column: u8, old_value: u8, new_value: u8) -> usize {
        let physical = self.events.append(&[entity, column, old_value, new_value, TIER_HOT]);
        let logical = self.offsets.assign(physical);

        while self.ticks.len() <= logical {
            self.ticks.push(0);
        }
        self.ticks[logical] = self.current_tick;

        logical
    }

    /// Query: what was entity's column value at a given tick?
    /// Scans backward from the given tick to find the last change.
    pub fn state_at(&self, entity: u8, column: u8, at_tick: u64) -> Option<u8> {
        let mut best_tick = 0u64;
        let mut best_value = None;

        for i in 0..self.offsets.len() {
            if !self.offsets.is_valid(i) { continue; }
            if let Some(phys) = self.offsets.resolve(i) {
                if self.events.get(COL_ENTITY, phys) != Some(entity) { continue; }
                if self.events.get(COL_COLUMN, phys) != Some(column) { continue; }

                let t = self.ticks.get(i).copied().unwrap_or(0);
                if t <= at_tick && t >= best_tick {
                    best_tick = t;
                    best_value = self.events.get(COL_NEW_VAL, phys);
                }
            }
        }

        best_value
    }

    /// Query: current value (latest state at current tick).
    pub fn current_state(&self, entity: u8, column: u8) -> Option<u8> {
        self.state_at(entity, column, self.current_tick)
    }

    /// History: all changes for an entity, ordered by tick.
    pub fn history(&self, entity: u8) -> Vec<StateChange> {
        let mut changes = Vec::new();

        for i in 0..self.offsets.len() {
            if !self.offsets.is_valid(i) { continue; }
            if let Some(phys) = self.offsets.resolve(i) {
                if self.events.get(COL_ENTITY, phys) != Some(entity) { continue; }

                changes.push(StateChange {
                    tick: self.ticks.get(i).copied().unwrap_or(0),
                    entity,
                    column: self.events.get(COL_COLUMN, phys).unwrap_or(0),
                    old_value: self.events.get(COL_OLD_VAL, phys).unwrap_or(0),
                    new_value: self.events.get(COL_NEW_VAL, phys).unwrap_or(0),
                });
            }
        }

        changes.sort_by_key(|c| c.tick);
        changes
    }

    /// Scan: find all entities that changed a specific column in a tick range.
    pub fn scan_range(&self, column: u8, from_tick: u64, to_tick: u64) -> Vec<(u8, StateChange)> {
        let mut results = Vec::new();

        for i in 0..self.offsets.len() {
            if !self.offsets.is_valid(i) { continue; }
            let t = self.ticks.get(i).copied().unwrap_or(0);
            if t < from_tick || t > to_tick { continue; }

            if let Some(phys) = self.offsets.resolve(i) {
                if self.events.get(COL_COLUMN, phys) != Some(column) { continue; }
                let entity = self.events.get(COL_ENTITY, phys).unwrap_or(0);
                results.push((entity, StateChange {
                    tick: t,
                    entity,
                    column,
                    old_value: self.events.get(COL_OLD_VAL, phys).unwrap_or(0),
                    new_value: self.events.get(COL_NEW_VAL, phys).unwrap_or(0),
                }));
            }
        }

        results.sort_by_key(|(_, c)| c.tick);
        results
    }

    /// Tiered compaction — promote events through temperature tiers.
    /// Hot → Warm: events outside the hot window get tier upgraded.
    /// Warm → Cold: events outside the warm window.
    /// Cold/Frozen beyond retention: tombstoned.
    pub fn compact_tiers(&mut self) -> CompactionReport {
        let mut promoted = 0usize;
        let mut dropped = 0usize;

        for i in 0..self.offsets.len() {
            if !self.offsets.is_valid(i) { continue; }
            let t = self.ticks.get(i).copied().unwrap_or(0);
            let age = self.current_tick.saturating_sub(t);

            if let Some(phys) = self.offsets.resolve(i) {
                let current_tier = self.events.get(COL_TIER, phys).unwrap_or(TIER_HOT);

                // Check retention — drop if beyond policy
                if self.retention != RETAIN_FOREVER && age > self.retention {
                    self.events.tombstone(phys);
                    self.offsets.invalidate(i);
                    dropped += 1;
                    continue;
                }

                // Promote tiers based on age
                let target_tier = if age <= self.hot_window {
                    TIER_HOT
                } else if age <= self.warm_window {
                    TIER_WARM
                } else {
                    TIER_COLD
                };

                if target_tier > current_tier {
                    self.events.set(COL_TIER, phys, target_tier);
                    promoted += 1;
                }
            }
        }

        CompactionReport { promoted, dropped }
    }

    /// Amnesic mode — forget everything. For privacy-critical shutdown.
    pub fn forget_all(&mut self) -> usize {
        let mut count = 0;
        for i in 0..self.offsets.len() {
            if self.offsets.is_valid(i) {
                if let Some(phys) = self.offsets.resolve(i) {
                    self.events.tombstone(phys);
                    self.offsets.invalidate(i);
                    count += 1;
                }
            }
        }
        count
    }

    /// Compact the underlying table — reclaim tombstoned space.
    pub fn compact_storage(&mut self) {
        let remap = self.events.compact();
        self.offsets.rebuild_from_remap(&remap);
    }

    pub fn event_count(&self) -> usize {
        self.events.live_count()
    }
}

#[derive(Debug)]
pub struct CompactionReport {
    pub promoted: usize,
    pub dropped: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    const COL_STATE: u8 = 0;
    const COL_PERM: u8 = 1;

    const STATE_NEW: u8     = b'N';
    const STATE_READY: u8   = b'R';
    const STATE_SLEEPING: u8 = b'S';
    const STATE_ZOMBIE: u8  = b'Z';

    #[test]
    fn test_record_and_query() {
        let mut tl = Timeline::new();

        tl.record(1, COL_STATE, 0, STATE_NEW);
        tl.advance();
        tl.record(1, COL_STATE, STATE_NEW, STATE_READY);
        tl.advance();

        assert_eq!(tl.current_state(1, COL_STATE), Some(STATE_READY));
        assert_eq!(tl.state_at(1, COL_STATE, 0), Some(STATE_NEW));
        assert_eq!(tl.state_at(1, COL_STATE, 1), Some(STATE_READY));
    }

    #[test]
    fn test_history_ordered() {
        let mut tl = Timeline::new();

        tl.record(5, COL_STATE, 0, STATE_NEW);
        tl.advance();
        tl.record(5, COL_STATE, STATE_NEW, STATE_READY);
        tl.advance();
        tl.record(5, COL_STATE, STATE_READY, STATE_SLEEPING);
        tl.advance();
        tl.record(5, COL_STATE, STATE_SLEEPING, STATE_ZOMBIE);

        let hist = tl.history(5);
        assert_eq!(hist.len(), 4);
        assert_eq!(hist[0].new_value, STATE_NEW);
        assert_eq!(hist[1].new_value, STATE_READY);
        assert_eq!(hist[2].new_value, STATE_SLEEPING);
        assert_eq!(hist[3].new_value, STATE_ZOMBIE);
    }

    #[test]
    fn test_time_travel_debugging() {
        let mut tl = Timeline::new();

        // Process 1 goes through lifecycle
        tl.record(1, COL_STATE, 0, STATE_NEW);     // tick 0
        tl.advance();
        tl.record(1, COL_STATE, STATE_NEW, STATE_READY);  // tick 1
        tl.advance();
        tl.record(1, COL_STATE, STATE_READY, STATE_SLEEPING); // tick 2
        tl.advance();
        // Bug happens here — process dies unexpectedly
        tl.record(1, COL_STATE, STATE_SLEEPING, STATE_ZOMBIE); // tick 3

        // Debugger: "what was process 1 doing at tick 2?"
        assert_eq!(tl.state_at(1, COL_STATE, 2), Some(STATE_SLEEPING));
        // "And right before it died?"
        assert_eq!(tl.state_at(1, COL_STATE, 3), Some(STATE_ZOMBIE));
        // "What was the transition?" — scan history
        let hist = tl.history(1);
        let death = hist.iter().find(|c| c.new_value == STATE_ZOMBIE).unwrap();
        assert_eq!(death.old_value, STATE_SLEEPING);
    }

    #[test]
    fn test_scan_range() {
        let mut tl = Timeline::new();

        tl.record(1, COL_STATE, 0, STATE_READY);   // tick 0
        tl.advance();
        tl.record(2, COL_STATE, 0, STATE_READY);   // tick 1
        tl.advance();
        tl.record(3, COL_STATE, 0, STATE_READY);   // tick 2
        tl.advance();
        tl.record(1, COL_PERM, 0, 1);              // tick 3 (different column)

        let changes = tl.scan_range(COL_STATE, 0, 2);
        assert_eq!(changes.len(), 3);

        let changes = tl.scan_range(COL_STATE, 1, 1);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].0, 2); // entity 2
    }

    #[test]
    fn test_tiered_compaction() {
        let mut tl = Timeline::new()
            .with_hot_window(5)
            .with_warm_window(20);

        // Record events at tick 0
        tl.record(1, COL_STATE, 0, STATE_NEW);

        // Advance 10 ticks — beyond hot window
        for _ in 0..10 { tl.advance(); }

        let report = tl.compact_tiers();
        assert_eq!(report.promoted, 1); // promoted from hot to warm

        // Advance 20 more ticks — beyond warm window
        for _ in 0..20 { tl.advance(); }

        let report = tl.compact_tiers();
        assert_eq!(report.promoted, 1); // promoted from warm to cold
    }

    #[test]
    fn test_retention_policy_drops() {
        let mut tl = Timeline::new()
            .with_retention(10);

        tl.record(1, COL_STATE, 0, STATE_NEW);

        // Advance past retention window
        for _ in 0..15 { tl.advance(); }

        let report = tl.compact_tiers();
        assert_eq!(report.dropped, 1);
        assert_eq!(tl.event_count(), 0);
    }

    #[test]
    fn test_amnesic_mode() {
        let mut tl = Timeline::new();

        for i in 0..10u8 {
            tl.record(i, COL_STATE, 0, STATE_READY);
            tl.advance();
        }
        assert_eq!(tl.event_count(), 10);

        let count = tl.forget_all();
        assert_eq!(count, 10);
        assert_eq!(tl.event_count(), 0);
    }

    #[test]
    fn test_multiple_entities_same_timeline() {
        let mut tl = Timeline::new();

        tl.record(1, COL_STATE, 0, STATE_READY);
        tl.record(2, COL_STATE, 0, STATE_NEW);
        tl.advance();
        tl.record(2, COL_STATE, STATE_NEW, STATE_READY);

        assert_eq!(tl.current_state(1, COL_STATE), Some(STATE_READY));
        assert_eq!(tl.current_state(2, COL_STATE), Some(STATE_READY));

        let hist1 = tl.history(1);
        assert_eq!(hist1.len(), 1);

        let hist2 = tl.history(2);
        assert_eq!(hist2.len(), 2);
    }

    #[test]
    fn test_audit_trail_is_free() {
        let mut tl = Timeline::new();

        // Simulate a file permission change audit
        tl.record(42, COL_PERM, 0b001, 0b011);  // read → read+write
        tl.advance();
        tl.record(42, COL_PERM, 0b011, 0b111);  // read+write → read+write+exec
        tl.advance();
        tl.record(42, COL_PERM, 0b111, 0b001);  // back to read-only

        let audit = tl.history(42);
        assert_eq!(audit.len(), 3);
        // Who escalated permissions? Scan shows exact tick and transition
        let escalation = audit.iter().find(|c| c.new_value == 0b111).unwrap();
        assert_eq!(escalation.old_value, 0b011);
    }

    #[test]
    fn test_undo_is_rewind() {
        let mut tl = Timeline::new();

        tl.record(1, COL_STATE, 0, b'A');
        tl.advance();
        tl.record(1, COL_STATE, b'A', b'B');
        tl.advance();
        tl.record(1, COL_STATE, b'B', b'C');

        // Current state
        assert_eq!(tl.current_state(1, COL_STATE), Some(b'C'));

        // "Undo" — what was the state one tick ago?
        let prev = tl.state_at(1, COL_STATE, 1);
        assert_eq!(prev, Some(b'B'));

        // "Undo twice"
        let prev_prev = tl.state_at(1, COL_STATE, 0);
        assert_eq!(prev_prev, Some(b'A'));
    }

    #[test]
    fn test_compact_reclaims_storage() {
        let mut tl = Timeline::new().with_retention(5);

        for i in 0..20u8 {
            tl.record(i, COL_STATE, 0, STATE_READY);
            tl.advance();
        }

        tl.compact_tiers();
        tl.compact_storage();

        // Events older than 5 ticks should be gone
        assert!(tl.event_count() < 20);
    }
}
