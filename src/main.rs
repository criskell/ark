#![no_std]
#![no_main]

#[link_section = ".multiboot"]
#[no_mangle]
#[used]
pub static MULTIBOOT_HEADER: [u32; 3] = [
    0x1BADB002,
    0b00000000,
    (-(0x1BADB002i32 + 0b00000000i32)) as u32,
];

use ark::{
    processor::x86::{gdt, idt},
    screen::vga::VGA_SCREEN,
};
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    idt::init_idt();

    VGA_SCREEN.lock().clear_screen();
    gdt::install();
    VGA_SCREEN.lock().write_string("Olá, mundo!");

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    VGA_SCREEN.lock().write_string("panicked // TODO");

    loop {}
}
