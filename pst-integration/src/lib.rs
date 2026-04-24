#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

// This crate exists only for integration tests.
// The real value is in tests/ below.

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use libpst::constraint::Constraint;
    use proctable::{ProcessTable, ProcessEntry, STATE_NEW, STATE_READY, PRIV_SYSTEM, PRIV_DRIVER, PRIV_USER};
    use pst_ipc::{EventLog, Message, PRIORITY_NORMAL, PRIORITY_HIGH, PRIORITY_INTERRUPT};
    use pst_sched::{Scheduler, SchedEntry, Action, DEFAULT_BUDGET};
    use pst_watchdog::{Watchdog, Violation, Resolution, WATCHDOG_ID, CHANNEL_FAULT};
    use pst_mem::{RegionAllocator, PERM_RW, PERM_READ, OWNER_FREE, FLAG_SHARED};
    use pst_vfs::{FileSystem, PERM_RWX, TYPE_FILE};

    // =========================================================================
    // Helper: build SchedEntry from ProcessTable row
    // =========================================================================

    fn sched_entry_from(pt: &ProcessTable, logical: usize, constraints: Vec<Constraint>) -> SchedEntry {
        SchedEntry {
            name: String::from(pt.get_name(logical).unwrap_or("?")),
            priority: 128,
            budget: DEFAULT_BUDGET,
            constraints,
            blocked: pt.get_state(logical) != Some(STATE_READY),
            pending_messages: 0,
        }
    }

    // =========================================================================
    // Test 1: Full boot sequence — proctable → scheduler → memory allocation
    // =========================================================================

    #[test]
    fn test_boot_sequence_end_to_end() {
        // 1. Register processes in the process table
        let mut pt = ProcessTable::new();
        let mut mem = RegionAllocator::new(64 * 1024 * 1024);

        let id_crypto = pt.register(ProcessEntry {
            name: String::from("cryptod"), state: STATE_READY,
            privilege: PRIV_SYSTEM, priority: 200, affinity: 0,
            constraints: vec![],
        });
        let id_vfs = pt.register(ProcessEntry {
            name: String::from("vfs"), state: STATE_READY,
            privilege: PRIV_SYSTEM, priority: 180, affinity: 0,
            constraints: vec![Constraint::After(String::from("cryptod"))],
        });
        let id_netd = pt.register(ProcessEntry {
            name: String::from("netd"), state: STATE_READY,
            privilege: PRIV_SYSTEM, priority: 180, affinity: 0,
            constraints: vec![Constraint::After(String::from("cryptod"))],
        });
        let id_comp = pt.register(ProcessEntry {
            name: String::from("compositor"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![
                Constraint::After(String::from("vfs")),
                Constraint::After(String::from("netd")),
            ],
        });

        // 2. Solver computes boot order
        let result = pt.solve_spawn_order();
        assert!(result.cycles.is_empty());

        let crypto_pos = result.order.iter().position(|n| n == "cryptod").unwrap();
        let vfs_pos = result.order.iter().position(|n| n == "vfs").unwrap();
        let netd_pos = result.order.iter().position(|n| n == "netd").unwrap();
        let comp_pos = result.order.iter().position(|n| n == "compositor").unwrap();
        assert!(crypto_pos < vfs_pos);
        assert!(crypto_pos < netd_pos);
        assert!(vfs_pos < comp_pos);
        assert!(netd_pos < comp_pos);

        // 3. Allocate memory for each process in boot order
        // Owner IDs offset by 1 to avoid collision with OWNER_FREE (0)
        for name in &result.order {
            let owner = pt.find_by_name(name).unwrap() as u8 + 1;
            let (_, region) = mem.alloc(
                16 * 1024, // 16KB per process
                owner,
                PERM_RW,
                0,
            ).unwrap();
            assert!(region.size == 16 * 1024);
        }

        assert_eq!(mem.used_bytes(), 4 * 16 * 1024);
    }

    // =========================================================================
    // Test 2: IPC triggers scheduler — message wakes blocked process
    // =========================================================================

    #[test]
    fn test_ipc_wakes_blocked_process() {
        let mut pt = ProcessTable::new();
        let mut ipc = EventLog::new();

        let id_server = pt.register(ProcessEntry {
            name: String::from("server"), state: STATE_READY,
            privilege: PRIV_SYSTEM, priority: 150, affinity: 0,
            constraints: vec![],
        });
        let id_client = pt.register(ProcessEntry {
            name: String::from("client"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![],
        });

        // Client sends request to server
        ipc.send(Message {
            sender: id_client as u8,
            receiver: id_server as u8,
            channel: 0,
            priority: PRIORITY_NORMAL,
            payload: b"GET /index".to_vec(),
        }).unwrap();

        // Build scheduler entries — server has pending message
        let pending = ipc.pending_count(id_server as u8);
        let entries = vec![
            SchedEntry {
                name: String::from("server"),
                priority: 150,
                budget: DEFAULT_BUDGET,
                constraints: vec![],
                blocked: false,
                pending_messages: pending,
            },
            SchedEntry {
                name: String::from("client"),
                priority: 100,
                budget: DEFAULT_BUDGET,
                constraints: vec![],
                blocked: true, // waiting for reply
                pending_messages: 0,
            },
        ];

        let sched = Scheduler::new();
        let order = sched.run_order(&entries);

        // Server runs (has messages), client stays blocked
        assert_eq!(order, vec!["server"]);
    }

    // =========================================================================
    // Test 3: Watchdog tombstones cycle → memory freed → proctable updated
    // =========================================================================

    #[test]
    fn test_watchdog_tombstones_frees_memory() {
        let mut pt = ProcessTable::new();
        let mut ipc = EventLog::new();
        let mut mem = RegionAllocator::new(64 * 1024 * 1024);
        let mut wd = Watchdog::new().with_tombstone_threshold(1);

        // Register two processes that deadlock
        let id_a = pt.register(ProcessEntry {
            name: String::from("a"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![Constraint::After(String::from("b"))],
        });
        let id_b = pt.register(ProcessEntry {
            name: String::from("b"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![Constraint::After(String::from("a"))],
        });

        // Allocate memory for both (owner = id + 1 to avoid OWNER_FREE collision)
        let owner_a = id_a as u8 + 1;
        let owner_b = id_b as u8 + 1;
        mem.alloc(4096, owner_a, PERM_RW, 0).unwrap();
        mem.alloc(4096, owner_b, PERM_RW, 0).unwrap();
        assert_eq!(mem.used_bytes(), 8192);

        // Build sched entries
        let entries = vec![
            SchedEntry {
                name: String::from("a"), priority: 100, budget: DEFAULT_BUDGET,
                constraints: vec![Constraint::After(String::from("b"))],
                blocked: false, pending_messages: 0,
            },
            SchedEntry {
                name: String::from("b"), priority: 100, budget: DEFAULT_BUDGET,
                constraints: vec![Constraint::After(String::from("a"))],
                blocked: false, pending_messages: 0,
            },
        ];

        // Watchdog tick detects cycle and tombstones
        let tick = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(!tick.violations.is_empty());

        // Free memory for tombstoned processes
        for r in &tick.resolutions {
            if let Resolution::Tombstoned(name) = r {
                let owner = match name.as_str() {
                    "a" => owner_a,
                    "b" => owner_b,
                    _ => continue,
                };
                mem.free_all(owner);
            }
        }

        // Memory should be reclaimed
        assert!(mem.used_bytes() < 8192);
    }

    // =========================================================================
    // Test 4: Process death cascade — kill process → free memory → GC IPC
    // =========================================================================

    #[test]
    fn test_process_death_cascade() {
        let mut pt = ProcessTable::new();
        let mut ipc = EventLog::new();
        let mut mem = RegionAllocator::new(64 * 1024 * 1024);

        let id_worker = pt.register(ProcessEntry {
            name: String::from("worker"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![],
        });

        // Worker has memory
        mem.alloc(8192, id_worker as u8 + 1, PERM_RW, 0).unwrap();
        mem.alloc(4096, id_worker as u8 + 1, PERM_RW, 0).unwrap();

        // Worker has pending messages
        ipc.send(Message {
            sender: 0, receiver: id_worker as u8,
            channel: 0, priority: PRIORITY_NORMAL,
            payload: b"task1".to_vec(),
        }).unwrap();
        ipc.send(Message {
            sender: 0, receiver: id_worker as u8,
            channel: 0, priority: PRIORITY_NORMAL,
            payload: b"task2".to_vec(),
        }).unwrap();

        assert_eq!(mem.regions_for(id_worker as u8 + 1).len(), 2);
        assert_eq!(ipc.pending_count(id_worker as u8), 2);

        // Kill worker
        pt.tombstone(id_worker);
        assert!(!pt.is_live(id_worker));

        // Free its memory
        mem.free_all(id_worker as u8 + 1);
        assert_eq!(mem.regions_for(id_worker as u8 + 1).len(), 0);

        // Drain and discard its messages
        let msgs = ipc.recv(id_worker as u8);
        for (id, _, _) in &msgs {
            let _ = ipc.ack(*id);
        }
        ipc.gc();

        assert_eq!(ipc.pending_count(id_worker as u8), 0);
        assert_eq!(ipc.live_messages(), 0);
    }

    // =========================================================================
    // Test 5: Shared memory between processes — IPC coordinates access
    // =========================================================================

    #[test]
    fn test_shared_memory_via_ipc() {
        let mut pt = ProcessTable::new();
        let mut ipc = EventLog::new();
        let mut mem = RegionAllocator::new(64 * 1024 * 1024);

        let id_writer = pt.register(ProcessEntry {
            name: String::from("writer"), state: STATE_READY,
            privilege: PRIV_SYSTEM, priority: 150, affinity: 0,
            constraints: vec![],
        });
        let id_reader = pt.register(ProcessEntry {
            name: String::from("reader"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![Constraint::ShareMemory(String::from("writer"))],
        });

        // Writer allocates shared region
        let (writer_alloc, region) = mem.alloc(4096, id_writer as u8 + 1, PERM_RW, 0).unwrap();

        // Share with reader (read-only)
        let (reader_alloc, shared_region) = mem.share(writer_alloc, id_reader as u8 + 1, PERM_READ).unwrap();

        // Same physical region
        assert_eq!(region.start, shared_region.start);

        // Writer notifies reader via IPC
        ipc.send(Message {
            sender: id_writer as u8,
            receiver: id_reader as u8,
            channel: 0,
            priority: PRIORITY_HIGH,
            payload: b"region-ready".to_vec(),
        }).unwrap();

        // Scheduler respects ShareMemory constraint — writer before reader
        let entries = vec![
            SchedEntry {
                name: String::from("writer"), priority: 150, budget: DEFAULT_BUDGET,
                constraints: vec![], blocked: false, pending_messages: 0,
            },
            SchedEntry {
                name: String::from("reader"), priority: 100, budget: DEFAULT_BUDGET,
                constraints: vec![Constraint::ShareMemory(String::from("writer"))],
                blocked: false,
                pending_messages: ipc.pending_count(id_reader as u8),
            },
        ];

        let sched = Scheduler::new();
        let order = sched.run_order(&entries);
        let w = order.iter().position(|n| n == "writer").unwrap();
        let r = order.iter().position(|n| n == "reader").unwrap();
        assert!(w < r);
    }

    // =========================================================================
    // Test 6: Hardware interrupt → IPC → scheduler → deferred work
    // =========================================================================

    #[test]
    fn test_interrupt_to_deferred_work() {
        let mut ipc = EventLog::new();

        // Hardware NIC fires IRQ — appended as interrupt priority
        ipc.send(Message {
            sender: 0, // hardware
            receiver: 1, // irq_handler process
            channel: 0,
            priority: PRIORITY_INTERRUPT,
            payload: b"irq:11".to_vec(),
        }).unwrap();

        // IRQ handler drains its messages
        let msgs = ipc.recv(1);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].2, b"irq:11");

        // IRQ handler posts deferred work
        ipc.send(Message {
            sender: 1,
            receiver: 2, // network stack
            channel: 0,
            priority: PRIORITY_HIGH,
            payload: b"packet-ready".to_vec(),
        }).unwrap();

        // Scheduler runs — network stack has pending message
        let entries = vec![
            SchedEntry {
                name: String::from("irq_handler"), priority: 255, budget: 2,
                constraints: vec![], blocked: false, pending_messages: 0,
            },
            SchedEntry {
                name: String::from("net_stack"), priority: 150, budget: DEFAULT_BUDGET,
                constraints: vec![Constraint::After(String::from("irq_handler"))],
                blocked: false,
                pending_messages: ipc.pending_count(2),
            },
            SchedEntry {
                name: String::from("user_app"), priority: 50, budget: DEFAULT_BUDGET,
                constraints: vec![], blocked: false, pending_messages: 0,
            },
        ];

        let sched = Scheduler::new();
        let order = sched.run_order(&entries);

        let irq = order.iter().position(|n| n == "irq_handler").unwrap();
        let net = order.iter().position(|n| n == "net_stack").unwrap();
        assert!(irq < net);
    }

    // =========================================================================
    // Test 7: VFS + memory — file write allocates, delete frees
    // =========================================================================

    #[test]
    fn test_vfs_backed_by_memory() {
        let mut fs = FileSystem::new();
        let mut mem = RegionAllocator::new(64 * 1024 * 1024);

        // Create file — allocate backing memory
        let file_id = fs.create_file("/data/log.txt", 1, pst_vfs::PERM_RW).unwrap();
        let (mem_id, region) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();

        // Write data
        fs.write(file_id, b"2026-04-24 system boot complete").unwrap();

        // Read it back
        let content = fs.read(file_id).unwrap();
        assert_eq!(content, b"2026-04-24 system boot complete");

        // Delete file — free backing memory
        fs.delete(file_id).unwrap();
        mem.free(mem_id, 1).unwrap();

        assert!(fs.find_path("/data/log.txt").is_none());
        assert_eq!(mem.get_owner(mem_id), Some(OWNER_FREE));
    }

    // =========================================================================
    // Test 8: Full system tick — watchdog + scheduler + IPC + memory
    // =========================================================================

    #[test]
    fn test_full_system_tick() {
        let mut pt = ProcessTable::new();
        let mut ipc = EventLog::new();
        let mut mem = RegionAllocator::new(64 * 1024 * 1024);
        let mut wd = Watchdog::new();

        // Register system services
        let ids: Vec<usize> = vec![
            pt.register(ProcessEntry {
                name: String::from("cryptod"), state: STATE_READY,
                privilege: PRIV_SYSTEM, priority: 200, affinity: 0,
                constraints: vec![],
            }),
            pt.register(ProcessEntry {
                name: String::from("vfs"), state: STATE_READY,
                privilege: PRIV_SYSTEM, priority: 180, affinity: 0,
                constraints: vec![Constraint::After(String::from("cryptod"))],
            }),
            pt.register(ProcessEntry {
                name: String::from("netd"), state: STATE_READY,
                privilege: PRIV_SYSTEM, priority: 180, affinity: 0,
                constraints: vec![Constraint::After(String::from("cryptod"))],
            }),
        ];

        // Allocate memory for each
        for &id in &ids {
            mem.alloc(16384, id as u8 + 1, PERM_RW, 0).unwrap();
            wd.monitor(pt.get_name(id).unwrap());
        }

        // Build scheduler entries
        let entries: Vec<SchedEntry> = ids.iter().map(|&id| {
            SchedEntry {
                name: String::from(pt.get_name(id).unwrap()),
                priority: 128,
                budget: DEFAULT_BUDGET,
                constraints: match pt.get_name(id).unwrap() {
                    "vfs" | "netd" => vec![Constraint::After(String::from("cryptod"))],
                    _ => vec![],
                },
                blocked: false,
                pending_messages: ipc.pending_count(id as u8),
            }
        }).collect();

        // Watchdog tick — should be clean
        let tick = wd.tick(&entries, &mut pt, &mut ipc);
        assert!(tick.violations.is_empty());

        // All processes should be scheduled
        let run_count = tick.schedule.iter()
            .filter(|a| matches!(a, Action::Run(_, _)))
            .count();
        assert_eq!(run_count, 3);

        // Send heartbeats
        for &id in &ids {
            wd.heartbeat(pt.get_name(id).unwrap());
        }

        // Memory accounted for
        assert_eq!(mem.used_bytes(), 3 * 16384);
        assert_eq!(mem.allocation_count(), 3);
    }

    // =========================================================================
    // Test 9: Cascading failure recovery
    // =========================================================================

    #[test]
    fn test_cascading_failure_recovery() {
        let mut pt = ProcessTable::new();
        let mut ipc = EventLog::new();
        let mut mem = RegionAllocator::new(64 * 1024 * 1024);
        let mut wd = Watchdog::new().with_tombstone_threshold(1);

        // 4 processes: healthy chain + one rogue pair
        pt.register(ProcessEntry {
            name: String::from("healthy_a"), state: STATE_READY,
            privilege: PRIV_SYSTEM, priority: 200, affinity: 0,
            constraints: vec![],
        });
        pt.register(ProcessEntry {
            name: String::from("healthy_b"), state: STATE_READY,
            privilege: PRIV_SYSTEM, priority: 150, affinity: 0,
            constraints: vec![Constraint::After(String::from("healthy_a"))],
        });
        pt.register(ProcessEntry {
            name: String::from("rogue_x"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![Constraint::After(String::from("rogue_y"))],
        });
        pt.register(ProcessEntry {
            name: String::from("rogue_y"), state: STATE_READY,
            privilege: PRIV_USER, priority: 100, affinity: 0,
            constraints: vec![Constraint::After(String::from("rogue_x"))],
        });

        // Allocate memory for all
        for i in 0..4u8 {
            mem.alloc(4096, i + 1, PERM_RW, 0).unwrap();
        }

        let entries = vec![
            SchedEntry { name: String::from("healthy_a"), priority: 200, budget: DEFAULT_BUDGET,
                constraints: vec![], blocked: false, pending_messages: 0 },
            SchedEntry { name: String::from("healthy_b"), priority: 150, budget: DEFAULT_BUDGET,
                constraints: vec![Constraint::After(String::from("healthy_a"))],
                blocked: false, pending_messages: 0 },
            SchedEntry { name: String::from("rogue_x"), priority: 100, budget: DEFAULT_BUDGET,
                constraints: vec![Constraint::After(String::from("rogue_y"))],
                blocked: false, pending_messages: 0 },
            SchedEntry { name: String::from("rogue_y"), priority: 100, budget: DEFAULT_BUDGET,
                constraints: vec![Constraint::After(String::from("rogue_x"))],
                blocked: false, pending_messages: 0 },
        ];

        // Watchdog tick
        let tick = wd.tick(&entries, &mut pt, &mut ipc);

        // Rogue pair should be detected
        assert!(tick.violations.iter().any(|v| matches!(v, Violation::Cycle(_))));

        // Free memory for tombstoned processes
        for r in &tick.resolutions {
            if let Resolution::Tombstoned(name) = r {
                if let Some(id) = pt.find_by_name(name) {
                    mem.free_all(id as u8 + 1);
                }
            }
        }

        // Healthy processes still running
        let healthy_runs: Vec<&str> = tick.schedule.iter().filter_map(|a| match a {
            Action::Run(n, _) if n.starts_with("healthy") => Some(n.as_str()),
            _ => None,
        }).collect();
        assert_eq!(healthy_runs.len(), 2);
    }
}
