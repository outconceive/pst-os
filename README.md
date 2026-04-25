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
| **pst-watchdog** | Cycle detection, heartbeat monitoring, escalating tombstone |
| **pst-mem** | Append-only region allocator with shared memory and coalescing |
| **pst-offset** | Immortal root with privilege enforcement |
| **pst-time** | Temporal dimension with tiered compaction |
| **pst-markout** | Markout parser + parametric renderer (no_std) |
| **pst-framebuffer** | Pixel renderer — Markout to VGA, no display server |
| **pst-terminal** | Terminal renderer — Markout to ANSI escape sequences |
| **pst-blk** | Block device interface — virtio-blk driver for persistence |
| **pst-integration** | 9 cross-crate tests proving subsystems work together |

## Markout reference

Markout is the declarative UI language. No JSX, no transpiler, no build step.

### Components

```
| Hello World                          Plain text (label)
| {input:name}                         Text input bound to "name"
| {password:pass}                      Password field
| {button:submit "Sign In" primary}    Styled button
| {checkbox:agree}                     Checkbox
| {radio:choice}                       Radio button
| {select:country}                     Dropdown select
| {textarea:notes}                     Multi-line text area
| {image:photo "photo.png"}            Image
| {link:docs "Documentation" ghost}    Hyperlink
| {divider:sep}                        Horizontal rule
| {spacer:gap}                         Whitespace
| {progress:loading}                   Progress indicator
```

### Containers

```
@card padding:16                       Card with border
| ...content...
@end card

@nav                                   Navigation bar
@header                                Page header
@footer                                Page footer
@section                               Section
@form                                  Form group
@heading                               Heading block
@list                                  Unordered list
@ordered-list                          Numbered list
@quote                                 Blockquote
@code-block                            Code block
```

### Parametric layout

```
@parametric
| {label:title "Dashboard"}
| {input:search center-x:title gap-y:1rem}
| {button:go "Search" after:search gap-x:8px center-y:search}
@end parametric
```

Constraint vocabulary: `center-x`, `center-y`, `left`, `right`, `top`, `bottom`, `gap-x`, `gap-y`, `width`, `height`, `after`. The solver computes absolute positions from relationships. No coordinates.

### Styles

Append a style name to any component: `primary`, `secondary`, `danger`, `warning`, `info`, `ghost`, `outline`, `dark`, `light`.

### Rich text editor

```
@editor bold italic heading code bind:notes
| ...editable content...
@end editor
```

### Data binding

State updates re-render only affected components. `{input:name}` binds to the `name` key. `{button:submit}` triggers on click. State flows through the constraint solver.

### Lists

```
@each:items
| {label:items.name}  {button:remove "×" danger}
@end each
```

Dynamic lists bound to state. Add/remove items, the solver re-renders.

### Rendering targets

The same Markout document renders on every target:

| Target | Renderer | Output |
|--------|----------|--------|
| Browser | Outconceive WASM | DOM elements |
| Terminal | pst-terminal | ANSI escape sequences |
| Desktop | pst-framebuffer | VGA pixels |
| Serial | pst-terminal | serial console |
| SSR | html::to_html | static HTML string |

One parser, one solver, one VNode tree, N renderers.

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
cargo test
```

137+ tests across all crates. Key tests:

- `test_privion_full_boot` — solver produces correct boot order from constraints
- `test_interrupt_as_high_priority_append` — IRQs are just priority appends
- `test_cycle_detected_and_tombstoned` — deadlock → watchdog tombstone → system continues
- `test_identity_never_changes` — file ID survives 10 create/delete/compact cycles
- `test_ls_prefix_scan` — directory listing is a string scan, not a tree walk

## What boots

PST OS boots on the seL4 microkernel into a windowed desktop with:

- **VGA framebuffer** — 2MB large page mapped through seL4 capabilities
- **Keyboard input** — PS/2 IRQ via IOAPIC, scancode translation
- **Multiple windows** — Tab switches focus, status bar, box-drawing borders
- **Markout shell** — type Markout, see it render live via pst-terminal
- **Text editor** — .txt and .md word processor, saves to disk
- **Code stepper** — syntax-highlighted Rust with side-by-side output
- **dt:// browser** — Markout pages from disk with `/pst/index.md` resolution
- **gh:// browser** — fetch Markout from GitHub via host proxy (crypto offload NIC)
- **Persistence** — virtio-blk driver, save/restore desktop across reboots
- **Network** — virtio-net driver, smoltcp TCP/IP stack

```
[vga] PDPT: exists
[vga] PD: mapped
[vga] Mapped! Writing to VGA framebuffer...
[vga] Desktop on screen!
[blk] virtio-blk at slot 4 — 2048 sectors (1024 KiB)
[net] virtio-net at slot 3 — MAC: 52:54:0:12:34:56
[kb] Keyboard ready
[shell] Markout shell ready.
```

217KB static ELF. x86_64. seL4 microkernel. No Wayland. No X11. No display server. No browser engine. Markout all the way down.

## Origin

PST OS started as [Outconceive UI](https://github.com/outconceive/ui), a web framework that replaced virtual DOM trees with flat parallel strings. The framework proved that positional identity eliminates the need for hierarchical data structures in UI rendering. PST OS extends that principle to an entire operating system.

Built on [Privion OS](https://github.com/user/privion), a privacy-focused microkernel OS using the formally verified seL4 kernel. The seL4 capability system serves as the hardware-enforced offset table — the two immortal positions that everything else builds on.

## License

MIT
