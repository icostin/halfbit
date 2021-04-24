.PHONY: quick-test dev-build release clean cov install inc-ver

quick-test: dev-build
	./target/debug/examples/hb -e fourty_two makefile

dev-build:
	cargo +stable test --features=use-libc,use-std
	cargo +stable build --features=use-libc,use-std --examples
	cargo +nightly test --features=use-libc,use-std,nightly
	cargo +nightly build --features=use-libc,use-std,nightly --examples

release:
	cargo +nightly build --release --features=use-libc,use-std,nightly --examples

clean:
	cargo clean

cov:
	cargo tarpaulin --features=use-libc,use-std -o Html

install:
	cargo +nightly install --path . --examples --features=use-libc,use-std

inc-ver:
	sed -i -E 's/version = .([0-9]+.[0-9]+.)([0-9]+).*/echo "version = \\\"\1$$((\2 + 1))\\\""/e' Cargo.toml

