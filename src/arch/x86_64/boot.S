.section .multiboot, "a", @progbits
.balign 8
header_start:
  .long 0xE85250D6
  .long 0 # 32-bit protected mode of i386
  .long header_end - header_start
  .long 0x100000000 - (0xE85250D6 + 0 + (header_end - header_start))

  .short 0 # type
  .short 0 # flags
  .long 8
header_end:

.text
.code32

.globl enter
enter:
  movl $stack_top, %esp

  call verify_multiboot
  call verify_cpuid_enabled
  call verify_long_mode

  movl $0x2f692f68, 0xB8000 # 'h', 'i'
  hlt

verify_multiboot:
  cmpl $0x36d76289, %eax
  jne .no_valid_bootloader
  ret

.no_valid_bootloader:
  mov $'0', %al
  jmp display_error

verify_cpuid_enabled:
  pushfl
  popl %eax 
  movl %eax, %ecx # tmp = eflags

  xorl $(1 << 21), %eax # 21º bit = ID. Flip this bit.

  pushl %eax
  popfl

  pushfl
  popl %eax

  push %ecx
  popfl # eflags = tmp

  cmpl %ecx, %eax
  je .no_cpuid
  ret
.no_cpuid:
  mov $'1', %al
  jmp display_error

verify_long_mode:
  movl $0x80000000, %eax # check if has extended processor info
  cpuid

  cmp $0x80000001, %eax # eax = result 1, containing the largest function available.
  jb .no_long_mode

  movl $0x80000001, %eax # "get extended processor info" leaf
  cpuid

  testl $(1 << 29), %edx
  jz .no_long_mode

  ret

.no_long_mode:
  mov $'2', %al
  jmp display_error

display_error:
  movl $0x4F524F45, 0xB8000 # 'E', 'R'
  movl $0x4F4F4F52, 0xB8004 # 'R', 'O'
  movl $0x4F204F52, 0xB8008 # 'R', ' '
  movw $0x4F20, 0xB800C
  movb %al, 0xB800C
  hlt

.bss
stack_bottom:
  .skip 4096

stack_top:
