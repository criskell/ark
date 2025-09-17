use core::fmt;
use core::fmt::Arguments;
use core::fmt::Write;

use crate::arch::interrupts;
use crate::arch::serial;

pub struct Serial;

pub static mut SERIAL: Serial = Serial;

impl fmt::Write for Serial {
    fn write_str(&mut self, str: &str) -> fmt::Result {
        serial::write_string(str);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: Arguments) {
    interrupts::without_interrupts(|| unsafe {
        #[allow(static_mut_refs)]
        SERIAL.write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::serial_print!("\n")
    };

    ($fmt:expr) => {
        $crate::serial_print!(concat!($fmt, "\n"))
    };

    ($fmt:expr, $($arg:tt)*) => {
      $crate::serial_print!(concat!($fmt, "\n"), $($arg)*)
    };
}
