use core::arch::asm;

#[inline(always)]
pub unsafe fn outportb(port: u16, data: u8) {
    asm!("out dx, al", in("dx") port, in("al") data);
}
