#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::vec::Vec;

use libpst::table::ParallelTable;

// Privilege levels — the alphabet of the privilege string
pub const PRIV_HARDWARE: u8 = 0;  // DMA controllers, bootloader
pub const PRIV_KERNEL: u8   = 1;  // solver, watchdog
pub const PRIV_DRIVER: u8   = 2;  // device drivers
pub const PRIV_SYSTEM: u8   = 3;  // vfs, netd, cryptod
pub const PRIV_USER: u8     = 4;  // applications

// Subsystem IDs — which table this entry belongs to
pub const SUB_PROCESS: u8  = b'P';
pub const SUB_FILE: u8     = b'F';
pub const SUB_IPC: u8      = b'I';
pub const SUB_MEMORY: u8   = b'M';
pub const SUB_SCHEDULE: u8 = b'S';

// Access types for privilege checks
pub const ACCESS_READ: u8    = 0b001;
pub const ACCESS_WRITE: u8   = 0b010;
pub const ACCESS_EXECUTE: u8 = 0b100;
pub const ACCESS_TOMBSTONE: u8 = 0b1000;

// Column indices
const COL_SUBSYSTEM: usize   = 0;
const COL_PRIVILEGE: usize   = 1;
const COL_PHYSICAL: usize    = 2;  // low byte of physical offset
const COL_PHYSICAL_HI: usize = 3;  // high byte of physical offset
const COL_STATUS: usize      = 4;

const STATUS_LIVE: u8      = b'L';
const STATUS_INVALID: u8   = b'X';

#[derive(Debug)]
pub enum OffsetError {
    NotFound,
    Invalidated,
    PrivilegeDenied,
    NotOwner,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedEntry {
    pub physical: u16,
    pub subsystem: u8,
    pub privilege: u8,
}

/// The immortal offset table.
///
/// This is the one data structure that never gets tombstoned.
/// It sits beneath every subsystem and enforces privilege checks
/// on every address resolution — before the solver or any service
/// sees the data.
///
/// In a hardware implementation, this maps to the MMU.
/// In PST OS on seL4, this maps to the capability space root.
pub struct RootOffsetTable {
    table: ParallelTable,
    count: usize,
}

impl RootOffsetTable {
    pub fn new() -> Self {
        Self {
            table: ParallelTable::new(&[
                "subsystem",
                "privilege",
                "physical_lo",
                "physical_hi",
                "status",
            ]),
            count: 0,
        }
    }

    /// Register a new entry. Returns the logical ID.
    /// This is the only way to create identity in the system.
    pub fn register(
        &mut self,
        subsystem: u8,
        privilege: u8,
        physical: u16,
    ) -> usize {
        let lo = (physical & 0xFF) as u8;
        let hi = (physical >> 8) as u8;
        self.table.append(&[subsystem, privilege, lo, hi, STATUS_LIVE]);
        let id = self.count;
        self.count += 1;
        id
    }

    /// Resolve a logical ID to its physical offset.
    /// Enforces privilege: the requester's privilege level must be
    /// less than or equal to the entry's privilege level, OR the
    /// requester must be PRIV_KERNEL.
    pub fn resolve(
        &self,
        logical: usize,
        requester_priv: u8,
        access: u8,
    ) -> Result<ResolvedEntry, OffsetError> {
        if logical >= self.count {
            return Err(OffsetError::NotFound);
        }

        let status = self.table.get(COL_STATUS, logical)
            .ok_or(OffsetError::NotFound)?;

        if status == STATUS_INVALID {
            return Err(OffsetError::Invalidated);
        }

        let entry_priv = self.table.get(COL_PRIVILEGE, logical)
            .ok_or(OffsetError::NotFound)?;

        // Privilege check:
        // - Kernel can access anything
        // - Hardware can access anything
        // - Same or higher privilege can read
        // - Only same privilege or kernel can write/tombstone
        if requester_priv != PRIV_KERNEL && requester_priv != PRIV_HARDWARE {
            if access & (ACCESS_WRITE | ACCESS_TOMBSTONE) != 0 {
                if requester_priv > entry_priv {
                    return Err(OffsetError::PrivilegeDenied);
                }
            }
            if access & ACCESS_READ != 0 {
                if requester_priv > entry_priv + 1 {
                    return Err(OffsetError::PrivilegeDenied);
                }
            }
        }

        let lo = self.table.get(COL_PHYSICAL, logical).unwrap_or(0) as u16;
        let hi = self.table.get(COL_PHYSICAL_HI, logical).unwrap_or(0) as u16;
        let physical = (hi << 8) | lo;
        let subsystem = self.table.get(COL_SUBSYSTEM, logical).unwrap_or(0);

        Ok(ResolvedEntry {
            physical,
            subsystem,
            privilege: entry_priv,
        })
    }

    /// Invalidate an entry. Only kernel or same privilege can do this.
    /// The entry is NOT tombstoned — it becomes unreachable but the
    /// logical ID is never reused. This is the "death" operation.
    pub fn invalidate(
        &mut self,
        logical: usize,
        requester_priv: u8,
    ) -> Result<(), OffsetError> {
        if logical >= self.count {
            return Err(OffsetError::NotFound);
        }

        let entry_priv = self.table.get(COL_PRIVILEGE, logical)
            .ok_or(OffsetError::NotFound)?;

        if requester_priv != PRIV_KERNEL && requester_priv > entry_priv {
            return Err(OffsetError::PrivilegeDenied);
        }

        self.table.set(COL_STATUS, logical, STATUS_INVALID);
        Ok(())
    }

    /// Check if a logical ID is live.
    pub fn is_live(&self, logical: usize) -> bool {
        if logical >= self.count { return false; }
        self.table.get(COL_STATUS, logical) == Some(STATUS_LIVE)
    }

    /// Scan for all live entries in a subsystem.
    pub fn scan_subsystem(&self, subsystem: u8) -> Vec<usize> {
        let mut results = Vec::new();
        for i in 0..self.count {
            if self.table.get(COL_STATUS, i) == Some(STATUS_LIVE)
                && self.table.get(COL_SUBSYSTEM, i) == Some(subsystem)
            {
                results.push(i);
            }
        }
        results
    }

    /// Scan for all live entries owned by a privilege level.
    pub fn scan_privilege(&self, privilege: u8) -> Vec<usize> {
        let mut results = Vec::new();
        for i in 0..self.count {
            if self.table.get(COL_STATUS, i) == Some(STATUS_LIVE)
                && self.table.get(COL_PRIVILEGE, i) == Some(privilege)
            {
                results.push(i);
            }
        }
        results
    }

    /// Total entries ever created (logical IDs never shrink).
    pub fn total_entries(&self) -> usize {
        self.count
    }

    /// Currently live entries.
    pub fn live_entries(&self) -> usize {
        (0..self.count)
            .filter(|&i| self.table.get(COL_STATUS, i) == Some(STATUS_LIVE))
            .count()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_resolve() {
        let mut root = RootOffsetTable::new();
        let id = root.register(SUB_PROCESS, PRIV_SYSTEM, 42);
        let entry = root.resolve(id, PRIV_KERNEL, ACCESS_READ).unwrap();
        assert_eq!(entry.physical, 42);
        assert_eq!(entry.subsystem, SUB_PROCESS);
        assert_eq!(entry.privilege, PRIV_SYSTEM);
    }

    #[test]
    fn test_kernel_can_access_anything() {
        let mut root = RootOffsetTable::new();
        let id = root.register(SUB_PROCESS, PRIV_HARDWARE, 0);
        assert!(root.resolve(id, PRIV_KERNEL, ACCESS_READ).is_ok());
        assert!(root.resolve(id, PRIV_KERNEL, ACCESS_WRITE).is_ok());
        assert!(root.resolve(id, PRIV_KERNEL, ACCESS_TOMBSTONE).is_ok());
    }

    #[test]
    fn test_user_cannot_write_system() {
        let mut root = RootOffsetTable::new();
        let id = root.register(SUB_FILE, PRIV_SYSTEM, 100);

        // User can't write to system-level entry
        assert!(matches!(
            root.resolve(id, PRIV_USER, ACCESS_WRITE),
            Err(OffsetError::PrivilegeDenied)
        ));
    }

    #[test]
    fn test_user_cannot_tombstone_kernel() {
        let mut root = RootOffsetTable::new();
        let id = root.register(SUB_PROCESS, PRIV_KERNEL, 0);

        assert!(matches!(
            root.invalidate(id, PRIV_USER),
            Err(OffsetError::PrivilegeDenied)
        ));
        assert!(root.is_live(id));
    }

    #[test]
    fn test_same_privilege_can_write() {
        let mut root = RootOffsetTable::new();
        let id = root.register(SUB_FILE, PRIV_USER, 50);
        assert!(root.resolve(id, PRIV_USER, ACCESS_WRITE).is_ok());
    }

    #[test]
    fn test_higher_privilege_can_write_lower() {
        let mut root = RootOffsetTable::new();
        let id = root.register(SUB_FILE, PRIV_USER, 50);
        assert!(root.resolve(id, PRIV_SYSTEM, ACCESS_WRITE).is_ok());
    }

    #[test]
    fn test_invalidate_makes_unreachable() {
        let mut root = RootOffsetTable::new();
        let id = root.register(SUB_PROCESS, PRIV_USER, 10);

        root.invalidate(id, PRIV_KERNEL).unwrap();
        assert!(!root.is_live(id));
        assert!(matches!(
            root.resolve(id, PRIV_KERNEL, ACCESS_READ),
            Err(OffsetError::Invalidated)
        ));
    }

    #[test]
    fn test_logical_id_never_reused() {
        let mut root = RootOffsetTable::new();
        let id0 = root.register(SUB_PROCESS, PRIV_USER, 0);
        let id1 = root.register(SUB_PROCESS, PRIV_USER, 1);
        root.invalidate(id0, PRIV_KERNEL).unwrap();

        let id2 = root.register(SUB_PROCESS, PRIV_USER, 2);
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2); // NOT 0 — never reused
        assert_eq!(root.total_entries(), 3);
        assert_eq!(root.live_entries(), 2);
    }

    #[test]
    fn test_scan_subsystem() {
        let mut root = RootOffsetTable::new();
        root.register(SUB_PROCESS, PRIV_SYSTEM, 0);
        root.register(SUB_FILE, PRIV_USER, 1);
        root.register(SUB_PROCESS, PRIV_USER, 2);
        root.register(SUB_IPC, PRIV_SYSTEM, 3);

        let procs = root.scan_subsystem(SUB_PROCESS);
        assert_eq!(procs.len(), 2);

        let files = root.scan_subsystem(SUB_FILE);
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_scan_privilege() {
        let mut root = RootOffsetTable::new();
        root.register(SUB_PROCESS, PRIV_SYSTEM, 0);
        root.register(SUB_FILE, PRIV_USER, 1);
        root.register(SUB_PROCESS, PRIV_USER, 2);

        let system = root.scan_privilege(PRIV_SYSTEM);
        assert_eq!(system.len(), 1);

        let user = root.scan_privilege(PRIV_USER);
        assert_eq!(user.len(), 2);
    }

    #[test]
    fn test_hardware_privilege() {
        let mut root = RootOffsetTable::new();
        let dma = root.register(SUB_MEMORY, PRIV_HARDWARE, 0);

        // Hardware can access its own entries
        assert!(root.resolve(dma, PRIV_HARDWARE, ACCESS_WRITE).is_ok());

        // User cannot access hardware entries
        assert!(matches!(
            root.resolve(dma, PRIV_USER, ACCESS_WRITE),
            Err(OffsetError::PrivilegeDenied)
        ));

        // But kernel can
        assert!(root.resolve(dma, PRIV_KERNEL, ACCESS_WRITE).is_ok());
    }

    #[test]
    fn test_driver_privilege_boundary() {
        let mut root = RootOffsetTable::new();

        let hw_region = root.register(SUB_MEMORY, PRIV_DRIVER, 0xA0);
        let user_file = root.register(SUB_FILE, PRIV_USER, 0x50);

        // Driver can access its own region
        assert!(root.resolve(hw_region, PRIV_DRIVER, ACCESS_WRITE).is_ok());

        // Driver can write user-level (higher priv can write lower)
        assert!(root.resolve(user_file, PRIV_DRIVER, ACCESS_WRITE).is_ok());

        // User cannot write driver-level
        assert!(matches!(
            root.resolve(hw_region, PRIV_USER, ACCESS_WRITE),
            Err(OffsetError::PrivilegeDenied)
        ));
    }

    #[test]
    fn test_out_of_range() {
        let root = RootOffsetTable::new();
        assert!(matches!(
            root.resolve(999, PRIV_KERNEL, ACCESS_READ),
            Err(OffsetError::NotFound)
        ));
    }

    #[test]
    fn test_the_two_immortal_positions() {
        let mut root = RootOffsetTable::new();

        // Position 0: the bootloader jump
        let bootloader = root.register(SUB_PROCESS, PRIV_HARDWARE, 0x0000);

        // Position 1: the solver / watchdog
        let solver = root.register(SUB_PROCESS, PRIV_KERNEL, 0x0001);

        // Both are live
        assert!(root.is_live(bootloader));
        assert!(root.is_live(solver));

        // User cannot tombstone either
        assert!(root.invalidate(bootloader, PRIV_USER).is_err());
        assert!(root.invalidate(solver, PRIV_USER).is_err());

        // Even system-level cannot tombstone kernel
        assert!(root.invalidate(solver, PRIV_SYSTEM).is_err());

        // Only kernel can tombstone kernel (but shouldn't — these are immortal by convention)
        // The offset table enforces the contract. The convention is the two immortal positions.
        assert_eq!(bootloader, 0);
        assert_eq!(solver, 1);
    }
}
