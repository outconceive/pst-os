#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use libpst::constraint::{Constraint, GapValue};
use libpst::solver::{ConstrainedNode, SolveResult, topological_sort, CycleAction};

// Tick budget — how many ticks a process gets per scheduling round
pub const DEFAULT_BUDGET: u64 = 10;

// Scheduling decisions
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Run(String, u64),
    Idle(u64),
    Tombstone(String),
}

// Process readiness
#[derive(Debug, Clone)]
pub struct SchedEntry {
    pub name: String,
    pub priority: u8,
    pub budget: u64,
    pub constraints: Vec<Constraint>,
    pub blocked: bool,
    pub pending_messages: usize,
}

#[derive(Debug)]
pub struct CycleReport {
    pub edges: Vec<(String, String)>,
    pub tombstoned: Vec<String>,
}

pub struct Scheduler {
    watchdog_threshold: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            watchdog_threshold: 3,
        }
    }

    pub fn with_watchdog_threshold(mut self, t: usize) -> Self {
        self.watchdog_threshold = t;
        self
    }

    /// Compute a single scheduling tick.
    /// Takes the current set of ready/blocked processes and returns
    /// an ordered list of actions for this tick.
    pub fn schedule(&self, entries: &[SchedEntry]) -> (Vec<Action>, Option<CycleReport>) {
        let mut nodes = Vec::new();

        for entry in entries {
            if entry.blocked && entry.pending_messages == 0 {
                continue;
            }

            let mut constraints = entry.constraints.clone();

            let effective_priority = if entry.pending_messages > 0 {
                entry.priority.saturating_add(1)
            } else {
                entry.priority
            };

            nodes.push(ConstrainedNode {
                name: entry.name.clone(),
                constraints,
                priority: effective_priority,
            });
        }

        let result = topological_sort(&nodes, CycleAction::Break);

        let mut actions = Vec::new();
        let mut cycle_report = None;

        if !result.cycles.is_empty() {
            let mut tombstoned = Vec::new();
            for (a, _) in &result.cycles {
                tombstoned.push(a.clone());
            }
            tombstoned.dedup();

            for name in &tombstoned {
                actions.push(Action::Tombstone(name.clone()));
            }

            cycle_report = Some(CycleReport {
                edges: result.cycles.clone(),
                tombstoned,
            });
        }

        // Build run list: topo order, then by priority within unconstrained groups
        let mut run_list: Vec<(&str, u8, u64)> = Vec::new();
        for name in &result.order {
            if actions.iter().any(|a| matches!(a, Action::Tombstone(n) if n == name)) {
                continue;
            }
            if let Some(entry) = entries.iter().find(|e| e.name == *name) {
                run_list.push((name.as_str(), entry.priority, entry.budget));
            }
        }

        // Stable sort by priority descending within topo order
        // (topo order already respects After constraints, this is secondary)
        // We don't re-sort — topo order IS the schedule. Priority was a hint
        // for the solver when multiple nodes had equal in-degree.

        for (name, _, budget) in &run_list {
            actions.push(Action::Run(String::from(*name), *budget));
        }

        if actions.is_empty() {
            actions.push(Action::Idle(1));
        }

        (actions, cycle_report)
    }

    /// Convenience: schedule and return just the run order as names
    pub fn run_order(&self, entries: &[SchedEntry]) -> Vec<String> {
        let (actions, _) = self.schedule(entries);
        actions.into_iter().filter_map(|a| match a {
            Action::Run(name, _) => Some(name),
            _ => None,
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn entry(name: &str, priority: u8, constraints: Vec<Constraint>) -> SchedEntry {
        SchedEntry {
            name: String::from(name),
            priority,
            budget: DEFAULT_BUDGET,
            constraints,
            blocked: false,
            pending_messages: 0,
        }
    }

    fn blocked(name: &str, priority: u8) -> SchedEntry {
        SchedEntry {
            name: String::from(name),
            priority,
            budget: DEFAULT_BUDGET,
            constraints: Vec::new(),
            blocked: true,
            pending_messages: 0,
        }
    }

    #[test]
    fn test_empty_schedule_idles() {
        let sched = Scheduler::new();
        let (actions, _) = sched.schedule(&[]);
        assert_eq!(actions, vec![Action::Idle(1)]);
    }

    #[test]
    fn test_single_process() {
        let sched = Scheduler::new();
        let entries = vec![entry("init", 255, vec![])];
        let order = sched.run_order(&entries);
        assert_eq!(order, vec!["init"]);
    }

    #[test]
    fn test_after_constraint_ordering() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("app", 100, vec![Constraint::After(String::from("vfs"))]),
            entry("vfs", 200, vec![Constraint::After(String::from("crypto"))]),
            entry("crypto", 255, vec![]),
        ];
        let order = sched.run_order(&entries);
        let crypto = order.iter().position(|n| n == "crypto").unwrap();
        let vfs = order.iter().position(|n| n == "vfs").unwrap();
        let app = order.iter().position(|n| n == "app").unwrap();
        assert!(crypto < vfs);
        assert!(vfs < app);
    }

    #[test]
    fn test_blocked_processes_skipped() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("running", 100, vec![]),
            blocked("sleeping", 200),
        ];
        let order = sched.run_order(&entries);
        assert_eq!(order, vec!["running"]);
    }

    #[test]
    fn test_blocked_with_messages_unblocks() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("a", 100, vec![]),
            SchedEntry {
                name: String::from("b"),
                priority: 200,
                budget: DEFAULT_BUDGET,
                constraints: Vec::new(),
                blocked: true,
                pending_messages: 3,
            },
        ];
        let order = sched.run_order(&entries);
        assert_eq!(order.len(), 2);
        assert!(order.contains(&String::from("b")));
    }

    #[test]
    fn test_cycle_detected_and_tombstoned() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("a", 100, vec![Constraint::After(String::from("b"))]),
            entry("b", 100, vec![Constraint::After(String::from("a"))]),
            entry("c", 100, vec![]),
        ];
        let (actions, report) = sched.schedule(&entries);

        assert!(report.is_some());
        let report = report.unwrap();
        assert!(!report.tombstoned.is_empty());

        // c should still run
        assert!(actions.iter().any(|a| matches!(a, Action::Run(n, _) if n == "c")));
    }

    #[test]
    fn test_diamond_dependency() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("d", 100, vec![
                Constraint::After(String::from("b")),
                Constraint::After(String::from("c")),
            ]),
            entry("b", 150, vec![Constraint::After(String::from("a"))]),
            entry("c", 150, vec![Constraint::After(String::from("a"))]),
            entry("a", 200, vec![]),
        ];
        let order = sched.run_order(&entries);
        let a = order.iter().position(|n| n == "a").unwrap();
        let b = order.iter().position(|n| n == "b").unwrap();
        let c = order.iter().position(|n| n == "c").unwrap();
        let d = order.iter().position(|n| n == "d").unwrap();
        assert!(a < b);
        assert!(a < c);
        assert!(b < d);
        assert!(c < d);
    }

    #[test]
    fn test_gap_after_constraint() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("dma_complete", 255, vec![]),
            entry("process_data", 100, vec![
                Constraint::GapAfter(
                    GapValue::from_ticks(2),
                    Some(String::from("dma_complete")),
                ),
            ]),
        ];
        let order = sched.run_order(&entries);
        let dma = order.iter().position(|n| n == "dma_complete").unwrap();
        let proc = order.iter().position(|n| n == "process_data").unwrap();
        assert!(dma < proc);
    }

    #[test]
    fn test_shared_memory_ordering() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("writer", 100, vec![]),
            entry("reader", 100, vec![
                Constraint::ShareMemory(String::from("writer")),
            ]),
        ];
        let order = sched.run_order(&entries);
        let w = order.iter().position(|n| n == "writer").unwrap();
        let r = order.iter().position(|n| n == "reader").unwrap();
        assert!(w < r);
    }

    #[test]
    fn test_hardware_interrupt_scheduling() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("user_app", 50, vec![]),
            entry("irq_handler", 255, vec![]),
            entry("deferred_work", 100, vec![
                Constraint::After(String::from("irq_handler")),
            ]),
        ];
        let order = sched.run_order(&entries);
        let irq = order.iter().position(|n| n == "irq_handler").unwrap();
        let deferred = order.iter().position(|n| n == "deferred_work").unwrap();
        assert!(irq < deferred);
    }

    #[test]
    fn test_privion_full_boot() {
        let sched = Scheduler::new();
        let entries = vec![
            entry("cryptod", 200, vec![]),
            entry("vfs", 180, vec![
                Constraint::After(String::from("cryptod")),
            ]),
            entry("netd", 180, vec![
                Constraint::After(String::from("cryptod")),
            ]),
            entry("driverd", 190, vec![]),
            entry("driver-nic", 170, vec![
                Constraint::After(String::from("driverd")),
            ]),
            entry("compositor", 100, vec![
                Constraint::After(String::from("vfs")),
                Constraint::After(String::from("netd")),
            ]),
        ];
        let order = sched.run_order(&entries);
        assert_eq!(order.len(), 6);

        let crypto = order.iter().position(|n| n == "cryptod").unwrap();
        let vfs = order.iter().position(|n| n == "vfs").unwrap();
        let netd = order.iter().position(|n| n == "netd").unwrap();
        let driverd = order.iter().position(|n| n == "driverd").unwrap();
        let nic = order.iter().position(|n| n == "driver-nic").unwrap();
        let comp = order.iter().position(|n| n == "compositor").unwrap();

        assert!(crypto < vfs);
        assert!(crypto < netd);
        assert!(driverd < nic);
        assert!(vfs < comp);
        assert!(netd < comp);
    }
}
