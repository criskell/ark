use core::arch::asm;

#[repr(C, packed)]
#[derive(Eq, PartialEq, Debug, Clone, Copy, Default)]
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
        self.0 |= (base as u64 & 0xFFFFFF) << 16;
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
    base: &'a GDT,
}

#[inline(always)]
pub const fn segment_selector(rpl: u8, index: u16) -> u16 {
    return (rpl as u16) | index << 3;
}

#[repr(C, packed)]
struct GDT {
    entries: [GdtEntry; 6],
}

static mut GDT: GDT = GDT {
    entries: [
        GdtEntry::new(0, 0, 0, 0),
        GdtEntry::new(!0, 0, 0b10011010, 0b1100),
        GdtEntry::new(!0, 0, 0b10010010, 0b1100),
        GdtEntry::new(!0, 0, 0b11111010, 0b1100),
        GdtEntry::new(!0, 0, 0b11110010, 0b1100),
        GdtEntry::new(!0, 0, 0, 0), // TSS
    ],
};

pub const KERNEL_CODE_SEGMENT_SELECTOR: u16 = segment_selector(0, 1);
pub const KERNEL_DATA_SEGMENT_SELECTOR: u16 = segment_selector(0, 2);
pub const USER_CODE_SEGMENT_SELECTOR: u16 = segment_selector(3, 3);
pub const USER_DATA_SEGMENT_SELECTOR: u16 = segment_selector(3, 4);

const fn tss_descriptor(base: u32, limit: u32) -> GdtEntry {
    let access: u8 = 0b10001001;
    let flags: u8 = 0b0000;

    GdtEntry::new(limit, base, access, flags)
}

#[repr(C, packed)]
pub struct Tss {
    pub prev_tss: u16,
    pub reserved0: u16,
    pub esp0: u32,
    pub ss0: u16,
    pub reserved1: u16,
    pub esp1: u32,
    pub ss1: u16,
    pub reserved2: u16,
    pub esp2: u32,
    pub ss2: u16,
    pub reserved3: u16,
    pub cr3: u32,
    pub eip: u32,
    pub eflags: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub es: u16,
    pub reserved4: u16,
    pub cs: u16,
    pub reserved5: u16,
    pub ss: u16,
    pub reserved6: u16,
    pub ds: u16,
    pub reserved7: u16,
    pub fs: u16,
    pub reserved8: u16,
    pub gs: u16,
    pub reserved9: u16,
    pub ldt: u16,
    pub reserved10: u16,
    pub t_flag: u16,
    pub io_map_base: u16,
}

static mut TSS: Tss = Tss {
    prev_tss: 0,
    reserved0: 0,
    esp0: 0,
    ss0: 0x8, // kernel data segment selector
    reserved1: 0,
    esp1: 0,
    ss1: 0,
    reserved2: 0,
    esp2: 0,
    ss2: 0,
    reserved3: 0,
    cr3: 0,
    eip: 0,
    eflags: 0,
    eax: 0,
    ecx: 0,
    edx: 0,
    ebx: 0,
    esp: 0,
    ebp: 0,
    esi: 0,
    edi: 0,
    es: 0,
    reserved4: 0,
    cs: 0,
    reserved5: 0,
    ss: 0,
    reserved6: 0,
    ds: 0,
    reserved7: 0,
    fs: 0,
    reserved8: 0,
    gs: 0,
    reserved9: 0,
    ldt: 0,
    reserved10: 0,
    t_flag: 0,
    io_map_base: 0xFFFF,
};

#[repr(C, packed)]
struct Stack([u8; 4096]);

static mut KERNEL_STACK: Stack = Stack([0; 4096]);

#[inline(never)]
pub fn install() {
    unsafe {
        let gdtr = GDTR {
            limit: 49,
            #[allow(static_mut_refs)]
            base: &GDT,
        };

        asm!("lgdt [{}]", in(reg) &gdtr);

        asm!(
            "mov ax, {0:x}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            in(reg) KERNEL_DATA_SEGMENT_SELECTOR,
            options(nostack, preserves_flags)
        );

        asm!(
            "call 3f",
            "jmp 4f",
            "3:",
            "pop {tmp}",
            "push {selector:e}",
            "push {tmp}",
            "retf",
            "4:",
            selector = in(reg) KERNEL_CODE_SEGMENT_SELECTOR,
            tmp = out(reg) _,
            options(preserves_flags),
        );

        TSS.esp0 = (&KERNEL_STACK.0[4095]) as *const _ as u32;
        TSS.esp1 = (&KERNEL_STACK.0[4095]) as *const _ as u32;

        #[allow(static_mut_refs)]
        let tss = &TSS as *const _ as u32;

        GDT.entries[5] = tss_descriptor(tss, core::mem::size_of::<Tss>() as u32 - 1);

        let tss_segment_selector: u16 = 5 << 3;
        asm!("ltr {0:x}", in(reg) tss_segment_selector);
    }
}
