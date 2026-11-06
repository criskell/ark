use core::{ops::Range, slice};

use crate::mem::{
    self,
    addr::PhysicalAddr,
    pmm::{memblock::MemBlock, FRAME_SIZE},
};

pub struct BitmapFrameAllocator {
    free_frames_bitmap: &'static mut [u64],
    next_word_index: usize,
    free_frames: usize,
}

impl BitmapFrameAllocator {
    pub const fn new() -> Self {
        Self {
            free_frames_bitmap: &mut [],
            next_word_index: 0,
            free_frames: 0,
        }
    }

    pub fn init(&mut self, memblock: MemBlock) {
        let words = ((memblock.memory_end().as_u64() / FRAME_SIZE) as usize).div_ceil(64);
        let bitmap_size = (words * 8) as u64;
        let bitmap_start = memblock
            .allocate_contiguous(bitmap_size)
            .expect("there is no space for the frame bitmap");

        // SAFETY: memblock reserved thius memory, and the direct map covers all RAM by now.
        self.free_frames_bitmap = unsafe {
            slice::from_raw_parts_mut(
                mem::direct_map()
                    .physical_to_virtual(bitmap_start)
                    .as_mut_ptr::<u64>(),
                words,
            )
        };
        self.free_frames_bitmap.fill(0);

        memblock.for_each_free(|start, end| {
            self.set_availability_range(start..end, true);
        });
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

    pub fn free_frames(&self) -> usize {
        self.free_frames
    }

    pub fn allocate(&mut self) -> Option<PhysicalAddr> {
        let words_count = self.free_frames_bitmap.len();

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

    pub fn deallocate(&mut self, frame: PhysicalAddr) {
        assert!(frame.is_aligned(FRAME_SIZE), "frame misaligned: {frame:?}");
        let index = self.frame_index(frame);

        assert!(!self.is_free(index), "frame double free: {frame:?}");
        self.set_free(index, true);
    }
}
