# Makefile - mNFTS build orchestration

.PHONY: all build build-rust build-swift clean test test-rust test-swift lint

RUST_TARGET_DIR := target
SWIFT_BUILD_DIR := swift/.build

all: build

build: build-rust build-swift

build-rust:
	cargo build --release

build-swift: build-rust
	cd swift && swift build -c release

clean:
	cargo clean
	cd swift && swift package clean

test: test-rust test-swift

test-rust:
	cargo test

test-swift: build-rust
	cd swift && swift test

lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

# Install CLI binary to /usr/local/bin
install: build
	cp swift/.build/release/mnfts /usr/local/bin/mnfts

uninstall:
	rm -f /usr/local/bin/mnfts
