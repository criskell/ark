.PHONY: run

run:
	RUSTFLAGS='-C link-args=-Tlinker.ld' cargo +nightly build --target i686.json && qemu-system-i386 -kernel target/i686/debug/ark