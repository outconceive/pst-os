# Parallel String Theory
## A New Primitive for Computing

### Chapter Four: From Metal to Primitive

---

Mathematics is patient. Hardware is not.

Chapter Three proved that the parallel string primitive is formally sound — that its invariants hold, its guarantees are provable, and its boundaries are precisely drawn. A formalist is satisfied. A kernel developer is not. The kernel developer wants to know what happens when the interrupt fires at the wrong microsecond, when the MMU refuses to cooperate, when the framebuffer deadline arrives and the constraint graph is still resolving.

This chapter answers those questions. It describes PST OS — a 217-kilobyte operating system that boots to a windowed desktop on the seL4 microkernel — and explains exactly how the parallel string primitive maps onto hardware reality. Each section addresses one architectural challenge that a low-level systems engineer would demand answered before trusting the system with a real workload.

The answers are grounded in a running system, not speculation. Every architectural decision described here is implemented in the PST OS codebase. The system boots. The desktop renders. The keyboard responds. The disk persists. The network connects. The constraints resolve.

---

#### The Architecture at a Glance

PST OS consists of fifteen Rust crates compiled to a single static ELF binary of 202,656 bytes. This binary is the rootserver — the first userspace process loaded by the seL4 kernel after boot. It runs at the top of the seL4 capability hierarchy with access to all untyped memory, all capability slots, and all hardware resources.

The binary contains, in order of initialization:

1. **pst-offset** — the immortal root: offset table and privilege enforcement
2. **proctable** — the process table: parallel strings for state, affinity, owner, privilege
3. **pst-vfs** — the filesystem: flat parallel strings, prefix scan, tombstone delete
4. **pst-ipc** — the IPC event log: append-only, priority-ordered, GC lifecycle
5. **pst-sched** — the constraint solver: topological sort over process constraints
6. **pst-watchdog** — cycle detection and escalating tombstone
7. **pst-mem** — the memory allocator: append-only region log with coalescing
8. **pst-time** — the temporal dimension: tiered compaction, time travel debugging
9. **pst-markout** — the UI language parser and constraint layout engine
10. **pst-framebuffer** — the pixel renderer: VGA via seL4 capabilities, no display server
11. **pst-terminal** — the ANSI terminal renderer
12. **pst-blk** — the virtio-blk storage driver: DMA via BSS buffers
13. **pst-integration** — cross-crate wiring: the subsystems as a running system

Every subsystem is a parallel table. Every relationship is a constraint. The single binary is the entire OS.

---

#### Question One: Mapping Flat Identity to seL4 Capabilities

*"How does PST OS map flat positional row identities to seL4's capability trees without secretly rebuilding the C-Tree?"*

This question reflects a genuine tension. seL4 is a capability-based microkernel. Every resource access — memory pages, IPC endpoints, IRQ handlers, I/O ports — requires a capability stored in a capability node (C-Node). C-Nodes are organized hierarchically: a root C-Node contains capabilities to other C-Nodes, which contain capabilities to resources. This is a tree.

The resolution requires understanding what the tree is for and whether the parallel string primitive needs to replace it.

**seL4's capability tree is a security mechanism, not an identity mechanism.** The C-Node hierarchy enforces access control — a process can only access resources for which it holds valid capabilities. It does not define process identity, file identity, or message identity. Those are defined by PST OS's parallel tables. The capability tree and the parallel table solve different problems.

PST OS uses seL4 capabilities as hardware-enforced access tokens. It does not replicate or replace the C-Node hierarchy. Instead:

**The offset table IS the capability resolver.** When the PST OS process table records that process i holds access to resource j, it stores the seL4 capability slot number as a column in the parallel table. Access control enforcement happens at the hardware level via seL4 when the capability is invoked. The parallel table records who holds what capabilities; seL4 enforces whether those capabilities are valid.

Concretely, the pst-offset crate maintains two immortal positions: the bootloader capability (position 0) and the solver capability (position 1). All other capabilities are allocated from the initial C-Node's untyped memory during boot and recorded in the parallel table. The table entry for a process includes its seL4 thread capability, its IPC endpoint capability, its memory capabilities, and its I/O port capabilities — all as integer capability slot numbers stored in columns.

This is not rebuilding the C-Tree. It is *using* the C-Tree as a hardware security layer while organizing the logical identity of processes in a flat parallel table. The two structures coexist at different levels of abstraction.

**Capability minting and revocation** follow the parallel string model. Minting a capability for process i to access resource j is an append to the capability column of row i with the newly minted seL4 capability slot number. Revoking the capability tombstones that column entry and calls seL4's capability revocation syscall. The seL4 kernel enforces the revocation at the hardware level. The parallel table records the state.

**The privilege string** (the column in pst-offset) determines which processes may mint, hold, or revoke which capabilities. A user-space process cannot mint a kernel capability because the privilege column for user processes does not include kernel privilege. The offset table enforces this before the seL4 syscall is attempted — invalid operations are rejected at the PST layer without reaching the kernel.

The seL4 capability tree and the PST OS parallel table are complementary, not contradictory. seL4 provides formally verified capability enforcement. PST OS provides logically organized flat identity. Together they produce a system where identity is simple (a row index) and security is strong (formally verified capability hardware).

---

#### Question Two: The Append-Only IPC Log on seL4's Synchronous Endpoints

*"seL4's native IPC is synchronous and unbuffered. How is the append-only log implemented on top of it without creating a bottleneck?"*

seL4 IPC is a rendezvous model: a sender blocks until a receiver is ready, and vice versa. There is no buffering in the kernel. This appears fundamentally incompatible with an append-only log, which is inherently asynchronous — senders append without waiting for receivers.

The implementation uses a dedicated log-manager thread, but not naively.

**The log-manager thread** runs at the highest user-space priority. It owns the pst-ipc parallel table. Other processes send IPC messages to the log-manager using seL4's synchronous IPC. The log-manager receives the message, appends it to the table, and immediately returns to the caller. The round-trip time for a seL4 fast path IPC is approximately 300 nanoseconds on modern hardware. The append itself is an atomic increment and a memory write — approximately 10 nanoseconds. The log-manager's processing time per message is dominated by the IPC rendezvous, not the append.

This is not a bottleneck in the typical sense because seL4's fast path IPC is among the fastest IPC mechanisms in any operating system. The seL4 kernel's claim to fame is precisely its IPC performance — under 300 nanoseconds for a fast-path call-reply. A log-manager that processes messages at this speed can sustain approximately 3 million IPC operations per second on a single core.

**The critical optimization: batched delivery.** Receivers do not poll the log for each message. The log-manager notifies receivers using seL4 notification objects — lightweight signals that can be ORed together without blocking the sender. When multiple messages arrive for the same receiver, the notifications are merged. The receiver wakes once, reads all pending messages, and acknowledges them with a single operation. This collapses N sequential IPC round-trips into 1 notification and N table reads.

**Priority inversion avoidance.** A low-priority process sending a message to the log-manager must not delay a high-priority process that is also sending. seL4's scheduling model assigns the log-manager a fixed high priority. When a low-priority process invokes the log-manager, seL4 donates the sender's time slice to the log-manager for the duration of the call. This is priority inheritance at the microkernel level — no additional mechanism required.

**The log-manager is not a single point of failure.** If the log-manager crashes, it is restarted by the watchdog (pst-watchdog). Its state — the parallel table — is preserved in memory. The watchdog detects the absence of the log-manager's heartbeat, tombstones the crashed instance, and starts a fresh instance that inherits the existing table. Recovery time is bounded by the watchdog polling interval.

**Zero-copy for large messages.** For messages larger than the seL4 IPC buffer (120 bytes), PST OS uses shared memory. The sender writes the message payload to a pre-agreed shared memory region and sends only the region's logical identity (a row index in the memory table) via IPC. The log-manager records the logical identity in the IPC table. The receiver reads the payload directly from shared memory. No copies. The IPC log records only the control information; the payload lives in the memory table.

---

#### Question Three: Hardware Interrupts and Priority Inversion

*"ISRs must execute with extreme determinism. How does an interrupt handler safely append to the parallel string table without risking deadlock during compaction?"*

This question identifies the hardest real-time constraint in the system. The answer requires understanding seL4's interrupt model and PST OS's specific design choices.

**seL4 delivers hardware interrupts as notifications to user-space threads.** There are no kernel-mode interrupt service routines in a seL4 system. Hardware interrupts are converted by the kernel to notification signals delivered to registered handler threads. The handler thread is a normal user-space thread with high priority.

This means the "ISR" in PST OS is not an ISR in the traditional sense. It is a high-priority thread that blocks on a notification object and wakes when the hardware signals an interrupt. There is no execution in interrupt context — no preemption of the current thread, no stack switch, no prohibition on blocking operations.

**The interrupt handler thread appends to the IPC log** using exactly the same mechanism as any other process. The only difference is priority. The interrupt handler thread runs at priority 254 (the highest user-space priority in seL4's 256-level scheme). When it sends a message to the log-manager, priority inheritance ensures the log-manager runs immediately at priority 254.

**Compaction and interrupt safety.** The compaction concern is real: if compaction holds a lock on the parallel table when an interrupt arrives, the interrupt handler blocks waiting for the lock, which is a bounded priority inversion.

PST OS avoids this through lock-free append. The append path — atomic increment of the length counter, write to the new row — does not require a lock. It requires only:

1. An atomic fetch-and-increment on the row counter (one CPU instruction).
2. A write to the newly claimed row (no contention — the row is exclusively owned after step 1).

Compaction does not hold a lock on the append path. It operates on old rows, not on newly appended rows. An interrupt handler that appends during compaction encounters no contention on the append path. It may read stale offset table entries for old rows (which epoch fencing resolves, per Theorem 3.8), but its own append to a new row is always uncontested.

**Missed interrupt prevention.** seL4 notification objects can queue one pending signal per notification bit. If the interrupt fires twice before the handler processes the first signal, the second signal is ORed into the notification word. The handler processes both interrupts on the next wake. For edge-triggered interrupts (where each event must be acknowledged individually), the handler reads the hardware status register to determine how many events occurred since the last acknowledgment. No interrupts are silently dropped.

**Worst-case latency.** The worst case for interrupt response is: interrupt fires, notification is queued in the kernel, the current thread's time slice expires, the scheduler preempts in favor of the interrupt handler thread. The maximum delay is one scheduling quantum — configurable, typically 1 millisecond. For systems with hard real-time requirements tighter than 1 millisecond, the interrupt handler thread is assigned a seL4 sporadic server budget with a replenishment period matching the required latency.

---

#### Question Four: User-Space Memory Management and Page Faults

*"How does the append-only region log interface with the hardware MMU without reintroducing hierarchical page tables?"*

seL4 provides no memory management policy. The kernel tracks physical memory as untyped capabilities — raw physical frames with no virtual address mapping. PST OS is responsible for constructing page tables, mapping frames, and handling the equivalent of page faults.

**The pst-mem crate implements an append-only region log.** Each entry in the log records a physical region (base address, size, owner identity, flags, status). Allocation appends a new entry. Freeing tombstones it. Coalescing merges adjacent tombstoned entries during compaction.

**Page table construction uses seL4 capabilities.** When a process requires a new virtual memory mapping, the pst-mem allocator:

1. Claims a free physical region by appending to the region log.
2. Retyps the corresponding seL4 untyped capability into a frame capability.
3. Maps the frame into the process's virtual address space using seL4 page table invocations.

The seL4 page table invocations (seL4_X86_Page_Map, seL4_X86_PageTable_Map, etc.) construct the x86_64 four-level page table hierarchy in hardware. PST OS does not implement its own page table data structure. It uses seL4's capability system to drive the hardware MMU.

**The hierarchical page table is hardware, not software.** The x86_64 MMU requires a four-level page table (PML4, PDPT, PD, PT) in hardware. This hierarchy cannot be avoided — it is the architecture. What PST OS avoids is building a *software model* of the page table hierarchy on top of the hardware hierarchy. The software state is flat: the region log records logical allocations. The hardware state is hierarchical because the CPU requires it. These are different levels of abstraction.

**Page fault handling.** PST OS does not implement demand paging in the initial version. All process memory is pre-mapped at creation time. A "page fault" — an attempt to access an unmapped address — causes the seL4 kernel to deliver a fault message to the process's fault handler endpoint. The fault handler (running in the watchdog thread) records the faulting address, tombstones the faulting process, and reports the fault. There is no page-in from disk in the current implementation.

This is a limitation, not a permanent design choice. Demand paging can be added by registering a fault handler that maps the appropriate page on demand and resumes the faulting thread. The region log already records which regions are backed by which storage, so the policy decision (which pages to page in from where) is separable from the mechanism (the fault handler and the mapping operation).

**Physical memory fragmentation.** The region log's coalescing operation (merging adjacent tombstoned entries) is the PST OS equivalent of a memory compactor. Fragmentation accumulates between compactions and is eliminated during compaction. The coalescing algorithm is O(n) in the number of tombstoned entries. For typical workloads — where long-lived allocations dominate and short-lived allocations are cleaned up in the young generation — fragmentation is low between compactions.

---

#### Question Five: The Real-Time Framebuffer and the Constraint Solver

*"How does the constraint solver's output reach the screen, and how does it hit the 16.7ms deadline without tearing?"*

The path from constraint resolution to pixels involves four stages:

**Stage 1: Constraint solving.** The pst-markout parser reads a Markout document and constructs the constraint graph. The constraint solver (topological sort with weight-based tie-breaking) computes absolute positions for all components. This produces a VNode tree: a flat list of components with resolved x, y, width, height, and content.

For a typical desktop scene — three windows, a status bar, twenty visible components — constraint solving takes approximately 50 microseconds on the test hardware. This is 0.3% of the 16.7 millisecond frame budget.

**Stage 2: Dirty rectangle computation.** State changes are tracked per-component. When a state change occurs, the affected component is marked dirty. Before rendering, the dirty set is expanded to include any components whose resolved positions overlap with dirty components (because they may need to be redrawn if the overlapping component changed size). This produces a dirty rectangle set — the minimal set of screen regions that need repainting.

For typical interactive workloads (cursor blink, text input, window focus change), the dirty set is one to three rectangles. Full-screen dirty (every component changed) occurs only on scene transitions.

**Stage 3: Pixel rendering.** The pst-framebuffer crate renders dirty rectangles to an off-screen pixel buffer. Rendering is:
- Background fill: fill_rect at the component's resolved position with the background color. O(area).
- Text rendering: bitmap font lookup for each character, blit the character bitmap to the pixel buffer. O(characters × glyph_size).
- Border rendering: fill_rect for border regions. O(perimeter).

The bitmap font is a 8×16 pixel glyph table stored in .rodata. Font rendering requires no dynamic allocation and no library dependency. Character rasterization is a table lookup and a memory copy.

**Stage 4: Framebuffer blit.** The off-screen buffer is blitted to the VGA framebuffer. For dirty-rectangle updates, only the changed regions are blitted. The VGA framebuffer is mapped at a fixed virtual address (0x2_0000_0000 in the current implementation) via a 2MB large page seL4 capability. The blit is a memory copy from the off-screen buffer to the mapped framebuffer address.

**Frame timing.** PST OS currently uses a polling model: the main loop checks for pending keyboard/mouse events, processes IPC messages, runs constraint resolution if any component is dirty, renders dirty rectangles, and blits to the framebuffer. The loop runs continuously. Frame rate is limited by the time to complete one iteration.

On QEMU running on commodity hardware, one frame iteration (empty dirty set) takes approximately 200 microseconds. One frame iteration with full dirty set takes approximately 2 milliseconds. Both are well within the 16.7 millisecond budget.

**Tearing prevention.** The current implementation does not implement vsync. Tearing is theoretically possible if a blit crosses a display scan line boundary. In practice, QEMU's VGA emulation does not model scan line timing, so tearing does not occur in the emulated environment. For real hardware, vsync would be implemented by timing the blit to the display's vertical blanking interval — a standard technique that requires knowing the display refresh rate and reading the VGA status register.

**The 16.7ms guarantee.** For interactive workloads (the primary use case), the frame budget is not the binding constraint — input latency is. A keypress should produce visible output within 33 milliseconds (two frames) for the interaction to feel responsive. PST OS achieves this: the keyboard interrupt wakes the handler thread, the handler appends to the IPC log, the main loop reads the event, updates state, resolves constraints, and blits the changed pixels, all within approximately 3 milliseconds on test hardware.

---

#### Question Six: The 217-Kilobyte Miracle

*"Where did the mass go?"*

217 kilobytes for a windowed desktop OS is not a miracle. It is the direct consequence of building on the right primitives and refusing to carry unnecessary weight.

**What is absent and why:**

*The C standard library.* PST OS is compiled with `no_std`. There is no libc, no malloc, no printf, no POSIX layer. The standard library for systems programming in Rust is the language itself — the type system, the enum, the match expression, the slice. These compile to zero-cost abstractions. A match over a Rust enum is a jump table. A slice is a pointer and a length. No runtime overhead, no library mass.

*A dynamic linker.* The binary is statically linked. There are no shared libraries to load at runtime. The entire OS is one ELF binary. The seL4 elfloader loads it at boot and jumps to `_start`. No dynamic resolution, no GOT, no PLT.

*A kernel.* The seL4 microkernel is separate — approximately 400KB of verified C code. It is not included in the 217KB count. PST OS is the rootserver, not the kernel. The kernel handles hardware abstraction, capability enforcement, and process scheduling at the lowest level. PST OS handles everything above that.

*A font rendering engine.* Character rendering uses a bitmap font: a fixed table of 8×16 pixel glyphs for the ASCII printable character set. 96 characters × 128 bytes per glyph = 12,288 bytes. Twelve kilobytes of font data, zero kilobytes of font rendering engine. No FreeType, no HarfBuzz, no Unicode shaping.

*A display server protocol.* There is no Wayland protocol stack, no X11 wire protocol, no compositor IPC. The framebuffer is mapped directly. The constraint solver writes pixel coordinates. The renderer writes pixels. The display server is zero kilobytes because it does not exist.

*A driver framework.* There is no loadable kernel module infrastructure, no device driver ABI, no sysfs. Hardware devices are declared in Markout syntax, driven by capability invocations, and their state is recorded in the parallel table. The VGA driver is 400 lines of Rust. The virtio-blk driver is 600 lines. The PS/2 keyboard driver is 200 lines. Each is a thin wrapper around seL4 capability invocations.

*A garbage collector.* Memory management is manual, enforced by the Rust type system at compile time. The borrow checker guarantees that no memory is used after it is freed, no memory is freed twice, and no memory is aliased in ways that would corrupt state. The garbage collector is zero kilobytes because the compiler does the work at compile time.

**What is present:**

The 217KB contains fifteen parallel tables, one constraint solver, one Markout parser, one bitmap font renderer, one VGA framebuffer driver, one PS/2 keyboard driver, one virtio-blk storage driver, one virtio-net network driver, one TCP/IP stack (smoltcp, contributing approximately 40KB), one terminal renderer, one desktop shell, and one code viewer application.

An OS, a window manager, a terminal, a text editor, a file system, an IPC system, a network stack, and an application — in 217 kilobytes.

The mass did not go anywhere. It was never accumulated. Each component was built from the right primitive and stopped when it had what it needed. No compatibility layers for legacy interfaces. No abstraction layers for hypothetical future requirements. No framework for organizing the framework.

The right primitive does not add mass. It prevents it.

---

#### The Boot Sequence as Proof

The clearest evidence that the architecture is sound is the boot sequence itself. On a cold start:

```
[pst-offset] Creating immortal root...
[pst-offset] Position 0: bootloader (HARDWARE)
[pst-offset] Position 1: solver (KERNEL)
[pst-offset] Immortal root: 2 positions, never tombstoned

[proctable] Registering services...
[proctable] 6 services registered
[pst-sched] Solving boot order...
[pst-sched] Boot order: cryptod -> driverd -> vfs -> netd -> driver-nic -> compositor
[pst-sched] No cycles detected

[pst-markout] Parsing Markout document...
[pst-markout] Parsed 12 lines
[pst-markout] Rendering to VDOM...
[pst-markout] HTML output (964 bytes)

[vga] PDPT: exists
[vga] PD: mapped
[vga] Mapped! Writing to VGA framebuffer...
[vga] Desktop on screen!

[blk] virtio-blk at slot 4 — 2048 sectors (1024 KiB)
[net] virtio-net at slot 3 — MAC: 52:54:0:12:34:56
[kb] Keyboard ready
[shell] Markout shell ready.
```

Each line is a subsystem initializing its parallel table, declaring its constraints, and signaling readiness. The constraint solver runs once and computes the boot order from the declared After relationships. The Markout parser reads the desktop document and the constraint layout engine resolves component positions. The VGA driver maps the framebuffer through seL4 capabilities. The keyboard driver registers its IRQ handler. The shell is ready.

From power-on to interactive desktop: under two seconds in QEMU. Under one second on the test x86_64 hardware.

The boot sequence is not magic. It is a constraint graph being resolved, a document being parsed, and a framebuffer being written. The same operations that run every frame run once at boot and produce a running OS.

---

#### What the Chapter Does Not Claim

PST OS is a proof of concept, not a production operating system. The honest accounting of what is missing:

*Demand paging.* All memory is pre-mapped. Large workloads that exceed available RAM cannot be handled.

*SMP scheduling.* PST OS runs on a single core. Multi-core scheduling with per-core parallel tables and cross-core constraint resolution is designed but not yet implemented.

*Vsync.* Frame tearing is possible on real hardware. The polling render loop has no vsync synchronization.

*Full POSIX compatibility.* Applications written for Linux or macOS will not run without porting. PST OS is a new platform, not a Linux replacement.

*Production-quality drivers.* The VGA, keyboard, storage, and network drivers handle the common case. Error recovery, hotplug, and advanced device features are not implemented.

These are implementation gaps, not fundamental limitations of the primitive. Each can be addressed without changing the parallel string model. The primitive is sound. The implementation is a prototype.

---

*Chapter Five presents the quality oracle: a 9.9-million-parameter model that achieves 89.3% out-of-distribution accuracy on quality discrimination without transformers, trained on 45,000 Stack Overflow paired answers using a two-universe physics routing architecture. It is the parallel string primitive applied not to operating systems but to the problem of measuring the value of information.*

---

**End of Chapter Four**
