use core::fmt;

static mut CONSOLES: [Option<&'static dyn fmt::Write>; 16] = [const { None }; 16];

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        unsafe {
            #[allow(static_mut_refs)]
            SERIAL.write_fmt(args).unwrap();
        }
        VGA_SCREEN.lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::device::debug::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
