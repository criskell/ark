use core::mem::size_of;

use super::{KernelBootContext, MemoryMap, MemoryRegion, MemoryRegionKind};
use crate::kernel_enter;

const MULTIBOOT2_MAGIC: u32 = 0x36d76289;
const TAG_END: u32 = 0;
const TAG_MEMORY_MAP: u32 = 6;

#[no_mangle]
pub extern "C" fn multiboot2_kernel_enter(magic: u32, boot_info_addr: u32) -> ! {
    assert_eq!(magic, MULTIBOOT2_MAGIC, "Invalid multiboot2 magic.");

    let boot_info = unsafe { BootInfo::from_addr(boot_info_addr as usize) };

    kernel_enter(KernelBootContext::from_boot_info(boot_info));
}

impl KernelBootContext {
    fn from_boot_info(boot_info: &BootInfo) -> KernelBootContext {
        let mut memory_map = None;

        for tag in boot_info.tags() {
            match tag {
                Tag::MemoryMap(multiboot2_map) => memory_map = Some(multiboot2_map.to_memory_map()),
                Tag::Other => {}
            }
        }

        KernelBootContext {
            memory_map: memory_map.expect("bootloader does not give memory map"),
        }
    }
}

#[repr(C)]
struct BootInfo {
    total_size: u32,
    _reserved: u32,
}

#[repr(C)]
struct TagHeader {
    typ: u32,
    size: u32,
}

#[repr(C)]
struct MemoryMapTag {
    header: TagHeader,
    entry_size: u32,
    _entry_version: u32,
}

#[repr(C)]
struct MemoryMapEntry {
    addr: u64,
    len: u64,
    typ: u32,
    _reserved: u32,
}

enum Tag<'a> {
    MemoryMap(&'a MemoryMapTag),
    Other,
}

impl BootInfo {
    unsafe fn from_addr<'a>(addr: usize) -> &'a BootInfo {
        assert!(
            addr != 0 && addr & 7 == 0,
            "multiboot2 boot info was provided at an invalid memory address."
        );

        unsafe { &*(addr as *const BootInfo) }
    }

    fn tags(&self) -> impl Iterator<Item = Tag<'_>> {
        let start = self as *const Self as usize;
        let end = start + self.total_size as usize;
        let mut cur = start + size_of::<Self>();

        core::iter::from_fn(move || {
            if cur + size_of::<TagHeader>() > end {
                return None;
            }

            let header = unsafe { &*(cur as *const TagHeader) };
            let size = header.size as usize;

            if header.typ == TAG_END || size < size_of::<TagHeader>() {
                return None;
            }

            let tag = match header.typ {
                TAG_MEMORY_MAP => Tag::MemoryMap(unsafe { &*(cur as *const MemoryMapTag) }),
                _ => Tag::Other,
            };

            cur += size.next_multiple_of(8);

            Some(tag)
        })
    }
}

impl MemoryMapTag {
    fn to_memory_map(&self) -> MemoryMap {
        let mut map = MemoryMap::new();

        for entry in self.entries() {
            map.push(MemoryRegion::from(entry))
                .expect("the multiboot2 bootloader memory map has too many regions");
        }

        map
    }

    fn entries(&self) -> impl Iterator<Item = &MemoryMapEntry> {
        let start = self as *const Self as usize;
        let end = start + self.header.size as usize;
        let step = self.entry_size as usize;

        let mut cur = start + size_of::<Self>();

        core::iter::from_fn(move || {
            if step < size_of::<MemoryMapEntry>() || cur + size_of::<MemoryMapEntry>() > end {
                return None;
            }

            let entry = unsafe { &*(cur as *const MemoryMapEntry) };

            cur += step;

            Some(entry)
        })
    }
}

impl From<&MemoryMapEntry> for MemoryRegion {
    fn from(entry: &MemoryMapEntry) -> Self {
        let kind = match entry.typ {
            1 => MemoryRegionKind::Usable,
            3 => MemoryRegionKind::AcpiReclaimable,
            4 => MemoryRegionKind::AcpiNvs,
            5 => MemoryRegionKind::Defective,
            _ => MemoryRegionKind::Reserved,
        };

        MemoryRegion {
            start_address: entry.addr,
            size: entry.len,
            kind,
        }
    }
}
