#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use libpst::constraint::Constraint;
use proctable::{ProcessTable, ProcessEntry, STATE_NEW, STATE_READY};
use pst_sched::{Scheduler, SchedEntry, Action, DEFAULT_BUDGET};
use pst_offset::{
    RootOffsetTable, SUB_PROCESS, PRIV_SYSTEM, PRIV_DRIVER, PRIV_USER,
    PRIV_KERNEL, PRIV_HARDWARE, ACCESS_READ,
};

use libprivos::mem::UntypedAllocator;
use libprivos::process::ProcessBuilder;

/// Service definition for declarative boot.
pub struct ServiceDef {
    pub name: String,
    pub privilege: u8,
    pub priority: u8,
    pub constraints: Vec<Constraint>,
}

impl ServiceDef {
    pub fn new(name: &str, privilege: u8) -> Self {
        Self {
            name: String::from(name),
            privilege,
            priority: 128,
            constraints: Vec::new(),
        }
    }

    pub fn priority(mut self, p: u8) -> Self {
        self.priority = p;
        self
    }

    pub fn after(mut self, dep: &str) -> Self {
        self.constraints.push(Constraint::After(String::from(dep)));
        self
    }

    pub fn share_memory(mut self, other: &str) -> Self {
        self.constraints.push(Constraint::ShareMemory(String::from(other)));
        self
    }
}

/// The PST boot manager — registers services, solves boot order,
/// and spawns them via seL4 capabilities.
pub struct PstBoot {
    proctable: ProcessTable,
    offset_root: RootOffsetTable,
    services: Vec<ServiceDef>,
}

impl PstBoot {
    pub fn new() -> Self {
        // Position 0: bootloader (hardware)
        let mut offset_root = RootOffsetTable::new();
        offset_root.register(SUB_PROCESS, PRIV_HARDWARE, 0x0000);

        // Position 1: solver/watchdog (kernel)
        offset_root.register(SUB_PROCESS, PRIV_KERNEL, 0x0001);

        Self {
            proctable: ProcessTable::new(),
            offset_root,
            services: Vec::new(),
        }
    }

    /// Register a service for boot.
    pub fn register(&mut self, def: ServiceDef) -> usize {
        let logical = self.proctable.register(ProcessEntry {
            name: def.name.clone(),
            state: STATE_NEW,
            privilege: def.privilege,
            priority: def.priority,
            affinity: 0,
            constraints: def.constraints.clone(),
        });

        // Register in the immortal offset table
        let offset_id = self.offset_root.register(
            SUB_PROCESS,
            def.privilege,
            logical as u16,
        );

        self.services.push(def);
        logical
    }

    /// Compute the boot order using the constraint solver.
    pub fn solve_boot_order(&self) -> Vec<String> {
        let result = self.proctable.solve_spawn_order();
        result.order
    }

    /// Execute the boot sequence — spawns services in solved order.
    ///
    /// This is the function that replaces init's hardcoded spawn sequence.
    /// It takes the seL4 allocator and spawns each service via ProcessBuilder
    /// in the order the constraint solver determined.
    ///
    /// Returns the list of spawned service names in order.
    pub fn boot(&mut self, alloc: &mut UntypedAllocator) -> Vec<String> {
        let order = self.solve_boot_order();

        for name in &order {
            if let Some(id) = self.proctable.find_by_name(name) {
                self.proctable.set_state(id, STATE_READY);
            }

            // In a full implementation, this would:
            // 1. Look up the service's ELF binary in the initrd
            // 2. Call ProcessBuilder::new(name, alloc).spawn()
            // 3. Grant the appropriate endpoint capabilities
            //
            // For now, we mark services as ready in the proctable.
            // The actual ProcessBuilder::spawn() requires the initrd
            // and endpoint caps which are init-specific setup.
        }

        order
    }

    pub fn proctable(&self) -> &ProcessTable {
        &self.proctable
    }

    pub fn offset_root(&self) -> &RootOffsetTable {
        &self.offset_root
    }
}

/// Build the Privion boot configuration using PST declarative constraints.
///
/// This replaces the hardcoded spawn sequence in init/main.rs:
///
/// ```text
/// // OLD (hardcoded):
/// ProcessBuilder::new("cryptod", &mut alloc).spawn();
/// ProcessBuilder::new("vfs", &mut alloc).spawn();
/// ...
///
/// // NEW (declarative):
/// let mut boot = privion_boot_config();
/// boot.boot(&mut alloc);
/// ```
pub fn privion_boot_config() -> PstBoot {
    let mut boot = PstBoot::new();

    boot.register(
        ServiceDef::new("cryptod", PRIV_SYSTEM).priority(200)
    );
    boot.register(
        ServiceDef::new("vfs", PRIV_SYSTEM).priority(180)
            .after("cryptod")
    );
    boot.register(
        ServiceDef::new("netd", PRIV_SYSTEM).priority(180)
            .after("cryptod")
    );
    boot.register(
        ServiceDef::new("driverd", PRIV_DRIVER).priority(190)
    );
    boot.register(
        ServiceDef::new("driver-nic", PRIV_DRIVER).priority(170)
            .after("driverd")
    );
    boot.register(
        ServiceDef::new("compositor", PRIV_USER).priority(100)
            .after("vfs")
            .after("netd")
    );

    boot
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_privion_boot_order() {
        let boot = privion_boot_config();
        let order = boot.solve_boot_order();

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

    #[test]
    fn test_two_immortal_positions() {
        let boot = privion_boot_config();
        let root = boot.offset_root();

        // Positions 0 and 1 are the immortal pair
        assert!(root.is_live(0)); // bootloader
        assert!(root.is_live(1)); // solver

        // User cannot tombstone them
        // (tested in pst-offset, but verify the boot config sets them up)
        let entry0 = root.resolve(0, PRIV_KERNEL, ACCESS_READ).unwrap();
        assert_eq!(entry0.privilege, PRIV_HARDWARE);

        let entry1 = root.resolve(1, PRIV_KERNEL, ACCESS_READ).unwrap();
        assert_eq!(entry1.privilege, PRIV_KERNEL);
    }

    #[test]
    fn test_all_services_in_offset_table() {
        let boot = privion_boot_config();
        let root = boot.offset_root();

        // 2 immortal + 6 services = 8 entries
        assert_eq!(root.total_entries(), 8);
        assert_eq!(root.live_entries(), 8);

        // All process entries
        let procs = root.scan_subsystem(SUB_PROCESS);
        assert_eq!(procs.len(), 8);
    }

    #[test]
    fn test_privilege_levels_correct() {
        let boot = privion_boot_config();
        let pt = boot.proctable();

        assert_eq!(pt.get_privilege(pt.find_by_name("cryptod").unwrap()), Some(PRIV_SYSTEM));
        assert_eq!(pt.get_privilege(pt.find_by_name("driverd").unwrap()), Some(PRIV_DRIVER));
        assert_eq!(pt.get_privilege(pt.find_by_name("compositor").unwrap()), Some(PRIV_USER));
    }

    #[test]
    fn test_add_new_service() {
        let mut boot = privion_boot_config();

        // Add a new service — boot order adjusts automatically
        boot.register(
            ServiceDef::new("logger", PRIV_SYSTEM).priority(190)
                .after("cryptod")
        );

        let order = boot.solve_boot_order();
        assert_eq!(order.len(), 7);

        let crypto = order.iter().position(|n| n == "cryptod").unwrap();
        let logger = order.iter().position(|n| n == "logger").unwrap();
        assert!(crypto < logger);
    }
}
