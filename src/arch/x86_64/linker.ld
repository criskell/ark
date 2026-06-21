ENTRY(enter)

SECTIONS {
  . = 1M;

  .multiboot : ALIGN(8) {
    *(.multiboot)
  }

  .text : {
    *(.text)
  }

  .data : {
    *(.data)
  }

  .rodata : {
    *(.rodata)
  }

  .bss : {
    *(.bss)
  }
}