# InferaDB Rust SDK - Development Commands

## Quick Reference

| Command | Description |
|---------|-------------|
| `cargo build --workspace --all-features` | Build the project with all features |
| `cargo test --lib` | Run unit tests |
| `cargo +nightly fmt --all` | Format code (requires nightly) |
| `cargo clippy --workspace --all-targets -- -D warnings` | Run clippy linting |
| `cargo doc --workspace --no-deps --open` | Build and open documentation |

## Setup (One-Time)

```bash
mise trust && mise install
rustup component add rustfmt clippy
rustup toolchain install nightly --component rustfmt
```

## Build Commands

```bash
# Build all workspace crates with all features
cargo build --workspace --all-features
```

## Test Commands

```bash
# Unit tests only
cargo test --lib

# Integration tests (requires local InferaDB via inferadb/deploy)
cargo test --test integration

# All tests
cargo test --lib --test integration
```

## Code Quality

```bash
# Format code (requires nightly toolchain)
cargo +nightly fmt --all

# Check formatting without modifying
cargo +nightly fmt --all -- --check

# Run clippy linter
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentation

```bash
# Build docs
cargo doc --workspace --no-deps

# Build and open in browser
cargo doc --workspace --no-deps --open

# Check docs for warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

## Coverage

```bash
# Run tests with coverage
cargo llvm-cov --lib --ignore-filename-regex 'proto|inferadb\.authorization\.v1'

# Generate HTML coverage report
cargo llvm-cov --lib --ignore-filename-regex 'proto|inferadb\.authorization\.v1' --html
```

## Code Generation

```bash
# Regenerate protobuf code from proto/inferadb.proto
rm -f src/transport/proto/inferadb.authorization.v1.rs
cargo build --features grpc
cargo +nightly fmt --all
```

## Running Examples

```bash
cargo run -p inferadb-examples --bin basic_check
cargo run -p inferadb-examples --bin batch_operations
cargo run -p inferadb-examples --bin axum_middleware
```

## System Utilities (Darwin/macOS)

Standard Unix utilities available: `git`, `ls`, `cd`, `grep`, `find`
Note: Some GNU flags may not be available on macOS (BSD versions).
