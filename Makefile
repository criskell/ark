.PHONY: run

run:
	RUSTFLAGS='-C link-args=-Tlinker.ld' cargo +nightly build --target i686.json && \
		qemu-system-i386 -kernel target/i686/debug/ark \
			-no-reboot -no-shutdown \
			-s \
			-serial stdio \
			-device \
			isa-debug-exit,iobase=0xf4,iosize=0x04 \
			-d int,cpu_reset,guest_errors \
			-monitor tcp:127.0.0.1:4321,server,nowait
	# open QEMU monitor with `telnet 127.0.0.1 4321`