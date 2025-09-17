use crate::{processor::x86::gdt, screen::vga};
use core::arch::asm;

use crate::println;

fn get_cs_register() -> u16 {
    let cs: u16;

    unsafe {
        asm!("mov {cs:x}, cs", cs = out(reg) cs);
    }

    return cs;
}

fn current_ring() -> u16 {
    return get_cs_register() & 0b11;
}

#[no_mangle]
pub extern "C" fn user_code() {
    vga::VGA_SCREEN.lock().clear_screen();
    println!("Estamos no ring: {}", current_ring());
    loop {}
}

fn enable_io_instructions() {
    unsafe {
        core::arch::asm!(
            "pushfd",
            "pop eax",
            "or eax, {mask}",
            "push eax",
            "popfd",
            mask = const (0b11 << 12), // O campo IOPL está nos bits 12 e 13
            options(nostack, preserves_flags)
        );
    }
}

pub unsafe fn switch_to_ring_3() -> ! {
    // Utilizamos instruções de I/O para mostrar uma mensagem na tela com VGA.
    // No entanto, essas instruções não são permitidas para serem executadas no ring 3.
    // Ao alterarmos o IOPL (I/O privilege) para 3, permitimos código executando em ring 3
    // utilizar portas de I/O.
    // O ideal é criarmos uma API de syscall para mostrar mensagem na tela, mas estamos apenas testando agora
    // a troca para ring 3.
    enable_io_instructions();

    asm!(
        "mov ds, {data_segment_selector:x}",
        "mov es, {data_segment_selector:x}",
        "mov fs, {data_segment_selector:x}",
        "mov gs, {data_segment_selector:x}",
        "mov {tmp:e}, esp",
        "push {data_segment_selector:e}", // SS (DS)
        "push {tmp:e}",                   // Current ESP
        "pushfd",                         // EFLAGS
        "push {code_segment_selector:e}", // CS
        "push {user_code}",
        "iretd",
        tmp = out(reg) _,
        data_segment_selector = in(reg) gdt::USER_DATA_SEGMENT_SELECTOR as u32,
        code_segment_selector = in(reg) gdt::USER_CODE_SEGMENT_SELECTOR as u32,
        user_code = in(reg) user_code
    );

    loop {}
}
