use crate::arch::io;

const COM1_SERIAL_PORT: u16 = 0x3F8;

const STATUS_REGISTER: u16 = 5;

pub fn write_byte(byte: u8) {
    unsafe {
        while io::inportb(COM1_SERIAL_PORT + STATUS_REGISTER) & 0x20 == 0 {}
        io::outportb(COM1_SERIAL_PORT, byte);
    }
}

pub fn write_string(str: &str) {
    for byte in str.bytes() {
        write_byte(byte);
    }
}
