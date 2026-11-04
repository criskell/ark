use core::arch::x86_64::__cpuid;
use core::fmt;
use core::ops::{BitOrAssign, Index, IndexMut, Range};
use core::{arch::asm, ops::BitOr};

use crate::mem::addr::{PhysicalAddr, VirtualAddr};
use crate::mem::MapAttributes;
use crate::mem::{pmm::FrameAllocator, MemoryAddressMapping};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MapSize(u64);

impl MapSize {
    pub const fn byte_count(self) -> u64 {
        self.0
    }

    const fn leaf_page_table_level(self) -> usize {
        match self.0 {
            0x4000_0000 => 1, // 1 GiB
            0x20_0000 => 2,   // 2 MiB
            0x1000 => 3,      // 4 KiB
            _ => unreachable!(),
        }
    }
}

const SIZE_4K: MapSize = MapSize(4 * 1024);
const SIZE_2M: MapSize = MapSize(2 * 1024 * 1024);
const SIZE_1G: MapSize = MapSize(1024 * 1024 * 1024);

impl fmt::Display for MapSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            b if b >= 1 << 30 => write!(f, "{} GiB", b >> 30),
            b if b >= 1 << 20 => write!(f, "{} MiB", b >> 20),
            b => write!(f, "{} KiB", b >> 10),
        }
    }
}

impl fmt::Debug for MapSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub fn supported_map_sizes() -> &'static [MapSize] {
    if supports_1g_pages() {
        &[SIZE_1G, SIZE_2M, SIZE_4K]
    } else {
        &[SIZE_2M, SIZE_4K]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    pub writable: bool,
    pub executable: bool,
    pub user_accessible: bool,
}

impl Permissions {
    pub const READ_ONLY: Self = Self {
        writable: false,
        executable: false,
        user_accessible: false,
    };

    pub const READ_WRITE: Self = Self {
        writable: true,
        executable: false,
        ..Self::READ_ONLY
    };

    pub const READ_EXECUTE: Self = Self {
        executable: true,
        ..Self::READ_ONLY
    };
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped,
    Conflict,
    OutOfMemory,
}

pub struct AddressSpace {
    root: PhysicalAddr,
    mapping: MemoryAddressMapping,
}

impl AddressSpace {
    /// # Safety
    ///
    /// There can be only one AddressSpace for the current address space accessible via `mapping`.
    pub unsafe fn current(mapping: MemoryAddressMapping) -> Self {
        Self {
            root: active_root_table(),
            mapping,
        }
    }

    /// # Safety
    ///
    /// The structures of the current address space and the memory provided
    /// by the frame allocator must be accessible via the `mapping`.
    pub unsafe fn copy_current_except(
        excluding: Range<VirtualAddr>,
        mapping: MemoryAddressMapping,
        frames: &mut impl FrameAllocator,
    ) -> Result<Self, MapError> {
        assert!(excluding.start < excluding.end, "empty range");

        let frame = frames.allocate().ok_or(MapError::OutOfMemory)?;
        let mut space = Self {
            root: frame,
            mapping,
        };

        // SAFETY: Guaranteed by the one who calls.
        let entries = unsafe { space.table_mut(active_root_table()) }.0;

        // SAFETY: No one uses this frame because it has just been released from the allocator.
        let root = unsafe { space.table_mut(frame) };
        root.0 = entries;

        let first = page_table_indices(excluding.start)[0];
        let last = page_table_indices(excluding.end - 1)[0];
        root.0[first..=last].fill(PageTableEntry::EMPTY);

        Ok(space)
    }

    pub fn map(
        &mut self,
        virtual_addr: VirtualAddr,
        physical_addr: PhysicalAddr,
        size: MapSize,
        attributes: MapAttributes,
        frames: &mut impl FrameAllocator,
    ) -> Result<(), MapError> {
        assert!(
            virtual_addr.is_aligned(size.byte_count())
                && physical_addr.is_aligned(size.byte_count()),
            "address misaligned for {size:?}: {virtual_addr:#x} -> {physical_addr:#x}"
        );

        let indices = page_table_indices(virtual_addr);
        let leaf_level = size.leaf_page_table_level();

        let mut table = self.root;
        for &index in &indices[..leaf_level] {
            table = self.next_table(table, index, frames)?;
        }

        // SAFETY: `table` is a valid table, obtained above.
        let entry = &mut unsafe { self.table_mut(table) }[indices[leaf_level]];
        if entry.is_present() {
            return Err(MapError::AlreadyMapped);
        }

        *entry = if size == SIZE_4K {
            PageTableEntry::page(physical_addr, attributes)
        } else {
            PageTableEntry::huge_page(physical_addr, attributes)
        };

        invalidate_virtual_translation_cache(virtual_addr);
        Ok(())
    }

    pub fn translate_virtual_addr(&self, virtual_addr: VirtualAddr) -> Option<PhysicalAddr> {
        let indices = page_table_indices(virtual_addr);
        let mut table = self.root;

        for (leaf_level, &index) in indices.iter().enumerate() {
            // SAFETY: `table` came from the root or from an existing entry.
            let entry = unsafe { self.table(table) }[index];
            if !entry.is_present() {
                return None;
            }

            let region_size = 1u64 << (12 + 9 * (3 - leaf_level));
            if leaf_level == 3 || entry.is_huge() {
                return Some(entry.addr() + virtual_addr.as_u64() % region_size);
            }

            table = entry.addr();
        }

        None
    }

    /// # Safety
    ///
    /// Address space need to map everything the kernel uses from this point on.
    pub unsafe fn activate(&self) {
        // SAFETY: Guaranteed by the caller.
        unsafe {
            asm!("mov cr3, {}", in(reg) self.root.as_u64(), options(nostack, preserves_flags) );
        }
    }

    fn next_table(
        &mut self,
        table: PhysicalAddr,
        index: usize,
        frames: &mut impl FrameAllocator,
    ) -> Result<PhysicalAddr, MapError> {
        // SAFETY: `table` came from `self.root` or from a valid present entry.
        let entry = unsafe { self.table_mut(table) }[index];

        if entry.is_present() {
            return if entry.is_huge() {
                Err(MapError::Conflict)
            } else {
                Ok(entry.addr())
            };
        }

        let frame = frames.allocate().ok_or(MapError::OutOfMemory)?;

        // SAFETY: The frame has just left the pmm and is no longer being used by anyone.
        let new_table = unsafe { self.table_mut(frame) };
        new_table.0.fill(PageTableEntry::EMPTY);

        // SAFETY: Same original parent table safety.
        let parent = unsafe { self.table_mut(table) };
        parent[index] = PageTableEntry::table(frame);

        Ok(frame)
    }

    /// # Safety
    ///
    /// `physical_addr` must be a page table accesible via `self.mapping`.
    unsafe fn table_mut(&mut self, physical_addr: PhysicalAddr) -> &mut PageTable {
        // SAFETY: Is its guaranteed by the caller, since the mutable borrow to
        // `self` prevents two tables from being modified simultaneously by this AddressSpace.
        unsafe {
            &mut *self
                .mapping
                .physical_to_virtual(physical_addr)
                .as_mut_ptr::<PageTable>()
        }
    }

    /// # Safety
    ///
    /// `physical_addr` must be a page table accesible via `self.mapping`.
    unsafe fn table(&self, physical_addr: PhysicalAddr) -> &PageTable {
        // SAFETY: Guaranteed by the caller.
        unsafe {
            &*self
                .mapping
                .physical_to_virtual(physical_addr)
                .as_mut_ptr::<PageTable>()
        }
    }
}

const ENTRY_COUNT: usize = 512;

const ADDR_MASK: u64 = ((1 << 52) - 1) & !0xFFF; // bits 12..=51

mod bits {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const CACHE_DISABLE: u64 = 1 << 4;
    pub const HUGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const NO_EXECUTE: u64 = 1 << 63;
}

impl MapAttributes {
    fn entry_bits(self) -> u64 {
        use bits::*;

        match self {
            MapAttributes::KernelReadOnly => NO_EXECUTE,
            MapAttributes::KernelReadWrite => WRITABLE | NO_EXECUTE,
            MapAttributes::KernelReadExecute => 0,
            MapAttributes::UserReadWrite => WRITABLE | NO_EXECUTE | USER,
            MapAttributes::UserReadExecute => USER,
            MapAttributes::Device => WRITABLE | NO_EXECUTE | CACHE_DISABLE,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct PageTableEntry(u64);

impl PageTableEntry {
    const EMPTY: Self = Self(0);

    fn page(addr: PhysicalAddr, attributes: MapAttributes) -> Self {
        Self::with_bits(addr, attributes.entry_bits())
    }

    fn huge_page(addr: PhysicalAddr, attributes: MapAttributes) -> Self {
        Self::with_bits(addr, attributes.entry_bits() | bits::HUGE)
    }

    fn table(addr: PhysicalAddr) -> Self {
        Self::with_bits(addr, bits::WRITABLE | bits::USER)
    }

    fn with_bits(addr: PhysicalAddr, bits: u64) -> Self {
        debug_assert!(
            addr.as_u64() & !ADDR_MASK == 0,
            "invalid addr in an entry: #{addr:#x}"
        );

        Self(addr.as_u64() | bits | bits::PRESENT)
    }

    fn is_present(self) -> bool {
        self.0 & bits::PRESENT != 0
    }

    fn is_huge(self) -> bool {
        self.0 & bits::HUGE != 0
    }

    fn addr(self) -> PhysicalAddr {
        PhysicalAddr::new(self.0 & ADDR_MASK)
    }
}

#[repr(C, align(4096))]
struct PageTable([PageTableEntry; ENTRY_COUNT]);

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

fn active_root_table() -> PhysicalAddr {
    let cr3: u64;

    // SAFETY: there are no side effects to reading the cr3 register.
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    };

    PhysicalAddr::new(cr3 & ADDR_MASK)
}

pub fn supports_1g_pages() -> bool {
    const PDPE1GB: u32 = 1 << 26;

    let max_extended_leaf = __cpuid(0x8000_0000).eax;
    if max_extended_leaf < 0x8000_0001 {
        return false;
    }

    __cpuid(0x8000_0001).edx & PDPE1GB != 0
}

fn page_table_indices(virtual_addr: VirtualAddr) -> [usize; 4] {
    [39, 30, 21, 12].map(|shift| ((virtual_addr.as_u64() >> shift) & 0x1ff) as usize)
}

fn invalidate_virtual_translation_cache(virtual_addr: VirtualAddr) {
    unsafe {
        // SAFETY: Only invalidates a TLB entry.
        asm!("invlpg [{}]", in(reg) virtual_addr.as_u64(), options(nostack, preserves_flags));
    }
}
