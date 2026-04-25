// Virtual memory mapper for loading processes into seL4 VSpaces.
//
// x86_64 seL4 page table hierarchy:
//   PML4 (VSpace root, seL4_X86_PML4Object)       — 512 GiB per entry
//   └─ PDPT (seL4_X86_PDPTObject)                  — 1 GiB per entry
//      └─ PageDirectory (seL4_X86_PageDirectoryObject) — 2 MiB per entry
//         └─ PageTable (seL4_X86_PageTableObject)   — 4 KiB per entry
//            └─ Page frame (seL4_X86_4K)            — 4 KiB
//
// seL4 requires each level to be explicitly allocated and mapped before
// the level below it. This module handles that hierarchy automatically.

use sel4_sys::*;
use crate::mem::{UntypedAllocator, AllocError};
use crate::elf::LoadSegment;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Address decomposition for x86_64 4-level paging
// ---------------------------------------------------------------------------

/// Extract 9-bit PML4 index from a virtual address.
#[inline] fn pml4_idx(va: u64) -> u64 { (va >> 39) & 0x1ff }
/// Extract 9-bit PDPT index from a virtual address.
#[inline] fn pdpt_idx(va: u64) -> u64 { (va >> 30) & 0x1ff }
/// Extract 9-bit PD index from a virtual address.
#[inline] fn pd_idx(va: u64)   -> u64 { (va >> 21) & 0x1ff }
/// Extract 9-bit PT index from a virtual address.
#[inline] fn pt_idx(va: u64)   -> u64 { (va >> 12) & 0x1ff }

// ---------------------------------------------------------------------------
// VSpaceMapper
// ---------------------------------------------------------------------------

/// Maps pages into a target process's VSpace by walking / allocating the
/// x86_64 page table hierarchy.
///
/// The mapper maintains a cache of already-mapped intermediate tables to
/// avoid redundant seL4 calls (an attempt to re-map an existing table
/// would return an error).
pub struct VSpaceMapper<'a> {
    vspace: seL4_CPtr,            // PML4 cap of the target process
    alloc:  &'a mut UntypedAllocator,
    /// Tracks (table_type, vaddr_key) for tables we've already mapped.
    /// Keyed by the virtual address bits that identify the table slot.
    #[cfg(feature = "alloc")]
    mapped: Vec<(u8, u64)>,       // (level 1=PDPT 2=PD 3=PT, key_vaddr)
}

const LEVEL_PDPT: u8 = 1;
const LEVEL_PD:   u8 = 2;
const LEVEL_PT:   u8 = 3;

#[derive(Debug)]
pub enum MapError {
    AllocFailed(AllocError),
    SeL4Error(seL4_Error),
}

impl From<AllocError> for MapError {
    fn from(e: AllocError) -> Self { MapError::AllocFailed(e) }
}

impl<'a> VSpaceMapper<'a> {
    pub fn new(vspace: seL4_CPtr, alloc: &'a mut UntypedAllocator) -> Self {
        Self {
            vspace,
            alloc,
            #[cfg(feature = "alloc")]
            mapped: Vec::new(),
        }
    }

    /// Map a single 4 KiB page frame at `vaddr` in the target VSpace.
    /// `frame` is a cap to the physical frame (from alloc.alloc_frame()).
    pub fn map_frame(
        &mut self,
        frame: seL4_CPtr,
        vaddr: u64,
        rights: seL4_CapRights_t,
        writable: bool,
    ) -> Result<(), MapError> {
        // Ensure the intermediate tables exist, allocating if needed.
        self.ensure_pdpt(vaddr)?;
        self.ensure_pd(vaddr)?;
        self.ensure_pt(vaddr)?;

        let err = unsafe {
            seL4_X86_Page_Map(
                frame,
                self.vspace,
                vaddr,
                rights,
                seL4_X86_Default_VMAttributes,
            )
        };
        if err != seL4_NoError {
            return Err(MapError::SeL4Error(err));
        }
        Ok(())
    }

    /// Load an ELF load segment into this VSpace, page by page.
    ///
    /// For each page:
    ///   1. Allocate a frame in the CALLER's VSpace (init's VSpace) temporarily.
    ///      Actually: allocate, map into init's VSpace to copy data, then map
    ///      into the target VSpace. seL4 allows one frame to be mapped into
    ///      multiple VSpaces.
    ///
    /// Phase 4 simplification: we map via init's own VSpace for copying, then
    /// remap into the target VSpace. Full dual-mapping will be completed in
    /// Phase 5 with proper frame sharing.
    pub fn load_segment(
        &mut self,
        seg: &LoadSegment,
        init_vspace: seL4_CPtr,
        staging_vaddr: u64, // a free vaddr in init's VSpace for temporary mapping
    ) -> Result<(), MapError> {
        let page_start = seg.page_start();
        let page_count = seg.page_count();

        let rights = if seg.is_writable() {
            seL4_CapRights_t::READ_WRITE
        } else {
            seL4_CapRights_t::READ_ONLY
        };

        for i in 0..page_count {
            let target_vaddr  = page_start + (i as u64) * 0x1000;
            let staging_vaddr = staging_vaddr + (i as u64) * 0x1000;

            // Allocate a fresh frame
            let frame = self.alloc.alloc_frame()?;

            // Map it into init's VSpace at the staging address so we can write to it
            let err = unsafe {
                seL4_X86_Page_Map(
                    frame, init_vspace, staging_vaddr,
                    seL4_CapRights_t::READ_WRITE,
                    seL4_X86_Default_VMAttributes,
                )
            };
            if err != seL4_NoError {
                return Err(MapError::SeL4Error(err));
            }

            // Copy file data into the page (zero the rest)
            let page_ptr = staging_vaddr as *mut u8;
            let (file_offset, page_dest_offset) = if target_vaddr >= seg.vaddr {
                ((target_vaddr - seg.vaddr) as usize, 0usize)
            } else {
                (0usize, (seg.vaddr - target_vaddr) as usize)
            };
            let file_data = seg.file_data;

            unsafe {
                // Zero the whole page first
                core::ptr::write_bytes(page_ptr, 0, 0x1000);

                // Copy file bytes that fall within this page
                let copy_start = file_offset;
                let copy_end = (file_offset + (0x1000 - page_dest_offset)).min(file_data.len());
                if copy_start < file_data.len() {
                    let src = &file_data[copy_start..copy_end];
                    core::ptr::copy_nonoverlapping(
                        src.as_ptr(),
                        page_ptr.add(page_dest_offset),
                        src.len(),
                    );
                }
            }

            // Unmap from init's VSpace (seL4 page unmap not shown — Phase 5)
            // For now, leave double-mapped (acceptable for initial dev)

            // Map into target VSpace
            self.map_frame(frame, target_vaddr, rights, seg.is_writable())?;
        }

        Ok(())
    }

    // --- Internal: ensure intermediate tables exist ---

    fn ensure_pdpt(&mut self, va: u64) -> Result<(), MapError> {
        let key = pml4_idx(va);
        if !self.is_mapped(LEVEL_PDPT, key) {
            let pdpt = self.alloc.retype(seL4_X86_PDPTObject, seL4_PageBits as seL4_Word)?;
            let err = unsafe {
                seL4_X86_PDPT_Map(pdpt, self.vspace, va, seL4_X86_Default_VMAttributes)
            };
            if err != seL4_NoError && err != seL4_DeleteFirst {
                return Err(MapError::SeL4Error(err));
            }
            self.mark_mapped(LEVEL_PDPT, key);
        }
        Ok(())
    }

    fn ensure_pd(&mut self, va: u64) -> Result<(), MapError> {
        let key = (pml4_idx(va) << 9) | pdpt_idx(va);
        if !self.is_mapped(LEVEL_PD, key) {
            let pd = self.alloc.retype(seL4_X86_PageDirectoryObject, seL4_PageBits as seL4_Word)?;
            let err = unsafe {
                seL4_X86_PageDirectory_Map(pd, self.vspace, va, seL4_X86_Default_VMAttributes)
            };
            if err != seL4_NoError && err != seL4_DeleteFirst {
                return Err(MapError::SeL4Error(err));
            }
            self.mark_mapped(LEVEL_PD, key);
        }
        Ok(())
    }

    fn ensure_pt(&mut self, va: u64) -> Result<(), MapError> {
        let key = (pml4_idx(va) << 18) | (pdpt_idx(va) << 9) | pd_idx(va);
        if !self.is_mapped(LEVEL_PT, key) {
            let pt = self.alloc.retype(seL4_X86_PageTableObject, seL4_PageBits as seL4_Word)?;
            let err = unsafe {
                seL4_X86_PageTable_Map(pt, self.vspace, va, seL4_X86_Default_VMAttributes)
            };
            if err != seL4_NoError && err != seL4_DeleteFirst {
                return Err(MapError::SeL4Error(err));
            }
            self.mark_mapped(LEVEL_PT, key);
        }
        Ok(())
    }

    fn is_mapped(&self, level: u8, key: u64) -> bool {
        #[cfg(feature = "alloc")]
        return self.mapped.iter().any(|&(l, k)| l == level && k == key);
        #[cfg(not(feature = "alloc"))]
        false
    }

    fn mark_mapped(&mut self, level: u8, key: u64) {
        #[cfg(feature = "alloc")]
        self.mapped.push((level, key));
    }
}
