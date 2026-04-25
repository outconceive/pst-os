# Architecture Overview

PST OS replaces hierarchical data structures with one primitive: parallel strings.

## The Stack

```
Markout source (disk or inline)
        │
        ▼
    pst-markout parser (no_std)
    ├── 18 component types
    ├── 14 container types
    ├── Grid layout (col-N)
    ├── Responsive breakpoints
    ├── Validation, events, animations
    └── Constraints (@parametric)
        │
        ▼
    pst-ui interaction layer (no_std)
    ├── State as parallel strings
    ├── Focus / hover / enabled
    ├── Tab order, click handling
    └── Dirty tracking
        │
        ▼
    Renderer (pluggable)
    ├── pst-framebuffer → VGA pixels
    ├── pst-terminal → ANSI sequences
    ├── html::to_html → HTML string
    └── Outconceive WASM → DOM
```

## Crates

| Crate | Purpose |
|-------|---------|
| **libpst** | ParallelTable, OffsetTable, constraint solver |
| **pst-markout** | Parser, VNode renderer, state system |
| **pst-framebuffer** | Pixel renderer with bitmap font |
| **pst-terminal** | ANSI terminal renderer |
| **pst-ui** | Interaction model as parallel strings |
| **pst-blk** | Virtio-blk block device |
| **proctable** | Process table with constraint-solved boot |
| **pst-vfs** | Flat filesystem (ls = prefix scan) |
| **pst-ipc** | Append-only IPC event log |
| **pst-sched** | Topological sort scheduler |
| **pst-watchdog** | Cycle detection, tombstoning |
| **pst-mem** | Region allocator with coalescing |
| **pst-offset** | Immortal root with privilege enforcement |
| **pst-time** | Temporal dimension with compaction |

## Key Principles

1. **Identity is position** — no generated keys, no pointers
2. **Mutation is append-only** — no in-place updates
3. **Deletion is tombstoning** — lazy compaction
4. **Relationships are constraints** — topological sort, not tree edges
5. **Two immortal positions** — bootloader + offset table root
