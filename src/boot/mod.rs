use core::ops::Range;

mod multiboot2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    AcpiReclaimable,
    AcpiNvs,
    Defective,
    Reserved,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start_address: u64,
    pub size: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    const EMPTY: Self = Self {
        start_address: 0,
        size: 0,
        kind: MemoryRegionKind::Reserved,
    };

    pub const fn end(&self) -> u64 {
        self.start_address + self.size
    }

    pub fn range(&self) -> Range<u64> {
        self.start_address..self.end()
    }
}

#[derive(Debug)]
pub struct MemoryMapFull;

pub struct MemoryMap<const C: usize = 128> {
    regions: [MemoryRegion; C],
    len: usize,
}

impl<const C: usize> MemoryMap<C> {
    pub fn new() -> Self {
        Self {
            regions: [MemoryRegion::EMPTY; C],
            len: 0,
        }
    }

    pub fn push(&mut self, region: MemoryRegion) -> Result<(), MemoryMapFull> {
        let slot = self.regions.get_mut(self.len).ok_or(MemoryMapFull)?;

        *slot = region;
        self.len += 1;

        Ok(())
    }

    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions[..self.len]
    }

    pub fn usable(&self) -> impl Iterator<Item = &MemoryRegion> {
        self.regions()
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable)
    }
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self::new()
    }
}

pub struct KernelBootContext {
    pub memory_map: MemoryMap,
}
