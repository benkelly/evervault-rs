.PHONY: help build release run test check lint fmt fmt-check clean install

help:
	@echo "evervault-rs — common tasks"
	@echo ""
	@echo "  make build       Build the debug binary"
	@echo "  make release     Build the optimized release binary"
	@echo "  make run         Run a sample round-trip (needs .env)"
	@echo "  make test        Run cargo test"
	@echo "  make check       Run cargo check"
	@echo "  make lint        Run clippy with warnings as errors"
	@echo "  make fmt         Format the codebase with rustfmt"
	@echo "  make fmt-check   Verify formatting without writing"
	@echo "  make clean       Remove build artifacts"
	@echo "  make install     Install the binary into ~/.cargo/bin"

build:
	cargo build

release:
	cargo build --release

run:
	cargo run -- --field card_number --value 4242424242424242

test:
	cargo test

check:
	cargo check

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clean:
	cargo clean

install:
	cargo install --path .
