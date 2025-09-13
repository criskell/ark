use core::arch::asm;

#[repr(C, align(8))]
#[derive(Eq, PartialEq, Debug)]
pub struct GdtEntry(u64);

impl GdtEntry {
    pub const fn new(limit: u32, base: u32, access: u8, flags: u8) -> Self {
        let mut entry = Self(0);

        entry.set_limit(limit);
        entry.set_base(base);
        entry.set_access(access);
        entry.set_flags(flags);

        entry
    }

    #[inline(always)]
    const fn set_limit(&mut self, limit: u32) {
        self.0 |= limit as u64 & 0xFFFF;
        self.0 |= ((limit as u64 >> 16) & 0xF) << 48;
    }

    #[inline(always)]
    const fn set_base(&mut self, base: u32) {
        self.0 |= (base as u64 & 0xFFFF) << 16;
        self.0 |= (base as u64 & 0xFF0000) << 32;
        self.0 |= (base as u64 & 0xFF000000) << 56;
    }

    #[inline(always)]
    const fn set_access(&mut self, access: u8) {
        self.0 |= (access as u64) << 40;
    }

    #[inline(always)]
    const fn set_flags(&mut self, flags: u8) {
        self.0 |= ((flags as u64) & 0x0F) << 52;
    }
}

#[repr(C, packed)]
struct GDTR<'a> {
    limit: u16,
    base: &'a [GdtEntry],
}

type GDT = [GdtEntry; 3];

static GDT: GDT = [
    // Null segment
    GdtEntry::new(0, 0, 0, 0),
    // Code segment
    // - Access = 0b10011010
    //   - Present = 1
    //   - DPL = 00
    //   - S = 1 (code or data segment)
    //   - Type = 1010 (executable, readable, non-conforming)
    // - Flags = 0b1100
    //   - Granularity = 1 (limit in 4KB pages)
    //   - D/B = 1 (32 bits segment)
    //   - L = 0 (not a 64 bits code segment)
    //   - AVL = 0 (reserved for my os)
    GdtEntry::new(0xFFFFFFFF, 0, 0b10011010, 0b1100),
    // Data segment
    // - Access = 0b10010010
    //   - Present = 1
    //   - DPL = 00
    //   - S = 1 (code or data segment)
    //   - Type = 0010 (writable, readable, up expandable, non-accessed)
    // - Flags = 0b1100
    //   - Granularity = 1 (limit in 4KB pages)
    //   - D/B = 1 (32 bits segment)
    //   - L = 0 (not a 64 bits code segment)
    //   - AVL = 0 (reserved for my os)
    GdtEntry::new(0xFFFFFFFF, 0, 0b10010010, 0b1100),
];

pub fn install() {
    let gdtr = GDTR {
        limit: (size_of::<GDT>() - 1) as u16,
        base: &GDT,
    };

    unsafe {
        asm!("lgdt [{}]", in(reg) &gdtr);
    }
}
