arch ?= x86_64
kernel := output/$(arch)/disk/boot/kernel.bin
ld_script := src/arch/$(arch)/linker.ld
output := output/$(arch)
disk := $(output)/disk
kernel_library := target/$(arch)/debug/libark.a

assembly_files := $(wildcard src/arch/$(arch)/*.S)
object_files := $(patsubst src/arch/$(arch)/%.S, output/$(arch)/%.o, $(assembly_files))

.PHONY: all build clean run kernel

all: run

kernel:
	cargo build

run: build
	qemu-system-x86_64 -cdrom output/$(arch)/ark.iso

build: $(kernel)
	@mkdir -p $(disk)/boot/grub
	@cp grub.cfg $(disk)/boot/grub	
	@grub-mkrescue -o output/$(arch)/ark.iso $(disk)

clean:
	@rm -rf output

$(kernel): kernel $(object_files) $(ld_script)
	@mkdir -p $(shell dirname $(kernel))
	ld -n -T $(ld_script) -o $(kernel) --gc-sections $(object_files) $(kernel_library)

output/$(arch)/%.o: src/arch/$(arch)/%.S 
	@mkdir -p $(shell dirname $@)
	@as --64 $< -o $@
