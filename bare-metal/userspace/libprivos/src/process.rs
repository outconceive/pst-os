// Process spawning via seL4 capability manipulation.
//
// Sequence:
//   1. Allocate VSpace, CSpace, TCB, IPC buffer frame, stack frames
//   2. Load the service ELF into the new VSpace using vm::VSpaceMapper
//   3. Copy endpoint capabilities into the new CSpace
//   4. Configure the TCB and resume the thread

use sel4_sys::*;
use crate::elf::{ElfBinary, ElfError};
use crate::initrd::Initrd;
use crate::mem::{UntypedAllocator, AllocError};
use crate::vm::{VSpaceMapper, MapError};

// Virtual address in init's VSpace used to temporarily stage the child's
// top stack page so we can write the sel4runtime auxv startup record.
// Must be in a 2 MiB region whose PDPT entry is not used by init itself.
// 0x5800_0000 = 1.375 GiB: PML4[0] → PDPT[1] → PD[192] → PT[0].
// Init's own code/data/stack live below 1 GiB (PDPT[0]), so PDPT[1] is free.
const INIT_STACK_STAGING: u64 = 0x5800_0000;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

// Virtual address layout for newly created processes.
// These are arbitrary but must not conflict with the ELF load segments.
const CHILD_STACK_TOP:   u64 = 0x7fff_f000; // stack grows down from here
const CHILD_STACK_PAGES: usize = 4;          // 16 KiB stack
const CHILD_IPC_VADDR:   u64 = 0x6000_0000; // IPC buffer virtual address
const INIT_STAGING_BASE: u64 = 0x5000_0000; // staging window in init's VSpace

// ---------------------------------------------------------------------------
// ProcessHandle
// ---------------------------------------------------------------------------

pub struct ProcessHandle {
    pub tcb: seL4_CPtr,
    #[cfg(feature = "alloc")]
    pub name: String,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum SpawnError {
    AllocFailed(AllocError),
    MapFailed(MapError),
    ElfParseError(ElfError),
    ElfNotFound,
    CapCopyFailed,
    TcbConfigFailed,
}

impl From<AllocError> for SpawnError {
    fn from(e: AllocError) -> Self { SpawnError::AllocFailed(e) }
}
impl From<MapError> for SpawnError {
    fn from(e: MapError) -> Self { SpawnError::MapFailed(e) }
}
impl From<ElfError> for SpawnError {
    fn from(e: ElfError) -> Self { SpawnError::ElfParseError(e) }
}

// ---------------------------------------------------------------------------
// ProcessBuilder
// ---------------------------------------------------------------------------

pub struct ProcessBuilder<'a> {
    #[cfg(feature = "alloc")]
    name: String,
    alloc: &'a mut UntypedAllocator,
    initrd: Option<&'a Initrd<'a>>,
    #[cfg(feature = "alloc")]
    endpoints: Vec<seL4_CPtr>,
}

impl<'a> ProcessBuilder<'a> {
    pub fn new(name: &str, alloc: &'a mut UntypedAllocator) -> Self {
        Self {
            #[cfg(feature = "alloc")]
            name: String::from(name),
            alloc,
            initrd: None,
            #[cfg(feature = "alloc")]
            endpoints: Vec::new(),
        }
    }

    /// Provide the initrd to load the service binary from.
    pub fn with_initrd(mut self, initrd: &'a Initrd<'a>) -> Self {
        self.initrd = Some(initrd);
        self
    }

    pub fn grant_endpoint(mut self, ep: seL4_CPtr) -> Self {
        #[cfg(feature = "alloc")]
        self.endpoints.push(ep);
        let _ = ep;
        self
    }

    /// Spawn the process.
    ///
    /// Requires `with_initrd()` to have been called, or the ELF loading step
    /// is skipped (useful for unit-testing the capability setup path).
    pub fn spawn(self) -> Result<ProcessHandle, SpawnError> {
        // --- 1. Allocate kernel objects ---
        let vspace    = self.alloc.create_vspace()?;
        let cspace    = self.alloc.create_cnode(8)?;
        let tcb       = self.alloc.create_tcb()?;
        let ipc_frame = self.alloc.alloc_frame()?;

        // --- 2. Copy endpoint caps into the child's CSpace ---
        #[cfg(feature = "alloc")]
        for (i, &ep) in self.endpoints.iter().enumerate() {
            let err = unsafe {
                seL4_CNode_Copy(
                    cspace, i as seL4_Word, 8,
                    seL4_CapInitThreadCNode, ep, 64,
                    seL4_CapRights_t::ALL,
                )
            };
            if err != seL4_NoError {
                return Err(SpawnError::CapCopyFailed);
            }
        }

        // --- 3. Load ELF into the new VSpace ---
        let entry_point = if let Some(initrd) = self.initrd {
            #[cfg(feature = "alloc")]
            let elf_bytes = initrd.find(&self.name)
                .map_err(|_| SpawnError::ElfNotFound)?;
            #[cfg(not(feature = "alloc"))]
            let elf_bytes: &[u8] = &[];

            let elf = ElfBinary::parse(elf_bytes)?;
            let entry = elf.entry_point();

            // Pre-allocate stack frames BEFORE creating VSpaceMapper.
            // VSpaceMapper holds &mut self.alloc, so we cannot call
            // self.alloc.alloc_frame() again while the mapper is alive.
            #[cfg(feature = "alloc")]
            let stack_frames: Vec<seL4_CPtr> = {
                let mut frames = Vec::new();
                for _ in 0..CHILD_STACK_PAGES {
                    frames.push(self.alloc.alloc_frame()?);
                }
                frames
            };

            let mut mapper = VSpaceMapper::new(vspace, self.alloc);
            for seg in elf.load_segments() {
                mapper.load_segment(
                    &seg,
                    seL4_CapInitThreadVSpace, // init's own VSpace for staging
                    INIT_STAGING_BASE,
                )?;
            }

            // Map the IPC buffer into the child's VSpace
            mapper.map_frame(
                ipc_frame,
                CHILD_IPC_VADDR,
                seL4_CapRights_t::READ_WRITE,
                true,
            )?;

            // Map the stack (CHILD_STACK_PAGES pages below CHILD_STACK_TOP)
            let stack_base = CHILD_STACK_TOP - (CHILD_STACK_PAGES as u64) * 0x1000;
            #[cfg(feature = "alloc")]
            for (p, frame) in stack_frames.into_iter().enumerate() {
                mapper.map_frame(
                    frame,
                    stack_base + p as u64 * 0x1000,
                    seL4_CapRights_t::READ_WRITE,
                    true,
                )?;
            }

            entry
        } else {
            0u64 // no initrd — entry point unknown, TCB not resumed
        };

        // --- 4. Configure and start the TCB ---
        let mut ctx = seL4_UserContext::default();
        ctx.rip = entry_point;
        ctx.rsp = CHILD_STACK_TOP; // 16-byte aligned at _start entry

        let err = unsafe {
            seL4_TCB_Configure(
                tcb,
                seL4_CapNull,           // fault endpoint (none yet)
                cspace, seL4_NilData,
                vspace, seL4_NilData,
                CHILD_IPC_VADDR,        // IPC buffer virtual address
                ipc_frame,
            )
        };
        if err != seL4_NoError { return Err(SpawnError::TcbConfigFailed); }

        unsafe {
            seL4_TCB_SetPriority(tcb, seL4_CapInitThreadTCB, 254);
            seL4_TCB_WriteRegisters(tcb, 0, 0, 2, &ctx); // 2 = write rip + rsp
        }

        if self.initrd.is_some() && entry_point != 0 {
            unsafe { seL4_TCB_Resume(tcb); }
        }

        Ok(ProcessHandle {
            tcb,
            #[cfg(feature = "alloc")]
            name: self.name,
        })
    }
}
