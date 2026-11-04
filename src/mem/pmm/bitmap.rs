use core::{ops::Range, slice};

use crate::{
    boot::MemoryMap,
    mem::{
        addr::PhysicalAddr,
        pmm::{FrameAllocator, FRAME_SIZE},
        MemoryAddressMapping,
    },
};

pub struct BitmapFrameAllocator {
    free_frames_bitmap: &'static mut [u64],
    next_word_index: usize,
    free_frames: usize,
    limit: PhysicalAddr,
    mapping: MemoryAddressMapping,
}

impl BitmapFrameAllocator {
    pub const fn new(limit: PhysicalAddr, mapping: MemoryAddressMapping) -> Self {
        Self {
            free_frames_bitmap: &mut [],
            next_word_index: 0,
            free_frames: 0,
            limit,
            mapping,
        }
    }

    pub fn init(&mut self, memory_map: &MemoryMap, reserved_regions: &[Range<PhysicalAddr>]) {
        let ram_end = memory_map
            .usable()
            .map(|region| PhysicalAddr::new(region.end()))
            .max()
            .expect("the memory has no usable RAM");

        let words = ((ram_end.as_u64() / FRAME_SIZE) as usize).div_ceil(64);
        let bitmap_size = (words * 8) as u64;

        let bitmap_start = find_free_area(memory_map, bitmap_size, reserved_regions, self.limit)
            .expect("there is no space for the frame bitmap");

        // SAFETY: Reserved below, and this is free, accessible RAM.
        self.free_frames_bitmap = unsafe {
            slice::from_raw_parts_mut(
                self.mapping
                    .physical_to_virtual(bitmap_start)
                    .as_mut_ptr::<u64>(),
                words,
            )
        };
        self.free_frames_bitmap.fill(0);

        for region in memory_map.usable() {
            let start = PhysicalAddr::new(region.start_address).align_up(FRAME_SIZE);
            let end = PhysicalAddr::new(region.end()).align_down(FRAME_SIZE);

            self.set_availability_range(start..end, true);
        }

        self.reserve(PhysicalAddr::new(0)..PhysicalAddr::new(FRAME_SIZE));

        for range in reserved_regions {
            self.reserve(range.clone());
        }

        self.reserve(bitmap_start..bitmap_start + bitmap_size);
    }

    pub fn reserve(&mut self, range: Range<PhysicalAddr>) {
        let start = range.start.align_down(FRAME_SIZE);
        let end = range.end.min(self.covered()).align_up(FRAME_SIZE);

        self.set_availability_range(start..end, false);
    }

    fn covered(&self) -> PhysicalAddr {
        PhysicalAddr::new(self.free_frames_bitmap.len() as u64 * 64 * FRAME_SIZE)
    }

    fn searchable_words(&self) -> usize {
        let limit_words = self.limit.as_u64() / FRAME_SIZE / 64;

        limit_words.min(self.free_frames_bitmap.len() as u64) as usize
    }

    fn set_availability_range(&mut self, range: Range<PhysicalAddr>, free: bool) {
        let mut frame = range.start;

        while frame < range.end {
            self.set_free(self.frame_index(frame), free);

            frame += FRAME_SIZE;
        }
    }

    fn frame_index(&self, addr: PhysicalAddr) -> usize {
        let frame = (addr.as_u64() / FRAME_SIZE) as usize;

        assert!(
            frame < self.free_frames_bitmap.len() * 64,
            "address outside the bitmap: {addr:?}"
        );

        frame
    }

    fn is_free(&self, frame: usize) -> bool {
        self.free_frames_bitmap[frame / 64] & (1u64 << (frame % 64)) != 0
    }

    fn set_free(&mut self, frame: usize, free: bool) {
        if self.is_free(frame) == free {
            return;
        }

        let mask = 1u64 << (frame % 64);

        if free {
            self.free_frames_bitmap[frame / 64] |= mask;
            self.free_frames += 1;
        } else {
            self.free_frames_bitmap[frame / 64] &= !mask;
            self.free_frames -= 1;
        }
    }
}

impl FrameAllocator for BitmapFrameAllocator {
    fn allocate(&mut self) -> Option<PhysicalAddr> {
        let words_count = self.searchable_words();

        for i in 0..words_count {
            let word_index = (self.next_word_index + i) % words_count;

            if self.free_frames_bitmap[word_index] != 0 {
                let bit = self.free_frames_bitmap[word_index].trailing_zeros() as usize;
                let frame = word_index * 64 + bit;

                self.set_free(frame, false);
                self.next_word_index = word_index;

                return Some(PhysicalAddr::new(frame as u64 * FRAME_SIZE));
            }
        }

        None
    }

    fn deallocate(&mut self, frame: PhysicalAddr) {
        assert!(frame.is_aligned(FRAME_SIZE), "frame misaligned: {frame:?}");
        let index = self.frame_index(frame);

        assert!(!self.is_free(index), "frame double free: {frame:?}");
        self.set_free(index, true);
    }

    fn set_limit(&mut self, limit: PhysicalAddr) {
        assert!(limit >= self.limit, "the limit can only increase");

        self.limit = limit;
    }

    fn free_frames(&self) -> usize {
        self.free_frames
    }
}

fn find_free_area(
    memory_map: &MemoryMap,
    size: u64,
    reserved_regions: &[Range<PhysicalAddr>],
    limit: PhysicalAddr,
) -> Option<PhysicalAddr> {
    for region in memory_map.usable() {
        let mut start =
            PhysicalAddr::new(region.start_address.max(FRAME_SIZE)).align_up(FRAME_SIZE);

        while let Some(reserved_region) = reserved_regions.iter().find(|reserved_region| {
            reserved_region.start < start + size && start < reserved_region.end
        }) {
            start = reserved_region.end.align_up(FRAME_SIZE);
        }

        let end = start + size;

        if end <= PhysicalAddr::new(region.end()) && end <= limit {
            return Some(start);
        }
    }

    None
}
