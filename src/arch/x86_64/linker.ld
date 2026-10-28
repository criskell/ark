ENTRY(enter)

PHDRS {
    text PT_LOAD FLAGS(5); /* R-X */
    rodata PT_LOAD FLAGS(4); /* R-- */
    data PT_LOAD FLAGS(6); /* RW- */
}

SECTIONS {
  . = 1M;

  .multiboot : ALIGN(8) {
    KEEP(*(.multiboot))
  } :text

  .text : {
    *(.text .text.*)
  } :text

  .rodata : ALIGN(4K) {
    *(.rodata .rodata.*)
  } :rodata

  .data : ALIGN(4K) {
    *(.data .data.*)
  } :data

  .bss : {
    *(.bss .bss.*)
    *(COMMON)
  } :data
}
