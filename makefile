.PHONY: dev-build release clean cov

dev-build:
	cargo test --features=use-libc,use-std
	cargo build --features=use-libc,use-std --examples

release:
	cargo build --release --features=use-libc,use-std --examples

clean:
	cargo clean

cov:
	cargo tarpaulin --features=use-libc,use-std -o Html
