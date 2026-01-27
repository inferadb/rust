# InferaDB Rust SDK - Development Commands
# Run `just --list` to see all available recipes

# Default recipe: run all checks
default: check

# Run all checks (format, lint, test)
check: fmt-check lint test

# Format code (requires nightly)
fmt:
    cargo +nightly fmt --all

# Check formatting without modifying files
fmt-check:
    cargo +nightly fmt --all -- --check

# Run clippy lints
lint:
    cargo +1.92 clippy --all-targets -- -D warnings

# Run clippy on minimal feature set
lint-minimal:
    cargo +1.92 clippy --no-default-features -- -D warnings

# Run tests
test:
    cargo +1.92 test --lib

# Run tests with all features
test-all:
    cargo +1.92 test --all-features

# Run doc tests
test-doc:
    cargo +1.92 test --doc --all-features

# Build the project
build:
    cargo +1.92 build

# Build with all features
build-all:
    cargo +1.92 build --all-features

# Check for unused dependencies (requires nightly)
udeps:
    cargo +nightly udeps --workspace --all-features

# Build documentation
doc:
    cargo +1.92 doc --all-features --no-deps

# Open documentation in browser
doc-open:
    cargo +1.92 doc --all-features --no-deps --open

# Run code coverage
coverage:
    cargo +1.92 llvm-cov --all-features --html

# Clean build artifacts
clean:
    cargo clean
