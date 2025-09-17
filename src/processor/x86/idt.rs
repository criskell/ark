use core::arch::asm;

use crate::{
    println,
    processor::x86::{interrupts, io},
};

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    zero: u8,
    type_attr: u8,
    offset_high: u16,
}

impl IdtEntry {
    const fn new() -> IdtEntry {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            zero: 0,
            type_attr: 0,
            offset_high: 0,
        }
    }

    unsafe fn set_handler(&mut self, handler_virtual_addr: u64) {
        self.offset_low = (handler_virtual_addr & 0xFFFF) as u16;
        self.offset_high = ((handler_virtual_addr >> 16) & 0xFFFF) as u16;
        self.selector = 0x8;
        self.type_attr = 0x8E | 0x60;
    }
}

#[repr(C, packed)]
pub struct Idt {
    entries: [IdtEntry; 256],
}

pub static mut IDT: Idt = Idt {
    entries: [IdtEntry::new(); 256],
};

#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub instruction_pointer: u32,
    pub code_segment_selector: u16,
    pub eflags: u32,
    pub stack_pointer: u32,
    pub stack_segment_selector: u16,
}

extern "x86-interrupt" fn divide_by_zero_handler(mut stack_frame: InterruptStackFrame) {
    crate::screen::vga::VGA_SCREEN.lock().write_char('Z');

    // FIXME: Just to test. Should panic instead.
    stack_frame.instruction_pointer += 2;

    loop {}
}

extern "C" fn double_fault_handler() {
    crate::screen::vga::VGA_SCREEN
        .lock()
        .write_string("Double fault");

    loop {}
}

#[no_mangle]
fn general_protection_fault_handler(error_code: u32) {
    println!("GENERAL PROTECTION FAULT {:#?}", error_code);

    loop {}
}

#[no_mangle]
fn general_protection_fault_handler_stub() {
    unsafe {
        asm!(
            "add esp, 24",
            "push eax",
            "push 0",
            "jmp general_protection_fault_handler"
        );
    }
}

pub fn init_idt() {
    unsafe {
        IDT.entries[0].set_handler(divide_by_zero_handler as u64);
        IDT.entries[8].set_handler(double_fault_handler as u64);
        IDT.entries[13].set_handler(general_protection_fault_handler_stub as u64);

        lidt(&raw const IDT);
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u32,
}

unsafe fn lidt(idt: *const Idt) {
    let idtr = Idtr {
        limit: (256 * 8) - 1,
        base: idt as *const _ as u32,
    };

    asm!("lidt [{}]", in(reg) &idtr);

    unsafe {
        io::outportb(0x21, 0xFF);
        io::outportb(0xA1, 0xFF);
    }

    interrupts::enable();
}
