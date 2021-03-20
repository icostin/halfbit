.PHONY: quick-test dev-build release clean cov install

quick-test: dev-build
	./target/debug/examples/hb -e fourty_two makefile

dev-build:
	cargo test --features=use-libc,use-std
	cargo build --features=use-libc,use-std --examples

release:
	cargo build --release --features=use-libc,use-std --examples

clean:
	cargo clean

cov:
	cargo tarpaulin --features=use-libc,use-std -o Html

install:
	cargo install --path . --examples --features=use-libc,use-std
