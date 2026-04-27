# Parallel String Theory
## A New Primitive for Computing

### Chapter Six: The Six Assumptions

#### How Forty Years of Engineering Became Invisible Law

---

Every field has its load-bearing assumptions. Physics assumed space and time were separate until Einstein showed they were the same thing. Biology assumed species were fixed until Darwin showed they were not. Medicine assumed ulcers were caused by stress until Barry Marshall drank a petri dish of *Helicobacter pylori* and proved they were caused by bacteria.

In each case, the assumption was not held because it was proven. It was held because it worked well enough for long enough that it stopped being questioned. It became infrastructure. It became intuition. It became, in the minds of the people who built on top of it, indistinguishable from physical law.

Computing has its own version of this phenomenon. Over the past forty years, six assumptions have been baked so deeply into the foundations of software engineering that they are no longer visible as assumptions. They appear in textbooks as facts. They appear in job interviews as knowledge. They appear in architecture meetings as constraints. They are taught to every computer science student in every university as the way things are.

They are not the way things are. They are the way things were decided to be, in a specific historical moment, by specific people solving specific problems with the tools available to them. Each decision was reasonable. Each decision calcified. And the accumulated weight of those six calcified decisions is the reason modern operating systems are tens of millions of lines of code, modern web frameworks ship hundreds of megabytes of dependencies, and a button on a webpage requires a virtual DOM, a reconciliation algorithm, a JavaScript runtime, a browser engine, and a GPU compositor to display four pixels of text.

This chapter names the six assumptions. It traces their origins, explains why they seemed right at the time, and shows what they cost. The next chapter shows what computing looks like when they are removed.

---

#### Assumption One: The Primacy of the Tree

**The stated belief:** The natural and most efficient way to model complex systems is through nesting — parent-child relationships. Processes must have parents. Files must live inside folders. UI components must be nested inside containers. Hierarchical structure is not a choice; it is the correct reflection of how complex systems are organized.

**The historical origin:** Hierarchical filesystems emerged in the 1960s with Multics and were inherited by Unix in 1969. The design was a practical response to two real constraints: limited memory made it expensive to store full path strings for every file, and hardware of the era performed sequential operations efficiently. A tree structure allowed partial paths to be followed incrementally, reducing memory usage and allowing the filesystem to be navigated with the limited I/O of the time.

Process trees emerged from the same era for similar reasons. The fork/exec model, where every process is the child of another process, was a convenient mechanism for inheriting open file descriptors, signal handlers, and environment variables. The tree was not a philosophical statement about the nature of processes. It was a convenience that hardened into convention.

UI component trees arrived later, in the 1990s, when graphical interfaces began to be built programmatically. The natural way to describe a window containing panels containing buttons in code was nesting — a tree of constructor calls reflecting the visual hierarchy. React's component tree, introduced in 2013, made this pattern explicit and gave it a name, but did not invent it. It inherited it from every widget toolkit that came before.

**What the assumption costs:** Tree data structures impose three classes of overhead that accumulate with scale.

*Pointer chasing* — navigating a tree from root to node requires following a chain of pointers whose length scales with tree depth. Each pointer dereference is a potential CPU cache miss. Each cache miss stalls the processor pipeline for hundreds of cycles. A deep directory path, a deeply nested component tree, a complex process ancestry — each requires multiple cache-unfriendly memory accesses that a flat structure would not.

*Rebalancing* — keeping trees balanced requires restructuring operations that cascade through the hierarchy. Inserting a node into a red-black tree requires rotations. Adding a file to a full directory requires inode allocation. Mounting a component requires reconciling it with its parent's constraint. The cost of maintaining hierarchical structure is paid continuously, on every mutation.

*Cascading updates* — when a node changes, everything that depends on it must be notified. In React, a state change in a parent component triggers re-rendering of all descendants by default. In an OS, killing a process sends SIGHUP to all its children. In a filesystem, renaming a directory invalidates all cached paths below it. The tree structure creates propagation paths for side effects that flat structures do not have.

**The assumption unexamined:** Nobody proved that processes need parents. Nobody proved that files need directories. Nobody proved that UI components need to be nested. These structures were adopted because they were useful in the 1960s and 1970s for hardware that no longer exists, for memory constraints that no longer apply, and for I/O characteristics that no longer dominate. They were never the only answer. They were the first answer.

---

#### Assumption Two: Identity is Referential

**The stated belief:** An entity's identity is tied to its location — its memory address, its path within a graph, its position in a namespace. To find something, you follow the reference chain to where it lives. If it moves, all references must be updated.

**The historical origin:** In the earliest stored-program computers, identity and location were genuinely inseparable. A variable was its memory address. A function was its entry point. A file was its disk block number. There was no conceptual separation between "where something is" and "what something is" because programs were small enough that everything fit in a programmer's head and nothing needed to move.

As systems grew, identity and location began to diverge, but the referential model was retained and elaborated rather than replaced. Virtual memory added a layer of indirection — virtual addresses mapped to physical addresses — but kept the fundamental model: an entity's identity is derived from its location, and finding it requires following a chain. Garbage collection added another layer to manage the case where referential identity becomes invalid — a pointer to freed memory is undefined behavior, so garbage collectors track all live pointers and move objects by updating every reference simultaneously.

The pointer abstraction in C made referential identity explicit and pervasive. Every data structure in systems programming is a web of pointers. A linked list is a chain of next-pointers. A tree is a graph of parent and child pointers. A hash table resolves collisions with pointer chains. The pointer is not just a mechanism; it is the fundamental model of identity.

**What the assumption costs:** The referential identity model produces three categories of cost.

*Indirection overhead* — following a reference chain to find an entity requires one memory access per level of indirection. A five-level directory path requires five inode lookups. A deeply nested component requires traversing the parent chain. A pointer graph with complex topology requires walking potentially many nodes to reach the target. Each access is a potential cache miss.

*Reference management complexity* — when an entity moves, every reference to it must be updated. This is why garbage collectors exist: to track all live references and update them when objects are relocated. It is why reference counting is complex: circular references cause leaks. It is why C++ has move semantics, destructors, and RAII: because the programmer must manually manage the moment when a referenced object's identity becomes invalid. Billions of lines of code exist to manage the consequences of identity being tied to location.

*The aliasing problem* — when multiple references point to the same entity, mutating the entity through one reference affects all other references. This is the fundamental source of concurrency bugs, data races, and the necessity of locks. When identity is referential, shared mutable state is inevitable. When identity is positional and append-only, the aliasing problem does not arise — each new state is a new position, not a mutation of an existing location.

**The assumption unexamined:** Identity does not require location. A person's identity is not their address — they remain the same person when they move. A document's identity is not its filename — it remains the same document when it is renamed. The conflation of identity with location is a historical artifact of the earliest computers, where the distinction did not matter because programs were small enough that nothing needed to move. As systems scaled, the conflation became a source of enormous complexity. It was never examined because it was never questioned.

---

#### Assumption Three: State Requires Reconciliation

**The stated belief:** When state changes in a complex system, the safest and most general way to update the representation is to construct a new version of the entire structure, compare it to the old version (diffing), and apply the minimum set of changes. This is the virtual DOM, the state management Redux cycle, the database transaction log. Diffing is how you know what changed.

**The historical origin:** The virtual DOM was introduced by React in 2013 as a solution to a specific problem: the browser DOM is slow to mutate, and arbitrary JavaScript could mutate it in arbitrary ways, making it difficult to predict or optimize. React's insight was to make state changes explicit — describe what the UI should look like, compute the difference from what it currently looks like, apply only the necessary mutations to the DOM. This was genuinely clever for the browser environment where direct DOM access was the only interface.

The diffing approach was not invented by React. Unix diff was introduced in 1974. Database write-ahead logs use before-images and after-images to enable rollback. Version control systems track diffs between snapshots. The pattern of "compare new state to old state, apply changes" is pervasive because it is general: it works for any state, regardless of structure.

**What the assumption costs:** Reconciliation-based state management imposes three categories of overhead.

*Allocation cost* — diffing requires constructing a complete representation of the new state before comparing it to the old state. In React, every state change produces a new virtual DOM tree — a tree of JavaScript objects representing the entire component hierarchy. This allocation cost is paid on every state change, regardless of how small the change is. A single character typed in a text field produces a new virtual DOM for the entire application.

*Comparison cost* — diffing the new tree against the old tree requires traversing both trees simultaneously, comparing each node. The cost is O(n) in the size of the tree for naive diffing, and O(n) even for optimized heuristic algorithms because the algorithm must visit every node to determine which ones changed. For large UI trees, this comparison is the bottleneck that limits React's performance.

*Conceptual cost* — the diffing model introduces a fundamental confusion between state and representation. The virtual DOM is not the state. It is a representation of the state, constructed for the purpose of comparison. The actual state lives elsewhere — in JavaScript variables, in Redux stores, in component hooks. The virtual DOM is a shadow that exists only to be diffed. Maintaining this shadow, keeping it consistent with actual state, and ensuring that the diff is correctly computed are sources of bugs that would not exist if the representation were directly mutable in well-defined ways.

**The assumption unexamined:** Diffing is necessary when the data structure does not record what changed. If a UI component's state is a string in a JavaScript object, and the JavaScript object can be mutated by any code in any way, then diffing is the only reliable way to determine what changed. But this is a consequence of the data model, not a universal truth. If state is positional — if each component has a known row index and its state is a column at that row — then what changed is directly observable: the column at the known row has a new value. Diffing is a solution to the problem that mutable referential state creates. It is not necessary when the data model does not create that problem.

---

#### Assumption Four: Encapsulation via Proximity

**The stated belief:** All attributes belonging to an entity must be grouped together in memory. A `Process` object should contain its state, its PID, its priority, its owner, and its file descriptor table in one contiguous block. This is encapsulation: the entity and all its properties live together. Object-oriented programming enshrined this principle as a design virtue.

**The historical origin:** The encapsulation principle emerged with Simula in the 1960s and was formalized by Smalltalk in the 1970s. Its original motivation was conceptual clarity: an object should own its data and the methods that operate on it. Grouping related data together in memory was a natural reflection of this conceptual ownership. When objects were small — a Point with x and y coordinates, a Window with width and height — the row-oriented layout was efficient because accessing all attributes of one object required reading one contiguous block.

C structs encode the same principle without the object-oriented philosophy: related fields are declared together and stored contiguously. The Linux kernel's `task_struct` — the C structure representing a process — is a famously large struct containing over a hundred fields, all grouped together for every process. This made sense when processes were few and their attributes were frequently accessed together.

**What the assumption costs:** Row-oriented storage (grouping all attributes of one entity together) optimizes for one access pattern: reading all attributes of a specific entity. It is hostile to a different access pattern: reading one attribute across many entities. Systems programming is dominated by the second pattern.

*The scheduler scans* — to find runnable processes, the scheduler must examine the state field of every process. In a row-oriented layout, each process's state field is separated from the next process's state field by all the other fields in `task_struct` — potentially hundreds of bytes. Scanning for runnable processes requires loading one cache line per process, even though only a few bytes per process (the state field) are relevant. The cache lines loaded for the irrelevant fields are wasted bandwidth.

*The renderer scans* — to determine which UI components are visible, the renderer must examine the visibility property of every component. In a row-oriented layout, visibility is interleaved with all other component properties. The scan loads irrelevant data.

*The database scans* — to compute an aggregate over one column of a large table, a row-oriented database must read every row in full. This is why column-oriented databases (C-Store, Vertica, Apache Parquet) were invented: they store each column separately, so column-aggregate queries load only the relevant column. Column-oriented storage is not a database trick. It is the correct layout for workloads dominated by column scans.

**The assumption unexamined:** The assumption that attributes should be grouped by entity is correct for workloads dominated by single-entity access. It is wrong for workloads dominated by attribute scans across many entities. Systems programming, OS kernel operations, and UI rendering are all dominated by attribute scans. The encapsulation principle was adopted for conceptual reasons and retained without examining whether it optimized the actual access patterns of the systems it was applied to.

---

#### Assumption Five: Domain-Specific Primitives

**The stated belief:** Different domains require fundamentally different data structures. Operating system schedulers require red-black trees for priority queues. Spatial layout engines require R-trees for rectangular region lookup. Filesystems require inode trees for directory traversal. Databases require B-trees for sorted index access. Each domain has its own primitive, and using the wrong primitive for a domain is incompetent engineering.

**The historical origin:** Domain-specific primitives emerged as each domain's practitioners optimized for their specific workloads. Red-black trees provide O(log n) insert, delete, and search with guaranteed worst-case bounds — ideal for a scheduler that must find the highest-priority runnable process. B-trees provide efficient disk-based sorted access — ideal for database indexes where the bottleneck is I/O. R-trees provide efficient spatial range queries — ideal for geographic databases.

Each primitive was developed by experts who understood their domain deeply and designed data structures specifically for its access patterns. The expertise is genuine. The primitives are well-engineered. They are taught in data structures courses as canonical solutions to canonical problems.

**What the assumption costs:** Domain-specific primitives impose two categories of cost.

*Implementation cost* — each new domain requires implementing, testing, and maintaining a new data structure. A red-black tree implementation is hundreds of lines of subtle code with numerous edge cases. An R-tree is more complex. A B-tree more complex still. Every system that uses these primitives carries the implementation complexity of each primitive it uses. The Linux kernel implements dozens of distinct data structures, each with its own correctness surface area.

*Integration cost* — when data flows between domains, it must be converted between their incompatible primitives. A process table entry in a red-black tree must be accessed differently than a file in an inode tree, which must be accessed differently than a UI component in a DOM tree. The interfaces between subsystems are conversion layers — each converting between the primitives of two adjacent domains.

**The assumption unexamined:** The scheduling problem is: given a set of processes with declared dependencies, determine the execution order. The layout problem is: given a set of UI components with declared spatial relationships, determine the pixel positions. The filesystem problem is: given a set of files with declared path prefixes, determine which files match a query. These are all the same problem: resolve a set of declared relationships over a set of entities with positional identity. The constraint vocabulary changes — temporal constraints for scheduling, spatial constraints for layout, prefix constraints for filesystem queries — but the structure of the problem is identical. Domain-specific primitives exist because each domain's practitioners optimized for their domain without noticing that neighboring domains had the same problem.

---

#### Assumption Six: Concurrency Requires Locks on Mutable State

**The stated belief:** To safely update a shared data structure in a multithreaded environment, you must acquire exclusive access to the nodes being updated. Without locks, concurrent mutations produce race conditions — undefined behavior where the result depends on the unpredictable interleaving of operations between threads.

**The historical origin:** The locking model for concurrency emerged with the first multithreaded operating systems in the 1960s and 1970s. Dijkstra introduced the semaphore in 1965 as a mechanism for mutual exclusion. Hoare introduced monitors in 1974 as a higher-level abstraction. The model was: identify the shared data that concurrent threads must access, protect it with a lock, ensure that only one thread accesses it at a time.

The model was correct for the data structures of the era: linked lists, trees, hash tables — all mutable, all requiring structural consistency across multiple fields simultaneously. Inserting a node into a linked list requires updating two pointers atomically. Inserting into a tree may require rebalancing multiple nodes. Without a lock, concurrent insertions can produce a corrupted structure.

Lock-free algorithms were developed as an alternative, but they are notoriously difficult to design correctly and are limited in the operations they can efficiently support. The reader-writer lock was developed to allow concurrent reads while serializing writes. Read-copy-update (RCU) was developed for Linux to allow reads to proceed without locks at the cost of deferred reclamation. Each advancement in concurrent data structure design produced more complex mechanisms to manage the fundamental problem: multiple threads sharing mutable state.

**What the assumption costs:** Lock-based concurrency produces three categories of cost.

*Deadlock risk* — when multiple locks must be held simultaneously, the order in which they are acquired must be consistent across all code paths. Violating this ordering produces deadlock. Ensuring consistent ordering across a large codebase is difficult, and deadlocks in production systems are notoriously difficult to reproduce and diagnose.

*Contention cost* — when multiple threads compete for the same lock, all but one must wait. Under high contention, threads spend more time waiting than working. Lock contention is the primary scalability bottleneck in multithreaded systems, and eliminating it is the motivation for most concurrent data structure research.

*Complexity cost* — reasoning about the correctness of a concurrent system with locks requires understanding all possible interleavings of all threads. For non-trivial systems, this state space is enormous. Concurrency bugs are among the most difficult to find and fix precisely because they depend on timing conditions that are rare in testing and common in production.

**The assumption unexamined:** Locking is necessary when multiple threads share mutable state. It is not necessary when state is append-only. An atomic fetch-and-increment on a row counter gives each thread its own uniquely owned row index. Writing to that row index is private — no other thread will write to the same position. The written value becomes visible to other threads immediately (by memory ordering guarantees on modern architectures) without requiring a lock. Append-only mutation does not eliminate concurrency, but it eliminates the shared mutable state that makes locking necessary.

---

#### The Weight of Six Assumptions

The six assumptions are not independent. They reinforce each other in ways that compound their collective cost.

Trees produce referential identity — the path to a node in the tree is its identity. Referential identity requires reconciliation — when references become stale, the system must diff old state against new to determine what changed. Reconciliation requires encapsulation by proximity — diffing works by comparing objects, and objects must group their fields together. Encapsulation by proximity prevents domain unification — each domain groups its objects differently, making a shared primitive impractical. Domain-specific primitives require mutable state — each domain's data structure is mutated in domain-specific ways. Mutable state requires locks — and the cycle is complete.

Remove one assumption and the others are weakened. Remove all six and the resulting system does not merely improve on the status quo — it becomes a fundamentally different kind of system. Not faster. Not smaller. Different in kind.

The 217-kilobyte PST OS is not a smaller version of a conventional OS. It is what an OS looks like when none of the six assumptions have been made.

The next chapter shows what that looks like in practice.

---

*Chapter Seven demonstrates how Parallel String Theory actively dismantles each of the six assumptions, showing the specific mechanisms by which the primitive eliminates the costs described in this chapter, and presenting the empirical evidence that the elimination is real rather than theoretical.*

---

**End of Chapter Six**
