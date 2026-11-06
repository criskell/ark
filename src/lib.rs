#![no_std]
#![feature(abi_x86_interrupt)]
#![cfg_attr(test, no_main)]

use core::panic::PanicInfo;

use crate::{
    arch::{cpu, interrupts::idt},
    boot::KernelBootContext,
    mem::{addr::PhysicalAddr, pmm},
};

#[macro_use]
pub mod device;

pub mod arch;
pub mod boot;
pub mod mem;

pub fn kernel_enter(context: KernelBootContext) -> ! {
    idt::init_idt();

    let hhdm_stats = mem::init(&context);

    for (size, bytes) in hhdm_stats.iter() {
        println!("direct map in {size}: {} MiB", bytes >> 20);
    }

    println!("free: {} MiB", pmm::free_frames() * 4 / 1024);

    let vga = mem::direct_map()
        .physical_to_virtual(PhysicalAddr::new(0xb8000))
        .as_mut_ptr::<u16>();

    unsafe {
        vga.write_volatile(0x0F41);
    }

    cpu::idle_forever()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    cpu::halt()
}
