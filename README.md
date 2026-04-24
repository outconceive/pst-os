# Parallel String Theory OS

An operating system where everything — processes, files, messages, schedules — is a flat table of parallel strings. No trees, no pointer graphs, no hierarchical data structures. Identity is position. Mutation is append-only. Maintenance is tombstone + compact.

## The idea

Every modern OS manages resources through trees: process trees, directory trees, page tables, priority queues. Every tree requires pointer chasing, rebalancing, and cascading updates when structure changes.

PST OS replaces all of them with one primitive: **parallel strings**. A process isn't a struct with fields — it's a row index across parallel columns (state, priority, affinity, owner). A file isn't a node in a directory tree — it's a line with a path string. `ls` is a prefix scan. `ps` is a column filter. They're the same operation.

This isn't theoretical. The parallel strings model is proven in [Outconceive UI](https://github.com/outconceive/ui), a web framework where it replaces virtual DOM trees. PST OS asks: if it works for UI, does it work for an entire operating system?

## Architecture

```
┌──────────────────────────────────────────────┐
│  Everything is a ParallelTable               │
│                                              │
│  Process table    = parallel strings         │
│  Filesystem       = parallel strings         │
│  IPC event log    = parallel strings         │
│  Scheduler input  = parallel strings         │
│                                              │
│  ls = prefix scan    ps = column filter      │
│  find = grep         top = scan + sort       │
│  kill = tombstone    rm = tombstone          │
│  They're all the same operation.             │
└──────────────────────────────────────────────┘
```

### Core rules

1. **Identity is position.** A process ID is a row number. A file ID is a row number. They never change, even after compaction — the offset table bridges the gap.
2. **Mutation is append-only.** Creating a process appends a row. Sending a message appends a row. Nothing is overwritten in place.
3. **Deletion is tombstoning.** Killing a process writes a tombstone marker. Deleting a file writes a tombstone. The row stays until compaction sweeps it.
4. **Scheduling is constraint solving.** No priority queue. Processes declare relationships (`after:cryptod`, `share-memory:writer`). A topological sort computes the execution order. Deadlocks are cycles — the watchdog tombstones them and the system keeps running.
5. **Two immortal positions.** The bootloader jump and the offset table root. Everything else is ephemeral strings.

### Fault tolerance

A kernel panic is just a cycle in the constraint graph. The topological sort fails on those nodes, the watchdog tombstones them, and the solver resumes on the next tick. The system's failure mode is a slower tick, not a cliff.

## Crates

| Crate | Description |
|-------|-------------|
| **libpst** | Core primitive — `ParallelTable` (append, tombstone, compact, scan), `OffsetTable` (O(1) logical→physical), `Constraint` enum, topological solver |
| **proctable** | Process table — register processes as rows, `solve_spawn_order()` computes boot sequence from `After` constraints |
| **pst-vfs** | Filesystem — files are rows, directories are naming conventions, `ls()` is prefix scan, `find()` is grep, delete is tombstone |
| **pst-ipc** | IPC event log — append-only message passing, priority-ordered delivery, broadcast, ack/GC lifecycle |
| **pst-sched** | Scheduler — topological sort over process constraints, cycle detection = watchdog tombstone, blocked processes wake on pending messages |

## Example: boot sequence

Instead of hardcoding spawn order:

```rust
let mut pt = ProcessTable::new();

pt.register(service("cryptod", PRIV_SYSTEM, vec![]));
pt.register(service("vfs", PRIV_SYSTEM, vec![
    Constraint::After("cryptod".into()),
]));
pt.register(service("netd", PRIV_SYSTEM, vec![
    Constraint::After("cryptod".into()),
]));
pt.register(service("compositor", PRIV_USER, vec![
    Constraint::After("vfs".into()),
    Constraint::After("netd".into()),
]));

let order = pt.solve_spawn_order();
// → cryptod, vfs, netd, compositor
// (vfs and netd are interchangeable — both just need cryptod first)
```

The solver figures out the order. Add a new service with its constraints and the boot sequence adjusts automatically.

## Example: filesystem

```rust
let mut fs = FileSystem::new();
fs.create_dir("/home", 0, PERM_RWX)?;
fs.create_file("/home/readme.md", 0, PERM_RW)?;
fs.create_file("/home/notes.txt", 0, PERM_RW)?;
fs.create_file("/etc/hosts", 0, PERM_RW)?;

// ls is a prefix scan
fs.ls("/home/");  // → ["/home/readme.md", "/home/notes.txt"]

// find is grep
fs.find("readme");  // → ["/home/readme.md"]

// delete is tombstone — the ID never gets reused
fs.delete(file_id)?;
fs.compact();  // reclaims space, offset table bridges the gap
```

## Example: IPC

```rust
let mut log = EventLog::new();

// Normal message
log.send(Message { sender: 1, receiver: 2, channel: 0,
    priority: PRIORITY_NORMAL, payload: b"data".to_vec() })?;

// Hardware interrupt — same mechanism, higher priority
log.send(Message { sender: 0, receiver: 2, channel: 0,
    priority: PRIORITY_INTERRUPT, payload: b"irq".to_vec() })?;

// Receiver drains — interrupt comes first
let msgs = log.recv(2);
// → [irq, data]  (sorted by priority)

// Ack + GC lifecycle
for (id, _, _) in &msgs { log.ack(*id)?; }
log.gc();      // tombstones read messages
log.compact(); // reclaims space
```

## Tests

```sh
cargo test --target x86_64-pc-windows-msvc
```

56 tests across all crates. Key tests:

- `test_privion_full_boot` — solver produces correct boot order from constraints
- `test_interrupt_as_high_priority_append` — IRQs are just priority appends
- `test_cycle_detected_and_tombstoned` — deadlock → watchdog tombstone → system continues
- `test_identity_never_changes` — file ID survives 10 create/delete/compact cycles
- `test_ls_prefix_scan` — directory listing is a string scan, not a tree walk

## Boot proof

PST OS boots on the seL4 microkernel and renders Markout from a cold boot:

```
Booting all finished, dropped to user space
PST

========================================
  Parallel String Theory OS
  Booting on seL4 microkernel...
========================================

[pst-offset] Creating immortal root...
[pst-offset] Position 0: bootloader (HARDWARE)
[pst-offset] Position 1: solver (KERNEL)
[pst-offset] Immortal root: 2 positions

[proctable] 6 services registered
[pst-sched] Boot order: cryptod -> driverd -> netd -> vfs -> driver-nic -> compositor
[pst-sched] No cycles detected

[pst-markout] Parsed 12 lines
[pst-markout] Rendering to VDOM...
[pst-markout] HTML output (964 bytes):

<div class="mc-app"><div class="mc-card">...</div></div>

========================================
  PST OS boot complete.
  Markout rendered on bare metal.
  The thesis is proven.
========================================
```

159KB static ELF. x86_64. seL4 microkernel. Serial output via `seL4_DebugPutChar`. Process table, constraint solver, Markout parser, parametric renderer, and HTML serializer — all running on bare metal in `no_std` Rust.

## Origin

PST OS started as [Outconceive UI](https://github.com/outconceive/ui), a web framework that replaced virtual DOM trees with flat parallel strings. The framework proved that positional identity eliminates the need for hierarchical data structures in UI rendering. PST OS extends that principle to an entire operating system.

Built on [Privion OS](https://github.com/user/privion), a privacy-focused microkernel OS using the formally verified seL4 kernel. The seL4 capability system serves as the hardware-enforced offset table — the two immortal positions that everything else builds on.

## License

MIT
