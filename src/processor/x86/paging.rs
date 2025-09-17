use core::arch::asm;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageEntry(pub u32);

impl PageEntry {
    pub fn new(addr: u32, flags: u32) -> Self {
        Self((addr & 0xFFFFF000) | (flags & 0xFFF))
    }
}

const PAGE_PRESENT: u32 = 1 << 0;
const PAGE_WRITABLE: u32 = 1 << 1;
#[allow(dead_code)]
const PAGE_USER: u32 = 1 << 2;

#[repr(align(4096))]
pub struct PageTable([PageEntry; 1024]);

static mut PAGE_TABLE: PageTable = PageTable([PageEntry(0); 1024]);
static mut PAGE_DIRECTORY: PageTable = PageTable([PageEntry(0); 1024]);

pub unsafe fn install_paging() {
    for i in 0..1024 {
        PAGE_TABLE.0[i] = PageEntry::new((i * 0x1000) as u32, PAGE_PRESENT | PAGE_WRITABLE);
    }

    #[allow(static_mut_refs)]
    let page_table = &PAGE_TABLE as *const _ as u32;

    PAGE_DIRECTORY.0[0] = PageEntry::new(page_table, PAGE_PRESENT | PAGE_WRITABLE);

    #[allow(static_mut_refs)]
    let page_directory = &PAGE_DIRECTORY as *const _ as u32;

    asm!("mov cr3, {0:e}", in(reg) page_directory, options(nostack, nomem));

    let mut cr0: u32;
    asm!(
      "mov {0:e}, cr0",
      out(reg) cr0
    );
    cr0 |= 0x80000000; // set pg bit (31)

    asm!("mov cr0, {0:e}", in(reg) cr0);
}
