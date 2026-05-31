#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(ark::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::{arch::asm, panic::PanicInfo};

use ark::{
    arch::x86::{gdt, idt, ring3},
    pci, println,
    screen::vga,
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

    vga::VGA_SCREEN.lock().clear_screen();
    pci::visit_buses();

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("Panicked.");

    loop {}
}
