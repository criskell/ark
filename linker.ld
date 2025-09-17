ENTRY(initialize)

SECTIONS {
  . = 1M;

  .multiboot : {
    *(.multiboot)
    . = ALIGN(4K);
  }

  .stack : {
    . = ALIGN(16);
    stack_bottom = .;
    . += 64K;
    stack_top = .;
  }

  .rodata : {
    *(.rodata*)
    . = ALIGN(4K);
  }
  
  .text : {
    *(.text*)
    . = ALIGN(4K);
  }

  .bss : {
    *(.bss*) *(COMMON)
    . = ALIGN(4K);
  }

  
  /DISCARD/ : {
    *(.eh_frame)
  }
}