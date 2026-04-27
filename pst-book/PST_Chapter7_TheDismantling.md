# Parallel String Theory
## A New Primitive for Computing

### Chapter Seven: The Dismantling

#### How One Primitive Removes Six Assumptions

---

Chapter Six named the six assumptions. It traced their origins, explained why they seemed correct when adopted, and measured what they cost. The accounting was honest: each assumption was reasonable at its moment of adoption. Each cost is real.

This chapter shows what happens when the assumptions are removed — not one at a time, as an incremental improvement to an existing architecture, but all at once, as the consequence of choosing a different primitive from the foundation.

The removal is not theoretical. It is demonstrated by a system that exists and runs: PST OS, 217 kilobytes, booting to a windowed desktop on a formally verified microkernel. Each section of this chapter connects one dismantled assumption to a specific, measurable outcome in that running system.

---

#### Dismantling Assumption One: The Tree is Replaced by Position

**The assumption removed:** Complex systems must be organized hierarchically. Processes have parents. Files live in directories. Components are nested in containers. The tree is the natural structure.

**The mechanism:** Position replaces hierarchy. Every entity is a row in a flat parallel table. Its identity is its row index. Its relationships are declared as constraints in a separate column, resolved by a topological sort.

The process table in PST OS has no parent-child links. A process is not a child of another process in any structural sense. It is a row with columns: state, priority, affinity, owner, privilege. Its relationship to other processes — the fact that `vfs` must start after `cryptod`, that `compositor` must start after both `vfs` and `netd` — is a constraint, not a structural pointer.

The filesystem has no directory tree. A file is a row with a path string. The directory hierarchy that users perceive when they list `/home/user/documents` is a convention imposed by the path naming scheme, not a structural property of the filesystem. Listing a directory is a prefix scan over the path string column — find all rows where the path column begins with `/home/user/documents/`. The directory does not exist as a node. It exists as a shared prefix.

The UI has no component tree. A button is a row at a specific column index in the Markout parallel strings. Its spatial relationship to the input field next to it — centered vertically, eight pixels to the right — is a constraint. The constraint is resolved by the parametric layout solver to produce absolute pixel coordinates. There is no parent container that the button is "inside." There is the button's position, and the declared constraints on that position.

**The empirical result:** The PST OS process table is 47 lines of Rust. It declares a parallel table, an append operation, a tombstone operation, and a constraint solver invocation. The entire process management subsystem is 47 lines because there are no parent-child links to maintain, no tree rotations to perform, no ancestry to traverse.

The Linux `task_struct` is over 700 lines of C, most of it managing the complexity of the process tree: parent pointers, child lists, sibling pointers, process group membership, session membership, namespace trees. This complexity does not exist in PST OS because the tree does not exist.

The virtual DOM in a typical React application adds approximately 40 kilobytes of JavaScript to the bundle for the diffing and reconciliation algorithms. It adds hundreds of kilobytes more for the component tree management. PST OS has no virtual DOM because there is no component tree to reconcile. The Markout renderer is 300 lines. It reads the parallel strings, runs the constraint solver, and writes pixel coordinates. No diffing. No reconciliation. No tree.

---

#### Dismantling Assumption Two: Identity Becomes Positional

**The assumption removed:** Identity is referential — tied to location in memory or position in a graph. Finding an entity requires following a reference chain. Moving an entity requires updating all references.

**The mechanism:** Identity is positional and immutable. A row index in the parallel table is assigned at creation and never changes. The offset table maps logical identity (row index) to physical memory location in O(1). Compaction can move the physical memory representation without changing the logical identity — the offset table entry for the row is updated, and all code that holds the logical identity continues to work correctly.

In PST OS, the identity of a process is its row index in the process table. When the process table is compacted — dead rows removed, surviving rows packed together — the logical identities do not change. Code that holds process identity 47 continues to reference the same process after compaction, because the offset table still maps 47 to wherever that process's data now lives in physical memory.

The identity of a file is its row index in the virtual filesystem. Renaming a file changes the path string column at that row. The row index does not change. Any other subsystem that holds the file's logical identity — an open file handle, a pending I/O operation, a cached directory listing — continues to reference the correct file through the offset table.

The identity of a UI component in a Markout document is its character offset in the source — the column index across the parallel strings. The component at offset 47 is that component regardless of what state changes have occurred, regardless of whether surrounding components have been inserted or removed (which would produce new rows, not reorder existing ones).

**The empirical result:** Pointer bugs — use-after-free, dangling references, null pointer dereferences, double frees — are a class of errors that emerges when identity is referential and identity can become invalid. They account for approximately 70% of the security vulnerabilities in memory-unsafe C and C++ codebases according to Microsoft's analysis of their codebase.

In PST OS, this class of errors does not exist. Logical identity never becomes invalid — an identity exists until the row is tombstoned, and tombstoned rows are never dereferenced (the offset table returns ⊥ for tombstoned rows, which the calling code handles as "entity not found"). The Rust type system enforces that offset table lookups check for tombstone before proceeding.

There is no garbage collector in PST OS. There is no reference counting. There is no lifetime tracking beyond what the Rust borrow checker enforces at compile time. Memory management is explicit, correct by construction, and zero-overhead because the identity model does not produce the aliasing and dangling reference problems that garbage collectors exist to solve.

---

#### Dismantling Assumption Three: State Requires No Reconciliation

**The assumption removed:** When state changes, the system must construct a new representation and diff it against the old representation to determine what changed. This is the virtual DOM, the state management cycle, the transaction log. Diffing is how the system knows what to update.

**The mechanism:** State is positional. Each UI component occupies a known row index in the parallel table. Its state is a known column at that row. When state changes, the cell at (row, column) has a new value. What changed is directly observable — no diffing required. The renderer marks that row as dirty and redraws only the components at dirty rows.

In PST OS, the keyboard handler appends a keypress event to the IPC log. The main loop reads the event, identifies the focused component (the component whose focus column is true), and updates the value column at that row. The renderer marks that row as dirty. At the next frame, the renderer redraws only the dirty components — the focused text input — and blits only the dirty rectangles to the framebuffer.

No virtual DOM is constructed. No diffing algorithm runs. No tree is traversed to propagate the state change to children. The change is a write to a specific cell in a flat table. The consequence is a mark on a known row. The rendering is a redraw of the marked rows.

**The empirical result:** The Markout renderer in PST OS is 300 lines of Rust. It has no diffing logic because there is nothing to diff. The state is flat and positional. Changes are local. Rendering is incremental by construction.

React's reconciliation algorithm — the engine that diffs virtual DOM trees — is approximately 15,000 lines of JavaScript in the React core package. It handles hundreds of edge cases: keys for list reconciliation, fiber scheduling for interruptible rendering, priority lanes for different update types, effects for post-render side effects.

All 15,000 lines exist to manage the complexity of diffing a tree of mutable JavaScript objects. The complexity is not intrinsic to the problem of rendering user interfaces. It is intrinsic to the data model that React inherited — mutable component trees with referential identity. Change the data model to flat positional tables and the reconciliation algorithm is not needed, not simplified — not needed.

---

#### Dismantling Assumption Four: Encapsulation Becomes Dimensional

**The assumption removed:** All attributes of an entity must be grouped together in memory. A process object holds its state, its PID, its priority, and its owner together. Encapsulation by proximity is good engineering.

**The mechanism:** Data is organized by dimension, not by entity. All state values are in the state column. All priority values are in the priority column. All affinity values are in the affinity column. The scheduler scans the state column to find runnable processes — a sequential scan of one array, cache-friendly, with no wasted bandwidth loading irrelevant fields.

In PST OS, the process table is four parallel arrays: state, affinity, owner, privilege. Each array is a contiguous block of memory. Scanning for runnable processes scans the state array from index 0 to n. The CPU loads cache lines containing state values, not cache lines containing mixed state, affinity, owner, and privilege values. The cache is used efficiently.

The parametric layout solver scans the constraints column to find components with pending constraints. The scan is over one array. The renderer scans the dirty column to find components that need repainting. The scan is over one array. Every system-level operation that processes many entities for one attribute accesses one array sequentially.

**The empirical result:** The Linux `task_struct` is approximately 9,500 bytes per process on a 64-bit system. A system with 1,000 processes holds approximately 9.5 megabytes of task structures in memory. Scanning all 1,000 processes to find runnable ones requires loading 1,000 cache lines (one per task_struct) to access one byte of state from each.

The PST OS process table stores state in a separate array. Scanning 1,000 processes for runnable state loads approximately 16 cache lines (1,000 bytes of state, 64 bytes per cache line). The ratio is roughly 60:1 — PST OS loads 60 times less data to answer the same query.

This is not a micro-optimization. It is a structural consequence of the data model. Column-oriented databases achieve orders-of-magnitude performance improvements over row-oriented databases for aggregate queries for the same reason. The cache efficiency of flat columnar storage is not incidental; it is the correct layout for the access patterns of systems software.

---

#### Dismantling Assumption Five: The Primitive Unifies Domains

**The assumption removed:** Different domains require different primitives. Schedulers need red-black trees. Layout engines need constraint solvers. Filesystems need inode hierarchies. Databases need B-trees. Domain expertise requires domain-specific data structures.

**The mechanism:** The constraint vocabulary changes. The primitive does not.

The OS scheduler is a constraint solver over temporal relationships. Processes declare `After` constraints — "I must start after process X." The topological sort resolves these constraints into an execution order. The data structure is a parallel table. The solver is a topological sort.

The parametric layout engine is a constraint solver over spatial relationships. Components declare spatial constraints — "I am centered horizontally relative to component X, with a gap of 8 pixels above." The topological sort resolves these constraints into pixel coordinates. The data structure is a parallel table. The solver is a topological sort.

The filesystem query is a constraint solver over prefix relationships. Files are rows with path strings. A directory query declares a prefix constraint — "return all files whose path begins with X." The prefix scan resolves this constraint into a set of matching rows. The data structure is a parallel table. The scan is a linear filter.

The IPC event log is a constraint solver over priority relationships. Messages are rows with priority values. Delivery is ordered by priority — "deliver the message with the highest priority value first." The priority sort resolves this constraint into a delivery order. The data structure is a parallel table. The sort is a standard sort.

Four domains. Four constraint vocabularies. One primitive.

**The empirical result:** The PST OS codebase shares the topological sort implementation across the scheduler, the layout engine, and the dependency resolver. One implementation. One test suite. One correctness surface area. When a bug is found in the topological sort, it is fixed once and the fix applies to all three domains.

The Linux kernel contains separate implementations of red-black trees (for the process scheduler), directory entry caches (for the filesystem), memory mapping trees (for virtual memory), and skip lists (for routing). Each implementation has its own bugs, its own maintenance burden, and its own conceptual model. They share no code because they are based on different primitives.

PST OS shares code across domains because it is based on one primitive. The 217-kilobyte binary size is partly explained by this sharing: there is one constraint solver, not six; one table append operation, not six; one tombstone mechanism, not six.

---

#### Dismantling Assumption Six: Concurrency Without Locks

**The assumption removed:** Concurrent access to shared data structures requires locks. Without locks, concurrent mutations produce race conditions and undefined behavior.

**The mechanism:** Append-only mutation removes shared mutable state. The append operation — atomic fetch-and-increment on the row counter — gives each thread a unique row index. Writing to that row is private. No other thread writes to the same row. There is no shared mutable state to protect.

In PST OS, the IPC log-manager appends messages using atomic row index allocation. The keyboard handler thread appends keyboard events. The network driver appends received packets. The disk driver appends I/O completions. All of these threads append to the same parallel table simultaneously. None of them acquires a lock on the table. None of them can race with another.

The only shared state is the row counter, and the shared state is protected by a single atomic increment instruction. The CPU architecture guarantees that atomic increments are linearizable — each increment is an instantaneous event that all threads observe in the same order. This is not a lock; it is a hardware primitive that costs one CPU instruction.

Tombstoning is similarly lock-free. Writing a tombstone marker to a specific row is a write to a uniquely owned position (the row belongs to its logical identity, and no other thread writes to the same row after the tombstone). The write is atomic at the word level, which is guaranteed by hardware for aligned word-sized writes.

The constraint solver is single-threaded — it runs on a dedicated solver thread that drains the constraint log and produces resolution orders. Other threads do not call the solver concurrently; they append constraints to the log and read results from the resolved output. The solver thread is the only consumer of the constraint log. There is no contention on the solver.

**The empirical result:** PST OS has no mutexes. It has no semaphores. It has no reader-writer locks. It has no lock-free algorithms beyond the atomic row counter increment. It has no deadlock detection. It has no priority inheritance protocols (beyond those provided by seL4's scheduling model for IPC).

The Linux kernel has thousands of locks. The kernel lock validator (lockdep) tracks lock acquisition order to detect potential deadlocks and is itself a substantial piece of code. The documentation for kernel locking fills entire chapters of the kernel developer guide. Lock-related bugs are a constant source of kernel panics in production systems.

PST OS eliminates this entire category of bugs and the entire category of tooling required to manage them, not by writing more careful locking code, but by not having shared mutable state to lock.

---

#### The Compound Effect

The six dismantlings are not additive. They are multiplicative.

Removing the tree also removes much of the need for reference management, because tree node pointers are the primary form of referential identity in most systems. Removing referential identity also removes most of the need for reconciliation, because diffing is primarily needed to detect changes in referentially-structured state. Removing reconciliation also reduces the encapsulation pressure, because row-oriented encapsulation was partly motivated by keeping together the data that diffing needed to compare. Reducing encapsulation pressure enables domain unification, because columnar storage is efficient across all domains with scan-dominated workloads. Domain unification reduces the implementation surface area to lock, and append-only mutation eliminates most of the remaining locking needs.

Each removal weakens the others. All six together produce a system that is not incrementally better than a conventional OS — it is structurally different.

The 217-kilobyte binary is the empirical measure of this structural difference. Modern operating systems are tens of millions of lines of code. The additional lines are not mostly implementing features that PST OS lacks. They are mostly managing the consequences of the six assumptions: the tree management code, the reference tracking code, the reconciliation algorithms, the domain-specific data structure implementations, the locking infrastructure.

Remove the assumptions. The code to manage their consequences disappears. What remains is the irreducible core: the business logic of what an operating system actually needs to do.

---

#### What Was Not Removed

Honesty requires accounting for what PST OS does not have that conventional operating systems do.

*Demand paging* — PST OS does not implement virtual memory paging from disk. All memory is pre-allocated. This is a missing feature, not a consequence of the primitive.

*POSIX compatibility* — PST OS does not implement the POSIX API. Applications written for Linux will not run without porting. This is a deliberate choice, not a limitation of the primitive.

*Driver diversity* — PST OS supports VGA, PS/2 keyboard, virtio-blk, and virtio-net. It does not support USB, Thunderbolt, PCIe enumeration beyond basic device discovery, or the wide range of device classes that a general-purpose OS must support. More drivers are a matter of engineering time, not architectural limitation.

*SMP* — PST OS runs on one core. Multi-core scheduling with per-core tables and cross-core constraint resolution is designed but not implemented.

These are gaps in the implementation of the primitive, not evidence that the primitive is insufficient. The primitive is sufficient. The implementation is a prototype.

The absence of the six assumptions explains the small binary. The absence of demand paging, POSIX, full driver support, and SMP explains what a production version of this OS would need to add. They are different categories of absence.

---

#### The Root of Simplicity

There is a way to state the result of this chapter in a single sentence.

The vast majority of the code in modern operating systems and web frameworks exists to manage the unintended consequences of six assumptions that nobody proved were necessary.

Remove the assumptions. Manage fewer consequences. Write less code. Build a smaller, faster, clearer system.

This is not magic. It is not cleverness. It is the straightforward consequence of finding a better primitive.

The tree was never necessary.

The pointer was never the only form of identity.

The diff was never the only way to detect change.

The object was never the only unit of encapsulation.

The domain was never as distinct as it appeared.

The lock was never as necessary as it seemed.

These were choices. They were not bad choices at the moment of adoption. They were reasonable responses to real constraints. But the constraints changed, the systems scaled, the costs accumulated, and the choices were never revisited.

PST revisited them.

The result is 217 kilobytes.

---

*Chapter Eight presents the future: multimodal quality detection, distributed parallel string systems, the Outconceive application platform, and the open research questions that the theory leaves unanswered. It also presents the falsification criteria — the experiments that would disprove the theory's central claim — and invites the reader to conduct them.*

---

**End of Chapter Seven**
