// Capability-based memory allocator.
//
// seL4 gives the initial thread all physical memory as "untyped" capabilities.
// To create any kernel object (endpoint, TCB, page frame, CNode), you must
// "retype" an untyped region into the desired object type. This is the only
// way to allocate kernel objects — there is no malloc in the kernel.

use sel4_sys::*;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// UntypedAllocator
// ---------------------------------------------------------------------------

/// Allocates seL4 kernel objects by retyping untyped capabilities.
pub struct UntypedAllocator {
    untypeds: Vec<UntypedRegion>,
    /// Next free slot in the initial thread's CSpace.
    next_free_slot: seL4_CPtr,
}

struct UntypedRegion {
    cap: seL4_CPtr,
    size_bits: u8,
    /// True = device MMIO memory. Never retype for regular objects.
    is_device: bool,
    used: usize,
}

#[derive(Debug)]
pub enum AllocError {
    OutOfMemory,
}

impl UntypedAllocator {
    /// Build the allocator from seL4's BootInfo.
    ///
    /// # Safety
    /// `bootinfo` must be a valid pointer to the seL4 BootInfo structure
    /// provided by the kernel to the initial thread.
    pub unsafe fn new(bootinfo: &seL4_BootInfo) -> Self {
        let mut untypeds = Vec::new();
        let start = bootinfo.untyped.start as usize;
        let end   = bootinfo.untyped.end   as usize;

        for i in 0..(end - start) {
            let desc = &bootinfo.untypedList[i];
            untypeds.push(UntypedRegion {
                cap:      (start + i) as seL4_CPtr,
                size_bits: desc.sizeBits,
                is_device: desc.isDevice != 0,
                used:      0,
            });
        }

        Self {
            untypeds,
            next_free_slot: bootinfo.empty.start,
        }
    }

    /// Allocate a 4 KiB page frame.
    pub fn alloc_frame(&mut self) -> Result<seL4_CPtr, AllocError> {
        self.retype(seL4_X86_4K, seL4_PageBits as seL4_Word)
    }

    /// Allocate a new IPC endpoint.
    pub fn create_endpoint(&mut self) -> Result<seL4_CPtr, AllocError> {
        self.retype(seL4_EndpointObject, seL4_EndpointBits as seL4_Word)
    }

    /// Allocate a new Thread Control Block.
    pub fn create_tcb(&mut self) -> Result<seL4_CPtr, AllocError> {
        self.retype(seL4_TCBObject, seL4_TCBBits as seL4_Word)
    }

    /// Allocate a CNode with 2^`depth` capability slots.
    pub fn create_cnode(&mut self, depth: u8) -> Result<seL4_CPtr, AllocError> {
        // Each slot = 8 bytes (one seL4_Word), so total = 2^depth * 8 = 2^(depth+3) bytes.
        let size_bits = (depth + 3) as seL4_Word;
        self.retype(seL4_CapTableObject, size_bits)
    }

    /// Allocate a VSpace root (PML4 on x86_64).
    pub fn create_vspace(&mut self) -> Result<seL4_CPtr, AllocError> {
        self.retype(seL4_X86_PML4Object, seL4_PageBits as seL4_Word)
    }

    /// Core retype: find a RAM untyped with enough space and create one object.
    pub fn retype(
        &mut self,
        obj_type: seL4_Word,
        size_bits: seL4_Word,
    ) -> Result<seL4_CPtr, AllocError> {
        let slot = self.next_free_slot;
        self.next_free_slot += 1;

        let needed = 1usize.checked_shl(size_bits as u32)
            .ok_or(AllocError::OutOfMemory)?;

        for region in &mut self.untypeds {
            if region.is_device { continue; }

            let capacity = 1usize.checked_shl(region.size_bits as u32)
                .unwrap_or(0);
            if capacity - region.used >= needed {
                let err = unsafe {
                    seL4_Untyped_Retype(
                        region.cap,
                        obj_type,
                        size_bits,
                        seL4_CapInitThreadCNode, // root CNode
                        0, 0,                    // node_index, node_depth (direct root placement)
                        slot,                    // offset in root CNode
                        1,                       // create 1 object
                    )
                };

                if err == seL4_NoError {
                    region.used += needed;
                    return Ok(slot);
                }
            }
        }

        self.next_free_slot -= 1;
        Err(AllocError::OutOfMemory)
    }

    /// Allocate a new notification object.
    pub fn create_notification(&mut self) -> Result<seL4_CPtr, AllocError> {
        self.retype(seL4_NotificationObject, seL4_NotificationBits as seL4_Word)
    }

    /// Reserve the next free CSpace slot without creating an object in it.
    /// Use this for operations (like seL4_IRQControl_Get) that fill the slot
    /// themselves via kernel invocation.
    pub fn next_slot(&mut self) -> seL4_CPtr {
        let slot = self.next_free_slot;
        self.next_free_slot += 1;
        slot
    }

    /// Iterator over device-memory caps (MMIO regions for drivers).
    pub fn device_untypeds(&self) -> impl Iterator<Item = (seL4_CPtr, u8)> + '_ {
        self.untypeds
            .iter()
            .filter(|r| r.is_device)
            .map(|r| (r.cap, r.size_bits))
    }
}
