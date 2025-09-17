use crate::arch::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

const ISA_EXIT_PORT: u16 = 0xF4;

pub fn exit(code: QemuExitCode) {
    unsafe {
        io::outportl(ISA_EXIT_PORT, code as u32);
    }
}
