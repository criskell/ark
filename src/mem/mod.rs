use crate::{
    boot::MemoryMap,
    mem::{
        addr::{PhysicalAddr, VirtualAddr},
        hhdm::{Stats, BOOT_HHDM_LIMIT, HHDM_OFFSET},
        pmm::FrameAllocator,
    },
};

pub mod addr;
pub mod hhdm;
pub mod pmm;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct MemoryAddressMapping {
    hhdm_offset: u64,
}

impl MemoryAddressMapping {
    pub fn from_hhdm_offset(hhdm_offset: VirtualAddr) -> Self {
        MemoryAddressMapping {
            hhdm_offset: hhdm_offset.as_u64(),
        }
    }

    pub fn virtual_to_physical(&self, virtual_addr: VirtualAddr) -> PhysicalAddr {
        debug_assert!(virtual_addr.as_u64() >= self.hhdm_offset);

        PhysicalAddr::new(virtual_addr.as_u64() - self.hhdm_offset)
    }

    pub fn physical_to_virtual(&self, physical_addr: PhysicalAddr) -> VirtualAddr {
        VirtualAddr::new(self.hhdm_offset + physical_addr.as_u64())
    }
}

unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

pub fn init(memory_map: MemoryMap) -> (impl FrameAllocator, MemoryAddressMapping, Stats) {
    let memory_address_mapping = MemoryAddressMapping::from_hhdm_offset(HHDM_OFFSET);

    let kernel_start = PhysicalAddr::new(&raw const __kernel_start as u64);
    let kernel_end = PhysicalAddr::new(&raw const __kernel_end as u64);
    let kernel_range = kernel_start..kernel_end;

    let mut frame_allocator = pmm::init(
        BOOT_HHDM_LIMIT,
        memory_address_mapping,
        &memory_map,
        &[kernel_range],
    );

    // SAFETY: called a signle time during boot, before any code modifies the mapping.
    let stats = unsafe { hhdm::init(&memory_map, memory_address_mapping, &mut frame_allocator) };

    frame_allocator.set_limit(PhysicalAddr::new(u64::MAX));

    (frame_allocator, memory_address_mapping, stats)
}
