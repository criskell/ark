#![no_std]
#![no_main]

use core::{
    arch::{asm, naked_asm},
    panic::PanicInfo,
};

use ark::{
    println,
    processor::x86::{gdt, idt, ring3},
};

#[link_section = ".multiboot"]
#[no_mangle]
#[used]
pub static MULTIBOOT_HEADER: [u32; 3] = [
    0x1BADB002,
    0b00000000,
    (-(0x1BADB002i32 + 0b00000000i32)) as u32,
];

extern "C" {
    static stack_top: u32;
}

#[no_mangle]
unsafe fn initialize() -> ! {
    asm!(
        "mov esp, {0}",
        "jmp {1}",
        in(reg) &stack_top,
        sym rust_main
    );

    loop {}
}

unsafe fn rust_main() -> ! {
    idt::init_idt();
    gdt::install();
    ring3::switch_to_ring_3();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("Panicked.");

    loop {}
}
