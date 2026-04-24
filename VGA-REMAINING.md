# VGA Display — What's Left

## What Works

- **IPC buffer** found at `0x826000` via bootinfo — seL4 invocations enabled
- **PCI probe** successful — VGA device at slot 2, BAR0=`0xFD000000`
- **Device untyped** found covering BAR: paddr=`0xFC000000`, 32MB, bits=25
- **Large page retype** works — 9 × 2MB pages created, VGA frame at slot 1192
- **seL4 syscall shims** working with IPC buffer for extra caps and MR4+
- **Object type constants** fixed (PDPT=5, PML4=6, 4K=7, LargePage=8, PT=9, PD=10)
- **Invocation labels** verified against gen_headers (PDPTMap=31, PageDirMap=33, PageTableMap=35, PageMap=37, IOPortControlIssue=42)

## What's Failing

PDPT map returns error 8 (`seL4_DeleteFirst`) — a PDPT already exists at the target virtual address. The rootserver's PML4 entry 0 covers 0-512GB with a single PDPT, so any vaddr below 512GB hits an existing PDPT.

PD map returns error 3 (`seL4_IllegalOperation`) — likely because the PD retype is producing a wrong object (object constants were wrong before, may still be an issue), or the PD map invocation label is incorrect.

## The Fix

The PDPT already exists — don't try to map a new one. Just map a **Page Directory** at the unused PDPT entry for our target address, then map the large page into that PD.

Specifically for vaddr `0x2_0000_0000` (8GB):
- PML4 index 0 — already has a PDPT (covers 0-512GB)
- PDPT index 2 — needs a PD mapped here
- PD index 0 — the 2MB large page maps directly here

Steps:
1. Skip PDPT allocation (it exists) — or allocate but ignore `DeleteFirst` error
2. Allocate and map PD at the PDPT entry for `0x2_0000_0000`
3. Map the 2MB large page directly (no PT needed for large pages)

## Debugging the PD Map

If PD map still fails with error 3 (`IllegalOperation`):
1. Verify `seL4_X86_PageDirectoryObject = 10` by checking `alloc.retype()` actually creates a PD
2. Verify `INV_X86_PAGE_DIR_MAP = 33` matches `X86PageDirectoryMap` in gen_headers
3. Print the cap slot returned by `alloc.retype()` and verify it's valid
4. Check if `seL4_X86_PageDirectory_Map` shim passes the VSpace cap correctly as extra cap 0

## Alternative Approach

If page table mapping continues to be problematic, use `VSpaceMapper` from libprivos — it already handles the full x86_64 page table hierarchy correctly. The issue is that `VSpaceMapper` calls `seL4_X86_Page_Map` etc. via `extern "C"` which links to our shims, so the shim labels must be correct.

Test: create a `VSpaceMapper` and call `map_frame()` with a regular 4K RAM frame at a known-free address. If that works, the shims are correct and we can use the mapper for VGA too (after retyping the device untyped into 4K frames, which requires splitting the large device untyped first).

## Files

- `bare-metal/userspace/pst-init/src/vga.rs` — VGA init module
- `bare-metal/userspace/pst-init/src/sel4_shims.rs` — seL4 API syscall shims
- `bare-metal/userspace/bindings/sel4-sys/src/lib.rs` — object type constants
- `bare-metal/userspace/bindings/sel4-sys/src/native.rs` — raw syscall wrappers
