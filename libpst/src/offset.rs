use alloc::vec::Vec;

#[derive(Debug)]
pub struct OffsetTable {
    logical_to_physical: Vec<Option<usize>>,
    physical_to_logical: Vec<usize>,
}

impl OffsetTable {
    pub fn new() -> Self {
        Self {
            logical_to_physical: Vec::new(),
            physical_to_logical: Vec::new(),
        }
    }

    pub fn assign(&mut self, physical: usize) -> usize {
        let logical = self.logical_to_physical.len();
        self.logical_to_physical.push(Some(physical));
        if physical >= self.physical_to_logical.len() {
            self.physical_to_logical.resize(physical + 1, usize::MAX);
        }
        self.physical_to_logical[physical] = logical;
        logical
    }

    pub fn resolve(&self, logical: usize) -> Option<usize> {
        self.logical_to_physical.get(logical).copied().flatten()
    }

    pub fn invalidate(&mut self, logical: usize) {
        if let Some(entry) = self.logical_to_physical.get_mut(logical) {
            if let Some(phys) = *entry {
                if phys < self.physical_to_logical.len() {
                    self.physical_to_logical[phys] = usize::MAX;
                }
            }
            *entry = None;
        }
    }

    pub fn is_valid(&self, logical: usize) -> bool {
        self.resolve(logical).is_some()
    }

    pub fn rebuild_from_remap(&mut self, remap: &[(usize, usize)]) {
        // Build old_phys → new_phys lookup
        let mut phys_map: Vec<(usize, usize)> = Vec::new();
        for &(old_phys, new_phys) in remap {
            phys_map.push((old_phys, new_phys));
        }

        // Update logical → physical mappings
        for entry in &mut self.logical_to_physical {
            if let Some(old_phys) = *entry {
                let new = phys_map.iter().find(|&&(o, _)| o == old_phys);
                *entry = new.map(|&(_, n)| n);
            }
        }

        // Rebuild physical → logical
        let max_phys = remap.iter().map(|&(_, new)| new).max().unwrap_or(0);
        self.physical_to_logical.clear();
        self.physical_to_logical.resize(max_phys + 1, usize::MAX);
        for (logical, entry) in self.logical_to_physical.iter().enumerate() {
            if let Some(phys) = *entry {
                if phys < self.physical_to_logical.len() {
                    self.physical_to_logical[phys] = logical;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.logical_to_physical.len()
    }

    pub fn live_count(&self) -> usize {
        self.logical_to_physical.iter().filter(|e| e.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_and_resolve() {
        let mut ot = OffsetTable::new();
        let id0 = ot.assign(0);
        let id1 = ot.assign(1);
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(ot.resolve(0), Some(0));
        assert_eq!(ot.resolve(1), Some(1));
    }

    #[test]
    fn test_invalidate() {
        let mut ot = OffsetTable::new();
        ot.assign(0);
        ot.assign(1);
        ot.invalidate(0);
        assert!(!ot.is_valid(0));
        assert!(ot.is_valid(1));
    }

    #[test]
    fn test_rebuild_from_remap() {
        let mut ot = OffsetTable::new();
        ot.assign(0); // logical 0 → physical 0
        ot.assign(1); // logical 1 → physical 1
        ot.assign(2); // logical 2 → physical 2

        ot.invalidate(1);

        // After compaction: physical 0 stays, physical 2 → physical 1
        ot.rebuild_from_remap(&[(0, 0), (2, 1)]);

        assert_eq!(ot.resolve(0), Some(0));
        assert!(!ot.is_valid(1));
        assert_eq!(ot.resolve(2), Some(1));
    }

    #[test]
    fn test_resolve_out_of_range() {
        let ot = OffsetTable::new();
        assert_eq!(ot.resolve(999), None);
    }
}
