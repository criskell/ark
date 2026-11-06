use spin::Mutex;

use crate::mem::addr::PhysicalAddr;

pub mod bitmap;
pub(super) mod memblock;

pub const FRAME_SIZE: u64 = 4096;

type SelectedAllocator = bitmap::BitmapFrameAllocator;

static ALLOCATOR: Mutex<SelectedAllocator> = Mutex::new(SelectedAllocator::new());

pub trait FrameAllocator {
    fn allocate(&self) -> Option<PhysicalAddr>;
    fn deallocate(&self, frame: PhysicalAddr);
}

pub struct Frames;

impl FrameAllocator for Frames {
    fn allocate(&self) -> Option<PhysicalAddr> {
        ALLOCATOR.lock().allocate()
    }

    fn deallocate(&self, frame: PhysicalAddr) {
        ALLOCATOR.lock().deallocate(frame)
    }
}

pub(super) fn init(memblock: memblock::MemBlock) {
    ALLOCATOR.lock().init(memblock);
}

pub fn free_frames() -> usize {
    ALLOCATOR.lock().free_frames()
}
