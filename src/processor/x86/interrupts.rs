use core::arch::asm;

#[inline]
pub fn is_enabled() -> bool {
    let mut eflags: u32;

    unsafe {
        asm!(
          "pushfd",
          "pop {eflags:e}",
          eflags = out(reg) eflags
        );
    }

    (eflags & (1 << 9)) != 0
}

#[inline]
pub fn enable() {
    unsafe {
        asm!("sti", options(nostack, preserves_flags));
    }
}

#[inline]
pub fn disable() {
    unsafe {
        asm!("cli", options(nostack, preserves_flags));
    }
}

#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let enabled = false;

    if enabled {
        disable();
    }

    let result = f();

    if enabled {
        disable();
    }

    result
}
