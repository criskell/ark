use core::ops::Range;

use crate::mem::addr::{PhysicalAddr, VirtualAddr};

mod multiboot2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Kernel,
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

    pub const fn is_ram(&self) -> bool {
        matches!(
            self.kind,
            MemoryRegionKind::Usable
                | MemoryRegionKind::Kernel
                | MemoryRegionKind::AcpiReclaimable
                | MemoryRegionKind::AcpiNvs
        )
    }
}

#[derive(Debug)]
pub struct MemoryMapFull;

#[derive(Copy, Clone)]
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

    pub fn mark(&mut self, range: Range<u64>, kind: MemoryRegionKind) -> Result<(), MemoryMapFull> {
        let original = *self;
        self.len = 0;

        for region in original.regions() {
            let start = region.start_address.max(range.start);
            let end = region.end().min(range.end);

            let is_intersecting = start < end;

            if !is_intersecting {
                self.push(*region)?;
                continue;
            }

            let pieces = [
                (region.start_address, start, region.kind), // before range
                (start, end, kind),                         // inside range
                (end, region.end(), region.kind),           // after range
            ];

            for (piece_start, piece_end, piece_kind) in pieces {
                if piece_start < piece_end {
                    self.push(MemoryRegion {
                        start_address: piece_start,
                        size: piece_end - piece_start,
                        kind: piece_kind,
                    })?;
                }
            }
        }

        Ok(())
    }
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BootDirectMap {
    pub offset: VirtualAddr,
    pub limit: PhysicalAddr,
}

pub struct KernelBootContext {
    pub memory_map: MemoryMap,
    pub direct_map: BootDirectMap,
}
