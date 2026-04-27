# Parallel String Theory
## A New Primitive for Computing

### Chapter Three: The Formal Foundation

---

Mathematics does not care about engineering convenience. A theory that runs fast but cannot be stated precisely is an implementation, not a primitive. A primitive must be defined with enough rigor that its properties can be proved, its invariants can be checked, and its limits can be stated without ambiguity.

This chapter provides that foundation. It begins with notation and formal definitions, then addresses five questions that a computer scientist focused on formal methods, distributed systems, or type theory would ask. The questions were stated in the previous chapter's closing section. They are not rhetorical. Each one probes a genuine potential weakness in the formal model.

The answers are given in full. Where the theory requires a formal assumption, the assumption is stated. Where a proof has structure, the structure is shown. Where a boundary exists, it is drawn precisely.

---

#### Notation and Primitive Definitions

Let ℕ denote the natural numbers including zero. Let ⊥ denote the undefined or tombstoned value. Let 𝔹 denote booleans {true, false}.

**Definition 3.1 (String).** A string S is a total function S : ℕ → V ∪ {⊥} where V is a value domain. S(i) = ⊥ means position i is undefined or tombstoned. S(i) ∈ V means position i has value S(i).

**Definition 3.2 (Parallel Table).** A parallel table T of width w is a tuple T = (S₀, S₁, ..., S_{w-1}, n) where each Sⱼ is a string over domain Vⱼ and n ∈ ℕ is the current length. T is valid when ∀j ∈ [0,w), ∀i ≥ n : Sⱼ(i) = ⊥.

**Definition 3.3 (Logical Identity).** The logical identity of an entity in table T is its row index i ∈ ℕ, i < n at the time of creation. The identity is immutable: once assigned, it does not change.

**Definition 3.4 (Append).** An append operation on T with values v₀,...,v_{w-1} produces T' = (S₀', ..., S_{w-1}', n+1) where Sⱼ'(n) = vⱼ for all j and Sⱼ'(i) = Sⱼ(i) for i < n. The new entity receives logical identity n.

**Definition 3.5 (Tombstone).** A tombstone operation on T at position i produces T' identical to T except that Sⱼ'(i) = ⊥ for all j. Tombstoning is idempotent: tombstoning an already tombstoned position produces the same table.

**Definition 3.6 (Offset Table).** An offset table O is a total function O : ℕ → ℕ ∪ {⊥} mapping logical identities to physical memory addresses. O(i) = ⊥ when identity i is tombstoned. O(i) = p when identity i is stored at physical address p.

**Definition 3.7 (Constraint).** A constraint C is a tuple C = (source, relation, target, weight) where source and target are logical identities, relation ∈ R is a relation drawn from a fixed vocabulary R, and weight ∈ ℚ is a rational-valued priority.

**Definition 3.8 (Constraint Graph).** The constraint graph G(T) induced by table T is a directed graph G = (V, E) where V = {i ∈ ℕ : i < n ∧ S₀(i) ≠ ⊥} and (i, j) ∈ E when there exists a constraint C with source = i and target = j.

**Definition 3.9 (Resolution).** A resolution of constraint graph G is a total ordering ≺ on V such that if (i, j) ∈ E then i ≺ j. A resolution exists if and only if G is a directed acyclic graph (DAG).

---

#### Question One: The Distributed Identity Problem

*"How does positional identity extend to distributed systems? If Node A and Node B operate independently without a central coordinator, how do they agree on flat positional identity?"*

This question identifies a genuine boundary condition of the primitive and requires a precise answer.

**The single-node case is clean.** Theorem 3.1 states that atomic fetch-and-increment on the length counter n provides globally unique logical identities within a single-node shared-memory system.

*Proof.* Let the length counter be an atomic register. An append operation atomically increments n and receives the pre-increment value as the new identity. Since the increment is atomic, no two concurrent appends receive the same value. The new identity is strictly greater than all previously assigned identities. Uniqueness follows. □

**The distributed case requires a formal assumption.** The primitive does not claim that positional identity is inherently distributed. It claims that within a *single totally-ordered log*, positional identity is well-defined. The question of how to maintain a totally-ordered log across distributed nodes is real, and the answer requires a choice.

**Definition 3.10 (Distributed Parallel Table).** A distributed parallel table DT is a collection of local tables {T_A, T_B, ...} together with a merge protocol M that produces a consensus table T* = M(T_A, T_B, ...) satisfying the identity uniqueness property.

Two merge protocols are available, with different guarantees:

**Protocol D1: Totally Ordered Log.** All appends are routed through a single sequencer that assigns globally unique monotonic identities. This is the Kafka/Raft approach. Identity is clean: i < j implies i was appended before j everywhere in the system. Cost: the sequencer is a single point of coordination, not a hierarchy but a single bottleneck.

**Protocol D2: Vector Identity.** Each node maintains a local counter. The logical identity of an entity is a pair (node_id, local_counter). Two entities from different nodes have identities that are incomparable under the natural order on ℕ, but comparable under a total order defined on pairs. The cost is that positional identity becomes two-dimensional. The offset table maps (node_id, local_counter) pairs rather than single integers.

The formal model accommodates both. For single-node systems and systems with a designated log sequencer, Definition 3.3 applies directly. For fully distributed systems without a sequencer, Definition 3.3 is generalized:

**Definition 3.3' (Distributed Logical Identity).** In a distributed parallel table with n nodes, the logical identity of an entity is a vector (node_id, local_counter, lamport_timestamp) ∈ ℕ³. Two identities are ordered lexicographically. The total order is well-defined and consistent with causality by the properties of Lamport timestamps.

The honest formal statement is this: Parallel String Theory in its simplest form assumes a totally ordered append log. This assumption holds trivially on a single machine and can be extended to distributed systems at the cost of either a sequencer or a vector identity scheme. The theory does not claim that flat positional identity is free in distributed systems. It claims it is available and well-defined given an appropriate identity assignment protocol.

This is not a weakness unique to the primitive. Every distributed system must solve the identity problem. The parallel string primitive makes the problem explicit rather than hiding it inside a tree node's memory address.

---

#### Question Two: Constraint Contradictions and Unsatisfiability

*"How does the formal mathematical model guarantee determinism and termination when faced with an unsatisfiable constraint graph?"*

**Theorem 3.2 (Resolution Existence).** A resolution of constraint graph G exists if and only if G is a DAG.

*Proof.* Standard result from graph theory. A topological ordering of a directed graph exists if and only if the graph contains no directed cycle. A directed cycle in G means entity i must precede entity j must precede ... must precede entity i, which is a contradiction. □

The formal model handles three cases:

**Case 1: G is a DAG.** Resolution exists. The topological sort algorithm (Kahn's algorithm or DFS-based) terminates in O(V + E) time and produces a valid ordering. Termination is guaranteed; determinism requires a tie-breaking rule when multiple orderings are valid.

**Definition 3.11 (Tie-Breaking).** When multiple entities have in-degree zero simultaneously during topological sort, they are ordered by constraint weight. Among entities with equal weight, they are ordered by logical identity (smaller identity first). This rule is deterministic and total: any two valid orderings that differ only in the ordering of concurrent entities will produce the same result under this rule.

**Case 2: G contains a cycle.** No resolution exists. The formal model specifies the response:

**Definition 3.12 (Cycle Response).** When the constraint solver detects a cycle C = {i₁, i₂, ..., iₖ, i₁} in G, it applies the minimum-weight tombstone: the entity iⱼ ∈ C with the lowest constraint weight (or lowest logical identity among ties) is tombstoned. The solver then retries resolution on G minus the tombstoned entity.

This is the formal statement of what Chapter One called the watchdog. Cycles are not errors that crash the system. They are constraint violations that trigger tombstoning of the weakest participant. The system's failure mode is always degraded functionality, never undefined behavior.

**Theorem 3.3 (Termination Under Cycles).** For any finite constraint graph G, repeated application of Definition 3.12 terminates with a DAG.

*Proof.* Each application of Definition 3.12 removes at least one entity from G. Since G is finite, the process terminates. The resulting graph contains no cycles (any remaining cycle would trigger another tombstone). Therefore the result is a DAG. □

**Case 3: Contradictory constraints.** Some constraint vocabularies allow direct contradictions: A must be above B, and A must be below B. These are syntactically valid but semantically unsatisfiable.

**Definition 3.13 (Constraint Validation).** A constraint C = (source, relation, target, weight) is validated at append time. If appending C to the constraint set would create an immediately detectable contradiction (for relations with computable satisfiability), the append is rejected and C is not added to the table. If the contradiction is not detectable at append time (it depends on the full graph), it is detected at resolution time and handled by Definition 3.12.

Constraint validation at append time is optional optimization. Correctness does not require it. Safety is guaranteed by Theorem 3.3 regardless of whether contradictions are detected early or late.

---

#### Question Three: Total vs. Partial Ordering of Append-Only Mutation

*"Does the formal model assume total ordering of all appends, or does it support partial ordering? If total, how does it represent concurrent non-causal events without artificial serialization?"*

This is the most technically subtle of the five questions, and it is where the formal model makes its most important choice.

**Definition 3.14 (Append History).** An append history H is a set of append events {a₁, a₂, ...} together with a causality relation → where aᵢ → aⱼ means aᵢ causally precedes aⱼ (aᵢ happened before aⱼ in the sense of Lamport 1978).

The causality relation → is a partial order on H: it is irreflexive, asymmetric, and transitive. It is not total: two events that neither causally precede nor follow each other are *concurrent*.

**Theorem 3.4 (Partial Order Sufficiency).** The parallel string primitive requires only a partial order on appends for correctness.

*Proof sketch.* The correctness properties of the primitive are:
1. Unique identity assignment: satisfied by atomic increment on each node's local counter (Definition 3.10).
2. Append-only invariant: satisfied by the monotonicity of local counters — a node never decreases its counter.
3. Constraint resolution: requires only the final state of the constraint graph, not the order in which constraints were appended. The topological sort is applied to the current graph, not to the history of graph construction.
4. Tombstone idempotence: satisfied by Definition 3.5 — tombstoning a tombstoned entity is a no-op.

None of these properties require a total order on appends. Two concurrent appends — events with no causal relationship — can receive identities that are incomparable under natural order and still produce a valid parallel table. The merge of two local tables produced by concurrent appends is well-defined under Protocol D2 (vector identity). □

**Corollary 3.5.** Artificial serialization of concurrent non-causal appends is unnecessary. Two processes appending independent entities simultaneously do not need to coordinate. Their appends produce entities with different identities that coexist in the table without conflict.

**The exception: constraint resolution.** When two concurrent appends both add constraints that affect the same entities, their interaction must be resolved. The formal model handles this through the constraint weight mechanism: concurrent constraints affecting the same entities are ordered by weight, with logical identity as the tiebreaker. This produces a deterministic resolution that does not require the two appenders to have communicated.

**Definition 3.15 (Concurrent Constraint Merge).** If events aᵢ and aⱼ are concurrent (neither aᵢ → aⱼ nor aⱼ → aᵢ) and both append constraints to the same graph, the merged constraint set is the union of the two constraint sets. Resolution applies Definition 3.11 (weight-based tie-breaking) to produce a deterministic total ordering. The result is the same regardless of which merge order is used, by the commutativity of set union.

The formal model therefore supports both total and partial ordering. Single-node systems use total ordering trivially (atomic increment provides causality). Distributed systems use partial ordering with merge protocols and weight-based tie-breaking for conflict resolution. Both are mathematically sound.

---

#### Question Four: Sum Types and Sparsity

*"Parallel strings naturally represent Product Types. How does the primitive handle extreme polymorphism without creating massive sparse matrices?"*

This question reveals a deep structural point about the relationship between the parallel string primitive and type theory.

**The formalist's concern is correct but its framing is imprecise.** Parallel strings as defined in Definition 3.2 are products: each row has Column 0 AND Column 1 AND Column 2. This is a product type. Trees are naturally sums: a node can be a leaf OR an internal node OR a nil. Sums express heterogeneity. Products express homogeneity.

The resolution requires two mechanisms.

**Mechanism 1: The Discriminator Column.** Every parallel table includes a distinguished discriminator column S₀ : ℕ → TypeTag where TypeTag is a finite enumeration. The discriminator column carries the sum type information. All other columns are typed relative to the discriminator value.

**Definition 3.16 (Typed Parallel Table).** A typed parallel table T = (S₀, S₁, ..., S_{w-1}, n) where S₀ is the discriminator column. For each type tag τ ∈ TypeTag, let Cτ ⊆ {1, ..., w-1} be the columns relevant to entities with discriminator value τ. An entity i with S₀(i) = τ has well-defined values Sⱼ(i) for j ∈ Cτ and is permitted to have ⊥ values for j ∉ Cτ.

This is exactly the Rust enum representation described in Chapter One. The discriminator column is the enum tag. The parallel columns are the enum variant fields. Sⱼ(i) = ⊥ for irrelevant columns is the encoding of "this variant does not have this field."

**Mechanism 2: Formal Sparsity Characterization.** The formalist's concern about sparse matrices deserves a precise statement.

**Definition 3.17 (Sparsity).** The sparsity of column Sⱼ is the ratio |{i : Sⱼ(i) = ⊥}| / n. A column is sparse when its sparsity approaches 1 — most entries are undefined.

**Theorem 3.6 (Sparsity Bound).** In a typed parallel table with k type tags of equal frequency, the expected sparsity of any non-discriminator column is (k-1)/k.

*Proof.* Each column is relevant to exactly one type tag (in the maximally heterogeneous case). With k type tags of equal frequency, 1/k of entities use each type tag. Therefore 1/k of rows have defined values in any given column, and (k-1)/k have ⊥. □

For k = 2 (two types) the sparsity is 1/2. For k = 10 the sparsity is 9/10. High polymorphism produces high sparsity. This is the formal statement of the concern.

**The formal response to high sparsity is column segmentation:**

**Definition 3.18 (Column Segmentation).** A column-segmented parallel table partitions columns by type tag. For type tag τ, the segment Tτ = (S₀, {Sⱼ : j ∈ Cτ}, nτ) contains only entities with S₀(i) = τ and only the columns relevant to τ. Each segment has sparsity 0 in its non-discriminator columns.

Column segmentation eliminates sparsity by separating the sum type into its constituent product types. The segments are related by the logical identity in S₀. Joining segments to retrieve all entities of all types is a merge on logical identity.

This is the formal analog of the Rust enum representation: each variant is a separate segment. The enum tag is the discriminator. Pattern matching is selection of the appropriate segment. Union of all segments produces the full typed parallel table.

**Theorem 3.7 (Sparsity Elimination).** Column segmentation produces segments with sparsity 0 in all non-discriminator columns.

*Proof.* By Definition 3.18, segment Tτ contains only entities where S₀(i) = τ, and only columns Cτ relevant to τ. Every entity in Tτ has defined values for every column in Cτ. Sparsity is 0. □

The formal model therefore handles sum types without sparse matrices: discriminator column for type tags, column segmentation for variant separation, logical identity for cross-segment lookup. The memory representation is dense. The type system is expressive.

---

#### Question Five: The Formal Proof of Identity Conservation

*"How do you mathematically prove that the bijection between logical identity and physical memory remains unbroken during concurrent compaction?"*

This is the most important formal proof in the chapter, because identity conservation is the invariant on which everything else depends.

**Definition 3.19 (Physical Table).** A physical table P is an array of memory addresses. P[i] is the physical memory address where entity i is currently stored.

**Definition 3.20 (Offset Table Consistency).** An offset table O is consistent with physical table P when ∀i ∈ [0,n) : (Sⱼ(i) ≠ ⊥ for some j) → O(i) = P[i].

In words: for every non-tombstoned entity, the offset table correctly maps its logical identity to its physical address.

**The compaction algorithm must maintain this consistency.** Compaction rewrites the physical table to remove gaps left by tombstoned entities, then updates the offset table. The question is whether any reader can observe an inconsistent state during this process.

**Theorem 3.8 (Identity Conservation Under Compaction).** Given a compaction algorithm that satisfies the following properties, no reader ever observes a torn state or stale address:

1. *Copy before update*: The compaction algorithm writes each entity to its new physical address before updating O(i).
2. *Atomic offset update*: Each update to O(i) is an atomic word-sized write.
3. *Epoch fencing*: Readers acquire a read epoch before dereferencing O(i). Compaction increments the epoch after completing all offset updates. Readers validate their epoch after dereferencing.

*Proof.*

Let a reader R begin reading entity i at time t₁ and a compactor C be moving entity i from physical address p_old to p_new during the interval [t₂, t₃].

**Case A: t₁ < t₂.** R reads O(i) = p_old before compaction begins. R dereferences p_old and reads valid data. Compaction has not yet moved i. No inconsistency. □

**Case B: t₁ > t₃.** R reads O(i) = p_new after compaction completes. R dereferences p_new and reads valid data. The entity is at p_new. No inconsistency. □

**Case C: t₂ ≤ t₁ ≤ t₃ (concurrent).** This is the critical case.

By property 1, C writes entity i to p_new before updating O(i). Therefore during the interval between the copy and the offset update, entity i exists at both p_old and p_new. At the moment C atomically updates O(i) from p_old to p_new (property 2), the entity is already valid at p_new.

If R reads O(i) = p_old before the atomic update, R dereferences p_old which still contains valid data (copy has occurred but old data has not been invalidated yet). Valid read. □

If R reads O(i) = p_new after the atomic update, R dereferences p_new which contains valid data. Valid read. □

**Epoch validation** (property 3) handles the case where R reads O(i) = p_old, then is preempted, then C invalidates p_old, then R dereferences p_old. The epoch check detects that compaction occurred during R's epoch. R retries the read. □

**Corollary 3.9 (No Torn Reads).** Under the three properties of Theorem 3.8, no reader observes a partial write or a tombstoned-but-accessible entity.

*Proof.* Tombstoning sets all columns of entity i to ⊥ and then sets O(i) = ⊥. By property 2, the O(i) = ⊥ update is atomic. A reader either sees O(i) = p (entity live, columns valid) or O(i) = ⊥ (entity tombstoned, read should return not-found). The intermediate state where columns are being set to ⊥ is protected by property 1: the offset table update to ⊥ occurs only after all column updates complete. □

**Definition 3.21 (Compaction Safety).** A compaction algorithm is safe if it satisfies properties 1, 2, and 3 of Theorem 3.8. The parallel string primitive requires safe compaction.

**Implementation note.** Property 2 (atomic word-sized write) holds on all modern architectures for naturally aligned word-sized values. Property 1 (copy before update) is a sequencing constraint enforced by a memory fence instruction. Property 3 (epoch fencing) is a standard technique in read-copy-update (RCU) synchronization, which has been proven correct in multiple formal verifications in systems such as Linux.

The proof is constructive: it specifies exactly what properties the compaction algorithm must satisfy and shows that satisfaction of those properties is sufficient for correctness. An implementation that satisfies properties 1, 2, and 3 is provably correct. An implementation that violates any of the three has a potential inconsistency that this proof identifies precisely.

---

#### Summary of the Formal Model

The five questions have been answered. Collecting the key results:

| Property | Formal Guarantee | Assumption Required |
|----------|-----------------|-------------------|
| Identity uniqueness (single-node) | Theorem 3.1 | Atomic fetch-and-increment |
| Identity uniqueness (distributed) | Definition 3.10, Protocol D2 | Vector identity scheme |
| Constraint resolution termination | Theorem 3.3 | Finite constraint graph |
| Resolution determinism | Definition 3.11 | Weight-based tie-breaking |
| Partial order sufficiency | Theorem 3.4 | Causality relation on appends |
| Sum type expressiveness | Theorem 3.7 | Column segmentation |
| Identity conservation under compaction | Theorem 3.8 | Safe compaction algorithm |
| No torn reads | Corollary 3.9 | Properties 1, 2, 3 of Theorem 3.8 |

The formal model is sound. Its assumptions are explicit. Its guarantees are proved. Its boundaries are drawn.

---

#### What the Formal Model Does Not Prove

Mathematical honesty requires stating what has not been proved as clearly as what has.

**The formal model does not prove that the primitive is optimal.** Theorem 3.3 proves that constraint resolution terminates. It does not prove that it terminates in less time than alternative approaches for any specific workload. Performance claims require empirical measurement, not formal proof.

**The formal model does not prove that constraint completeness is decidable in general.** For rich constraint vocabularies, determining whether a constraint set is satisfiable is in general undecidable (by reduction to the halting problem for sufficiently expressive constraint languages). Definition 3.13 handles this by deferring to runtime detection when static detection is infeasible.

**The formal model does not prove that column segmentation is always the right response to sparsity.** Theorem 3.7 proves that segmentation eliminates sparsity. It does not prove that eliminating sparsity is always better than tolerating it. For tables where cross-segment joins are frequent, the join overhead may exceed the memory savings from density.

These are honest boundaries. The proof covers what it claims to cover and no more.

---

#### The Mathematics and the Metal

The formal definitions in this chapter were not written first and then implemented. They were written after — derived from working systems, extracted from code that runs, formalized to make the implicit explicit.

PST OS satisfies Definition 3.16 (typed parallel table): the process table discriminates by process type, the filesystem discriminates by file type, the IPC log discriminates by message type. Compaction in PST OS satisfies the three properties of Theorem 3.8: generational compaction with epoch fencing, copy-before-update, atomic offset table writes.

The formal model is not aspirational. It describes what exists.

The next chapter shows it running.

---

*Chapter Four presents PST OS: a 217-kilobyte operating system that boots to a windowed desktop on the seL4 formally verified microkernel, implementing each subsystem as a parallel table satisfying the definitions of this chapter. The formal properties proved here are observable in the running system.*

---

**End of Chapter Three**
