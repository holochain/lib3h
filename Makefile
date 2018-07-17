all: test

.PHONY: env lint test

env:
	(which rustup || curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y)
	rustup toolchain install nightly
	rustup default nightly
	rustup update
	rustup component add rustfmt-preview --toolchain nightly
	rustup component add clippy-preview --toolchain nightly

lint:
	cargo +nightly fmt
	cargo +nightly clippy -- -A needless_return
	cargo +nightly build --verbose --all

test: lint
	cargo +nightly test --verbose --all
