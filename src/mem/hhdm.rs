use crate::{
    arch::address_space::{self, AddressSpace, MapSize, Permissions},
    boot::MemoryMap,
    mem::{
        addr::{PhysicalAddr, VirtualAddr},
        pmm::{FrameAllocator, FRAME_SIZE},
        MemoryAddressMapping,
    },
};

pub const HHDM_OFFSET: VirtualAddr = VirtualAddr::new(!0u64 << 47);
pub const BOOT_HHDM_LIMIT: PhysicalAddr = PhysicalAddr::new(1 << 30);

const ISA_END: PhysicalAddr = PhysicalAddr::new(0x100000);

const MAX_RANGES: usize = 129;
const MAX_SIZES: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    sizes: &'static [MapSize],
    bytes: [u64; MAX_SIZES],
}

impl Stats {
    pub fn iter(&self) -> impl Iterator<Item = (MapSize, u64)> + '_ {
        self.sizes.iter().copied().zip(self.bytes)
    }
}

/// # Safety
///
/// It must be called excatly once during boot, and the memory provided by
/// the frame allocator must be accessible via the virtual addresses
/// returned by `mapping`.
pub unsafe fn init(
    memory_map: &MemoryMap,
    mapping: MemoryAddressMapping,
    frames: &mut impl FrameAllocator,
) -> Stats {
    let ranges = memory_ranges(memory_map);
    let ram_end = ranges
        .as_slice()
        .last()
        .map_or(FRAME_SIZE, |&(_, end)| end.as_u64());

    let sizes = address_space::supported_map_sizes();

    assert!(sizes.len() <= MAX_SIZES);

    let mut stats = Stats {
        sizes,
        bytes: [0; MAX_SIZES],
    };

    let start = mapping.physical_to_virtual(PhysicalAddr::new(0));

    // SAFETY: Guaranteed by the caller.
    let mut space = unsafe {
        AddressSpace::copy_current_except(start..start + ram_end, mapping, frames)
            .expect("failed to create the new address space")
    };

    for &(range_start, range_end) in ranges.as_slice() {
        let mut current_physical_addr = range_start;

        while current_physical_addr < range_end {
            let (i, size) = sizes
                .iter()
                .copied()
                .enumerate()
                .find(|(_, size)| {
                    current_physical_addr.is_aligned(size.byte_count())
                        && current_physical_addr + size.byte_count() <= range_end
                })
                .expect("range misaligned");

            space
                .map(
                    mapping.physical_to_virtual(current_physical_addr),
                    current_physical_addr,
                    size,
                    Permissions::READ_WRITE,
                    frames,
                )
                .expect("failed to mount HHDM map");

            stats.bytes[i] += size.byte_count();
            current_physical_addr += size.byte_count();
        }
    }

    // SAFETY: the new space is a copy of the current one, with the identity map includeed, but with the HHDM adjusted to cover the entire RAM.
    unsafe { space.activate() };

    stats
}

fn memory_ranges(memory_map: &MemoryMap) -> Ranges {
    let mut ranges = Ranges::new();

    // Dont forget to map the ISA range
    ranges.push(PhysicalAddr::new(0), ISA_END);

    for region in memory_map.regions() {
        if region.is_ram() {
            let start = PhysicalAddr::new(region.start_address).align_down(FRAME_SIZE);
            let end = PhysicalAddr::new(region.end()).align_up(FRAME_SIZE);

            ranges.push(start, end);
        }
    }

    ranges.sort_and_merge();
    ranges
}

struct Ranges {
    items: [(PhysicalAddr, PhysicalAddr); MAX_RANGES],
    len: usize,
}

impl Ranges {
    const fn new() -> Self {
        Self {
            items: [(PhysicalAddr::new(0), PhysicalAddr::new(0)); MAX_RANGES],
            len: 0,
        }
    }

    fn push(&mut self, start: PhysicalAddr, end: PhysicalAddr) {
        assert!(
            self.len < MAX_RANGES,
            "too many RAM ranges in the memory map"
        );

        if start < end {
            self.items[self.len] = (start, end);
            self.len += 1;
        }
    }

    fn as_slice(&self) -> &[(PhysicalAddr, PhysicalAddr)] {
        &self.items[..self.len]
    }

    fn sort_and_merge(&mut self) {
        self.items[..self.len].sort_unstable();

        let mut merged = 0;

        for i in 0..self.len {
            let (start, end) = self.items[i];

            if merged > 0 && start <= self.items[merged - 1].1 {
                let last = &mut self.items[merged - 1].1;
                *last = (*last).max(end);
            } else {
                self.items[merged] = (start, end);
                merged += 1;
            }
        }

        self.len = merged;
    }
}
