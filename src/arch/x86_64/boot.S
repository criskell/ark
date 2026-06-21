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
  call configure_paging_tables
  call enable_paging

  # 32-bit compatibility submode now

  lgdt gdtr

  # This is necessary because it is loading a 32-bit code selector.
  ljmp $kernel_code_segment_offset, $long_mode_enter

  # Now is full long mode

enable_paging:
  movl $p4_table, %eax
  movl %eax, %cr3

  # Enable PAE (Physical Address Extension)
  movl %cr4, %eax
  orl $(1 << 5), %eax
  movl %eax, %cr4

  # Enable long mode bit in EFER (Extended Feature Enable Register)
  movl $0xC0000080, %ecx # EFER register is at 0xC0000080 address
  rdmsr # Result = %eax
  orl $(1 << 8), %eax
  wrmsr

  # Enable paging in CR0
  movl %cr0, %eax
  orl $(1 << 31), %eax
  movl %eax, %cr0

  ret

configure_paging_tables:
  movl $p3_table, %eax
  orl $0b11, %eax # bit 0 = present, bit 1 = writable
  movl %eax, p4_table

  movl $p2_table, %eax
  orl $0b11, %eax
  movl %eax, p3_table

  movl $0, %ecx

.map_p2_table_entry:
  movl $0x200000, %eax # 2MiB
  mull %ecx # EDX:EAX *= EAX * ECX
  orl $0b10000011, %eax
  movl %eax, p2_table(, %ecx, 8)

  incl %ecx
  cmpl $512, %ecx
  jne .map_p2_table_entry

  ret

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
# Page tables should be aligned to 4KiB because bits 0-11 are used for flags. Placing page tables at beginning of the section
# allows the linker to align the entire section without adding unused padding.
.balign 4096

p4_table: # PML4
  .skip 4096

p3_table: # PDP
  .skip 4096

p2_table: # PD
  .skip 4096

p1_table:
  .skip 4096

stack_bottom:
  .skip 64

stack_top:

.section .rodata
gdt:
  .quad 0

.equ kernel_code_segment_offset, . - gdt
  # Left to right: Executable, Descriptor type (1 for code and data segments), Present, 64-bit flag  
  .quad (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)
  # Data segment is not necessary in 64 bit mode

gdtr:
  .word . - gdt - 1
  .quad gdt

.code64
long_mode_enter:
  movq $0x2F4E2F452F522F45, %rax
  movq %rax, 0xB8000
  hlt