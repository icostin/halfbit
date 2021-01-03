.PHONY: dev-build release clean

dev-build:
	cargo test --features=use-libc,use-std
	cargo build --features=use-libc,use-std --examples

release:
	cargo build --release --features=use-libc,use-std --examples

clean:
	cargo clean
