#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::vec::Vec;

use libpst::table::ParallelTable;
use libpst::offset::OffsetTable;

// Column indices
const COL_OWNER: usize  = 0;
const COL_PERM: usize   = 1;
const COL_FLAGS: usize  = 2;

// Permission bits
pub const PERM_READ: u8    = 0b001;
pub const PERM_WRITE: u8   = 0b010;
pub const PERM_EXECUTE: u8 = 0b100;
pub const PERM_RW: u8      = PERM_READ | PERM_WRITE;
pub const PERM_RWX: u8     = PERM_READ | PERM_WRITE | PERM_EXECUTE;

// Flags
pub const FLAG_SHARED: u8  = 0b0001;
pub const FLAG_DMA: u8     = 0b0010;
pub const FLAG_PINNED: u8  = 0b0100;

// Owner 0 = kernel/free
pub const OWNER_FREE: u8   = 0;
pub const OWNER_KERNEL: u8 = 1;

#[derive(Debug)]
pub enum MemError {
    OutOfMemory,
    NotFound,
    NotOwner,
    AlreadyFree,
    InvalidSize,
    OverlapDetected,
}

#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub start: u64,
    pub size: u64,
}

pub struct RegionAllocator {
    meta: ParallelTable,
    offsets: OffsetTable,
    regions: Vec<Option<Region>>,
    total_capacity: u64,
    next_offset: u64,
}

impl RegionAllocator {
    pub fn new(total_capacity: u64) -> Self {
        Self {
            meta: ParallelTable::new(&["owner", "perm", "flags"]),
            offsets: OffsetTable::new(),
            regions: Vec::new(),
            total_capacity,
            next_offset: 0,
        }
    }

    pub fn alloc(
        &mut self,
        size: u64,
        owner: u8,
        perm: u8,
        flags: u8,
    ) -> Result<(usize, Region), MemError> {
        if size == 0 { return Err(MemError::InvalidSize); }

        // Try to find a freed region that fits (first-fit from tombstoned regions)
        if let Some((logical, region)) = self.find_free_region(size) {
            // Reuse: overwrite the freed region's metadata
            if let Some(phys) = self.offsets.resolve(logical) {
                self.meta.set(COL_OWNER, phys, owner);
                self.meta.set(COL_PERM, phys, perm);
                self.meta.set(COL_FLAGS, phys, flags);
                return Ok((logical, region));
            }
        }

        // Append new region at end of address space
        if self.next_offset + size > self.total_capacity {
            return Err(MemError::OutOfMemory);
        }

        let region = Region {
            start: self.next_offset,
            size,
        };
        self.next_offset += size;

        let physical = self.meta.append(&[owner, perm, flags]);
        let logical = self.offsets.assign(physical);

        while self.regions.len() <= logical {
            self.regions.push(None);
        }
        self.regions[logical] = Some(region);

        Ok((logical, region))
    }

    pub fn free(&mut self, logical_id: usize, owner: u8) -> Result<Region, MemError> {
        if !self.offsets.is_valid(logical_id) {
            return Err(MemError::NotFound);
        }

        let phys = self.offsets.resolve(logical_id).ok_or(MemError::NotFound)?;
        let current_owner = self.meta.get(COL_OWNER, phys).ok_or(MemError::NotFound)?;

        if current_owner == OWNER_FREE {
            return Err(MemError::AlreadyFree);
        }
        if current_owner != owner && owner != OWNER_KERNEL {
            return Err(MemError::NotOwner);
        }

        // Mark as free (don't tombstone — region can be reused)
        self.meta.set(COL_OWNER, phys, OWNER_FREE);
        self.meta.set(COL_PERM, phys, 0);
        self.meta.set(COL_FLAGS, phys, 0);

        self.regions.get(logical_id)
            .and_then(|r| *r)
            .ok_or(MemError::NotFound)
    }

    /// Share a region — grants read access to another process
    /// by creating a new allocation entry pointing to the same physical region.
    pub fn share(
        &mut self,
        logical_id: usize,
        target_owner: u8,
        perm: u8,
    ) -> Result<(usize, Region), MemError> {
        if !self.offsets.is_valid(logical_id) {
            return Err(MemError::NotFound);
        }

        let region = self.regions.get(logical_id)
            .and_then(|r| *r)
            .ok_or(MemError::NotFound)?;

        let physical = self.meta.append(&[target_owner, perm, FLAG_SHARED]);
        let new_logical = self.offsets.assign(physical);

        while self.regions.len() <= new_logical {
            self.regions.push(None);
        }
        self.regions[new_logical] = Some(region);

        Ok((new_logical, region))
    }

    pub fn get_region(&self, logical_id: usize) -> Option<Region> {
        if !self.offsets.is_valid(logical_id) { return None; }
        self.regions.get(logical_id).and_then(|r| *r)
    }

    pub fn get_owner(&self, logical_id: usize) -> Option<u8> {
        let phys = self.offsets.resolve(logical_id)?;
        self.meta.get(COL_OWNER, phys)
    }

    pub fn get_perm(&self, logical_id: usize) -> Option<u8> {
        let phys = self.offsets.resolve(logical_id)?;
        self.meta.get(COL_PERM, phys)
    }

    pub fn get_flags(&self, logical_id: usize) -> Option<u8> {
        let phys = self.offsets.resolve(logical_id)?;
        self.meta.get(COL_FLAGS, phys)
    }

    /// Scan for all regions owned by a process.
    pub fn regions_for(&self, owner: u8) -> Vec<(usize, Region)> {
        let physicals = self.meta.scan(COL_OWNER, |v| v == owner);
        let mut results = Vec::new();
        for phys in physicals {
            if let Some(logical) = self.find_logical(phys) {
                if let Some(region) = self.regions.get(logical).and_then(|r| *r) {
                    results.push((logical, region));
                }
            }
        }
        results
    }

    /// Free all regions owned by a process (process death cleanup).
    pub fn free_all(&mut self, owner: u8) -> usize {
        let owned = self.regions_for(owner);
        let count = owned.len();
        for (logical, _) in owned {
            let _ = self.free(logical, owner);
        }
        count
    }

    /// Coalesce adjacent free regions into larger blocks.
    pub fn coalesce(&mut self) -> usize {
        let mut free_regions: Vec<(usize, Region)> = Vec::new();

        for (logical, region_opt) in self.regions.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if let Some(region) = region_opt {
                if let Some(phys) = self.offsets.resolve(logical) {
                    if self.meta.get(COL_OWNER, phys) == Some(OWNER_FREE) {
                        free_regions.push((logical, *region));
                    }
                }
            }
        }

        if free_regions.len() < 2 { return 0; }

        // Sort by start address
        free_regions.sort_by_key(|(_, r)| r.start);

        let mut merged = 0;
        let mut i = 0;
        while i < free_regions.len() - 1 {
            let (id_a, region_a) = free_regions[i];
            let (id_b, region_b) = free_regions[i + 1];

            if region_a.start + region_a.size == region_b.start {
                // Adjacent — merge b into a
                let new_size = region_a.size + region_b.size;
                self.regions[id_a] = Some(Region { start: region_a.start, size: new_size });

                // Tombstone b
                if let Some(phys) = self.offsets.resolve(id_b) {
                    self.meta.tombstone(phys);
                    self.offsets.invalidate(id_b);
                }

                // Update free_regions for next iteration
                free_regions[i] = (id_a, Region { start: region_a.start, size: new_size });
                free_regions.remove(i + 1);
                merged += 1;
            } else {
                i += 1;
            }
        }

        merged
    }

    pub fn used_bytes(&self) -> u64 {
        let mut total = 0u64;
        for (logical, region_opt) in self.regions.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if let Some(region) = region_opt {
                if let Some(phys) = self.offsets.resolve(logical) {
                    if self.meta.get(COL_OWNER, phys) != Some(OWNER_FREE) {
                        total += region.size;
                    }
                }
            }
        }
        total
    }

    pub fn free_bytes(&self) -> u64 {
        self.total_capacity - self.next_offset + self.freed_bytes()
    }

    fn freed_bytes(&self) -> u64 {
        let mut total = 0u64;
        for (logical, region_opt) in self.regions.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if let Some(region) = region_opt {
                if let Some(phys) = self.offsets.resolve(logical) {
                    if self.meta.get(COL_OWNER, phys) == Some(OWNER_FREE) {
                        total += region.size;
                    }
                }
            }
        }
        total
    }

    pub fn allocation_count(&self) -> usize {
        self.meta.live_count()
    }

    fn find_free_region(&self, min_size: u64) -> Option<(usize, Region)> {
        for (logical, region_opt) in self.regions.iter().enumerate() {
            if !self.offsets.is_valid(logical) { continue; }
            if let Some(region) = region_opt {
                if region.size >= min_size {
                    if let Some(phys) = self.offsets.resolve(logical) {
                        if self.meta.get(COL_OWNER, phys) == Some(OWNER_FREE) {
                            return Some((logical, *region));
                        }
                    }
                }
            }
        }
        None
    }

    fn find_logical(&self, physical: usize) -> Option<usize> {
        for i in 0..self.offsets.len() {
            if self.offsets.resolve(i) == Some(physical) {
                return Some(i);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1024 * 1024;

    #[test]
    fn test_alloc_and_query() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id, region) = mem.alloc(4096, 2, PERM_RW, 0).unwrap();
        assert_eq!(region.start, 0);
        assert_eq!(region.size, 4096);
        assert_eq!(mem.get_owner(id), Some(2));
        assert_eq!(mem.get_perm(id), Some(PERM_RW));
    }

    #[test]
    fn test_sequential_allocs() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (_, r1) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        let (_, r2) = mem.alloc(8192, 2, PERM_RW, 0).unwrap();
        assert_eq!(r1.start, 0);
        assert_eq!(r2.start, 4096);
        assert_eq!(mem.used_bytes(), 4096 + 8192);
    }

    #[test]
    fn test_free_and_reuse() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id1, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        let (_, _) = mem.alloc(4096, 2, PERM_RW, 0).unwrap();

        mem.free(id1, 1).unwrap();

        // Next alloc should reuse the freed region
        let (id3, r3) = mem.alloc(4096, 3, PERM_RW, 0).unwrap();
        assert_eq!(r3.start, 0); // reused the first slot
        assert_eq!(mem.get_owner(id3), Some(3));
    }

    #[test]
    fn test_out_of_memory() {
        let mut mem = RegionAllocator::new(4096);
        mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        assert!(mem.alloc(1, 1, PERM_RW, 0).is_err());
    }

    #[test]
    fn test_double_free() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        mem.free(id, 1).unwrap();
        assert!(matches!(mem.free(id, 1), Err(MemError::AlreadyFree)));
    }

    #[test]
    fn test_wrong_owner_cant_free() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        assert!(matches!(mem.free(id, 99), Err(MemError::NotOwner)));
    }

    #[test]
    fn test_kernel_can_free_anything() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id, _) = mem.alloc(4096, 5, PERM_RW, 0).unwrap();
        mem.free(id, OWNER_KERNEL).unwrap();
    }

    #[test]
    fn test_shared_memory() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id1, r1) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        let (id2, r2) = mem.share(id1, 2, PERM_READ).unwrap();

        // Same physical region
        assert_eq!(r1.start, r2.start);
        assert_eq!(r1.size, r2.size);

        // Different owners and perms
        assert_eq!(mem.get_owner(id1), Some(1));
        assert_eq!(mem.get_owner(id2), Some(2));
        assert_eq!(mem.get_perm(id1), Some(PERM_RW));
        assert_eq!(mem.get_perm(id2), Some(PERM_READ));
        assert_eq!(mem.get_flags(id2), Some(FLAG_SHARED));
    }

    #[test]
    fn test_regions_for_owner() {
        let mut mem = RegionAllocator::new(64 * MB);
        mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        mem.alloc(8192, 1, PERM_RW, 0).unwrap();
        mem.alloc(4096, 2, PERM_RW, 0).unwrap();

        let owner1 = mem.regions_for(1);
        assert_eq!(owner1.len(), 2);

        let owner2 = mem.regions_for(2);
        assert_eq!(owner2.len(), 1);
    }

    #[test]
    fn test_free_all_on_process_death() {
        let mut mem = RegionAllocator::new(64 * MB);
        mem.alloc(4096, 5, PERM_RW, 0).unwrap();
        mem.alloc(8192, 5, PERM_RW, 0).unwrap();
        mem.alloc(4096, 5, PERM_RW, 0).unwrap();
        mem.alloc(4096, 2, PERM_RW, 0).unwrap();

        let freed = mem.free_all(5);
        assert_eq!(freed, 3);
        assert_eq!(mem.regions_for(5).len(), 0);
        assert_eq!(mem.regions_for(2).len(), 1);
    }

    #[test]
    fn test_coalesce_adjacent() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id1, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        let (id2, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        let (id3, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();

        mem.free(id1, 1).unwrap();
        mem.free(id2, 1).unwrap();

        let merged = mem.coalesce();
        assert_eq!(merged, 1);

        // The merged free region should be 8192 bytes
        let free_regions = mem.regions_for(OWNER_FREE);
        assert_eq!(free_regions.len(), 1);
        assert_eq!(free_regions[0].1.size, 8192);
        assert_eq!(free_regions[0].1.start, 0);
    }

    #[test]
    fn test_coalesce_non_adjacent_untouched() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id1, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();
        let (_, _) = mem.alloc(4096, 2, PERM_RW, 0).unwrap(); // separator
        let (id3, _) = mem.alloc(4096, 1, PERM_RW, 0).unwrap();

        mem.free(id1, 1).unwrap();
        mem.free(id3, 1).unwrap();

        let merged = mem.coalesce();
        assert_eq!(merged, 0); // not adjacent, can't merge
    }

    #[test]
    fn test_dma_flag() {
        let mut mem = RegionAllocator::new(64 * MB);
        let (id, _) = mem.alloc(4096, 1, PERM_RW, FLAG_DMA).unwrap();
        assert_eq!(mem.get_flags(id), Some(FLAG_DMA));
    }

    #[test]
    fn test_zero_size_rejected() {
        let mut mem = RegionAllocator::new(64 * MB);
        assert!(matches!(mem.alloc(0, 1, PERM_RW, 0), Err(MemError::InvalidSize)));
    }
}
