# Bug Report — PST OS

Findings from a full audit of every source file in the workspace.
Severity: **Critical** → **High** → **Medium** → **Low/Performance**

---

## BUG-01 · Integer overflow bypasses ELF segment bounds check *(Critical)*

**File:** `bare-metal/userspace/libprivos/src/elf.rs:125–128`

```rust
let file_start = ph.p_offset as usize;
let file_end   = file_start + ph.p_filesz as usize;  // ← no overflow check
if file_end > data.len() {
    return None;
}
```

`ph.p_offset` and `ph.p_filesz` are both `u64`, cast to `usize` and added without a
checked add.  On a 64-bit target the arithmetic is 64-bit too, so a crafted ELF with
`p_offset = 0xFFFF_FFFF_FFFF_FF00` and `p_filesz = 0x200` wraps to
`file_end = 0x0100`, which is smaller than `data.len()`, passing the guard.  The
subsequent `&data[file_start..file_end]` then panics (or UB in unsafe context) because
`file_start` is way past the slice end.

**Fix:** use `file_start.checked_add(ph.p_filesz as usize)` and return `None` on overflow.

---

## BUG-02 · Non-page-aligned ELF segment first page silently zero-filled *(Critical)*

**File:** `bare-metal/userspace/libprivos/src/vm.rs:152`

```rust
let file_offset = (target_vaddr - seg.vaddr) as usize;
```

When a PT_LOAD segment is not page-aligned (e.g. `seg.vaddr = 0x1008`), the first page
starts at `0x1000 < seg.vaddr`.  The subtraction `0x1000 - 0x1008` wraps (both are `u64`)
to `0xFFFF_FFFF_FFFF_FFF8`.  Cast to usize that exceeds any `file_data.len()`, so the
subsequent guard:

```rust
if copy_start < file_data.len() { … }   // false — copy never executes
```

is never entered.  The entire first page is zero-filled and the actual segment file bytes
(which should be written at offset `seg.vaddr & 0xFFF` within the page) are silently
discarded.  Any binary with a non-page-aligned load segment will misbehave or crash.

**Fix:** When `target_vaddr < seg.vaddr`, the in-page destination offset is
`seg.vaddr - target_vaddr` (page prefix to zero) and file data starts at offset 0.
Guard the subtraction:

```rust
let (file_offset, page_dest_offset) = if target_vaddr >= seg.vaddr {
    ((target_vaddr - seg.vaddr) as usize, 0usize)
} else {
    (0usize, (seg.vaddr - target_vaddr) as usize)
};
```

---

## BUG-03 · READ privilege check off-by-one allows unprivileged reads *(High)*

**File:** `pst-offset/src/lib.rs:134–138`

```rust
if access & ACCESS_READ != 0 {
    if requester_priv > entry_priv + 1 {   // ← +1 is wrong
        return Err(OffsetError::PrivilegeDenied);
    }
}
```

The comment directly above this block states:

> *Same or higher privilege can read*

"Higher privilege" means a **lower** numeric level (PRIV_HARDWARE=0, PRIV_KERNEL=1,
PRIV_DRIVER=2, PRIV_SYSTEM=3, PRIV_USER=4).  The `+ 1` permits the first less-privileged
tier to read entries it should not:

| Entry privilege | Should be readable by | Actually readable by (current) |
|---|---|---|
| PRIV_HARDWARE (0) | PRIV_KERNEL (1) + kernel/hw bypass | also PRIV_KERNEL (1) only — accidentally correct here |
| PRIV_KERNEL (1) | PRIV_KERNEL (1) + bypass | also PRIV_DRIVER (2) — **wrong** |
| PRIV_DRIVER (2) | ≤ PRIV_DRIVER (2) | also PRIV_SYSTEM (3) — **wrong** |
| PRIV_SYSTEM (3) | ≤ PRIV_SYSTEM (3) | also PRIV_USER (4) — **wrong** |

A PRIV_USER process can read PRIV_SYSTEM entries.  No existing test covers a READ
cross-privilege check, so this passes the test suite silently.

**Fix:** Remove the `+ 1`:

```rust
if requester_priv > entry_priv {
    return Err(OffsetError::PrivilegeDenied);
}
```

---

## BUG-04 · `page_end()` can overflow u64 producing a wrong page range *(Medium)*

**File:** `bare-metal/userspace/libprivos/src/elf.rs:163–164`

```rust
pub fn page_end(&self) -> u64 {
    (self.vaddr + self.memsz + 0xfff) & !0xfff
}
```

If `vaddr + memsz` is close to `u64::MAX`, adding `0xfff` wraps.  The masked result is
then a small value near 0, making `page_count()` underflow (wraps to a huge usize),
and the page-mapping loop in `vm.rs` attempts to allocate millions of frames before OOM.

**Fix:** use saturating addition or a checked variant:

```rust
pub fn page_end(&self) -> u64 {
    self.vaddr
        .saturating_add(self.memsz)
        .saturating_add(0xfff)
        & !0xfff
}
```

---

## BUG-05 · Program-header iteration: `phoff + i * phentsz` can overflow *(Medium)*

**File:** `bare-metal/userspace/libprivos/src/elf.rs:115–116`

```rust
let phoff   = self.header.e_phoff as usize;
let phentsz = self.header.e_phentsize as usize;
// …
let off = phoff + i * phentsz;        // ← no overflow check
```

`e_phoff` is `u64` (max 2⁶⁴−1 cast to usize) and `e_phnum` / `e_phentsize` are `u16`,
so the product `i * phentsz` is at most ~4 GiB and won't overflow a 64-bit usize.
However `phoff + i * phentsz` can wrap if the ELF header reports an implausibly large
`e_phoff`.  On 64-bit targets the subsequent check `off + phentsz > data.len()` would
catch this because `data.len()` is small, but the wrapping arithmetic is still
technically UB-adjacent and should be guarded explicitly.

**Fix:** use `phoff.checked_add(i.checked_mul(phentsz)?)?` and return `None` on overflow.

---

## BUG-06 · `RegionAllocator::alloc()` overflow in capacity check *(Low)*

**File:** `pst-mem/src/lib.rs:88`

```rust
if self.next_offset + size > self.total_capacity {
    return Err(MemError::OutOfMemory);
}
```

Both values are `u64`.  If `size` is near `u64::MAX` the addition wraps before the
comparison, making the check pass when the allocation should be rejected.

**Fix:**

```rust
if self.next_offset.checked_add(size).map_or(true, |end| end > self.total_capacity) {
    return Err(MemError::OutOfMemory);
}
```

---

## BUG-07 · `find_logical()` O(n²) reverse lookup *(Performance)*

**Files:**
- `pst-ipc/src/lib.rs:207–214`
- `pst-mem/src/lib.rs:308–315`
- `proctable/src/lib.rs:126–134` (same pattern in `scan_by_state`)

All three structs reverse-map physical → logical by iterating every entry:

```rust
fn find_logical(&self, physical: usize) -> Option<usize> {
    for i in 0..self.offsets.len() {
        if self.offsets.resolve(i) == Some(physical) {
            return Some(i);
        }
    }
    None
}
```

`recv()` in `pst-ipc` calls this once per pending message, making message drain O(n²)
in the number of messages.  Under interrupt load (many small messages) this degrades
badly.

**Fix:** maintain a reverse `Vec<Option<usize>>` alongside the offset table, updated in
`assign()` and `invalidate()`, so the lookup is O(1).

---

---

## BUG-08 · `write()` allows writing data bytes into a directory entry *(High)*

**File:** `pst-vfs/src/lib.rs:94–99`

```rust
pub fn write(&mut self, logical_id: usize, content: &[u8]) -> Result<(), FsError> {
    if content.len() > MAX_DATA { return Err(FsError::DataTooLarge); }
    if !self.offsets.is_valid(logical_id) { return Err(FsError::NotFound); }
    self.data[logical_id] = Some(content.to_vec());  // ← no TYPE_DIR check
    Ok(())
}
```

There is no check for `TYPE_DIR`.  A caller can `write()` arbitrary bytes into a
directory entry, setting `data[dir_id]` to a non-`None` value.  `read()` on that
directory then silently returns the injected bytes instead of an error.

**Fix:** Resolve the physical position, check `meta.get(COL_TYPE, phys) == Some(TYPE_DIR)`,
and return `Err(FsError::NotADirectory)` before the write.

---

## BUG-09 · `delete()` returns the wrong error for a non-empty directory *(Medium)*

**File:** `pst-vfs/src/lib.rs:124`

```rust
if !self.ls(&prefix).is_empty() {
    return Err(FsError::NotADirectory);  // ← IS a directory; should be "not empty"
}
```

When a caller tries to delete a directory that still has children, the error code
`NotADirectory` is returned.  The entry *is* a directory — the correct condition is
that it is not empty.  Callers pattern-matching on the error to distinguish "wrong
type" from "has children" will mishandle both cases.

**Fix:** Add a `DirectoryNotEmpty` variant to `FsError` and return it here.

---

## BUG-10 · `delete()` never clears `names` / `data` — memory leak per deletion *(Medium)*

**File:** `pst-vfs/src/lib.rs:130–133`

```rust
if let Some(phys) = self.offsets.resolve(logical_id) {
    self.meta.tombstone(phys);
    self.offsets.invalidate(logical_id);
    // ← names[logical_id] and data[logical_id] are never set to None
}
```

After tombstoning the metadata row and invalidating the offset entry, the
`names` and `data` `Vec` slots still hold the file's path string and content
bytes.  Those allocations are never reclaimed.  In a long-running system
(or a filesystem that cycles files through delete/create), memory grows
without bound.

**Fix:**
```rust
self.names[logical_id] = None;
self.data[logical_id]  = None;
```

---

## BUG-11 · `ack()` on a pending (undelivered) message causes silent loss *(High)*

**File:** `pst-ipc/src/lib.rs:119–127`

```rust
pub fn ack(&mut self, logical_id: usize) -> Result<(), IpcError> {
    let phys = self.offsets.resolve(logical_id).ok_or(IpcError::NotFound)?;
    if self.meta.get(COL_STATUS, phys) == Some(STATUS_READ) {
        return Err(IpcError::AlreadyRead);  // ← only guards the READ state
    }
    self.meta.set(COL_STATUS, phys, STATUS_READ);
    Ok(())
}
```

The only rejection is for `STATUS_READ`.  A caller that obtains a message ID
before `recv()` has run (status still `STATUS_PENDING`) can call `ack()` on it.
The status is promoted to `STATUS_READ`, and the next `gc()` tombstones the
message — before the intended recipient ever called `recv()`.  The message is
silently lost with no error returned to either side.

**Fix:** Also guard against `STATUS_PENDING`:
```rust
match self.meta.get(COL_STATUS, phys) {
    Some(STATUS_READ)    => return Err(IpcError::AlreadyRead),
    Some(STATUS_PENDING) => return Err(IpcError::NotDelivered),
    _ => {}
}
```

---

## BUG-12 · `state_at()` tie-breaking is non-deterministic *(Medium)*

**File:** `pst-time/src/lib.rs:116`

```rust
if t <= at_tick && t >= best_tick {
    best_tick = t;
    best_value = self.events.get(COL_NEW_VAL, phys);
}
```

When two events share the same tick (`t == best_tick`), the condition `t >= best_tick`
is satisfied and the later entry in iteration order overwrites `best_value`.  Iteration
order depends on the physical layout of the `ParallelTable` — it changes after
`compact_storage()`.  The same query before and after a compaction can return
different values even when no events were added or removed.

**Fix:** Use strict greater-than so that the first recorded event at a given tick wins
(append-order is deterministic):
```rust
if t <= at_tick && t > best_tick {
```

---

## BUG-13 · Initial stack pointer is 8-byte aligned, not 16-byte *(High)*

**File:** `bare-metal/userspace/libprivos/src/process.rs:190`

```rust
ctx.rsp = CHILD_STACK_TOP - 8; // 16-byte aligned stack  ← comment is wrong
```

`CHILD_STACK_TOP = 0x7fff_f000`.  Subtracting 8 gives `0x7fff_eff8`.
`0x7fff_eff8 % 16 == 8`, so RSP is 8-byte aligned, not 16.

The x86-64 System V ABI requires RSP to be 16-byte aligned at the entry point
(`_start` / `main`).  An 8-byte misalignment causes SSE/AVX instructions to
fault with `#GP` and breaks any code that uses `movaps`, `movdqa`, or similar.

**Fix:** Subtract 8 to account for the implicit return address the ABI expects:
the canonical pattern is `rsp = (STACK_TOP & !0xf) - 8`.  Since `CHILD_STACK_TOP`
is already page-aligned (low 12 bits zero), the correct value is:
```rust
ctx.rsp = CHILD_STACK_TOP - 8;   // pushes RSP to 0x7fffeff8 — wrong
// Should be:
ctx.rsp = CHILD_STACK_TOP - 8;   // only correct if STACK_TOP % 16 == 0
// CHILD_STACK_TOP = 0x7ffff000, which is 16-byte aligned (0x...000 % 16 == 0)
// so CHILD_STACK_TOP - 8 = 0x7fffeff8, which is 8 mod 16 — still wrong.
ctx.rsp = CHILD_STACK_TOP;       // RSP = 0x7ffff000, 16-byte aligned at entry
```

---

## BUG-14 · All services in `init` are spawned without `.with_initrd()` — never start *(Critical)*

**File:** `bare-metal/userspace/init/src/main.rs:68–105`

```rust
ProcessBuilder::new("cryptod", &mut alloc)
    .grant_endpoint(crypto_ep)
    .spawn()                          // ← no .with_initrd()
    .expect("failed to start cryptod");
// … same pattern for vfs, netd, driverd, driver-nic, compositor
```

`ProcessBuilder::spawn()` gates ELF loading and `seL4_TCB_Resume` on
`self.initrd.is_some() && entry_point != 0` (process.rs:209).  Without a call to
`.with_initrd(&initrd)`, `entry_point` stays `0` and `seL4_TCB_Resume` is never
called.  All six services have their kernel objects created but their threads are
never scheduled.  The system hangs in the watchdog spin loop with no services
running.

**Fix:** Chain `.with_initrd(&initrd)` on every `ProcessBuilder`:
```rust
ProcessBuilder::new("cryptod", &mut alloc)
    .with_initrd(&initrd)
    .grant_endpoint(crypto_ep)
    .spawn()
    .expect("failed to start cryptod");
```

---

## BUG-15 · Shift by `size_bits` panics / UB when `size_bits >= 64` *(High)*

**File:** `bare-metal/userspace/libprivos/src/mem.rs:100`

```rust
let needed = 1usize << size_bits;  // ← size_bits comes from seL4 BootInfo
```

`size_bits` is typed as `seL4_Word` (usize).  The seL4 spec allows `size_bits`
up to 47 for normal memory regions, but the value comes from untrusted BootInfo
data.  A malformed or out-of-spec value ≥ 64 causes a panic in debug builds and
undefined behaviour in release builds (Rust shifts are undefined for amounts
≥ bit-width in release/`wrapping_shr` semantics are not guaranteed here).

**Fix:**
```rust
let needed = 1usize.checked_shl(size_bits as u32)
    .ok_or(AllocError::OutOfMemory)?;
```

---

## BUG-16 · Same shift-overflow in capacity calculation *(High)*

**File:** `bare-metal/userspace/libprivos/src/mem.rs:105`

```rust
let capacity = 1usize << region.size_bits;
```

`region.size_bits` is `u8` stored from BootInfo.  A value of 64 or above causes
the same panic / UB as BUG-15.  The value is read directly from the kernel-provided
`untypedList[i].sizeBits` field and is never range-checked before the shift.

**Fix:** Same pattern — use `checked_shl` and propagate the error.

---

## BUG-17 · `initrd::find()` — `offset + size` can overflow usize *(Medium)*

**File:** `bare-metal/userspace/libprivos/src/initrd.rs:67`

```rust
let offset = u64::from_le_bytes(...) as usize;
let size   = u64::from_le_bytes(...) as usize;
// …
let end = offset + size;          // ← no overflow check
if end > self.data.len() {
    return Err(InitrdError::Truncated);
}
return Ok(&self.data[offset..end]);
```

A crafted initrd with `offset = 0xFFFF_FFFF_FFFF_FF00` and `size = 0x200` wraps
`end` to `0x100` on a 64-bit target.  `end > self.data.len()` is false, so the
bounds guard passes and `self.data[offset..end]` panics because `offset` is far
past the slice end.

**Fix:**
```rust
let end = offset.checked_add(size).ok_or(InitrdError::Truncated)?;
```

---

## BUG-18 · Terminal renderer tracks column position in bytes, not display width *(Medium)*

**File:** `pst-terminal/src/lib.rs` (throughout)

```rust
VNode::Text(t) => {
    out.push_str(text);
    ctx.col += text.len();  // ← byte count, not display columns
}
```

Multi-byte UTF-8 characters are wider than one byte but occupy one display column.
For example, the box-drawing character `─` is 3 UTF-8 bytes but one column wide.
`ctx.col` accumulates byte counts, so `ansi::cursor_to(row, col)` in `render_card`
positions side-borders at wrong columns whenever any multi-byte character has been
rendered on the same line — borders are shifted right by `(bytes − columns)` per
character.

**Fix:** Replace `text.len()` with a display-width calculation, e.g. by counting
Unicode scalar values or using a `unicode_width`-style lookup, depending on the
character set in use.

---

## BUG-19 · `set_focus()` panics on out-of-bounds index *(Medium)*

**File:** `pst-ui/src/lib.rs:217–221`

```rust
pub fn set_focus(&mut self, idx: usize) {
    if let Some(old) = self.focused { self.rows[old].focus = false; }
    self.rows[idx].focus = true;     // ← no bounds check
    self.focused = Some(idx);
}
```

`set_focus` is a public API.  Any caller passing `idx >= self.rows.len()` causes
an index-out-of-bounds panic.  `handle_click` and `tab_next` are the primary
callers within the crate; both derive `idx` from iteration, so they are safe —
but external callers are not.

**Fix:**
```rust
if idx >= self.rows.len() { return; }
```

---

## BUG-20 · seL4 IPC message word count not validated against `seL4_MsgMaxLength` *(High)*

**File:** `bare-metal/userspace/libprivos/src/ipc.rs:44` and `81`

```rust
let info = seL4_MessageInfo_t::new(
    msg.label,
    0,
    0,
    msg.words.len() as seL4_Word,  // ← no bound check
);
for (i, &word) in msg.words.iter().enumerate() {
    // SAFETY: i is bounded by msg.words.len() <= seL4_MsgMaxLength
    unsafe { seL4_SetMR(i as i32, word) };
}
```

The SAFETY comment claims a bound that is never enforced.  `seL4_MsgMaxLength` is
typically 120 words.  If `msg.words.len() > 120`, `seL4_SetMR` writes past the
IPC buffer, corrupting adjacent kernel-mapped memory, and the kernel may silently
truncate the message info length, causing the receiver to see a different word
count than the sender intended.

**Fix:**
```rust
let word_count = msg.words.len().min(seL4_MsgMaxLength as usize);
// and iterate only 0..word_count
```

---

## BUG-21 · `rebuild_from_remap()` is O(L × R) — quadratic after compaction *(Performance)*

**File:** `libpst/src/offset.rs:46–71`

```rust
for entry in &mut self.logical_to_physical {
    if let Some(old_phys) = *entry {
        let new = phys_map.iter().find(|&&(o, _)| o == old_phys); // O(R) linear scan
        *entry = new.map(|&(_, n)| n);
    }
}
```

For every logical entry (L total, including invalidated ones), the code does a
linear search through the remap table (R live entries).  This is O(L × R).
Called from `compact()` in every subsystem, it runs after every GC cycle.
As tables grow, compaction cost scales quadratically.

**Fix:** Sort `phys_map` by old-physical before the loop and use binary search,
or build a `HashMap<usize, usize>` for O(1) lookup per entry.

---

## BUG-22 · `find_name_by_id()` uses IPC sender byte as a slice index *(Medium)*

**File:** `pst-watchdog/src/lib.rs:197–207`

```rust
fn find_name_by_id(&self, id: u8, entries: &[SchedEntry]) -> String {
    entries.get(id as usize)
        .map(|e| e.name.clone())
        .unwrap_or_else(|| { … })
}
```

`id` is the `sender` field from an IPC message — a process ID.  The function
treats this ID as a direct index into the `entries` slice, which is a filtered
subset of currently-runnable processes.  If any process is blocked or has been
removed from `entries`, the slice position no longer corresponds to the process ID.
The function silently returns the wrong process name (or the fallback numeric
string), causing violations to be attributed to the wrong process and potentially
tombstoning an innocent service.

**Fix:** Look up the name by scanning `entries` for the matching sender, or maintain
a stable `id → name` table outside the schedule slice.

---

## BUG-23 · Pending-message scheduling boost is documented but never implemented *(Medium)*

**File:** `pst-sched/src/lib.rs:69–73`

```rust
// Processes with pending messages get an implicit boost:
// they run before processes at the same priority with no messages
if entry.pending_messages > 0 {
    // No extra constraint needed — priority handles it
}
```

The comment promises that processes with pending messages run ahead of equal-priority
processes with no messages.  The code block is a no-op; no constraint is added and
`priority` is not adjusted.  Kahn's algorithm iterates a `BTreeMap` (alphabetical
key order), so when multiple nodes are simultaneously ready, the tiebreaker is
alphabetical name order — not pending-message count, not priority.  Latency-sensitive
message handlers may be starved by alphabetically-earlier peers.

**Fix:** Either increment `priority` for processes with `pending_messages > 0`
(so the solver sees a higher value) or add a soft `After` constraint that orders
message-less processes behind message-pending ones.

---

## BUG-24 · Non-UTF-8 fault payload causes `ack()` to be skipped, leaking the message *(Medium)*

**File:** `pst-watchdog/src/lib.rs:120–131`

```rust
for (id, sender, payload) in &fault_msgs {
    if let Ok(fault_str) = core::str::from_utf8(payload) {
        // … process fault …
        let _ = ipc.ack(*id);    // ← inside the Ok branch
    }
    // If payload is not valid UTF-8, ack() is never called
}
// …
ipc.gc();  // only tombstones STATUS_READ — this message stays STATUS_DELIVERED
```

`recv()` transitions the message to `STATUS_DELIVERED`.  `gc()` only tombstones
`STATUS_READ` entries.  A fault message whose payload is not valid UTF-8 is never
acknowledged, stays `STATUS_DELIVERED` forever, and is replayed on every subsequent
`tick()` call as a new violation — potentially escalating and tombstoning the
sending process unfairly on every tick.

**Fix:** Move `ipc.ack(*id)` outside the `if let Ok` block so every received
message is acknowledged regardless of payload encoding.

---

## BUG-25 · `ConstrainedNode::priority` is stored but never consulted by the solver *(Medium)*

**File:** `libpst/src/solver.rs:29–113`

The `ConstrainedNode` struct has a `priority: u8` field populated by both the
scheduler and the process table, but `topological_sort()` never reads it.  When
multiple nodes have equal in-degree (are simultaneously schedulable), they are
added to the work queue in `BTreeMap` key order — alphabetical by name.  A
high-priority interrupt handler named `"zirq"` would be scheduled after a
low-priority daemon named `"alogd"`, regardless of priority values.

**Fix:** Use a `BinaryHeap` keyed by `(in_degree == 0, priority descending)` instead
of a `VecDeque`, so equal-depth nodes are ordered by priority.

---

---

## BUG-27 · Parametric container returns zero height — all following content overlaps it *(High)*

**File:** `pst-framebuffer/src/lib.rs:154–159`

```rust
if style.contains("position:relative") {
    // Parametric container — children have absolute positions
    for child in &el.children {
        cy = render_vnode(fb, child, x, y, bg, fg);
    }
    return cy;   // ← still equals the input y
}
```

Each child inside a parametric block is wrapped in a `position:absolute` div by the
solver.  The `position:absolute` handler (line 152) always returns `cy` unchanged.
After the loop `cy` is still the original `y` passed in, so the container reports that
it consumed zero vertical space.  Any element rendered after a `@parametric` block
starts at the same Y coordinate and overlaps the entire parametric block.

The solver already computed the container height and wrote it into the style string:
`"position:relative;width:…px;height:…px"` (render.rs:387).  The framebuffer
renderer parses `parse_position(style)` but discards the height via `_ph`:

```rust
// position:absolute handler — line 146
let (px, py, pw, _ph) = parse_position(style);  // _ph thrown away
```

**Fix:** Read the container height from the `position:relative` style and return
`y + container_h`:
```rust
if style.contains("position:relative") {
    let (_, _, _, container_h) = parse_position(style);
    for child in &el.children {
        render_vnode(fb, child, x, y, bg, fg);
    }
    return y + container_h;
}
```

---

## BUG-28 · `gap-y` / `gap-x` with no resolvable reference silently positions element at origin *(Medium)*

**File:** `pst-markout/src/render.rs:331–354`

```rust
let first_ref: Option<String> = constraints.iter()
    .flat_map(|c| c.references())
    .next()
    .map(|s| String::from(s));

// …
Constraint::GapY(gap, ref_opt) => {
    let r = ref_opt.as_ref().or(first_ref.as_ref());
    if let Some(rr) = r.and_then(|n| solved.get(n.as_str())) {
        y = rr.1 + rr.3 + gap.pixels;
    }
    // ← if r is None or the name isn't in solved yet, y stays 0.0
}
```

Two cases produce a silent y=0:

1. **No reference at all:** `{input:search gap-y:16}` — no explicit name after `gap-y:`,
   no other constraint to supply `first_ref`.  `r = None`, constraint is silently skipped,
   element lands at `(0, 0)`.

2. **`first_ref` resolves to the wrong element:** `{input:search center-x:title gap-y:16}` —
   `first_ref` is `"title"` (from `center-x`).  The bare `gap-y:16` places the element
   16 px below `title`, which may not be the author's intent.  More critically, if the
   author then writes `{button:go gap-y:8}` expecting it to appear 8 px below the input,
   `first_ref` will be `"search"` (the first reference from the button's `gap-y` constraint),
   but only if `"search"` is already in `solved`; otherwise the button also goes to y=0.

**Fix:** When `ref_opt` is `None` and `first_ref` is `None`, fall back to the element
most recently added to `solved` (the natural predecessor), rather than silently
leaving the coordinate at zero.

---

## BUG-29 · Card height includes trailing inter-child gap, making cards 4 px too tall *(Low)*

**File:** `pst-framebuffer/src/lib.rs:225–234`

```rust
let mut inner_y = card_y + pad;
for child in &el.children {
    inner_y = render_vnode(fb, child, card_x + pad, inner_y, bg, fg);
    inner_y += cfg.gap;       // ← gap added after EVERY child, including the last
}
let card_h = inner_y - card_y + pad;
```

After rendering the last child, `cfg.gap` (default 4 px) is added to `inner_y`
unconditionally.  The height formula then includes that trailing gap, making the card
`cfg.gap` pixels taller than its content requires.  With the default gap of 4, every
card is 4 px taller than it should be.  The visual effect is a slightly too-large empty
strip below the last element inside every card.

**Fix:** Subtract the trailing gap before computing height:
```rust
if inner_y > card_y + pad { inner_y -= cfg.gap; }
let card_h = inner_y - card_y + pad;
```

---

## BUG-26 · Stale `saved_bg` after view switch leaves a cursor-shaped artifact *(High)*

**Files:**
- `bare-metal/userspace/pst-init/src/desktop.rs:197–198`
- `bare-metal/userspace/pst-init/src/ps2.rs:258–282`

`draw_cursor()` uses a save-and-restore technique: before drawing the cursor it
saves the 16×16 pixels underneath it into `saved_bg`, and on the *next* call it
restores those saved pixels first (erasing the old cursor).

```rust
// draw_cursor() — step 1: restore old position
if self.cursor_drawn {
    // writes self.saved_bg back to framebuffer at (cursor_x, cursor_y)
}
```

When the user switches views, `render_desktop()` rewrites every pixel on screen:

```rust
// desktop.rs:197–198
focused = (focused + 1) % windows.len();
render_desktop(&windows, focused, fb_vaddr);   // entire framebuffer overwritten
// cursor_drawn is still true; saved_bg still holds pixels from the old view
```

`cursor_drawn` and `saved_bg` are not touched.  On the first mouse movement
after the switch, `draw_cursor()` restores `saved_bg` — a 16×16 block of pixels
from the *previous view* — onto the newly rendered framebuffer at the old cursor
coordinates.  That patch of the old view is visible for one frame (until the
cursor moves again and overwrites it), producing a visible rectangular ghost of
the previous screen content.

The same issue occurs when returning from the editor and code-viewer modes
(desktop.rs:214, 219), which also call `render_desktop()` without resetting the
cursor state.

**Fix:** After every `render_desktop()` call, mark the cursor as undrawn so the
restore step is skipped on the next paint:
```rust
render_desktop(&windows, focused, fb_vaddr);
ps2.cursor_drawn = false;   // saved_bg is now stale — skip restore on next move
```

---

*Generated 2026-04-25 — audited all `.rs` sources under `libpst/`, `pst-*/`, `proctable/`,
`bare-metal/userspace/libprivos/`, and `bare-metal/userspace/init/`.*
