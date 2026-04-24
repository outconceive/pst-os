#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use proctable::ProcessTable;
use pst_ipc::{EventLog, Message, PRIORITY_INTERRUPT, STATUS_PENDING};
use pst_sched::{Scheduler, SchedEntry, CycleReport, Action};

// Watchdog channels — reserved IPC channel IDs
pub const CHANNEL_WATCHDOG: u8     = 250;
pub const CHANNEL_FAULT: u8        = 251;
pub const CHANNEL_HEARTBEAT: u8    = 252;

// Watchdog is always process 0
pub const WATCHDOG_ID: u8 = 0;

// Violation types
#[derive(Debug, Clone, PartialEq)]
pub enum Violation {
    Cycle(Vec<(String, String)>),
    HeartbeatTimeout(String, u64),
    ConstraintViolation(String, String),
    ResourceExhaustion(String),
}

// What the watchdog decided to do
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    Tombstoned(String),
    Warned(String),
    Restarted(String),
    Ignored,
}

#[derive(Debug)]
pub struct WatchdogTick {
    pub violations: Vec<Violation>,
    pub resolutions: Vec<Resolution>,
    pub schedule: Vec<Action>,
}

pub struct Watchdog {
    max_missed_heartbeats: u64,
    heartbeats: Vec<(String, u64)>, // (process name, last seen tick)
    current_tick: u64,
    violation_counts: Vec<(String, usize)>,
    tombstone_threshold: usize,
}

impl Watchdog {
    pub fn new() -> Self {
        Self {
            max_missed_heartbeats: 5,
            heartbeats: Vec::new(),
            current_tick: 0,
            violation_counts: Vec::new(),
            tombstone_threshold: 3,
        }
    }

    pub fn with_heartbeat_timeout(mut self, ticks: u64) -> Self {
        self.max_missed_heartbeats = ticks;
        self
    }

    pub fn with_tombstone_threshold(mut self, t: usize) -> Self {
        self.tombstone_threshold = t;
        self
    }

    /// Register a process for heartbeat monitoring.
    pub fn monitor(&mut self, name: &str) {
        if !self.heartbeats.iter().any(|(n, _)| n == name) {
            self.heartbeats.push((String::from(name), self.current_tick));
        }
    }

    /// Record a heartbeat from a process.
    pub fn heartbeat(&mut self, name: &str) {
        for (n, tick) in &mut self.heartbeats {
            if n == name {
                *tick = self.current_tick;
                return;
            }
        }
    }

    /// Run one watchdog tick.
    /// Checks the scheduler for cycles, drains fault messages,
    /// checks heartbeats, and produces a tick report.
    pub fn tick(
        &mut self,
        entries: &[SchedEntry],
        proctable: &mut ProcessTable,
        ipc: &mut EventLog,
    ) -> WatchdogTick {
        self.current_tick += 1;
        let scheduler = Scheduler::new();

        let mut violations = Vec::new();
        let mut resolutions = Vec::new();

        // 1. Run scheduler — detect cycles
        let (schedule, cycle_report) = scheduler.schedule(entries);

        if let Some(report) = cycle_report {
            violations.push(Violation::Cycle(report.edges.clone()));
            for name in &report.tombstoned {
                let action = self.handle_violation(name, proctable);
                resolutions.push(action);
            }
        }

        // 2. Drain fault messages from IPC
        let fault_msgs = ipc.recv(WATCHDOG_ID);
        for (id, sender, payload) in &fault_msgs {
            if let Ok(fault_str) = core::str::from_utf8(payload) {
                let sender_name = self.find_name_by_id(*sender, entries);
                violations.push(Violation::ConstraintViolation(
                    sender_name.clone(),
                    String::from(fault_str),
                ));
                let action = self.handle_violation(&sender_name, proctable);
                resolutions.push(action);
            }
            let _ = ipc.ack(*id);
        }

        // 3. Check heartbeat timeouts
        let timeout_threshold = if self.current_tick > self.max_missed_heartbeats {
            self.current_tick - self.max_missed_heartbeats
        } else {
            0
        };

        let timed_out: Vec<(String, u64)> = self.heartbeats.iter()
            .filter(|(_, last)| *last < timeout_threshold)
            .cloned()
            .collect();

        for (name, last_tick) in timed_out {
            violations.push(Violation::HeartbeatTimeout(
                name.clone(),
                self.current_tick - last_tick,
            ));
            let action = self.handle_violation(&name, proctable);
            resolutions.push(action);
        }

        // 4. GC acknowledged fault messages
        ipc.gc();

        WatchdogTick {
            violations,
            resolutions,
            schedule,
        }
    }

    fn handle_violation(
        &mut self,
        name: &str,
        proctable: &mut ProcessTable,
    ) -> Resolution {
        let count = self.increment_violation(name);

        if count >= self.tombstone_threshold {
            if let Some(id) = proctable.find_by_name(name) {
                proctable.tombstone(id);
                self.remove_heartbeat(name);
                return Resolution::Tombstoned(String::from(name));
            }
        }

        Resolution::Warned(String::from(name))
    }

    fn increment_violation(&mut self, name: &str) -> usize {
        for (n, count) in &mut self.violation_counts {
            if n == name {
                *count += 1;
                return *count;
            }
        }
        self.violation_counts.push((String::from(name), 1));
        1
    }

    fn remove_heartbeat(&mut self, name: &str) {
        self.heartbeats.retain(|(n, _)| n != name);
    }

    fn find_name_by_id(&self, id: u8, entries: &[SchedEntry]) -> String {
        entries.get(id as usize)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| {
                let mut s = String::from("process-");
                // Manual u8 to string since we're no_std
                if id >= 100 { s.push((b'0' + id / 100) as char); }
                if id >= 10 { s.push((b'0' + (id / 10) % 10) as char); }
                s.push((b'0' + id % 10) as char);
                s
            })
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn violation_count(&self, name: &str) -> usize {
        self.violation_counts.iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::string::String;
    use libpst::constraint::Constraint;

    fn sched_entry(name: &str, priority: u8, constraints: Vec<Constraint>) -> SchedEntry {
        SchedEntry {
            name: String::from(name),
            priority,
            budget: 10,
            constraints,
            blocked: false,
            pending_messages: 0,
        }
    }

    fn proc_entry(name: &str) -> proctable::ProcessEntry {
        proctable::ProcessEntry {
            name: String::from(name),
            state: proctable::STATE_READY,
            privilege: proctable::PRIV_SYSTEM,
            priority: 128,
            affinity: 0,
            constraints: vec![],
        }
    }

    #[test]
    fn test_clean_tick_no_violations() {
        let mut wd = Watchdog::new();
        let mut pt = ProcessTable::new();
        let mut ipc = EventLog::new();

        pt.register(proc_entry("init"));
        let entries = vec![sched_entry("init", 200, vec![])];

        let result = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(result.violations.is_empty());
        assert!(result.resolutions.is_empty());
    }

    #[test]
    fn test_cycle_detected_warns_then_tombstones() {
        let mut wd = Watchdog::new().with_tombstone_threshold(2);
        let mut pt = ProcessTable::new();
        pt.register(proc_entry("a"));
        pt.register(proc_entry("b"));
        let mut ipc = EventLog::new();

        let entries = vec![
            sched_entry("a", 100, vec![Constraint::After(String::from("b"))]),
            sched_entry("b", 100, vec![Constraint::After(String::from("a"))]),
        ];

        // First tick — warns
        let r1 = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(!r1.violations.is_empty());
        assert!(r1.resolutions.iter().any(|r| matches!(r, Resolution::Warned(_))));

        // Second tick — tombstones
        let r2 = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(r2.resolutions.iter().any(|r| matches!(r, Resolution::Tombstoned(_))));
    }

    #[test]
    fn test_heartbeat_timeout() {
        let mut wd = Watchdog::new().with_heartbeat_timeout(3);
        let mut pt = ProcessTable::new();
        pt.register(proc_entry("worker"));
        let mut ipc = EventLog::new();

        wd.monitor("worker");

        let entries = vec![sched_entry("worker", 100, vec![])];

        // Ticks 1-3: worker is fine (within threshold)
        for _ in 0..3 {
            let r = wd.tick(&entries, &mut pt, &mut ipc);
            assert!(r.violations.is_empty());
        }

        // Ticks 4+: no heartbeat — should trigger timeout
        let r = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(r.violations.iter().any(|v| matches!(v, Violation::HeartbeatTimeout(n, _) if n == "worker")));
    }

    #[test]
    fn test_heartbeat_resets_timer() {
        let mut wd = Watchdog::new().with_heartbeat_timeout(3);
        let mut pt = ProcessTable::new();
        pt.register(proc_entry("worker"));
        let mut ipc = EventLog::new();

        wd.monitor("worker");

        let entries = vec![sched_entry("worker", 100, vec![])];

        // Advance 2 ticks
        wd.tick(&entries, &mut pt, &mut ipc);
        wd.tick(&entries, &mut pt, &mut ipc);

        // Heartbeat arrives
        wd.heartbeat("worker");

        // 3 more ticks — should be fine because heartbeat reset the timer
        wd.tick(&entries, &mut pt, &mut ipc);
        wd.tick(&entries, &mut pt, &mut ipc);
        let r = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(r.violations.is_empty());
    }

    #[test]
    fn test_fault_message_processed() {
        let mut wd = Watchdog::new();
        let mut pt = ProcessTable::new();
        pt.register(proc_entry("buggy"));
        let mut ipc = EventLog::new();

        // buggy sends a fault report to watchdog
        ipc.send(Message {
            sender: 0,
            receiver: WATCHDOG_ID,
            channel: CHANNEL_FAULT,
            priority: PRIORITY_INTERRUPT,
            payload: b"segfault".into(),
        }).unwrap();

        let entries = vec![sched_entry("buggy", 100, vec![])];
        let r = wd.tick(&entries, &mut pt, &mut ipc);

        assert!(r.violations.iter().any(|v| matches!(v, Violation::ConstraintViolation(_, msg) if msg == "segfault")));
    }

    #[test]
    fn test_repeated_violations_escalate() {
        let mut wd = Watchdog::new().with_tombstone_threshold(3);
        let mut pt = ProcessTable::new();
        let id = pt.register(proc_entry("flaky"));
        let mut ipc = EventLog::new();

        let entries = vec![
            sched_entry("flaky", 100, vec![Constraint::After(String::from("missing"))]),
            // "missing" isn't in entries — solver treats flaky as unconstrained
        ];

        // Simulate 3 fault messages
        for _ in 0..3 {
            ipc.send(Message {
                sender: 0, receiver: WATCHDOG_ID, channel: CHANNEL_FAULT,
                priority: PRIORITY_INTERRUPT, payload: b"crash".into(),
            }).unwrap();
            wd.tick(&entries, &mut pt, &mut ipc);
        }

        assert_eq!(wd.violation_count("buggy"), 0); // wrong name
        assert!(wd.violation_count("flaky") >= 3 || !pt.is_live(id));
    }

    #[test]
    fn test_tombstoned_process_removed_from_heartbeat() {
        let mut wd = Watchdog::new().with_tombstone_threshold(1).with_heartbeat_timeout(2);
        let mut pt = ProcessTable::new();
        pt.register(proc_entry("doomed"));
        pt.register(proc_entry("accomplice"));
        let mut ipc = EventLog::new();

        wd.monitor("doomed");

        let entries = vec![
            sched_entry("doomed", 100, vec![Constraint::After(String::from("accomplice"))]),
            sched_entry("accomplice", 100, vec![Constraint::After(String::from("doomed"))]),
        ];

        // Two-node cycle — immediate tombstone at threshold 1
        let r = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(r.resolutions.iter().any(|r| matches!(r, Resolution::Tombstoned(n) if n == "doomed")));

        // Subsequent ticks shouldn't report heartbeat timeout for tombstoned process
        for _ in 0..5 {
            let r = wd.tick(&entries, &mut pt, &mut ipc);
            let has_doomed_timeout = r.violations.iter().any(|v|
                matches!(v, Violation::HeartbeatTimeout(n, _) if n == "doomed")
            );
            assert!(!has_doomed_timeout);
        }
    }

    #[test]
    fn test_system_survives_cycle() {
        let mut wd = Watchdog::new().with_tombstone_threshold(1);
        let mut pt = ProcessTable::new();
        pt.register(proc_entry("a"));
        pt.register(proc_entry("b"));
        pt.register(proc_entry("healthy"));
        let mut ipc = EventLog::new();

        let entries = vec![
            sched_entry("a", 100, vec![Constraint::After(String::from("b"))]),
            sched_entry("b", 100, vec![Constraint::After(String::from("a"))]),
            sched_entry("healthy", 100, vec![]),
        ];

        let r = wd.tick(&entries, &mut pt, &mut ipc);

        // Cycle processes tombstoned, healthy survives
        assert!(r.schedule.iter().any(|a| matches!(a, Action::Run(n, _) if n == "healthy")));
        assert!(pt.is_live(2)); // healthy
    }
}
