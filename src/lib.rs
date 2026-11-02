#![no_std]
#![feature(abi_x86_interrupt)]
#![cfg_attr(test, no_main)]

use core::panic::PanicInfo;

use crate::{arch::idt, boot::KernelBootContext, mem::OffsetMapping};

#[macro_use]
pub mod device;

pub mod arch;
pub mod boot;
pub mod mem;

pub fn kernel_enter(context: KernelBootContext) -> ! {
    idt::init_idt();

    let mapping = OffsetMapping(0xFFFF_8000_0000_0000);

    let vga = mapping.virtual_to_physical(0xb8000) as *mut u16;

    unsafe {
        vga.write_volatile(0x0F41);
    }

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    loop {}
}
