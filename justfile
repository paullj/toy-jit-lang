[private]
default:
    @just help

help:
    @just --list --unsorted

# Run the CLI in development mode
dev *args:
    @cargo run --package cli -- {{args}}

# Run the CLI in release mode
run *args:
    @cargo run --release --package cli -- {{args}}

# Run the tests
test:
    @cargo llvm-cov nextest

# Check lint issues
lint:
    @cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
format:
    @cargo fmt --all -- --check
