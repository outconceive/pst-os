# PST OS Roadmap

## What's Done

### Core Crates (130 tests, all green)
- **libpst** — ParallelTable, OffsetTable, Constraint, topological solver
- **proctable** — process table with constraint-solved spawn order
- **pst-vfs** — flat filesystem (ls = prefix scan, find = grep)
- **pst-ipc** — append-only IPC event log with priority delivery
- **pst-sched** — constraint solver computing execution order
- **pst-watchdog** — cycle detection, heartbeat monitoring, escalating tombstone
- **pst-mem** — append-only region allocator with shared memory and coalescing
- **pst-offset** — immortal root with privilege enforcement
- **pst-time** — temporal dimension with tiered compaction
- **pst-markout** — Markout parser + parametric renderer (no_std)
- **pst-framebuffer** — pixel renderer with bitmap font
- **pst-integration** — 9 cross-crate tests proving subsystems work together

### Bare Metal Boot (working)
- seL4 microkernel boots, loads rootserver
- Custom entry point saves bootinfo from kernel's rdi register
- Heap allocator initializes
- Constraint solver computes boot order from process table
- Markout parses and renders to HTML on serial
- Framebuffer renders 320×200 pixels in memory
- All output via seL4_DebugPutChar syscall

### VGA Display (done)
- PCI bus probe, device untyped, 2MB large pages, page table mapping
- CONFIG_HUGE_PAGE=1 shifted object type constants — fixed

### Keyboard Input (done)
- PS/2 IRQ via IOAPIC, scancode-to-ASCII translation

### Terminal Renderer (done)
- pst-terminal crate — Markout to ANSI escape sequences (7 tests)

### Markout Shell (done)
- Type Markout, render live to serial via pst-terminal

### Multiple Windows (done)
- Desktop with Tab focus, status bar, box-drawing borders

### Persistence (done)
- virtio-blk driver, flat filesystem, save/restore desktop

### Network (done)
- virtio-net driver, smoltcp TCP/IP stack

### Browser (done)
- dt:// protocol — Markout pages from disk with /pst/index.md
- gh:// protocol — fetch from GitHub via host proxy

### Apps (done)
- Text editor (.txt/.md word processor with save to disk)
- Code stepper (syntax highlighting, side-by-side output)

### Outconceive Convergence (done)
- Same Markout → same VNode tree → HTML / ANSI / pixels
- The web framework and the OS are the same thing

---

## Step 1 — Pixels on Screen

### What's left
1. Fix page table mapping for VGA framebuffer
   - PDPT already exists at PML4[0] — skip allocation, just map PD
   - Verify PD map invocation label and arguments
   - Map 2MB large page at the PD entry
2. Write VGA text mode characters to mapped memory
3. Boot QEMU with `-display gtk` to see the result

### Estimated effort: 1-2 sessions

---

## Step 2 — Keyboard Input

### What's needed
1. PS/2 keyboard IRQ handler
   - Register IRQ 1 via `seL4_IRQControl_Get`
   - Bind to a notification cap
   - Wait for notification, read scancode from I/O port 0x60
2. Scancode → ASCII translation table
3. Append keypress to IPC event log as a message
4. Route keyboard events to the focused "process" (initially just the shell)

### Dependencies
- I/O port cap for 0x60 (keyboard data port) — same mechanism as PCI, already working
- IRQ registration — libprivos already has `IrqHandler::register()`

---

## Step 3 — Markout Shell

### What's needed
1. A text buffer that accumulates keystrokes
2. On Enter: parse the buffer as Markout, render to VGA
3. The shell IS a `@parametric` block that re-solves on every render
4. Type `{button:go "Click me" primary}` → button appears on screen
5. Type `@card` ... `@end card` → card renders
6. Backspace, cursor, basic line editing

### Dependencies
- Step 1 (pixels on screen)
- Step 2 (keyboard input)
- pst-markout (already works)

---

## Step 4 — Multiple Windows

### What's needed
1. Each "app" is a `@parametric` block in a top-level desktop Markout document
2. The desktop document is rendered to the full framebuffer
3. Moving a window = editing a constraint (`gap-x:1rem:terminal`)
4. Focus management: keyboard events route to the focused block
5. Window list in a status bar

### Dependencies
- Steps 1-3
- pst-ipc for routing events between windows
- pst-sched for determining render order

---

## Step 5 — Persistence

### What's needed
1. NVMe or virtio-blk driver (PCI device, similar to VGA probe)
2. pst-vfs backed by disk blocks instead of memory
3. Save/restore desktop Markout document across reboots
4. pst-time history persisted to disk

### Dependencies
- Block device driver (new crate)
- pst-vfs already has the interface

---

## Step 6 — Network

### What's needed
1. Virtio-net driver (Privion already has partial driver-nic)
2. TCP/IP stack (minimal — or use smoltcp crate)
3. Fetch data from network, display in Markout
4. `{label:weather fetch:/api/weather}` — declarative data binding

### Dependencies
- NIC driver
- pst-ipc for routing network events

---

## Step 7 — Outconceive UI Convergence

### What's needed
1. Port the full Outconceive web runtime to render on the PST framebuffer
2. Rich text editor (`@editor`) running on bare metal
3. CSS-like theming applied to framebuffer rendering
4. The desktop IS an Outconceive instance

### The moment this works
The web framework and the OS are the same thing. One Markout document, one renderer, one constraint solver — from bare metal to browser and back.

---

## Architecture Summary

```
User types Markout
        │
        ▼
pst-markout parser
        │
        ▼
Parametric constraint solver (libpst)
        │
        ▼
VNode tree
        │
        ├──→ pst-framebuffer → VGA pixels (bare metal)
        ├──→ html::to_html → serial output
        └──→ Outconceive WASM → browser DOM (web)

Same code path. Different output target.
No Wayland. No X11. No display server.
```
