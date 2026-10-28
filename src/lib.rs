#![no_std]
#![feature(abi_x86_interrupt)]
#![cfg_attr(test, no_main)]

// use core::panic::{self, PanicInfo};

use core::panic::PanicInfo;

use crate::boot::KernelBootContext;

pub mod arch;
pub mod memory;
#[macro_use]
pub mod device;
pub mod boot;

pub fn kernel_enter(context: KernelBootContext) -> ! {
    let total_memory: u64 = context.memory_map.usable().map(|r| r.size).sum();

    println!("total_memory: {} MiB", total_memory / (1024 * 1024));

    for region in context.memory_map.regions() {
        println!(
            " {:#018x} - {:#018x}  {:?}",
            region.start_address,
            region.end(),
            region.kind
        );
    }

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    loop {}
}
