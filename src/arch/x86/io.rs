use core::arch::asm;

#[inline(always)]
pub unsafe fn outportb(port: u16, data: u8) {
    asm!("out dx, al", in("dx") port, in("al") data);
}

#[inline(always)]
pub unsafe fn outportl(port: u16, data: u32) {
    asm!("out dx, al", in("dx") port, in("eax") data);
}

#[inline(always)]
pub unsafe fn inportb(port: u16) -> u8 {
    let mut data: u8;

    asm!("in al, dx", in("dx") port, out("al") data);

    return data;
}
