pub mod gdt;
pub mod io;
pub mod ring3;

use core::arch::asm;

pub fn wait_for_interrupt() {
    // SAFETY: just pauses the cpu until an interrupt arrives
    unsafe {
        asm!("hlt", options(preserves_flags, nostack, nomem));
    };
}

pub fn idle_forever() -> ! {
    loop {
        wait_for_interrupt();
    }
}

pub fn halt() -> ! {
    // SAFETY: nothing will ever run again!
    unsafe { asm!("cli", options(nomem, nostack)) };

    loop {
        // that should never hapen hahahah
        wait_for_interrupt();
    }
}
