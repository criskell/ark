#![no_std]
#![cfg_attr(test, no_main)]

use core::panic::{self, PanicInfo};

#[no_mangle]
pub extern "C" fn kernel_enter() {
    let message = b"Hello WorlZ!";
    let color_byte = 0x1f;

    let mut hello_colored = [color_byte; 24];

    for (i, char_byte) in message.into_iter().enumerate() {
        hello_colored[i * 2] = *char_byte;
    }

    let buffer_ptr = (0xb8000 + 1988) as *mut _;

    unsafe {
        *buffer_ptr = hello_colored;
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// #![feature(custom_test_frameworks)]
// #![test_runner(crate::test_runner)]
// #![reexport_test_harness_main = "test_main"]
// #![feature(abi_x86_interrupt)]

// use crate::qemu::QemuExitCode;
// pub mod arch;
// pub mod memory;
// #[macro_use]
// pub mod screen;
// pub mod pci;
// pub mod qemu;
// pub mod serial;
// pub mod text;

// pub trait Testable {
//     fn run(&self);
// }

// impl<T> Testable for T
// where
//     T: Fn(),
// {
//     fn run(&self) {
//         serial_print!("{}...\t", core::any::type_name::<T>());
//         self();
//         serial_print!("[ok]");
//     }
// }

// pub fn test_runner(tests: &[&dyn Testable]) {
//     println!("Running {} tests", tests.len());

//     for test in tests {
//         test.run();
//     }

//     qemu::exit(QemuExitCode::Success);
// }
