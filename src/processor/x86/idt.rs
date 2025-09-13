use core::arch::asm;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IdtEntry {
    /// Handler (low) address.
    offset_low: u16,

    /// Code segment.
    selector: u16,

    /// Optional stack.
    ist: u8,

    /// Gate (trap, interrupt) type + privileges.
    attributes: u8,

    /// Handler (middle) address.
    offset_middle: u16,

    /// Handler (high) address.
    offset_high: u32,

    /// Reserved bits.
    zero: u32,
}

impl IdtEntry {
    const fn new() -> IdtEntry {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist: 0,
            attributes: 0,
            offset_middle: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    fn set_handler(&mut self, handler: extern "C" fn()) {
        let addr = handler as u64;

        self.offset_low = addr as u16;
        self.offset_middle = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        // kernel code segment
        self.selector = 0x08;
        self.attributes = 0x8E; // interrupt gate, present
    }
}

#[repr(C, align(16))]
struct Idt {
    entries: [IdtEntry; 256],
}

static mut IDT: Idt = Idt {
    entries: [IdtEntry::new(); 256],
};

pub fn init_idt() {
    unsafe {
        extern "C" fn divide_by_zero_handler() {
            crate::screen::vga::VGA_SCREEN.lock().write_char('Z');
        }

        IDT.entries[0].set_handler(divide_by_zero_handler);

        lidt(&raw const IDT);
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

unsafe fn lidt(idt: *const Idt) {
    let idtr = Idtr {
        base: idt as *const _ as u64,
        limit: (core::mem::size_of::<Idt>() - 1) as u16,
    };

    asm!("lidt [{}]", in(reg) &idtr, options(nostack, preserves_flags));
}
