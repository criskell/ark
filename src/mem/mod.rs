use spin::Once;

use crate::{
    boot::KernelBootContext,
    mem::{
        addr::{PhysicalAddr, VirtualAddr},
        direct_map::Stats,
        pmm::memblock::MemBlock,
    },
};

pub mod addr;
pub mod direct_map;
pub mod pmm;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct MemoryAddressMapping {
    direct_map_offset: u64,
}

impl MemoryAddressMapping {
    pub const fn from_hhdm_offset(direct_map_offset: VirtualAddr) -> Self {
        MemoryAddressMapping {
            direct_map_offset: direct_map_offset.as_u64(),
        }
    }

    pub fn virtual_to_physical(&self, virtual_addr: VirtualAddr) -> PhysicalAddr {
        debug_assert!(virtual_addr.as_u64() >= self.direct_map_offset);

        PhysicalAddr::new(virtual_addr.as_u64() - self.direct_map_offset)
    }

    pub fn physical_to_virtual(&self, physical_addr: PhysicalAddr) -> VirtualAddr {
        VirtualAddr::new(self.direct_map_offset + physical_addr.as_u64())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapAttributes {
    KernelReadOnly,
    KernelReadWrite,
    KernelReadExecute,
    UserReadWrite,
    UserReadExecute,
    Device,
}

static DIRECT_MAP: Once<MemoryAddressMapping> = Once::new();

pub fn direct_map() -> MemoryAddressMapping {
    *DIRECT_MAP.r#try().expect("mem::init has not run yet")
}

pub fn init(context: &KernelBootContext) -> Stats {
    let boot_direct_map = context.direct_map;
    let mapping =
        *DIRECT_MAP.call_once(|| MemoryAddressMapping::from_hhdm_offset(boot_direct_map.offset));

    // SAFETY: the bootloader mapped everything below `boot_direct_map.limit`.
    let memblock = unsafe { MemBlock::new(&context.memory_map, boot_direct_map.limit) };

    // SAFETY: called a single time during boot and memblock only hands out memory the boot direct map covers.
    let stats = unsafe { direct_map::init(&context.memory_map, mapping, &memblock) };

    // SAFETY: `direct_map` has mapped all physical RAM.
    unsafe { memblock.set_current_limit(PhysicalAddr::new(u64::MAX)) };

    pmm::init(memblock);

    stats
}
