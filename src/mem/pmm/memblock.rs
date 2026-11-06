use core::{cell::RefCell, slice};

use crate::{
    boot::{MemoryMap, MemoryRegionKind},
    mem::{
        self,
        addr::{PhysicalAddr, VirtualAddr},
        pmm::{FrameAllocator, FRAME_SIZE},
    },
};

const INITIAL_CAPACITY: usize = 128;

pub struct MemBlock(RefCell<State>);

impl MemBlock {
    pub fn new(memory_map: &MemoryMap, limit: PhysicalAddr) -> Self {
        let mut state = State {
            memory: RegionList::new(),
            can_resize: false,
            reserved: RegionList::new(),
            limit,
        };

        for region in memory_map.regions() {
            let start = PhysicalAddr::new(region.start_address);
            let end = PhysicalAddr::new(region.end());

            match region.kind {
                MemoryRegionKind::Usable => state.add(List::Memory, Range::inward(start, end)),
                MemoryRegionKind::Kernel => {
                    state.add(List::Memory, Range::inward(start, end));
                    state.add(List::Reserved, Range::outward(start, end))
                }
                _ => {}
            }

            state.add(
                List::Reserved,
                Range::new(PhysicalAddr::new(0), PhysicalAddr::new(FRAME_SIZE)),
            );
        }

        MemBlock(RefCell::new(state))
    }

    pub fn set_current_limit(&self, limit: PhysicalAddr) {
        self.0.borrow_mut().limit = limit;
    }

    pub fn allow_resize(&self) {
        self.0.borrow_mut().can_resize = true;
    }

    pub fn allocate_contiguous(&self, size: u64) -> Option<PhysicalAddr> {
        let size = size.next_multiple_of(FRAME_SIZE);
        let mut state = self.0.borrow_mut();

        let start = state.find(size, Range::EMPTY)?;
        state.add(List::Reserved, Range::new(start, start + size));

        Some(start)
    }

    pub fn memory_end(&self) -> PhysicalAddr {
        let state = self.0.borrow();

        state
            .memory
            .ranges()
            .last()
            .map_or(PhysicalAddr::new(0), |range| range.end)
    }

    pub fn for_each_free(&self, f: impl FnMut(PhysicalAddr, PhysicalAddr)) {
        self.0.borrow().for_each_free(f);
    }
}

impl FrameAllocator for MemBlock {
    fn allocate(&self) -> Option<PhysicalAddr> {
        self.allocate_contiguous(FRAME_SIZE)
    }

    fn deallocate(&self, frame: PhysicalAddr) {
        self.0
            .borrow_mut()
            .remove(List::Reserved, Range::new(frame, frame + FRAME_SIZE))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Range {
    start: PhysicalAddr,
    end: PhysicalAddr,
}

impl Range {
    const EMPTY: Self = Self::new(PhysicalAddr::new(0), PhysicalAddr::new(0));

    const fn new(start: PhysicalAddr, end: PhysicalAddr) -> Self {
        Self { start, end }
    }

    fn inward(start: PhysicalAddr, end: PhysicalAddr) -> Self {
        Self::new(start.align_up(FRAME_SIZE), end.align_down(FRAME_SIZE))
    }

    fn outward(start: PhysicalAddr, end: PhysicalAddr) -> Self {
        Self::new(start.align_down(FRAME_SIZE), end.align_up(FRAME_SIZE))
    }

    fn is_empty(self) -> bool {
        self.start >= self.end
    }

    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum List {
    Memory,
    Reserved,
}

struct State {
    memory: RegionList,
    reserved: RegionList,
    limit: PhysicalAddr,
    can_resize: bool,
}

impl State {
    fn list(&self, which: List) -> &RegionList {
        match which {
            List::Memory => &self.memory,
            List::Reserved => &self.reserved,
        }
    }

    fn list_mut(&mut self, which: List) -> &mut RegionList {
        match which {
            List::Memory => &mut self.memory,
            List::Reserved => &mut self.reserved,
        }
    }

    fn add(&mut self, which: List, range: Range) {
        if range.is_empty() {
            return;
        }

        if self.list(which).is_full() {
            self.grow(which, Range::EMPTY);
        }

        let list = self.list_mut(which);
        let index = list
            .ranges()
            .iter()
            .position(|existing| existing.start > range.start)
            .unwrap_or(list.len);

        list.insert(index, range);
        list.merge();
    }

    fn remove(&mut self, which: List, range: Range) {
        if range.is_empty() {
            return;
        }

        if self.list(which).is_full() {
            self.grow(which, Range::EMPTY);
        }

        let list = self.list_mut(which);
        let mut index = 0;

        while index < list.len {
            let existing = list.ranges()[index];

            if !existing.overlaps(range) {
                index += 1;
                continue;
            }

            let left = Range::new(existing.start, range.start);
            let right = Range::new(range.end, existing.end);

            list.remove(index);

            if !right.is_empty() {
                list.insert(index, right);
            }

            if !left.is_empty() {
                list.insert(index, left);
                index += 1;
            }

            if !right.is_empty() {
                index += 1;
            }
        }
    }

    fn for_each_free(&self, mut f: impl FnMut(PhysicalAddr, PhysicalAddr)) {
        for memory in self.memory.ranges() {
            let mut start = memory.start;

            for reserved in self.reserved.ranges() {
                if reserved.end <= start {
                    continue;
                }

                if reserved.start >= memory.end {
                    continue;
                }

                if reserved.start > start {
                    f(start, reserved.start);
                }

                start = start.max(reserved.end);
            }

            if start < memory.end {
                f(start, memory.end);
            }
        }
    }

    fn find(&self, size: u64, exclude: Range) -> Option<PhysicalAddr> {
        let mut best: Option<PhysicalAddr> = None;

        self.for_each_free(|start, end| {
            let free = Range::new(start, end.min(self.limit));

            let pieces = if exclude.overlaps(free) {
                [
                    Range::new(free.start, exclude.start),
                    Range::new(exclude.end, free.end),
                ]
            } else {
                [free, Range::EMPTY]
            };

            for piece in pieces {
                let piece = Range::inward(piece.start, piece.end);

                if piece.is_empty() || piece.end - piece.start < size {
                    continue;
                }

                let candidate = piece.end - size;

                best = Some(best.map_or(candidate, |current| current.max(candidate)));
            }
        });

        best
    }

    fn grow(&mut self, which: List, adding: Range) {
        assert!(
            self.can_resize,
            "memblock: a region list if full and resizing is not allowed yet"
        );

        let old_capacity = self.list(which).capacity();
        let new_capacity = old_capacity * 2;
        let new_size = storage_size(new_capacity);

        let start = self
            .find(new_size, adding)
            .expect("memblock: no memory to grow a region list");

        // SAFETY: range is free RAM, and resizing is only allowed once the direct map
        // covers all RAW below the limit.
        let new_slots: &'static mut [Range] = unsafe {
            slice::from_raw_parts_mut(
                mem::direct_map()
                    .physical_to_virtual(start)
                    .as_mut_ptr::<Range>(),
                new_capacity,
            )
        };

        let list = self.list_mut(which);
        new_slots[..list.len].copy_from_slice(list.ranges());
        let old_storage = core::mem::replace(&mut list.storage, Storage::Allocated(new_slots));

        self.add(List::Reserved, Range::new(start, start + new_size));

        if let Storage::Allocated(old_slots) = old_storage {
            let old_start =
                mem::direct_map().virtual_to_physical(VirtualAddr::new(old_slots.as_ptr() as u64));

            self.remove(
                List::Reserved,
                Range::new(old_start, old_start + storage_size(old_capacity)),
            );
        }
    }
}

fn storage_size(capacity: usize) -> u64 {
    ((capacity * size_of::<Range>()) as u64).next_multiple_of(FRAME_SIZE)
}

#[allow(clippy::large_enum_variant)]
enum Storage {
    Initial([Range; INITIAL_CAPACITY]),
    Allocated(&'static mut [Range]),
}

struct RegionList {
    storage: Storage,
    len: usize,
}

impl RegionList {
    const fn new() -> Self {
        Self {
            storage: Storage::Initial([Range::EMPTY; INITIAL_CAPACITY]),
            len: 0,
        }
    }

    fn slots(&self) -> &[Range] {
        match &self.storage {
            Storage::Initial(slots) => slots,
            Storage::Allocated(slots) => slots,
        }
    }

    fn slots_mut(&mut self) -> &mut [Range] {
        match &mut self.storage {
            Storage::Initial(slots) => slots,
            Storage::Allocated(slots) => slots,
        }
    }

    fn capacity(&self) -> usize {
        self.slots().len()
    }

    fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    fn ranges(&self) -> &[Range] {
        &self.slots()[..self.len]
    }

    fn insert(&mut self, index: usize, range: Range) {
        assert!(!self.is_full(), "memblock: region list overflow");

        let len = self.len;
        let slots = self.slots_mut();

        slots.copy_within(index..len, index + 1);
        slots[index] = range;

        self.len += 1;
    }

    fn remove(&mut self, index: usize) {
        let len = self.len;

        self.slots_mut().copy_within(index + 1..len, index);
        self.len -= 1;
    }

    fn merge(&mut self) {
        let mut index = 0;

        while index + 1 < self.len {
            let current = self.ranges()[index];
            let next = self.ranges()[index + 1];

            if next.start <= current.end {
                self.slots_mut()[index].end = current.end.max(next.end);
                self.remove(index + 1);
            } else {
                index += 1;
            }
        }
    }
}
