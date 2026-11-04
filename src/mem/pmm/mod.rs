use core::ops::Range;

use crate::{
    boot::MemoryMap,
    mem::{addr::PhysicalAddr, pmm::bitmap::BitmapFrameAllocator, MemoryAddressMapping},
};

pub mod bitmap;

pub const FRAME_SIZE: u64 = 4096;

pub trait FrameAllocator {
    fn allocate(&mut self) -> Option<PhysicalAddr>;
    fn deallocate(&mut self, frame: PhysicalAddr);
    fn set_limit(&mut self, limit: PhysicalAddr);
    fn free_frames(&self) -> usize;
}

pub fn init(
    initial_mapping_limit: PhysicalAddr,
    mapping: MemoryAddressMapping,
    memory_map: &MemoryMap,
    reserved_regions: &[Range<PhysicalAddr>],
) -> impl FrameAllocator {
    let mut allocator = BitmapFrameAllocator::new(initial_mapping_limit, mapping);

    allocator.init(&memory_map, reserved_regions);

    allocator
}
