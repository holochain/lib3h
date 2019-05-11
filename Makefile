.PHONY: all test tools fmt clean

all: test

test: tools
	cargo fmt -- --check
	RUST_BACKTRACE=1 cargo test

tools:
	rustup override set nightly-2019-01-24
	rustup component add rustfmt-preview

fmt:
	cargo fmt

clean:
	rm -rf target
