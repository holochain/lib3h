.PHONY: all test fmt clean tools

SHELL = /usr/bin/env sh
RUST_VER_CUR = "$(shell rustc --version)"
RUST_VER_WANT = "rustc 1.33.0-nightly (19f8958f8 2019-01-23)"
RUST_TAG_WANT = "nightly-2019-01-24"

all: test

test: tools
	cargo fmt -- --check
	RUST_BACKTRACE=1 cargo test

fmt: tools
	cargo fmt

clean:
	rm -rf target

tools:
	@if [ ${RUST_VER_CUR} != ${RUST_VER_WANT} ]; then \
		echo "# Makefile # incorrect rust toolchain version"; \
		echo "# Makefile #    got:" ${RUST_VER_CUR}; \
		echo "# Makefile #   want:" ${RUST_VER_WANT}; \
		if rustup --version >/dev/null; then \
			echo "# Makefile # found rustup, setting override"; \
			rustup override set ${RUST_TAG_WANT}; \
			rustup component add rustfmt-preview; \
		else \
			echo "# Makefile # rustup not found, cannot continue"; \
			exit 1; \
		fi \
	else \
		echo "# Makefile # rust toolchain ok:" ${RUST_VER_CUR}; \
	fi
