ENTRY(_start)

SECTIONS {
  . = 1M;

  .multiboot : { *(.multiboot) }
  .text : { *(.text*) }
  .rodata : { *(.rodata*) }
  .bss : { *(.bss*) *(COMMON) }
  /DISCARD/ : { *(.eh_frame) }
}