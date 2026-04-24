#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use libpst::table::ParallelTable;
use libpst::offset::OffsetTable;
use libpst::constraint::Constraint;
use libpst::solver::{ConstrainedNode, SolveResult, solve_schedule};

// Process state characters — same scan/filter alphabet everywhere
pub const STATE_READY: u8    = b'R';
pub const STATE_SLEEPING: u8 = b'S';
pub const STATE_BLOCKED: u8  = b'B';
pub const STATE_ZOMBIE: u8   = b'Z';
pub const STATE_NEW: u8      = b'N';

// Privilege levels
pub const PRIV_KERNEL: u8  = 0;
pub const PRIV_DRIVER: u8  = 1;
pub const PRIV_SYSTEM: u8  = 2;
pub const PRIV_USER: u8    = 3;

// Column indices
const COL_STATE: usize     = 0;
const COL_PRIVILEGE: usize = 1;
const COL_PRIORITY: usize  = 2;
const COL_AFFINITY: usize  = 3;

#[derive(Debug)]
pub struct ProcessEntry {
    pub name: String,
    pub state: u8,
    pub privilege: u8,
    pub priority: u8,
    pub affinity: u8,
    pub constraints: Vec<Constraint>,
}

pub struct ProcessTable {
    table: ParallelTable,
    offsets: OffsetTable,
    names: Vec<Option<String>>,
    constraints: Vec<Vec<Constraint>>,
}

impl ProcessTable {
    pub fn new() -> Self {
        Self {
            table: ParallelTable::new(&["state", "privilege", "priority", "affinity"]),
            offsets: OffsetTable::new(),
            names: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn register(&mut self, entry: ProcessEntry) -> usize {
        let physical = self.table.append(&[
            entry.state,
            entry.privilege,
            entry.priority,
            entry.affinity,
        ]);
        let logical = self.offsets.assign(physical);

        while self.names.len() <= logical {
            self.names.push(None);
        }
        self.names[logical] = Some(entry.name);

        while self.constraints.len() <= logical {
            self.constraints.push(Vec::new());
        }
        self.constraints[logical] = entry.constraints;

        logical
    }

    pub fn tombstone(&mut self, logical_id: usize) {
        if let Some(physical) = self.offsets.resolve(logical_id) {
            self.table.tombstone(physical);
            self.offsets.invalidate(logical_id);
        }
    }

    pub fn is_live(&self, logical_id: usize) -> bool {
        self.offsets.is_valid(logical_id)
    }

    pub fn get_state(&self, logical_id: usize) -> Option<u8> {
        let phys = self.offsets.resolve(logical_id)?;
        self.table.get(COL_STATE, phys)
    }

    pub fn set_state(&mut self, logical_id: usize, state: u8) {
        if let Some(phys) = self.offsets.resolve(logical_id) {
            self.table.set(COL_STATE, phys, state);
        }
    }

    pub fn get_name(&self, logical_id: usize) -> Option<&str> {
        self.names.get(logical_id)?.as_deref()
    }

    pub fn get_privilege(&self, logical_id: usize) -> Option<u8> {
        let phys = self.offsets.resolve(logical_id)?;
        self.table.get(COL_PRIVILEGE, phys)
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        for (i, n) in self.names.iter().enumerate() {
            if n.as_deref() == Some(name) && self.offsets.is_valid(i) {
                return Some(i);
            }
        }
        None
    }

    pub fn scan_by_state(&self, state: u8) -> Vec<usize> {
        let physicals = self.table.scan(COL_STATE, |v| v == state);
        let mut logicals = Vec::new();
        for phys in physicals {
            for (logical, _) in self.names.iter().enumerate() {
                if self.offsets.resolve(logical) == Some(phys) {
                    logicals.push(logical);
                    break;
                }
            }
        }
        logicals
    }

    pub fn solve_spawn_order(&self) -> SolveResult {
        let mut nodes = Vec::new();
        for (logical, name_opt) in self.names.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if let Some(name) = name_opt {
                let constraints = self.constraints.get(logical)
                    .cloned()
                    .unwrap_or_default();
                let priority = self.offsets.resolve(logical)
                    .and_then(|p| self.table.get(COL_PRIORITY, p))
                    .unwrap_or(0);
                nodes.push(ConstrainedNode {
                    name: name.clone(),
                    constraints,
                    priority,
                });
            }
        }
        solve_schedule(&nodes)
    }

    pub fn compact(&mut self) {
        let remap = self.table.compact();
        self.offsets.rebuild_from_remap(&remap);
    }

    pub fn live_count(&self) -> usize {
        self.table.live_count()
    }

    pub fn total_count(&self) -> usize {
        self.table.len()
    }

    pub fn tombstone_count(&self) -> usize {
        self.table.tombstone_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(name: &str, priv_level: u8, deps: Vec<Constraint>) -> ProcessEntry {
        ProcessEntry {
            name: String::from(name),
            state: STATE_NEW,
            privilege: priv_level,
            priority: 128,
            affinity: 0,
            constraints: deps,
        }
    }

    #[test]
    fn test_register_and_lookup() {
        let mut pt = ProcessTable::new();
        let id = pt.register(service("cryptod", PRIV_SYSTEM, vec![]));
        assert_eq!(pt.get_name(id), Some("cryptod"));
        assert_eq!(pt.get_state(id), Some(STATE_NEW));
        assert_eq!(pt.get_privilege(id), Some(PRIV_SYSTEM));
        assert!(pt.is_live(id));
    }

    #[test]
    fn test_find_by_name() {
        let mut pt = ProcessTable::new();
        pt.register(service("cryptod", PRIV_SYSTEM, vec![]));
        pt.register(service("vfs", PRIV_SYSTEM, vec![]));
        assert_eq!(pt.find_by_name("vfs"), Some(1));
        assert_eq!(pt.find_by_name("missing"), None);
    }

    #[test]
    fn test_tombstone_and_scan() {
        let mut pt = ProcessTable::new();
        let id0 = pt.register(service("a", PRIV_USER, vec![]));
        let id1 = pt.register(service("b", PRIV_USER, vec![]));
        pt.set_state(id0, STATE_READY);
        pt.set_state(id1, STATE_READY);

        pt.tombstone(id0);
        assert!(!pt.is_live(id0));
        assert!(pt.is_live(id1));

        let ready = pt.scan_by_state(STATE_READY);
        assert_eq!(ready, vec![id1]);
    }

    #[test]
    fn test_solve_spawn_order() {
        let mut pt = ProcessTable::new();

        pt.register(service("cryptod", PRIV_SYSTEM, vec![]));
        pt.register(service("vfs", PRIV_SYSTEM, vec![
            Constraint::After(String::from("cryptod")),
        ]));
        pt.register(service("netd", PRIV_SYSTEM, vec![
            Constraint::After(String::from("cryptod")),
        ]));
        pt.register(service("compositor", PRIV_USER, vec![
            Constraint::After(String::from("vfs")),
            Constraint::After(String::from("netd")),
        ]));

        let result = pt.solve_spawn_order();
        assert!(result.cycles.is_empty());

        let crypto_pos = result.order.iter().position(|n| n == "cryptod").unwrap();
        let vfs_pos = result.order.iter().position(|n| n == "vfs").unwrap();
        let net_pos = result.order.iter().position(|n| n == "netd").unwrap();
        let comp_pos = result.order.iter().position(|n| n == "compositor").unwrap();

        assert!(crypto_pos < vfs_pos);
        assert!(crypto_pos < net_pos);
        assert!(vfs_pos < comp_pos);
        assert!(net_pos < comp_pos);
    }

    #[test]
    fn test_compact_preserves_identity() {
        let mut pt = ProcessTable::new();
        let id0 = pt.register(service("a", PRIV_USER, vec![]));
        let id1 = pt.register(service("b", PRIV_USER, vec![]));
        let id2 = pt.register(service("c", PRIV_USER, vec![]));

        pt.tombstone(id1);
        pt.compact();

        assert_eq!(pt.live_count(), 2);
        assert_eq!(pt.get_name(id0), Some("a"));
        assert!(!pt.is_live(id1));
        assert_eq!(pt.get_name(id2), Some("c"));
    }

    #[test]
    fn test_privion_boot_sequence() {
        let mut pt = ProcessTable::new();

        pt.register(service("cryptod", PRIV_SYSTEM, vec![]));
        pt.register(service("vfs", PRIV_SYSTEM, vec![
            Constraint::After(String::from("cryptod")),
        ]));
        pt.register(service("netd", PRIV_SYSTEM, vec![
            Constraint::After(String::from("cryptod")),
        ]));
        pt.register(service("driverd", PRIV_DRIVER, vec![]));
        pt.register(service("driver-nic", PRIV_DRIVER, vec![
            Constraint::After(String::from("driverd")),
        ]));
        pt.register(service("compositor", PRIV_USER, vec![
            Constraint::After(String::from("vfs")),
            Constraint::After(String::from("netd")),
        ]));

        let result = pt.solve_spawn_order();
        assert!(result.cycles.is_empty());
        assert_eq!(result.order.len(), 6);

        // cryptod before vfs and netd
        let crypto = result.order.iter().position(|n| n == "cryptod").unwrap();
        let vfs = result.order.iter().position(|n| n == "vfs").unwrap();
        let netd = result.order.iter().position(|n| n == "netd").unwrap();
        assert!(crypto < vfs);
        assert!(crypto < netd);

        // driverd before driver-nic
        let driverd = result.order.iter().position(|n| n == "driverd").unwrap();
        let nic = result.order.iter().position(|n| n == "driver-nic").unwrap();
        assert!(driverd < nic);

        // compositor after vfs and netd
        let comp = result.order.iter().position(|n| n == "compositor").unwrap();
        assert!(vfs < comp);
        assert!(netd < comp);
    }
}
