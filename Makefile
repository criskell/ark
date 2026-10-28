arch ?= x86_64
kernel := output/$(arch)/disk/boot/kernel.bin
ld_script := src/arch/$(arch)/linker.ld
output := output/$(arch)
disk := $(output)/disk
kernel_library := target/$(arch)/debug/libark.a
grub_mkrescue ?= $(shell command -v grub-mkrescue || command -v grub2-mkrescue)

assembly_files := $(wildcard src/arch/$(arch)/*.S)
object_files := $(patsubst src/arch/$(arch)/%.S, output/$(arch)/%.o, $(assembly_files))

.PHONY: all build clean run kernel

all: run

kernel:
	cargo build

run: build
	qemu-system-x86_64 -cdrom output/$(arch)/ark.iso \
		-chardev file,id=seabios,path=seabios.log \
		-device isa-debugcon,iobase=0x402,chardev=seabios \
		-m 4G

build: $(kernel)
	@mkdir -p $(disk)/boot/grub
	@cp grub.cfg $(disk)/boot/grub
	@$(grub_mkrescue) -o output/$(arch)/ark.iso $(disk)

clean:
	@rm -rf output

$(kernel): kernel $(object_files) $(ld_script)
	@mkdir -p $(shell dirname $(kernel))
	ld -n -T $(ld_script) -o $(kernel) --gc-sections $(object_files) $(kernel_library)

output/$(arch)/%.o: src/arch/$(arch)/%.S
	@mkdir -p $(shell dirname $@)
	@as --64 $< -o $@
