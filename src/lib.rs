#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

use crate::qemu::QemuExitCode;
pub mod arch;
pub mod mem;
#[macro_use]
pub mod screen;
pub mod qemu;
pub mod serial;
pub mod text;

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_print!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());

    for test in tests {
        test.run();
    }

    qemu::exit(QemuExitCode::Success);
}
