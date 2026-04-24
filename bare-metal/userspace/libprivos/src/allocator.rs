// Global heap allocator for Privion OS userspace processes.
//
// Uses linked_list_allocator backed by a static buffer.
// The buffer is allocated in BSS (zero-initialized at startup).
//
// Size: 4 MiB — enough for the init process to spawn all services
// and manage their endpoints before any dynamic memory pressure.
//
// SAFETY: This is initialized once at the start of _start() before
// any alloc usage. init() must be called exactly once, before any
// allocation attempt.

use linked_list_allocator::LockedHeap;

const HEAP_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// The backing store for the heap. Placed in BSS; zero on startup.
static mut HEAP: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];

/// Initialize the global heap. Call this once at the very start of _start(),
/// before any use of Vec, Box, or other alloc types.
///
/// # Safety
/// Must be called exactly once, in a single-threaded context, before any
/// allocation occurs.
pub unsafe fn init() {
    // SAFETY: HEAP is a static mut accessed here only once before any
    // concurrent access is possible (called at the start of _start()).
    // Use addr_of_mut! to obtain a raw pointer without creating a mutable
    // reference, which is undefined behavior on static muts (Rust 2024).
    let heap_start = core::ptr::addr_of_mut!(HEAP) as *mut u8;
    ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
}
